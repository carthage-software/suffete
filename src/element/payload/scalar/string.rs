use std::mem::size_of;

use mago_atom::Atom;

/// PHP `string` and its many refinements: `non-empty-string`, `truthy-string`,
/// `lowercase-string`, `numeric-string`, `callable-string`, literal values,
/// and combinations thereof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StringInfo {
    pub literal: StringLiteral,
    pub casing: StringCasing,
    pub flags: StringRefinementFlags,
}

/// Three states for the literal-value field: no literal info, came-from-a-
/// literal-source-but-value-unknown, or exact value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StringLiteral {
    None,
    Unspecified,
    Value(Atom),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum StringCasing {
    #[default]
    Unspecified,
    Lowercase,
    Uppercase,
}

/// Boolean refinements that stack: a string can be both `non-empty` and
/// `truthy` and `numeric`, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct StringRefinementFlags(u8);

impl StringRefinementFlags {
    pub const EMPTY: Self = Self(0);

    const IS_NUMERIC: u8 = 1 << 0;
    const IS_TRUTHY: u8 = 1 << 1;
    const IS_NON_EMPTY: u8 = 1 << 2;
    const IS_CALLABLE: u8 = 1 << 3;

    #[inline]
    pub const fn is_numeric(self) -> bool {
        self.0 & Self::IS_NUMERIC != 0
    }

    #[inline]
    pub const fn is_truthy(self) -> bool {
        self.0 & Self::IS_TRUTHY != 0
    }

    #[inline]
    pub const fn is_non_empty(self) -> bool {
        self.0 & Self::IS_NON_EMPTY != 0
    }

    #[inline]
    pub const fn is_callable(self) -> bool {
        self.0 & Self::IS_CALLABLE != 0
    }

    #[inline]
    #[must_use]
    pub const fn with_is_numeric(self, on: bool) -> Self {
        Self(if on { self.0 | Self::IS_NUMERIC } else { self.0 & !Self::IS_NUMERIC })
    }

    #[inline]
    #[must_use]
    pub const fn with_is_truthy(self, on: bool) -> Self {
        Self(if on { self.0 | Self::IS_TRUTHY } else { self.0 & !Self::IS_TRUTHY })
    }

    #[inline]
    #[must_use]
    pub const fn with_is_non_empty(self, on: bool) -> Self {
        Self(if on { self.0 | Self::IS_NON_EMPTY } else { self.0 & !Self::IS_NON_EMPTY })
    }

    #[inline]
    #[must_use]
    pub const fn with_is_callable(self, on: bool) -> Self {
        Self(if on { self.0 | Self::IS_CALLABLE } else { self.0 & !Self::IS_CALLABLE })
    }
}

// `StringLiteral` is the size driver: `Value(Atom)` is 8 bytes, plus 1 byte
// tag, padded to 16. Plus 1 byte casing + 1 byte flags = 18, padded to 24.
const _: () = assert!(size_of::<StringInfo>() <= 24);
const _: () = assert!(size_of::<StringLiteral>() <= 16);
const _: () = assert!(size_of::<StringRefinementFlags>() == 1);
