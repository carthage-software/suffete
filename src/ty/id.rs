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
}

define_handle! {
    /// Handle to an interned `&'static [TypeId]`. Used by payloads that
    /// carry a sequence of type arguments (object generic args, callable
    /// parameter type lists, conditional/derived input lists).
    TypeListId
}
