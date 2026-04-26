use std::num::NonZeroU32;

use crate::ElementKind;
use crate::handle::define_handle;

/// An interned handle to a single [`Element`](crate::Element).
///
/// Layout: 32 bits, niche-optimized via `NonZeroU32`. The high 6 bits hold the
/// [`ElementKind`] tag (1..=63). The low 26 bits hold the per-kind arena slot
/// (0..=2^26-1, ≈67M).
///
/// Two `ElementId`s compare equal iff they refer to the same canonical
/// element; this is the interner's contract. Equality is one `u32` compare,
/// hashing is trivial.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ElementId(NonZeroU32);

impl ElementId {
    const KIND_BITS: u32 = 6;
    const SLOT_BITS: u32 = u32::BITS - Self::KIND_BITS;
    const SLOT_MASK: u32 = (1u32 << Self::SLOT_BITS) - 1;

    /// Maximum addressable slot per kind. Each per-kind arena tops out here.
    pub const MAX_SLOT: u32 = Self::SLOT_MASK;

    /// Construct an `ElementId` from a kind and slot. `slot` must fit in
    /// [`Self::MAX_SLOT`]; in release builds this is unchecked.
    #[inline]
    pub const fn new(kind: ElementKind, slot: u32) -> Self {
        debug_assert!(slot <= Self::MAX_SLOT, "element slot overflow");
        let raw = ((kind as u32) << Self::SLOT_BITS) | (slot & Self::SLOT_MASK);
        // SAFETY: `kind as u32 >= 1` (discriminants start at 1), so the shifted
        // kind contributes a non-zero high bit, making the whole value non-zero.
        unsafe { Self(NonZeroU32::new_unchecked(raw)) }
    }

    #[inline]
    pub const fn kind(self) -> ElementKind {
        let tag = (self.0.get() >> Self::SLOT_BITS) as u8;
        // SAFETY: every `ElementId` is constructed from a valid `ElementKind`
        // discriminant (1..=63 fits in 6 bits) via `Self::new`.
        unsafe { std::mem::transmute::<u8, ElementKind>(tag) }
    }

    #[inline]
    pub const fn slot(self) -> u32 {
        self.0.get() & Self::SLOT_MASK
    }
}

define_handle! {
    /// Handle to an interned `&'static [ElementId]`. Used by payloads that
    /// carry a sequence of elements (object intersections, iterable
    /// intersections, etc.).
    ElementListId
}
