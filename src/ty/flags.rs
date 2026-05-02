/// Provenance and analysis-state bits attached to a [`Type`](crate::Type).
///
/// Flow flags do not participate in the denotational meaning of a
/// type: two unions with identical elements but different flags
/// inhabit the same set of values. They affect diagnostics,
/// narrowing, and substitution.
///
/// Flow flags **do** participate in the [`TypeId`](crate::TypeId) interner
/// key: two unions with identical elements but different flags get different
/// `TypeId`s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FlowFlags(u16);

impl FlowFlags {
    pub const EMPTY: Self = Self(0);

    const HAD_TEMPLATE: u16 = 1 << 0;
    const FROM_TEMPLATE_DEFAULT: u16 = 1 << 1;
    const POPULATED: u16 = 1 << 2;
    const POSSIBLY_UNDEFINED: u16 = 1 << 3;
    const POSSIBLY_UNDEFINED_FROM_TRY: u16 = 1 << 4;
    const IGNORE_NULLABLE_ISSUES: u16 = 1 << 5;
    const IGNORE_FALSABLE_ISSUES: u16 = 1 << 6;
    const NULLSAFE_NULL: u16 = 1 << 7;
    const BY_REFERENCE: u16 = 1 << 8;
    const REFERENCE_FREE: u16 = 1 << 9;

    #[inline]
    #[must_use]
    pub const fn bits(self) -> u16 {
        self.0
    }

    #[inline]
    #[must_use]
    pub const fn from_bits(bits: u16) -> Self {
        Self(bits)
    }
}

macro_rules! flag_accessors {
    ($($getter:ident, $setter:ident, $bit:ident);* $(;)?) => {
        impl FlowFlags {
            $(
                #[inline]
                pub const fn $getter(self) -> bool {
                    self.0 & Self::$bit != 0
                }

                #[inline]
                #[must_use]
                pub const fn $setter(self, on: bool) -> Self {
                    if on { Self(self.0 | Self::$bit) } else { Self(self.0 & !Self::$bit) }
                }
            )*
        }
    };
}

flag_accessors! {
    had_template, with_had_template, HAD_TEMPLATE;
    from_template_default, with_from_template_default, FROM_TEMPLATE_DEFAULT;
    populated, with_populated, POPULATED;
    possibly_undefined, with_possibly_undefined, POSSIBLY_UNDEFINED;
    possibly_undefined_from_try, with_possibly_undefined_from_try, POSSIBLY_UNDEFINED_FROM_TRY;
    ignore_nullable_issues, with_ignore_nullable_issues, IGNORE_NULLABLE_ISSUES;
    ignore_falsable_issues, with_ignore_falsable_issues, IGNORE_FALSABLE_ISSUES;
    nullsafe_null, with_nullsafe_null, NULLSAFE_NULL;
    by_reference, with_by_reference, BY_REFERENCE;
    reference_free, with_reference_free, REFERENCE_FREE;
}
