use core::mem::size_of;

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
#[non_exhaustive]
pub enum StringLiteral {
    None,
    Unspecified,
    Value(Atom),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
#[non_exhaustive]
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
    #[must_use]
    pub const fn is_numeric(self) -> bool {
        self.0 & Self::IS_NUMERIC != 0
    }

    #[inline]
    #[must_use]
    pub const fn is_truthy(self) -> bool {
        self.0 & Self::IS_TRUTHY != 0
    }

    #[inline]
    #[must_use]
    pub const fn is_non_empty(self) -> bool {
        self.0 & Self::IS_NON_EMPTY != 0
    }

    #[inline]
    #[must_use]
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

    /// Bitwise AND of two flag sets ; keeps a flag only when both sides
    /// have it. Used by the join's string-axis merge.
    #[inline]
    #[must_use]
    pub const fn and(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Bitwise OR of two flag sets ; keeps a flag when either side has
    /// it. Used by the meet's string-axis composition.
    #[inline]
    #[must_use]
    pub const fn or(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

// `StringLiteral` is the size driver: `Value(Atom)` is 8 bytes, plus 1 byte
// tag, padded to 16. Plus 1 byte casing + 1 byte flags = 18, padded to 24.
const _: () = assert!(size_of::<StringInfo>() <= 24, "size budget exceeded");
const _: () = assert!(size_of::<StringLiteral>() <= 16, "size budget exceeded");
const _: () = assert!(size_of::<StringRefinementFlags>() == 1, "size budget exceeded");

impl core::fmt::Display for StringInfo {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let label = match self.literal {
            StringLiteral::Value(value) => return write!(f, "string('{}')", value.as_str()),
            StringLiteral::Unspecified => label_literal_string(self),
            StringLiteral::None => label_general_string(self),
        };

        f.write_str(label)
    }
}

#[inline]
const fn label_literal_string(info: &StringInfo) -> &'static str {
    if info.flags.is_truthy() {
        if info.flags.is_numeric() {
            "truthy-numeric-literal-string"
        } else {
            match info.casing {
                StringCasing::Lowercase => "truthy-lowercase-literal-string",
                StringCasing::Uppercase => "truthy-uppercase-literal-string",
                StringCasing::Unspecified => "truthy-literal-string",
            }
        }
    } else if info.flags.is_numeric() {
        "numeric-literal-string"
    } else if info.flags.is_non_empty() {
        match info.casing {
            StringCasing::Lowercase => "lowercase-non-empty-literal-string",
            StringCasing::Uppercase => "uppercase-non-empty-literal-string",
            StringCasing::Unspecified => "non-empty-literal-string",
        }
    } else {
        match info.casing {
            StringCasing::Lowercase => "lowercase-literal-string",
            StringCasing::Uppercase => "uppercase-literal-string",
            StringCasing::Unspecified => "literal-string",
        }
    }
}

#[inline]
const fn label_general_string(info: &StringInfo) -> &'static str {
    if info.flags.is_callable() {
        return match info.casing {
            StringCasing::Lowercase => "lowercase-callable-string",
            StringCasing::Uppercase => "uppercase-callable-string",
            StringCasing::Unspecified => "callable-string",
        };
    }

    if info.flags.is_truthy() {
        if info.flags.is_numeric() {
            return "truthy-numeric-string";
        }
        return match info.casing {
            StringCasing::Lowercase => "truthy-lowercase-string",
            StringCasing::Uppercase => "truthy-uppercase-string",
            StringCasing::Unspecified => "truthy-string",
        };
    }

    if info.flags.is_numeric() {
        return "numeric-string";
    }

    if info.flags.is_non_empty() {
        return match info.casing {
            StringCasing::Lowercase => "lowercase-non-empty-string",
            StringCasing::Uppercase => "uppercase-non-empty-string",
            StringCasing::Unspecified => "non-empty-string",
        };
    }

    match info.casing {
        StringCasing::Lowercase => "lowercase-string",
        StringCasing::Uppercase => "uppercase-string",
        StringCasing::Unspecified => "string",
    }
}
