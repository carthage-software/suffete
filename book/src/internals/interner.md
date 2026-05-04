# Interning and the arenas

Every interesting value in suffete — an Element, a Type, an element list, a known-items list, a defining entity, a callable signature — is **interned**. This chapter covers how interning works and why suffete leans on it for both correctness and performance.

This is internals reading. The public API works without understanding it. But if you're optimising analyser-side code or debugging an arena-related issue, this chapter is the one.

## What interning gives you

Two `ElementId`s with the same logical content are *the same handle*. Equality is `u32 == u32`. Hashing is `u32 as u64`. Two parts of a process that construct the same Element independently end up with the same handle and never know they raced.

Three consequences:

1. **Cheap equality.** No structural compare, no recursion, no hash chain walk. One CPU compare.
2. **Stable identity.** A `TypeId` constructed today is the same handle as one constructed tomorrow (same process). Useful for caching.
3. **Compact representation.** Every Element is one `u32`. A `TypeId` is one `u64`. A union of `n` Elements is one `u64` plus a borrowed slice of `n` `u32`s.

## What interning costs

Interning has a fixed memory cost: the dedup table. Suffete uses `dashmap::DashMap` for the dedup, which is concurrent-friendly but not zero-cost. The lookup is a hash of the value plus a bucket walk.

For a content-keyed value, the hash is over the content (the kind tag plus the payload). For a slice-keyed value, the hash is over the slice contents.

The intern operation:

1. Hash the input.
2. Probe the dashmap.
3. On a hit, return the existing slot.
4. On a miss, allocate the value, push it into the per-kind arena, return the new slot.

Steps 2-3 dominate: the hash is fast, the dashmap lookup is the cost.

## The arena types

Two basic shapes:

- **`Arena<T>`** — interns whole values of `T`. One slot per unique `T`. Used for: per-element-kind payloads, defining entities, callable signatures, known-items lists, etc.
- **`SliceArena<T>`** — interns slices of `T`. One slot per unique slice content. The slice is leaked to `'static` on first sight. Used for: the union body slice, object type-args, derived type lists, known-property lists, etc.

Both store underlying values in a `boxcar::Vec` (a thread-safe append-only vector). Both expose 1-based slot indices ; slot 0 is reserved as the `NonZero{U32,U64}` niche.

## The interner singleton

There is exactly one interner per process. It contains every arena, lives behind a `OnceLock` for the process lifetime, and is `Sync` ; concurrent use from multiple threads is supported and uncontended in the common case (the dashmap shards by hash).

## Per-element-kind arenas

Each payload-bearing Element kind gets its own arena. The interner exposes per-kind intern + lookup methods. Trivial-kind elements (no payload) don't need interning ; their handles are constructed directly from the kind tag and the canonical slot, and the prelude exposes them as constants.

## The Type interner

The Type arena is keyed on the content slice plus the flags. Two Types with the same elements (in any order) and the same flags are the same `TypeId`.

Type interning does:

1. Sort and dedup the input slice (canonicalisation).
2. Intern the sorted slice into the element-list slice arena to get a stable handle.
3. Intern the `(slice_handle, flags)` pair into the Type arena to get a slot.
4. Construct the `TypeId` from `(slot, flags, meta=0, reserved=0)`.

The sort+dedup step uses `sort_unstable + dedup`, with a SIMD prefilter that skips both when the input is already canonical. Most analyser-side calls hit the fast path because the join layer produces canonical output.

A singleton-Type cache is layered on top: interning a one-element slice is a hashmap lookup keyed on the single Element ID, skipping the sort + the dashmap hash entirely.

## Memory growth

Arenas only grow. There is no GC, no reference counting, no compaction. The trade-off:

- **Pro**: every handle is stable for the process lifetime; no use-after-free; no churn.
- **Con**: a long-running analyser session that processes many distinct types accumulates them all.

In practice, the analyser's working set saturates: most types are constructed once and reused many times, and the arena converges. Mago's expected memory budget for the type arenas is in the tens of megabytes for a large monorepo.

If the analyser truly needs per-session deallocation (e.g. an LSP server processing many distinct codebases), the recommendation is to spawn a fresh process per session. Suffete does not support arena resets within a process.

## Concurrency

The dashmap is the concurrency point. Two threads interning the same value race on the dashmap entry; whichever wins inserts, the other reads back the winner's slot. Both end up with the same handle.

The arenas themselves (boxcar) are append-only and lock-free for read; writes synchronise minimally. Two threads pushing different values append independently.

## Performance numbers

Approximate, on a modern x86_64 desktop:

- **Trivial-kind handle access**: zero ; it's a `const`.
- **Cache-hit intern**: ~30ns.
- **Cache-miss intern** (first time this payload is seen): ~100ns plus the cost of the boxed clone if the content is fresh.
- **Singleton-Type cache hit**: ~10ns.
- **Slow-path Type intern**: ~100-300ns depending on slice length.
- **Payload lookup by handle**: ~5ns; one boxcar index plus a deref.

The lattice's hot loops are dominated by the family rules, not interning. Interning is the constant-cost overhead.

> **See also:** [The ElementId tag layout](./element-id-layout.md), [SIMD scans](./simd.md), [Performance philosophy](./performance.md).
