use std::num::NonZeroU32;

use crate::handle::define_handle;

/// An interned handle to a [`Type`](crate::Type) (a union of one or more
/// elements + flow flags).
///
/// Layout: 32 bits, niche-optimized via `NonZeroU32`. The integer is an opaque
/// slot index into the global type arena; equal `TypeId`s denote the same
/// canonical type.
///
/// Two `TypeId`s compare equal iff they refer to the same canonical
/// (atom-set, flow-flags) pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TypeId(NonZeroU32);

impl TypeId {
    /// Construct a `TypeId` from a raw slot index (1-based; slot `0` is reserved
    /// for the `NonZero` niche). Reserved for use by the interner.
    #[inline]
    pub(crate) const fn from_slot(slot: u32) -> Self {
        // SAFETY: caller is responsible for `slot != 0`. Used only by the interner.
        unsafe { Self(NonZeroU32::new_unchecked(slot)) }
    }

    #[inline]
    pub(crate) const fn slot(self) -> u32 {
        self.0.get()
    }

    /// Resolve this handle to its [`Type`](crate::Type) value via the
    /// process-global interner.
    ///
    /// # Panics
    ///
    /// Panics if the slot is not present, which can only happen when the
    /// handle was forged or constructed before the boot routine ran.
    //
    // Not implemented as `std::convert::AsRef<Type>` because the trait
    // narrows the return lifetime to `&self`'s borrow, defeating the whole
    // point: the underlying storage is genuinely process-global and the
    // `&'static` return is part of the API contract callers depend on.
    #[allow(clippy::should_implement_trait)]
    #[inline]
    pub fn as_ref(self) -> &'static crate::Type {
        crate::interner::interner().get_type(self)
    }

    /// Build a singleton union from one element, with empty flow flags.
    #[inline]
    pub fn singleton(element: crate::ElementId) -> Self {
        crate::interner::interner().intern_type(&[element], crate::FlowFlags::EMPTY)
    }

    /// Build a union from a slice of elements, with empty flow flags.
    /// Atoms are sorted, deduplicated, and (per the basic canonical form)
    /// empty input collapses to `never`.
    #[inline]
    pub fn union(elements: &[crate::ElementId]) -> Self {
        crate::interner::interner().intern_type(elements, crate::FlowFlags::EMPTY)
    }

    /// Singleton type wrapping a literal integer.
    #[inline]
    pub fn int_literal(value: i64) -> Self {
        Self::singleton(crate::ElementId::int_literal(value))
    }

    /// Singleton type wrapping an integer range. Either bound may be `None`
    /// for open (`-∞` / `+∞`).
    #[inline]
    pub fn int_range(lower: Option<i64>, upper: Option<i64>) -> Self {
        Self::singleton(crate::ElementId::int_range(lower, upper))
    }

    /// Singleton type wrapping a literal float.
    #[inline]
    pub fn float_literal(value: f64) -> Self {
        Self::singleton(crate::ElementId::float_literal(value))
    }

    /// Singleton type wrapping a literal string.
    #[inline]
    pub fn string_literal(value: &str) -> Self {
        Self::singleton(crate::ElementId::string_literal(value))
    }
}

define_handle! {
    /// Handle to an interned `&'static [TypeId]`. Used by payloads that
    /// carry a sequence of type arguments (object generic args, callable
    /// parameter type lists, conditional/derived input lists).
    TypeListId
}
