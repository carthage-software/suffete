//! SIMD-accelerated scans over `&[ElementId]` slices.
//!
//! `ElementId` is a `NonZeroU32` newtype with the [`ElementKind`] tag in
//! the high 6 bits and the per-kind arena slot in the low 26 bits.
//! Slices of `ElementId` are therefore contiguous 32-bit lanes ; ideal
//! substrate for the wide-element-set scans the lattice and join layers
//! perform on every operation.
//!
//! Six primitives:
//!
//! - [`contains`]: equality scan ; "is `needle` anywhere in `slice`?"
//! - [`position_of`]: equality scan returning the first matching index
//!   (`None` if absent). Used by the build-buffer's `remove` / `replace`
//!   paths that need the index to splice in place.
//! - [`any_of_kind`]: kind-only scan ; "does any element have kind
//!   `kind`?"
//! - [`all_of_kind`]: kind-only scan ; "do all elements share kind
//!   `kind`?"
//! - [`count_of_kind`]: counts elements of a given kind, useful for
//!   the literal-collapse threshold checks in `join`.
//! - [`is_sorted_strict`]: adjacent-pair comparison ; "is the slice
//!   strictly increasing (sorted + unique) under unsigned u32 order?"
//!   The interner uses this as a fast-path gate: callers that already
//!   produce canonical output (the join layer, most lattice rules) can
//!   skip the `sort_unstable + dedup` inside `intern_type`.
//!
//! ## Implementation
//!
//! Two SIMD code paths plus a scalar fallback:
//!
//! - **`x86_64` + AVX2**: 8 lanes (256 bits) per iteration. AVX2 is
//!   runtime-detected via [`std::arch::is_x86_feature_detected`].
//! - **`aarch64` + NEON**: 4 lanes (128 bits) per iteration. NEON is
//!   part of the base AArch64 ISA, so no runtime detection is needed.
//! - **Other architectures or slices shorter than the lane count**:
//!   tight scalar loop.
//!
//! The two SIMD paths share the same per-lane recipe (broadcast +
//! compare-equal + reduce). The lane width drives the threshold and
//! tail handling per architecture.
//!
//! ## Safety
//!
//! All public entry points are safe. The SIMD code paths use unaligned
//! loads (`_mm256_loadu_si256` on x86_64, `vld1q_u32` on aarch64 ; the
//! latter has no aligned/unaligned distinction) and only execute when:
//! 1. The CPU advertises the required feature (runtime for AVX2,
//!    static for NEON), and
//! 2. The remaining slice length is at least the lane count
//!    (bounds-checked at the public entry).
//!
//! The `unsafe` blocks are minimal and locally documented.

#![allow(
    clippy::missing_inline_in_public_items,
    // SIMD intrinsics naturally cluster in a single unsafe block;
    // splitting them obscures the recipe and adds no safety value.
    clippy::multiple_unsafe_ops_per_block,
    // Lane-count divisions (`slice.len() / 8`, `/ 4`) are exact by
    // construction ; the remainder feeds the scalar tail.
    clippy::integer_division,
    clippy::integer_division_remainder_used,
    clippy::arithmetic_side_effects,
    // Tag references in module docs (`KIND_TAG_SHIFT`, intrinsic names)
    // would over-quote the prose.
    clippy::doc_markdown,
    // Scalar baselines deliberately use `iter().any` rather than
    // `slice.contains`: they sit alongside SIMD variants and the
    // shape is easier to read uniformly.
    clippy::manual_contains,
    clippy::explicit_iter_loop,
)]

use crate::ElementId;
use crate::ElementKind;

/// Below this length, scalar wins on AVX2 (8-lane setup + tail handling
/// outweighs the parallel work).
#[cfg(target_arch = "x86_64")]
const AVX2_THRESHOLD: usize = 8;

/// Below this length, scalar wins on NEON (4-lane setup + tail handling
/// outweighs the parallel work).
#[cfg(target_arch = "aarch64")]
const NEON_THRESHOLD: usize = 4;

/// The shift amount used to extract the kind tag from a packed
/// `ElementId`. Hard-wired to match `ElementId::SLOT_BITS` because the
/// SIMD shift intrinsics take the count as a const-generic argument
/// rather than a runtime value. The compile-time check below pins the
/// two values together.
const KIND_TAG_SHIFT: u32 = 26;
const _: () = assert!(KIND_TAG_SHIFT == ElementId::SLOT_BITS, "SIMD kind shift must match ElementId::SLOT_BITS");

/// `true` iff `needle` appears anywhere in `slice`.
///
/// Equivalent to `slice.contains(&needle)` and
/// `slice.iter().any(|e| *e == needle)`, vectorised on x86_64+AVX2 and
/// aarch64+NEON.
#[must_use]
pub fn contains(slice: &[ElementId], needle: ElementId) -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        if slice.len() >= AVX2_THRESHOLD && std::is_x86_feature_detected!("avx2") {
            // SAFETY: AVX2 detected at runtime; slice length checked above.
            unsafe {
                return contains_avx2(slice, needle);
            }
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        if slice.len() >= NEON_THRESHOLD {
            // SAFETY: NEON is baseline on AArch64; slice length checked above.
            unsafe {
                return contains_neon(slice, needle);
            }
        }
    }

    contains_scalar(slice, needle)
}

