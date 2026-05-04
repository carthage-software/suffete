# The ElementId tag layout

`ElementId` is a `NonZeroU32` newtype. The 32 bits are split into two fields:

```text
bit  31 30 29 28 27 26 | 25 24 23 ... 2 1 0
     [   kind tag    ] | [   arena slot   ]
       6 bits             26 bits
```

| Field | Bits | Width | Range |
|---|---|---|---|
| **Kind tag** | 31..26 | 6 | 1..=63 (64 values reserved; 30+ used) |
| **Arena slot** | 25..0 | 26 | 0..=2^26-1 ≈ 67 million |

The kind tag is 1-based: tag 0 is reserved as the `NonZeroU32` niche, so `ElementId(0)` is impossible and `Option<ElementId>` is the same size as `ElementId`.

## Why this layout

Three reasons:

1. **One-shot dispatch.** A `kind()` call is a right-shift by 26 plus a cast through `ElementKind`. No memory access, no branch.
2. **Compact `ElementId`.** 32 bits per Element handle means the union body for an `n`-element type takes `4n` bytes ; a hot cache line fits 16 Elements.
3. **SIMD-friendly.** A slice of `ElementId` is a slice of `u32`. AVX2 processes 8 lanes per 256-bit register; NEON processes 4 per 128-bit. The kind extraction is a vectorised right-shift; the equality scan is a vectorised compare. See [SIMD scans](./simd.md).

## Why 6 bits for the kind

PHP's type system has roughly 30 distinct Element kinds. 6 bits gives 63 usable values (excluding the niche), which leaves room for growth. 5 bits would have been tight; 7 would have wasted a bit.

## Why 26 bits for the slot

26 bits gives 67 million slots per kind. The largest arena in any real-world analyser is the `Object` arena (every distinct `(name, type_args, flags)` tuple is one entry), which tops out in the millions on the largest codebases. 67 million is far beyond what any analyser is expected to need.

The slot space is *per-kind*, so two Elements with the same arena-slot index but different kinds are still distinct `ElementId`s.

## How `kind()` works

```rust,ignore
impl ElementId {
    const KIND_BITS: u32 = 6;
    pub(super) const SLOT_BITS: u32 = u32::BITS - Self::KIND_BITS;  // 26
    const SLOT_MASK: u32 = (1u32 << Self::SLOT_BITS) - 1;            // 0x03FF_FFFF

    pub const fn kind(self) -> ElementKind {
        let tag = (self.0.get() >> Self::SLOT_BITS) as u8;
        // SAFETY: every ElementId is constructed from a valid ElementKind.
        unsafe { core::mem::transmute(tag) }
    }
}
```

The `transmute` is sound because every `ElementId` is constructed via `ElementId::new(kind, slot)`, which sets the high bits to `kind as u8 << 26`. Tag values that don't correspond to an `ElementKind` discriminant cannot occur.

## Why the kind tag is in the *high* bits

Two reasons:

- **Sort order matches kind order.** Sorting a `&[ElementId]` by raw `u32` value clusters Elements by kind first, then by slot within kind. The interner relies on this for fast canonical-form detection (the `simd::is_sorted_strict` primitive checks adjacent strict ordering).
- **SIMD shift access.** AVX2's `vpsrld` (packed shift right logical, dword) takes an immediate count; right-shifting 8 lanes by 26 in one instruction extracts the kind tag from each lane simultaneously.

If the tag were in the low bits, a left-shift would be needed to compare against a `ElementKind` value, but the resulting compare would still work; the high-bit choice is a small win for the SIMD path.

## The `NonZero` niche

`NonZeroU32` requires the value to be non-zero. `ElementId(0)` is therefore impossible, which lets `Option<ElementId>` be 32 bits (the niche stores `None` as the all-zero pattern).

The kind tag is 1-based: `ElementKind::Null` is discriminant 1, not 0. Since the tag occupies the high 6 bits, the all-zero `u32` would correspond to "kind 0, slot 0", which is never a valid Element ; the niche is naturally available.

## TypeId is similar but bigger

`TypeId` is `NonZeroU64`, with this layout:

```text
bit 63 ... 32 | 31 ... 16 | 15 ... 8 | 7 ... 0
[ slot: 32 ]  | [flags:16] | [meta:8] | [reserved: 8]
```

- **slot**: 32 bits ; index into the type-content arena.
- **flags**: 16 bits ; `FlowFlags` bitset.
- **meta**: 8 bits ; consumer-defined.
- **reserved**: 8 bits ; always zero.

Flags and meta ride on the handle (they don't change the slot), so toggling a flag is bit-twiddling, not a re-intern.

## A subtle case: `TypeId` equality

Two `TypeId`s compare equal iff *all 64 bits* match. That includes flags and meta. To compare *just content*: `t1.content_eq(&t2)` ignores flags and meta. The full `==` is for "are these the exact same handle for use in caches keyed on the full handle".

> **See also:** [Interning and the arenas](./interner.md), [SIMD scans](./simd.md).
