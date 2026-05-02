use core::mem::size_of;

use mago_atom::Atom;

use crate::TypeId;
use crate::handle::define_handle;

define_handle! {
    /// Handle to an interned [`Signature`].
    SignatureId
}

define_handle! {
    /// Handle to an interned `&'static [ParamInfo]`,
    /// a [`Signature`]'s parameter list.
    ParamListId
}

/// Full signature info for an anonymous callable or closure.
///
/// `parameters = None` means the signature is unspecified (e.g. `callable`
/// with no `(...)`). `throws = None` means no annotation, not "throws nothing".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Signature {
    pub parameters: Option<ParamListId>,
    pub return_type: TypeId,
    pub throws: Option<TypeId>,
    pub flags: SignatureFlags,
}

/// One parameter inside a [`Signature`]. Carries enough for subtyping
/// (contravariant on type, name match for keyed-arg dispatch) and for
/// diagnostics (default presence, by-reference, variadic).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParamInfo {
    pub name: Atom,
    pub type_: TypeId,
    pub flags: ParamFlags,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SignatureFlags(u8);

impl SignatureFlags {
    pub const EMPTY: Self = Self(0);

    /// `true` when the signature accepts unlimited extra arguments at the end
    /// (i.e. its last parameter is variadic). Cached on the signature so
    /// callers don't need to walk the param list.
    const IS_VARIADIC: u8 = 1 << 0;

    /// `true` for `pure-callable(...)`.
    const IS_PURE: u8 = 1 << 1;

    #[inline]
    #[must_use] 
    pub const fn is_variadic(self) -> bool {
        self.0 & Self::IS_VARIADIC != 0
    }

    #[inline]
    #[must_use] 
    pub const fn is_pure(self) -> bool {
        self.0 & Self::IS_PURE != 0
    }

    #[inline]
    #[must_use]
    pub const fn with_is_variadic(self, on: bool) -> Self {
        Self(if on { self.0 | Self::IS_VARIADIC } else { self.0 & !Self::IS_VARIADIC })
    }

    #[inline]
    #[must_use]
    pub const fn with_is_pure(self, on: bool) -> Self {
        Self(if on { self.0 | Self::IS_PURE } else { self.0 & !Self::IS_PURE })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ParamFlags(u8);

impl ParamFlags {
    pub const EMPTY: Self = Self(0);

    const HAS_DEFAULT: u8 = 1 << 0;
    const BY_REFERENCE: u8 = 1 << 1;
    const VARIADIC: u8 = 1 << 2;

    #[inline]
    #[must_use] 
    pub const fn has_default(self) -> bool {
        self.0 & Self::HAS_DEFAULT != 0
    }

    #[inline]
    #[must_use] 
    pub const fn by_reference(self) -> bool {
        self.0 & Self::BY_REFERENCE != 0
    }

    #[inline]
    #[must_use] 
    pub const fn variadic(self) -> bool {
        self.0 & Self::VARIADIC != 0
    }

    #[inline]
    #[must_use]
    pub const fn with_has_default(self, on: bool) -> Self {
        Self(if on { self.0 | Self::HAS_DEFAULT } else { self.0 & !Self::HAS_DEFAULT })
    }

    #[inline]
    #[must_use]
    pub const fn with_by_reference(self, on: bool) -> Self {
        Self(if on { self.0 | Self::BY_REFERENCE } else { self.0 & !Self::BY_REFERENCE })
    }

    #[inline]
    #[must_use]
    pub const fn with_variadic(self, on: bool) -> Self {
        Self(if on { self.0 | Self::VARIADIC } else { self.0 & !Self::VARIADIC })
    }
}

const _: () = assert!(size_of::<Signature>() <= 24, "size budget exceeded");
const _: () = assert!(size_of::<ParamInfo>() <= 24, "size budget exceeded");
const _: () = assert!(size_of::<SignatureFlags>() == 1, "size budget exceeded");
const _: () = assert!(size_of::<ParamFlags>() == 1, "size budget exceeded");