/// Index of the first occurrence of `needle` in `slice`, or `None`.
///
/// Equivalent to `slice.iter().position(|e| *e == needle)`, vectorised
/// on x86_64+AVX2 and aarch64+NEON.
#[must_use]
pub fn position_of(slice: &[ElementId], needle: ElementId) -> Option<usize> {
    #[cfg(target_arch = "x86_64")]
    {
        if slice.len() >= AVX2_THRESHOLD && std::is_x86_feature_detected!("avx2") {
            // SAFETY: AVX2 detected at runtime; slice length checked above.
            unsafe {
                return position_of_avx2(slice, needle);
            }
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        if slice.len() >= NEON_THRESHOLD {
            // SAFETY: NEON is baseline on AArch64; slice length checked above.
            unsafe {
                return position_of_neon(slice, needle);
            }
        }
    }

    position_of_scalar(slice, needle)
}

/// `true` iff at least one element in `slice` has kind `kind`.
///
/// Equivalent to `slice.iter().any(|e| e.kind() == kind)`, vectorised
/// on x86_64+AVX2 and aarch64+NEON.
#[must_use]
pub fn any_of_kind(slice: &[ElementId], kind: ElementKind) -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        if slice.len() >= AVX2_THRESHOLD && std::is_x86_feature_detected!("avx2") {
            // SAFETY: AVX2 detected at runtime; slice length checked above.
            unsafe {
                return any_of_kind_avx2(slice, kind);
            }
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        if slice.len() >= NEON_THRESHOLD {
            // SAFETY: NEON is baseline on AArch64; slice length checked above.
            unsafe {
                return any_of_kind_neon(slice, kind);
            }
        }
    }

    any_of_kind_scalar(slice, kind)
}

