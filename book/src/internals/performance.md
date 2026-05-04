# Performance philosophy

Suffete is a hot path. An analyser checking a large codebase calls `refines`, `overlaps`, `meet`, `join`, `subtract`, `narrow` and the predicates millions of times. A constant-factor regression in any of those shows up as a percentage of total analysis time.

This chapter is the philosophy. Specific optimisations are in the [interner](./interner.md), [element-id layout](./element-id-layout.md), and [SIMD](./simd.md) chapters.

## The principles

### 1. Performance is a feature, not an afterthought

Performance regressions stop landing in suffete the same way correctness regressions stop. Every PR runs the codspeed benchmark suite; a regression beyond a small threshold blocks the merge. The benchmarks cover the lattice operations on a representative mix of inputs (small unions, large unions, deep generics, intersections, narrowed mixed).

This is not "ship and optimise later". The cost model is part of the design.

### 2. Compact representation beats clever rules

Most of suffete's speed comes from:

- `ElementId` is a `u32`. Equality is one compare.
- `TypeId` is a `u64`. Equality is one compare.
- Slices of `ElementId` are dense u32 arrays. SIMD-friendly.

The lattice rules themselves are not particularly clever ; they are the textbook subtype rules. The speed comes from the data being amenable to fast comparison and SIMD scans, not from algorithmic tricks in the rules.

### 3. Intern aggressively, free nothing

The interner never frees. Every `ElementId` and `TypeId` is `'static`. The trade-off is committed: stable handles in exchange for accumulated memory.

In practice, the analyser's working set saturates. After ingesting a codebase, the arena is "warmed" and most subsequent queries hit existing entries. A long-running session converges; it does not grow without bound except as new types appear.

For sessions that need true per-context isolation (LSP servers across distinct codebases), the recommendation is process-per-session rather than arena-reset.

### 4. Pure functions, no global mutable state

Every operation is a pure function of its inputs (`TypeId`s, `&World`, `LatticeOptions`). Same inputs → same output. No internal state.

This buys:

- **Concurrency.** The lattice can be called from any thread without coordination.
- **Memoisation.** Callers can cache results keyed on the input handles, without worrying about invalidation.
- **Testability.** Property tests work because the operations are deterministic. The algebraic-law battery checks identities that only hold for pure functions.

The single piece of mutable state that exists — the interner — is concurrent-safe and append-only. Reads see a consistent snapshot.

### 5. Hot paths are tuned, cold paths are clear

The hot paths (the lattice operations, the SIMD primitives, the interner's lookup) are optimised aggressively: hand-rolled assembly, careful branch ordering, prefilter gates, inline annotations. The cold paths (the construction APIs, the diagnostic helpers, the serialize layer) prioritise clarity over speed.

The split is informed by codspeed. A function that doesn't show up in the benchmarks gets readable code; a function that does gets the optimisation effort.

### 6. Codspeed is the source of truth

We do not benchmark locally. Local microbenchmarks vary by CPU, OS, background load, and disk cache. The numbers don't transfer.

Codspeed runs every PR on a controlled VM and reports relative deltas to the base commit. A "5% regression" means 5% on Codspeed's hardware, which is not your hardware, but is *the same hardware* as the previous run. The signal is reproducible.

Locally we ship hunches and let codspeed vote. If a change is hopefully an improvement, we push and look at the benchmark output. If codspeed flags it as a regression, we revert; if it's a win, we keep.

### 7. SIMD is for the things SIMD is good at

SIMD wins when the data is naturally u32 or u64 lanes, the operation is per-lane independent (compare-equal, shift-extract, etc.), and the slice is long enough to amortise the setup. SIMD does not win when the data is heterogeneous (each Element triggers a different per-kind branch), the work per Element is heavyweight (a recursive `refines` call cannot be vectorised), or the slice is short.

Suffete uses SIMD for the slice scans and as prefilters in the lattice (does this slice contain any `Negated` Element? gate the family rule on the answer). The lattice's per-pair work is *not* SIMD; it can't be.

### 8. Allocation is a tax

Every allocation is a roundtrip to the allocator. The lattice's hot path is allocation-free: handles are passed by value, the report is mutated in-place, the workspace lives on the stack.

Callers should follow the same: reuse `LatticeReport` instances across queries, reuse `TypeBuilder` instances when constructing many types, prefer single-element prelude constants over building one-element types from scratch.

### 9. Branches are a tax too

Modern CPUs predict branches well, but predictable branches are a constant cost; mispredicted branches are 10-20× worse. Suffete's hot loops minimise both:

- Family dispatch in the lattice is a `match` on `(a.kind(), b.kind())`, which the compiler turns into a jump table for the common case.
- The universal axioms (top, bot, placeholder) are checked first; once they fire, the rest of the dispatch is dead code.
- The SIMD scans use early-exit: on a match, branch out of the loop; on a miss, branch back to the loop start. The loop-back is well-predicted.

### 10. The lattice is not the only thing that matters

Suffete is one piece of an analyser's pipeline. The analyser's parser, the docblock interpreter, the symbol table, the codebase ingestion, the diagnostic emitter — all of these have their own performance budgets. Suffete's job is to be one constant-factor cost in that pipeline, not to dominate it.

If suffete becomes the bottleneck of a real analyser, that's a bug. The expected cost of suffete in a representative analysis is measured in low single-digit percent of total analyser time.

## What doesn't matter

A few things that *seem* like they should matter, but don't:

- **The exact number of cycles per refines call.** It varies by case. The codspeed delta is what matters.
- **Avoiding small allocations in cold paths.** `Vec::new` is cheap. Prematurely optimising it makes the code worse and saves nothing.
- **Inlining everything.** `#[inline]` helps when LLVM doesn't see the benefit; it hurts when overused (code bloat, instruction cache pressure). Suffete uses `#[inline]` for the small-and-hot functions and lets LLVM decide for the rest.
- **Lock-free data structures everywhere.** The interner uses dashmap because it's a concurrent hashmap, not because every read needs to be lock-free. The lattice's hot path doesn't take locks at all.

## When in doubt

Ship it. Look at codspeed. Revert if it regresses; keep it if it doesn't. Don't theorise about performance ; measure.

> **See also:** [Interning and the arenas](./interner.md), [The ElementId tag layout](./element-id-layout.md), [SIMD scans](./simd.md).
