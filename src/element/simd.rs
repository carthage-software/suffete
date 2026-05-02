//! SIMD-accelerated scans over `&[ElementId]` slices.
//!
//! `ElementId` is a `NonZeroU32` newtype with the [`ElementKind`] tag in
//! the high 6 bits and the per-kind arena slot in the low 26 bits.
//! Slices of `ElementId` are therefore contiguous 32-bit lanes ; ideal
//! substrate for the wide-element-set scans the lattice and join layers
//! perform on every operation.
//!
//! Three primitives:
//!
//! - [`contains`]: equality scan ; "is `needle` anywhere in `slice`?"
//! - [`any_of_kind`]: kind-only scan ; "does any element have kind
//!   `kind`?"
//! - [`count_of_kind`]: counts elements of a given kind, useful for
//!   the literal-collapse threshold checks in `join`.
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
fn any_of_kind_scalar(slice: &[ElementId], kind: ElementKind) -> bool {
    slice.iter().any(|e| e.kind() == kind)
}

#[inline]
fn count_of_kind_scalar(slice: &[ElementId], kind: ElementKind) -> usize {
    slice.iter().filter(|e| e.kind() == kind).count()
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
#[inline]
unsafe fn contains_avx2(slice: &[ElementId], needle: ElementId) -> bool {
    use core::arch::x86_64::_mm256_cmpeq_epi32;
    use core::arch::x86_64::_mm256_loadu_si256;
    use core::arch::x86_64::_mm256_movemask_epi8;
    use core::arch::x86_64::_mm256_set1_epi32;

    // SAFETY: caller guarantees slice.len() >= SIMD_THRESHOLD (8).
    let target = unsafe { _mm256_set1_epi32(needle.raw() as i32) };
    let chunks = slice.len() / 8;
    let base = slice.as_ptr().cast::<core::arch::x86_64::__m256i>();
    for i in 0..chunks {
        // SAFETY: i in 0..chunks; base + i*32 is in-bounds for the slice.
        unsafe {
            let v = _mm256_loadu_si256(base.add(i));
            let cmp = _mm256_cmpeq_epi32(v, target);
            if _mm256_movemask_epi8(cmp) != 0 {
                return true;
            }
        }
    }

    // Tail: scalar over remainder. `chunks * 8` cannot exceed
    // `slice.len()` and is upper-bounded by `usize::MAX / 8`, so the
    // multiplication never overflows on any addressable slice.
    let tail_start = chunks.saturating_mul(8);
    contains_scalar(&slice[tail_start..], needle)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
#[inline]
unsafe fn any_of_kind_avx2(slice: &[ElementId], kind: ElementKind) -> bool {
    use core::arch::x86_64::_mm256_cmpeq_epi32;
    use core::arch::x86_64::_mm256_loadu_si256;
    use core::arch::x86_64::_mm256_movemask_epi8;
    use core::arch::x86_64::_mm256_set1_epi32;
    use core::arch::x86_64::_mm256_srli_epi32;

    // SAFETY: caller guarantees slice.len() >= AVX2_THRESHOLD (8).
    // Shift count is the const-generic `KIND_TAG_SHIFT`; the
    // compile-time assert pins it to `ElementId::SLOT_BITS`.
    let target = unsafe { _mm256_set1_epi32(kind as i32) };
    let chunks = slice.len() / 8;
    let base = slice.as_ptr().cast::<core::arch::x86_64::__m256i>();
    for i in 0..chunks {
        // SAFETY: i in 0..chunks.
        unsafe {
            let v = _mm256_loadu_si256(base.add(i));
            let kinds = _mm256_srli_epi32::<{ KIND_TAG_SHIFT as i32 }>(v);
            let cmp = _mm256_cmpeq_epi32(kinds, target);
            if _mm256_movemask_epi8(cmp) != 0 {
                return true;
            }
        }
    }

    let tail_start = chunks.saturating_mul(8);
    any_of_kind_scalar(&slice[tail_start..], kind)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
#[inline]
unsafe fn count_of_kind_avx2(slice: &[ElementId], kind: ElementKind) -> usize {
    use core::arch::x86_64::_mm256_cmpeq_epi32;
    use core::arch::x86_64::_mm256_loadu_si256;
    use core::arch::x86_64::_mm256_movemask_epi8;
    use core::arch::x86_64::_mm256_set1_epi32;
    use core::arch::x86_64::_mm256_srli_epi32;

    // SAFETY: caller guarantees slice.len() >= AVX2_THRESHOLD (8).
    let target = unsafe { _mm256_set1_epi32(kind as i32) };
    let chunks = slice.len() / 8;
    let base = slice.as_ptr().cast::<core::arch::x86_64::__m256i>();
    let mut total: u32 = 0;
    for i in 0..chunks {
        // SAFETY: i in 0..chunks.
        unsafe {
            let v = _mm256_loadu_si256(base.add(i));
            let kinds = _mm256_srli_epi32::<{ KIND_TAG_SHIFT as i32 }>(v);
            let cmp = _mm256_cmpeq_epi32(kinds, target);
            // Each matching 32-bit lane sets four 8-bit positions in the
            // movemask result. Counting set bits and dividing by 4 gives
            // the lane count.
            let mask = _mm256_movemask_epi8(cmp) as u32;
            total = total.wrapping_add(mask.count_ones() / 4);
        }
    }

    let tail_start = chunks.saturating_mul(8);
    (total as usize).saturating_add(count_of_kind_scalar(&slice[tail_start..], kind))
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
    fn count_of_kind_long_slice_with_partial_chunks() {
        let mut v = vec![INT; 8];
        v.extend([STRING; 8]);
        v.push(INT);
        assert_eq!(count_of_kind(&v, ElementKind::Int), 9);
        assert_eq!(count_of_kind(&v, ElementKind::String), 8);
        assert_eq!(count_of_kind(&v, ElementKind::Null), 0);
    }
}