/// `true` iff every element in `slice` has kind `kind`. Returns
/// `false` for empty slices (vacuous-true is rarely what callers
/// want here ; `is_X` predicates already gate on non-empty).
///
/// Equivalent to `!slice.is_empty() && slice.iter().all(|e| e.kind() == kind)`,
/// vectorised on x86_64+AVX2 and aarch64+NEON.
#[must_use]
pub fn all_of_kind(slice: &[ElementId], kind: ElementKind) -> bool {
    if slice.is_empty() {
        return false;
    }
    #[cfg(target_arch = "x86_64")]
    {
        if slice.len() >= AVX2_THRESHOLD && std::is_x86_feature_detected!("avx2") {
            // SAFETY: AVX2 detected at runtime; slice length checked above.
            unsafe {
                return all_of_kind_avx2(slice, kind);
            }
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        if slice.len() >= NEON_THRESHOLD {
            // SAFETY: NEON is baseline on AArch64; slice length checked above.
            unsafe {
                return all_of_kind_neon(slice, kind);
            }
        }
    }

    all_of_kind_scalar(slice, kind)
}

/// `true` iff `slice` is strictly increasing under unsigned u32 order
/// (every adjacent pair satisfies `a < b`, no equal-neighbour pairs).
/// Slices of length 0 or 1 are vacuously strict.
///
/// Equivalent to `slice.windows(2).all(|w| w[0].raw() < w[1].raw())`,
/// vectorised on x86_64+AVX2 and aarch64+NEON. The interner's slow
/// path uses this to skip `sort_unstable + dedup` for already-canonical
/// inputs.
#[must_use]
pub fn is_sorted_strict(slice: &[ElementId]) -> bool {
    // Need at least 9 elements for the chunked path: each chunk reads
    // an 8-lane window plus its 1-lane shifted neighbour.
    #[cfg(target_arch = "x86_64")]
    {
        if slice.len() >= 9 && std::is_x86_feature_detected!("avx2") {
            // SAFETY: AVX2 detected at runtime; length checked above.
            unsafe {
                return is_sorted_strict_avx2(slice);
            }
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        if slice.len() >= 5 {
            // SAFETY: NEON is baseline on AArch64; length checked above.
            unsafe {
                return is_sorted_strict_neon(slice);
            }
        }
    }

    is_sorted_strict_scalar(slice)
}

/// Number of elements in `slice` whose kind is `kind`.
///
/// Equivalent to `slice.iter().filter(|e| e.kind() == kind).count()`,
/// vectorised on x86_64+AVX2 and aarch64+NEON.
#[must_use]
pub fn count_of_kind(slice: &[ElementId], kind: ElementKind) -> usize {
    #[cfg(target_arch = "x86_64")]
    {
        if slice.len() >= AVX2_THRESHOLD && std::is_x86_feature_detected!("avx2") {
            // SAFETY: AVX2 detected at runtime; slice length checked above.
            unsafe {
                return count_of_kind_avx2(slice, kind);
            }
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        if slice.len() >= NEON_THRESHOLD {
            // SAFETY: NEON is baseline on AArch64; slice length checked above.
            unsafe {
                return count_of_kind_neon(slice, kind);
            }
        }
    }

    count_of_kind_scalar(slice, kind)
}

#[inline]
fn contains_scalar(slice: &[ElementId], needle: ElementId) -> bool {
    slice.iter().any(|e| *e == needle)
}

#[inline]
fn position_of_scalar(slice: &[ElementId], needle: ElementId) -> Option<usize> {
    slice.iter().position(|e| *e == needle)
}

#[inline]
#[allow(clippy::missing_asserts_for_indexing)]
fn is_sorted_strict_scalar(slice: &[ElementId]) -> bool {
    slice.windows(2).all(|w| w[0].raw() < w[1].raw())
}

#[inline]
fn any_of_kind_scalar(slice: &[ElementId], kind: ElementKind) -> bool {
    slice.iter().any(|e| e.kind() == kind)
}

#[inline]
fn count_of_kind_scalar(slice: &[ElementId], kind: ElementKind) -> usize {
    slice.iter().filter(|e| e.kind() == kind).count()
}

#[inline]
fn all_of_kind_scalar(slice: &[ElementId], kind: ElementKind) -> bool {
    slice.iter().all(|e| e.kind() == kind)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
#[inline]
unsafe fn contains_avx2(slice: &[ElementId], needle: ElementId) -> bool {
    use core::arch::asm;
    use core::arch::x86_64::_mm256_set1_epi32;

    let chunks = slice.len() / 8;
    if chunks == 0 {
        return contains_scalar(slice, needle);
    }

    // `target_feature(avx2)` makes the intrinsic safe in this body.
    let target = _mm256_set1_epi32(needle.raw() as i32);
    let bytes = chunks.wrapping_mul(32);
    // SAFETY: `bytes` <= slice.len()*4, so `base + bytes` is one past
    // the last full chunk and stays within the slice's allocation.
    let base_end = unsafe { slice.as_ptr().cast::<u8>().add(bytes) };
    let neg_off: isize = -(bytes as isize);
    let found: u32;

    // SAFETY: the asm reads `[base_end + neg_off]` for `neg_off ∈ [-bytes, 0)`,
    // i.e. `[base_end - bytes, base_end)` = the chunked prefix of `slice`.
    // `out(reg)` (not `lateout`) is required because `found`'s register
    // is clobbered by `vpmovmskb` mid-loop, before the `base`/`off`/`target`
    // inputs are dead; aliasing with any of them would corrupt the
    // following iteration's load address.
    unsafe {
        asm!(
            "2:",
            "vmovdqu  ymm1, [{base} + {off}]",
            "vpcmpeqd ymm1, ymm1, {target}",
            "vpmovmskb {found:e}, ymm1",
            "test  {found:e}, {found:e}",
            "jnz   3f",
            "add   {off}, 32",
            "jl    2b",
            "xor   {found:e}, {found:e}",
            "3:",
            base = in(reg) base_end,
            off = inout(reg) neg_off => _,
            target = in(ymm_reg) target,
            found = out(reg) found,
            out("ymm1") _,
            options(nostack, readonly, pure),
        );
    }

    if found != 0 {
        return true;
    }

    let tail_start = chunks.wrapping_mul(8);
    contains_scalar(&slice[tail_start..], needle)
}

/// Hand-tuned AVX2 position scan. Same pointer-offset trickery as
/// [`contains_avx2`]; the asm exits with the final `neg_off` and the
/// `vpmovmskb` byte-mask. The position is then
/// `(bytes + neg_off + tzcnt(mask)) / 4` (4 mask bits per matched
/// 32-bit lane). Returns `None` when no chunked match was found and
/// the scalar tail also misses.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
#[inline]
unsafe fn position_of_avx2(slice: &[ElementId], needle: ElementId) -> Option<usize> {
    use core::arch::asm;
    use core::arch::x86_64::_mm256_set1_epi32;

    let chunks = slice.len() / 8;
    if chunks == 0 {
        return position_of_scalar(slice, needle);
    }

    // `target_feature(avx2)` makes the intrinsic safe in this body.
    let target = _mm256_set1_epi32(needle.raw() as i32);
    let bytes = chunks.wrapping_mul(32);
    // SAFETY: `bytes` <= slice.len()*4, so `base + bytes` is one past
    // the last full chunk and stays within the slice's allocation.
    let base_end = unsafe { slice.as_ptr().cast::<u8>().add(bytes) };
    let mut neg_off: isize = -(bytes as isize);
    let mask: u32;

    // SAFETY: reads `[base_end + neg_off]` for `neg_off ∈ [-bytes, 0)`,
    // i.e. the chunked prefix of `slice`. On hit, jumps to `3:` with
    // both `mask` and the matching chunk's `neg_off` live; on miss,
    // `mask` is zeroed at the loop fall-through. `out(reg)` (not
    // `lateout`) keeps `mask` distinct from the inputs since
    // `vpmovmskb` writes it mid-loop while inputs are still live.
    unsafe {
        asm!(
            "2:",
            "vmovdqu  ymm1, [{base} + {off}]",
            "vpcmpeqd ymm1, ymm1, {target}",
            "vpmovmskb {mask:e}, ymm1",
            "test  {mask:e}, {mask:e}",
            "jnz   3f",
            "add   {off}, 32",
            "jl    2b",
            "xor   {mask:e}, {mask:e}",
            "3:",
            base = in(reg) base_end,
            off = inout(reg) neg_off,
            target = in(ymm_reg) target,
            mask = out(reg) mask,
            out("ymm1") _,
            options(nostack, readonly, pure),
        );
    }

    if mask != 0 {
        // tzcnt of the byte mask = byte offset of the first matching
        // lane within the 32-byte chunk. (bytes + neg_off) is the
        // chunk's byte offset from slice start; divide by 4 for u32.
        let tz = mask.trailing_zeros() as isize;
        let byte_off = (bytes as isize).wrapping_add(neg_off).wrapping_add(tz);
        return Some((byte_off / 4) as usize);
    }

    let tail_start = chunks.wrapping_mul(8);
    position_of_scalar(&slice[tail_start..], needle).map(|i| tail_start.wrapping_add(i))
}

/// Hand-tuned AVX2 strict-sort scan. Per chunk loads two overlapping
/// 8-lane windows (`v[i..i+8]` and `v[i+1..i+9]`), XORs both against
/// `0x80000000` to map unsigned u32 order onto AVX2's signed `vpcmpgtd`,
/// and requires every lane of `v[i+1..] > v[i..]`. Bails to false on
/// the first failed chunk; the scalar tail covers the final partial
/// window plus the seam to it.
///
/// `chunks = (len - 1) / 8`: each chunk consumes 8 elements and peeks
/// one ahead, so the last chunk needs `len >= chunks*8 + 1` (always
/// true under that ceiling division).
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
#[inline]
unsafe fn is_sorted_strict_avx2(slice: &[ElementId]) -> bool {
    use core::arch::asm;
    use core::arch::x86_64::_mm256_set1_epi32;

    // SAFETY: caller guaranteed slice.len() >= 9, so (len - 1) >= 8.
    let chunks = (slice.len() - 1) / 8;
    if chunks == 0 {
        return is_sorted_strict_scalar(slice);
    }

    // `target_feature(avx2)` makes the intrinsic safe in this body.
    let sign_mask = _mm256_set1_epi32(i32::MIN);
    let bytes = chunks.wrapping_mul(32);
    // SAFETY: bytes <= (len-1)*4, so `slice.as_ptr() + bytes` is at
    // most `slice.as_ptr() + (len-1)*4`, still inside the allocation;
    // the +4 sub-load reads `v[chunks*8]`, valid because slice.len()
    // >= chunks*8 + 1 by the chunks definition.
    let base_end = unsafe { slice.as_ptr().cast::<u8>().add(bytes) };
    let neg_off: isize = -(bytes as isize);
    let mismatch: u32;

    // SAFETY: reads `[base_end + neg_off]` and `[base_end + neg_off + 4]`
    // for `neg_off ∈ [-bytes, 0)`. `out(reg)` (not `lateout`) is needed
    // because `mismatch` is written by `vpmovmskb` mid-loop while the
    // `base`/`off` inputs are still live; aliasing would corrupt the
    // next iteration's load address.
    unsafe {
        asm!(
            "2:",
            "vmovdqu  ymm0, [{base} + {off}]",
            "vmovdqu  ymm1, [{base} + {off} + 4]",
            "vpxor    ymm0, ymm0, {sign}",
            "vpxor    ymm1, ymm1, {sign}",
            "vpcmpgtd ymm0, ymm1, ymm0",
            "vpmovmskb {mismatch:e}, ymm0",
            "cmp  {mismatch:e}, -1",
            "jne  3f",
            "add  {off}, 32",
            "jl   2b",
            "mov  {mismatch:e}, -1",
            "3:",
            base = in(reg) base_end,
            off = inout(reg) neg_off => _,
            sign = in(ymm_reg) sign_mask,
            mismatch = out(reg) mismatch,
            out("ymm0") _,
            out("ymm1") _,
            options(nostack, readonly, pure),
        );
    }

    if mismatch != u32::MAX {
        return false;
    }

    let tail_start = chunks.wrapping_mul(8);
    is_sorted_strict_scalar(&slice[tail_start..])
}

/// Hand-tuned AVX2 kind scan. Same pointer-offset trickery as
/// [`contains_avx2`] ; per-iteration body is `vmovdqu / vpsrld /
/// vpcmpeqd / vpmovmskb / test / jnz / add / jl` (8 instructions, one
/// fewer than the intrinsic version's `inc + cmp + jb` tail).
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
#[inline]
unsafe fn any_of_kind_avx2(slice: &[ElementId], kind: ElementKind) -> bool {
    use core::arch::asm;
    use core::arch::x86_64::_mm256_set1_epi32;

    let chunks = slice.len() / 8;
    if chunks == 0 {
        return any_of_kind_scalar(slice, kind);
    }
    // `target_feature(avx2)` makes the intrinsic safe in this body.
    let target = _mm256_set1_epi32(kind as i32);
    let bytes = chunks.wrapping_mul(32);
    // SAFETY: same as [`contains_avx2`].
    let base_end = unsafe { slice.as_ptr().cast::<u8>().add(bytes) };
    let neg_off: isize = -(bytes as isize);
    let found: u32;

    // SAFETY: reads `[base_end + neg_off]` for `neg_off ∈ [-bytes, 0)`.
    // `out(reg)` (not `lateout`) keeps `found` distinct from inputs;
    // it's overwritten mid-loop by `vpmovmskb` while inputs are live.
    unsafe {
        asm!(
            "2:",
            "vmovdqu  ymm1, [{base} + {off}]",
            "vpsrld   ymm1, ymm1, 26",
            "vpcmpeqd ymm1, ymm1, {target}",
            "vpmovmskb {found:e}, ymm1",
            "test  {found:e}, {found:e}",
            "jnz   3f",
            "add   {off}, 32",
            "jl    2b",
            "xor   {found:e}, {found:e}",
            "3:",
            base = in(reg) base_end,
            off = inout(reg) neg_off => _,
            target = in(ymm_reg) target,
            found = out(reg) found,
            out("ymm1") _,
            options(nostack, readonly, pure),
        );
    }

    if found != 0 {
        return true;
    }

    let tail_start = chunks.wrapping_mul(8);
    any_of_kind_scalar(&slice[tail_start..], kind)
}

/// Hand-tuned AVX2 count. Pointer-offset loop; per-iteration popcnt of
/// the 32-bit movemask, divided by 4 (4 mask bits per matching 32-bit
/// lane) and accumulated.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
#[inline]
unsafe fn count_of_kind_avx2(slice: &[ElementId], kind: ElementKind) -> usize {
    use core::arch::asm;
    use core::arch::x86_64::_mm256_set1_epi32;

    let chunks = slice.len() / 8;
    if chunks == 0 {
        return count_of_kind_scalar(slice, kind);
    }

    // `target_feature(avx2)` makes the intrinsic safe in this body.
    let target = _mm256_set1_epi32(kind as i32);
    let bytes = chunks.wrapping_mul(32);
    // SAFETY: same as [`contains_avx2`].
    let base_end = unsafe { slice.as_ptr().cast::<u8>().add(bytes) };
    let neg_off: isize = -(bytes as isize);
    let total_bytes: u64;

    // SAFETY: reads `[base_end + neg_off]` for `neg_off ∈ [-bytes, 0)`.
    // Accumulates `popcnt` of each iteration's movemask in `total`.
    // `out(reg)` for both `total` and `scratch` keeps them distinct
    // from the live inputs (they're written every iteration via
    // `vpmovmskb`/`add`).
    unsafe {
        asm!(
            "xor   {total}, {total}",
            "2:",
            "vmovdqu  ymm1, [{base} + {off}]",
            "vpsrld   ymm1, ymm1, 26",
            "vpcmpeqd ymm1, ymm1, {target}",
            "vpmovmskb {scratch:e}, ymm1",
            "popcnt {scratch}, {scratch}",
            "add  {total}, {scratch}",
            "add  {off}, 32",
            "jl   2b",
            base = in(reg) base_end,
            off = inout(reg) neg_off => _,
            target = in(ymm_reg) target,
            total = out(reg) total_bytes,
            scratch = out(reg) _,
            out("ymm1") _,
            options(nostack, readonly, pure),
        );
    }

    // Each matching 32-bit lane sets 4 mask bits, so the byte popcnt
    // counts each lane four times.
    let total = (total_bytes / 4) as usize;
    let tail_start = chunks.wrapping_mul(8);
    total.saturating_add(count_of_kind_scalar(&slice[tail_start..], kind))
}

/// Hand-tuned AVX2 "all elements have kind". Pointer-end trick;
/// per iteration: `vmovdqu / vpsrld / vpcmpeqd / vpmovmskb / cmp /
/// jne / add / jl`. Bails to false on the first non-matching lane.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
#[inline]
unsafe fn all_of_kind_avx2(slice: &[ElementId], kind: ElementKind) -> bool {
    use core::arch::asm;
    use core::arch::x86_64::_mm256_set1_epi32;

    let chunks = slice.len() / 8;
    if chunks == 0 {
        return all_of_kind_scalar(slice, kind);
    }
    // `target_feature(avx2)` makes the intrinsic safe in this body.
    let target = _mm256_set1_epi32(kind as i32);
    let bytes = chunks.wrapping_mul(32);
    // SAFETY: `bytes` <= slice.len()*4.
    let base_end = unsafe { slice.as_ptr().cast::<u8>().add(bytes) };
    let neg_off: isize = -(bytes as isize);
    let mismatch: u32;

    // SAFETY: reads `[base_end + neg_off]` for `neg_off ∈ [-bytes, 0)`.
    // `out(reg)` (not `lateout`) keeps `mismatch` distinct from inputs;
    // `vpmovmskb` writes it mid-loop while `base`/`off` are still live.
    unsafe {
        asm!(
            "2:",
            "vmovdqu  ymm1, [{base} + {off}]",
            "vpsrld   ymm1, ymm1, 26",
            "vpcmpeqd ymm1, ymm1, {target}",
            "vpmovmskb {mismatch:e}, ymm1",
            "cmp  {mismatch:e}, -1",
            "jne  3f",
            "add  {off}, 32",
            "jl   2b",
            "mov  {mismatch:e}, -1",
            "3:",
            base = in(reg) base_end,
            off = inout(reg) neg_off => _,
            target = in(ymm_reg) target,
            mismatch = out(reg) mismatch,
            out("ymm1") _,
            options(nostack, readonly, pure),
        );
    }

    if mismatch != u32::MAX {
        return false;
    }
    let tail_start = chunks.wrapping_mul(8);
    all_of_kind_scalar(&slice[tail_start..], kind)
}

#[cfg(target_arch = "aarch64")]
#[inline]
unsafe fn all_of_kind_neon(slice: &[ElementId], kind: ElementKind) -> bool {
    use core::arch::aarch64::vceqq_u32;
    use core::arch::aarch64::vdupq_n_u32;
    use core::arch::aarch64::vld1q_u32;
    use core::arch::aarch64::vminvq_u32;
    use core::arch::aarch64::vshrq_n_u32;

    // SAFETY: caller guarantees slice.len() >= NEON_THRESHOLD (4).
    let target = unsafe { vdupq_n_u32(kind as u32) };
    let chunks = slice.len() / 4;
    let base = slice.as_ptr().cast::<u32>();
    for i in 0..chunks {
        // SAFETY: i in 0..chunks.
        unsafe {
            let v = vld1q_u32(base.add(i.wrapping_mul(4)));
            let kinds = vshrq_n_u32::<{ KIND_TAG_SHIFT as i32 }>(v);
            let cmp = vceqq_u32(kinds, target);
            // vminvq returns the minimum lane ; if any lane is 0
            // (mismatch), the min is 0.
            if vminvq_u32(cmp) == 0 {
                return false;
            }
        }
    }

    let tail_start = chunks.wrapping_mul(4);
    all_of_kind_scalar(&slice[tail_start..], kind)
}

/// NEON strict-sort scan. Per chunk loads `v[i..i+4]` and `v[i+1..i+5]`
/// and tests `v[i+1..] > v[i..]` lane-wise via `vcgtq_u32` (unsigned
/// compare; no sign-flip needed, unlike AVX2). `vminvq` reduces the
/// mask: any failing lane brings the min to 0.
#[cfg(target_arch = "aarch64")]
#[inline]
unsafe fn is_sorted_strict_neon(slice: &[ElementId]) -> bool {
    use core::arch::aarch64::vcgtq_u32;
    use core::arch::aarch64::vld1q_u32;
    use core::arch::aarch64::vminvq_u32;

    let chunks = (slice.len() - 1) / 4;
    let base = slice.as_ptr().cast::<u32>();
    for i in 0..chunks {
        // SAFETY: i in 0..chunks; lo reads v[i*4..i*4+4], hi reads
        // v[i*4+1..i*4+5]. The +1 offset is in-bounds because
        // chunks*4 + 1 <= slice.len() by the chunks definition.
        unsafe {
            let lo = vld1q_u32(base.add(i.wrapping_mul(4)));
            let hi = vld1q_u32(base.add(i.wrapping_mul(4).wrapping_add(1)));
            let cmp = vcgtq_u32(hi, lo);
            if vminvq_u32(cmp) == 0 {
                return false;
            }
        }
    }

    let tail_start = chunks.wrapping_mul(4);
    is_sorted_strict_scalar(&slice[tail_start..])
}

/// NEON position scan. Per chunk: `vld1q + vceqq` builds a 4×u32 mask
/// (all-ones on match, zero otherwise). `vandq` against `[1, 2, 4, 8]`
/// keeps the lane's weight on a hit and 0 elsewhere; `vaddvq` reduces
/// to a 0..15 nibble whose `trailing_zeros` is the first matching lane.
#[cfg(target_arch = "aarch64")]
#[inline]
unsafe fn position_of_neon(slice: &[ElementId], needle: ElementId) -> Option<usize> {
    use core::arch::aarch64::vaddvq_u32;
    use core::arch::aarch64::vandq_u32;
    use core::arch::aarch64::vceqq_u32;
    use core::arch::aarch64::vdupq_n_u32;
    use core::arch::aarch64::vld1q_u32;
    use core::arch::aarch64::vmaxvq_u32;

    // SAFETY: caller guarantees slice.len() >= NEON_THRESHOLD (4).
    let target = unsafe { vdupq_n_u32(needle.raw()) };
    let chunks = slice.len() / 4;
    let base = slice.as_ptr().cast::<u32>();
    let weights: [u32; 4] = [1, 2, 4, 8];
    // SAFETY: 4-lane load from a 4-element local array.
    let weight_v = unsafe { vld1q_u32(weights.as_ptr()) };

    for i in 0..chunks {
        // SAFETY: i in 0..chunks; base + i*4 lanes is in-bounds for the slice.
        unsafe {
            let v = vld1q_u32(base.add(i.wrapping_mul(4)));
            let cmp = vceqq_u32(v, target);
            if vmaxvq_u32(cmp) == 0 {
                continue;
            }
            // Each matched lane = all-1s, masked by its weight (1/2/4/8);
            // unmatched lanes contribute 0. Sum gives a 0..15 nibble whose
            // low set bit is the first matching lane index.
            let weighted = vandq_u32(cmp, weight_v);
            let nibble = vaddvq_u32(weighted);
            let lane = nibble.trailing_zeros() as usize;
            return Some(i.wrapping_mul(4).wrapping_add(lane));
        }
    }

    let tail_start = chunks.saturating_mul(4);
    position_of_scalar(&slice[tail_start..], needle).map(|i| tail_start.wrapping_add(i))
}

#[cfg(target_arch = "aarch64")]
#[inline]
unsafe fn contains_neon(slice: &[ElementId], needle: ElementId) -> bool {
    use core::arch::aarch64::vceqq_u32;
    use core::arch::aarch64::vdupq_n_u32;
    use core::arch::aarch64::vld1q_u32;
    use core::arch::aarch64::vmaxvq_u32;

    // SAFETY: caller guarantees slice.len() >= NEON_THRESHOLD (4).
    // NEON loads have no aligned/unaligned distinction on AArch64.
    let target = unsafe { vdupq_n_u32(needle.raw()) };
    let chunks = slice.len() / 4;
    let base = slice.as_ptr().cast::<u32>();
    for i in 0..chunks {
        // SAFETY: i in 0..chunks; base + i*4 lanes is in-bounds for the slice.
        unsafe {
            let v = vld1q_u32(base.add(i.wrapping_mul(4)));
            let cmp = vceqq_u32(v, target);
            if vmaxvq_u32(cmp) != 0 {
                return true;
            }
        }
    }

    let tail_start = chunks.saturating_mul(4);
    contains_scalar(&slice[tail_start..], needle)
}

#[cfg(target_arch = "aarch64")]
#[inline]
unsafe fn any_of_kind_neon(slice: &[ElementId], kind: ElementKind) -> bool {
    use core::arch::aarch64::vceqq_u32;
    use core::arch::aarch64::vdupq_n_u32;
    use core::arch::aarch64::vld1q_u32;
    use core::arch::aarch64::vmaxvq_u32;
    use core::arch::aarch64::vshrq_n_u32;

    // SAFETY: caller guarantees slice.len() >= NEON_THRESHOLD (4).
    let target = unsafe { vdupq_n_u32(kind as u32) };
    let chunks = slice.len() / 4;
    let base = slice.as_ptr().cast::<u32>();
    for i in 0..chunks {
        // SAFETY: i in 0..chunks.
        unsafe {
            let v = vld1q_u32(base.add(i.wrapping_mul(4)));
            let kinds = vshrq_n_u32::<{ KIND_TAG_SHIFT as i32 }>(v);
            let cmp = vceqq_u32(kinds, target);
            if vmaxvq_u32(cmp) != 0 {
                return true;
            }
        }
    }

    let tail_start = chunks.saturating_mul(4);
    any_of_kind_scalar(&slice[tail_start..], kind)
}

#[cfg(target_arch = "aarch64")]
#[inline]
unsafe fn count_of_kind_neon(slice: &[ElementId], kind: ElementKind) -> usize {
    use core::arch::aarch64::vaddvq_u32;
    use core::arch::aarch64::vceqq_u32;
    use core::arch::aarch64::vdupq_n_u32;
    use core::arch::aarch64::vld1q_u32;
    use core::arch::aarch64::vshrq_n_u32;

    // SAFETY: caller guarantees slice.len() >= NEON_THRESHOLD (4).
    let target = unsafe { vdupq_n_u32(kind as u32) };
    let chunks = slice.len() / 4;
    let base = slice.as_ptr().cast::<u32>();
    let mut total: u32 = 0;
    for i in 0..chunks {
        // SAFETY: i in 0..chunks.
        unsafe {
            let v = vld1q_u32(base.add(i.wrapping_mul(4)));
            let kinds = vshrq_n_u32::<{ KIND_TAG_SHIFT as i32 }>(v);
            let cmp = vceqq_u32(kinds, target);
            let bits = vshrq_n_u32::<31>(cmp);
            total = total.wrapping_add(vaddvq_u32(bits));
        }
    }

    let tail_start = chunks.saturating_mul(4);
    (total as usize).saturating_add(count_of_kind_scalar(&slice[tail_start..], kind))
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::panic,
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::missing_assert_message,
        clippy::missing_docs_in_private_items,
        clippy::std_instead_of_alloc,
        clippy::std_instead_of_core
    )]

    use super::*;
    use crate::prelude::FALSE;
    use crate::prelude::INT;
    use crate::prelude::NULL;
    use crate::prelude::STRING;
    use crate::prelude::TRUE;

    fn distinct_long() -> Vec<ElementId> {
        vec![INT, STRING, NULL, TRUE, FALSE, INT, INT, STRING, NULL, INT, FALSE, INT, INT, STRING, INT, INT, INT]
    }

    #[test]
    fn contains_matches_scalar() {
        let v = distinct_long();
        for &needle in [INT, STRING, NULL, TRUE, FALSE].iter() {
            assert_eq!(contains(&v, needle), v.contains(&needle));
        }

        assert!(!contains(&v, ElementId::int_literal(999)));
    }

    #[test]
    fn contains_short_slice_falls_through_scalar() {
        let v = vec![INT, STRING, NULL];
        assert!(contains(&v, INT));
        assert!(contains(&v, STRING));
        assert!(!contains(&v, ElementId::int_literal(7)));
    }

    #[test]
    fn contains_empty() {
        let v: Vec<ElementId> = vec![];
        assert!(!contains(&v, INT));
    }

    #[test]
    fn any_of_kind_matches_scalar() {
        let v = distinct_long();
        for kind in [
            ElementKind::Int,
            ElementKind::String,
            ElementKind::Null,
            ElementKind::True,
            ElementKind::False,
            ElementKind::Float,
        ] {
            let expected = v.iter().any(|e| e.kind() == kind);
            assert_eq!(any_of_kind(&v, kind), expected, "kind {kind:?}");
        }
    }

    #[test]
    fn count_of_kind_matches_scalar() {
        let v = distinct_long();
        for kind in [ElementKind::Int, ElementKind::String, ElementKind::Null, ElementKind::True, ElementKind::False] {
            let expected = v.iter().filter(|e| e.kind() == kind).count();
            assert_eq!(count_of_kind(&v, kind), expected, "kind {kind:?}");
        }
    }

    #[test]
    fn all_of_kind_matches_scalar() {
        let homogeneous = vec![INT; 16];
        assert!(all_of_kind(&homogeneous, ElementKind::Int));
        assert!(!all_of_kind(&homogeneous, ElementKind::String));

        let mixed = distinct_long();
        assert!(!all_of_kind(&mixed, ElementKind::Int));

        // Bail on first mismatch in tail.
        let mut almost = vec![INT; 8];
        almost.push(STRING);
        assert!(!all_of_kind(&almost, ElementKind::Int));

        // Empty returns false (no `is_X` should claim true on never).
        let empty: Vec<ElementId> = Vec::new();
        assert!(!all_of_kind(&empty, ElementKind::Int));
    }

    #[test]
    fn position_of_matches_scalar() {
        let v = distinct_long();
        for &needle in [INT, STRING, NULL, TRUE, FALSE].iter() {
            assert_eq!(position_of(&v, needle), v.iter().position(|e| *e == needle), "needle {needle:?}");
        }
        assert_eq!(position_of(&v, ElementId::int_literal(999)), None);
    }

    #[test]
    fn position_of_first_chunk_hit() {
        let mut v = vec![INT; 16];
        v[3] = STRING;
        assert_eq!(position_of(&v, STRING), Some(3));
    }

    #[test]
    fn position_of_second_chunk_hit() {
        let mut v = vec![INT; 16];
        v[10] = STRING;
        assert_eq!(position_of(&v, STRING), Some(10));
    }

    #[test]
    fn position_of_tail_only_hit() {
        // Chunked region is INT-only, hit in the scalar tail.
        let mut v = vec![INT; 8];
        v.push(STRING);
        assert_eq!(position_of(&v, STRING), Some(8));
    }

    #[test]
    fn position_of_short_slice_falls_through_scalar() {
        let v = vec![INT, STRING, NULL];
        assert_eq!(position_of(&v, STRING), Some(1));
        assert_eq!(position_of(&v, NULL), Some(2));
        assert_eq!(position_of(&v, ElementId::int_literal(7)), None);
    }

    #[test]
    fn position_of_empty() {
        let v: Vec<ElementId> = vec![];
        assert_eq!(position_of(&v, INT), None);
    }

    #[test]
    fn position_of_first_match_among_duplicates() {
        let v = vec![INT, INT, STRING, STRING, INT, STRING, INT, INT, INT, INT];
        assert_eq!(position_of(&v, STRING), Some(2));
        assert_eq!(position_of(&v, INT), Some(0));
    }

    #[test]
    fn is_sorted_strict_short_slices() {
        let empty: Vec<ElementId> = vec![];
        assert!(is_sorted_strict(&empty));
        assert!(is_sorted_strict(&[INT]));
        let asc = [ElementId::int_literal(1), ElementId::int_literal(2)];
        assert!(is_sorted_strict(&asc));
        let dup = [ElementId::int_literal(1), ElementId::int_literal(1)];
        assert!(!is_sorted_strict(&dup));
        let desc = [ElementId::int_literal(2), ElementId::int_literal(1)];
        assert!(!is_sorted_strict(&desc));
    }

    /// Build a strictly-increasing sorted+deduped slice from a pool of
    /// literal int ids. The interner doesn't guarantee slot order matches
    /// literal value order, so we sort+dedup the raw ids first.
    fn sorted_int_ids(count: usize) -> Vec<ElementId> {
        let mut v: Vec<ElementId> = (0..(count as i64).wrapping_mul(2)).map(ElementId::int_literal).collect();
        v.sort_unstable();
        v.dedup();
        v.truncate(count);
        v
    }

    #[test]
    fn is_sorted_strict_long_strict_increasing() {
        let v = sorted_int_ids(20);
        assert!(is_sorted_strict(&v));
    }

    #[test]
    fn is_sorted_strict_duplicate_in_chunk() {
        let mut v = sorted_int_ids(20);
        v[5] = v[4];
        assert!(!is_sorted_strict(&v));
    }

    #[test]
    fn is_sorted_strict_descending_in_chunk() {
        let mut v = sorted_int_ids(20);
        v.swap(3, 4);
        assert!(!is_sorted_strict(&v));
    }

    #[test]
    fn is_sorted_strict_failure_in_tail() {
        let mut v = sorted_int_ids(10);
        v[9] = v[8];
        assert!(!is_sorted_strict(&v));
    }

    #[test]
    fn is_sorted_strict_seam_inside_chunk() {
        let mut v = sorted_int_ids(9);
        v[8] = v[0];
        assert!(!is_sorted_strict(&v));
    }

    #[test]
    fn is_sorted_strict_cross_kind_unsigned_order() {
        // Mix kinds (int, string, bool, null). Sort+dedup, then verify
        // the unsigned-u32 strict-sort holds across the high-tag boundary
        // — exercising the AVX2 sign-flip XOR.
        let mut v = vec![INT, STRING, NULL, TRUE, FALSE];
        v.extend(sorted_int_ids(8));
        v.sort_unstable();
        v.dedup();
        assert!(is_sorted_strict(&v));
    }

    #[test]
    fn count_of_kind_long_slice_with_partial_chunks() {
        let mut v = vec![INT; 8];
        v.extend([STRING; 8]);
        v.push(INT);
        assert_eq!(count_of_kind(&v, ElementKind::Int), 9);
        assert_eq!(count_of_kind(&v, ElementKind::String), 8);
        assert_eq!(count_of_kind(&v, ElementKind::Null), 0);
    }
}
