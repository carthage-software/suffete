# SIMD scans

The `element::simd` module provides SIMD-accelerated scans over `&[ElementId]` slices. Six primitives, all of them dispatch through scalar fallback / AVX2 (x86_64) / NEON (aarch64), with hand-tuned inline assembly on the AVX2 path.

This is the lowest level of the crate. The lattice and predicate code consume these primitives; the user-facing API is unaware of them.

## The primitives

| Function | Returns | Used by |
|---|---|---|
| `contains(slice, needle)` | `bool` ; needle anywhere in slice | join's canonicalize, predicates |
| `position_of(slice, needle)` | `Option<usize>` ; first index of needle | TypeBuilder::remove, TypeBuilder::replace |
| `any_of_kind(slice, kind)` | `bool` ; some Element has this kind | predicates::contains_X, lattice prefilters |
| `all_of_kind(slice, kind)` | `bool` ; every Element has this kind (false on empty) | predicates::is_X |
| `count_of_kind(slice, kind)` | `usize` ; count of Elements with this kind | join's literal-collapse threshold |
| `is_sorted_strict(slice)` | `bool` ; slice is strictly increasing under unsigned u32 order | interner's canonicality fast path |

The dispatch:

- **`x86_64` + AVX2** (runtime-detected via `std::is_x86_feature_detected`): 8 lanes (256 bits) per iteration. Hand-rolled inline assembly using FFmpeg's pointer-end trickery (one register holds a negative byte offset that doubles as both the load displacement and the loop counter).
- **`aarch64` + NEON** (baseline ISA, no runtime check): 4 lanes (128 bits) per iteration. NEON intrinsics.
- **Other architectures or short slices**: tight scalar loop.

The thresholds are 8 lanes for AVX2 and 4 lanes for NEON. Below that, scalar wins because the SIMD setup outweighs the parallel work.

## Why ElementId-as-u32 is a perfect fit

`ElementId` is a `NonZeroU32` ([layout chapter](./element-id-layout.md)). A slice of `ElementId` is contiguous 32-bit lanes. Equality scans (`contains`, `position_of`) and kind scans (`any_of_kind`, `all_of_kind`, `count_of_kind`) reduce to a per-chunk:

1. Load 8 (AVX2) or 4 (NEON) lanes into a SIMD register.
2. (For kind scans) Right-shift by 26 to extract the kind tag.
3. Compare-equal against a broadcasted needle (or kind value).
4. Reduce to a scalar via movemask (AVX2) or maxv/minv (NEON).
5. Branch on the reduction.

The per-chunk cost is constant; the per-iteration overhead is one load, two-to-three SIMD ops, one branch.

## The FFmpeg pointer-end trick

The AVX2 paths use a register-saving technique borrowed from FFmpeg. Instead of:

```asm
mov  rcx, 0
.loop:
  vmovdqu  ymm1, [rsi + rcx*4]
  ; ... compare ...
  inc  rcx
  cmp  rcx, r8        ; r8 = chunk count
  jb   .loop
```

The trick is:

```asm
mov  rdi, [end of slice]   ; pointer to end-of-chunked-region
mov  rcx, -bytes           ; negative byte offset
.loop:
  vmovdqu  ymm1, [rdi + rcx]
  ; ... compare ...
  add  rcx, 32             ; byte stride = 32 (8 lanes × 4)
  jl   .loop
```

The negative offset doubles as the loop counter and the addressing-mode displacement. The loop tail is `add + jl` (two instructions, one micro-op fusion) instead of `inc + cmp + jb` (three instructions).

The savings are small per iteration but real, especially on long slices.

## NEON: intrinsics, not assembly

NEON lacks a per-lane movemask instruction; the scalar reduction is done with `vmaxvq_u32` (max across lanes) or `vminvq_u32` (min across lanes) plus a per-element-position bit-set trick. The intrinsics version is fast enough that hand-rolled assembly doesn't help.

NEON is *baseline* on AArch64 (every aarch64 CPU has it). No runtime detection needed; the SIMD path runs whenever the slice is long enough.

## Where the primitives are called from

The SIMD primitives are called by hot paths the lattice traverses:

- `lattice::overlaps` and `lattice::refines` use `simd::any_of_kind` as a prefilter for the `Negated`, `Intersected`, `Mixed`, `Object`, etc. families ; "if no Element has this kind, skip the family rule entirely".
- `lattice::join`'s canonicalisation uses `simd::contains` to detect well-known dominators (`MIXED`, `NEVER`, `BOOL`, `RESOURCE`, etc.) before applying the rule.
- `predicates::is_int`, `is_string`, etc. use `simd::all_of_kind` as their core.
- `predicates::contains_int`, `contains_string`, etc. use `simd::any_of_kind`.
- `intern_type`'s slow path uses `simd::is_sorted_strict` to skip the sort + dedup when the input is already canonical.
- `TypeBuilder::remove` and `TypeBuilder::replace` use `simd::position_of`.

## When the SIMD threshold isn't met

Most analyser-side unions are small (1-5 Elements). The threshold gates ensure that the SIMD code only runs when there's enough work to pay back the setup. On short slices, the scalar fallback runs ; LLVM autovectorises what it can.

## Why hand-rolled assembly

The autovectoriser generates correct SIMD code, but it doesn't know:

- That the loop bound is the chunk count (not the byte count).
- That the broadcast can be done once outside the loop.
- That `vpcmpeqd + vpmovmskb + test + jnz` is a tighter early-exit than the equivalent generated code.
- That the FFmpeg pointer-end trick saves one micro-op per iteration.

For the AVX2 paths, hand-rolled assembly is consistently 10-30% faster than the autovectorised scalar on long slices. For the NEON paths, intrinsics-with-careful-loop are within 5% of hand-rolled assembly, and the maintenance is much lower.

## Safety

Every SIMD function is marked `unsafe fn`. The public entry points are safe and gate on the threshold + the runtime feature detection (for AVX2). Inside the SIMD function:

- Unaligned loads (`vmovdqu`, `vld1q_u32`) — both architectures support these without alignment.
- Bounds: the function is called only when the slice is at least `THRESHOLD` lanes; the chunk count is `len / lanes_per_chunk`, capped to fit.
- Tail handling: each function has a scalar tail loop for the leftover lanes after the chunked region.

The unsafe blocks are local to each function and have SAFETY comments documenting the invariants.

## Performance numbers

Approximate, on a modern x86_64 desktop, for a slice of 64 Elements:

- `simd::contains` (hit early): ~5ns.
- `simd::contains` (miss to end): ~20ns.
- `simd::any_of_kind` (hit early): ~6ns.
- `simd::any_of_kind` (miss to end): ~25ns.
- `simd::is_sorted_strict` (already sorted, 64 elements): ~30ns.
- Scalar equivalent (64 elements): ~3-5x slower across the board.

For typical analyser unions (5-10 Elements), the scalar fallback runs and the cost is on the order of the per-Element work itself ; nanoseconds per call.

> **See also:** [The ElementId tag layout](./element-id-layout.md), [Interning and the arenas](./interner.md), [Performance philosophy](./performance.md).
