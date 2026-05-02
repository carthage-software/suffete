use core::fmt::Display;
use core::fmt::Formatter;
use core::fmt::Result as FmtResult;
use core::mem::size_of;

/// `mixed` and its narrowed forms (`non-null-mixed`, `truthy-mixed`,
/// `falsy-mixed`, `isset-from-loop`).
///
/// All five well-known `Mixed` slots differ only in their flag bits. Vanilla
/// `mixed` is `MixedInfo::EMPTY`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct MixedInfo(u8);

impl MixedInfo {
    pub const EMPTY: Self = Self(0);

    const IS_NON_NULL: u8 = 1 << 0;
    const IS_EMPTY: u8 = 1 << 1;
    const IS_TRUTHY: u8 = 1 << 2;
    const IS_FALSY: u8 = 1 << 3;
    const IS_ISSET_FROM_LOOP: u8 = 1 << 4;

    /// Truthiness is one of three states encoded in two bits
    /// (`IS_TRUTHY` / `IS_FALSY`):
    /// - both clear → `Undetermined`
    /// - `IS_TRUTHY` set → `Truthy`
    /// - `IS_FALSY` set → `Falsy`
    ///
    /// Both bits set simultaneously is invalid; constructors reject it.
    #[inline]
    #[must_use] 
    pub const fn truthiness(self) -> Truthiness {
        match (self.0 & Self::IS_TRUTHY != 0, self.0 & Self::IS_FALSY != 0) {
            (false, false) => Truthiness::Undetermined,
            (true, false) => Truthiness::Truthy,
            (false, true) => Truthiness::Falsy,
            (true, true) => Truthiness::Undetermined, // unreachable in practice
        }
    }

    #[inline]
    #[must_use] 
    pub const fn is_non_null(self) -> bool {
        self.0 & Self::IS_NON_NULL != 0
    }

    #[inline]
    #[must_use] 
    pub const fn is_empty(self) -> bool {
        self.0 & Self::IS_EMPTY != 0
    }

    #[inline]
    #[must_use] 
    pub const fn is_isset_from_loop(self) -> bool {
        self.0 & Self::IS_ISSET_FROM_LOOP != 0
    }

    #[inline]
    #[must_use]
    pub const fn with_is_non_null(self, on: bool) -> Self {
        Self(if on { self.0 | Self::IS_NON_NULL } else { self.0 & !Self::IS_NON_NULL })
    }

    #[inline]
    #[must_use]
    pub const fn with_is_empty(self, on: bool) -> Self {
        Self(if on { self.0 | Self::IS_EMPTY } else { self.0 & !Self::IS_EMPTY })
    }

    #[inline]
    #[must_use]
    pub const fn with_truthiness(self, t: Truthiness) -> Self {
        let cleared = self.0 & !(Self::IS_TRUTHY | Self::IS_FALSY);
        let set = match t {
            Truthiness::Undetermined => 0,
            Truthiness::Truthy => Self::IS_TRUTHY,
            Truthiness::Falsy => Self::IS_FALSY,
        };
        Self(cleared | set)
    }

    #[inline]
    #[must_use]
    pub const fn with_is_isset_from_loop(self, on: bool) -> Self {
        Self(if on { self.0 | Self::IS_ISSET_FROM_LOOP } else { self.0 & !Self::IS_ISSET_FROM_LOOP })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Truthiness {
    #[default]
    Undetermined,
    Truthy,
    Falsy,
}

const _: () = assert!(size_of::<MixedInfo>() == 1, "size budget exceeded");

impl Display for MixedInfo {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let label = if self.is_empty() {
            match self.truthiness() {
                Truthiness::Truthy => "empty-truthy-mixed",
                Truthiness::Falsy => "empty-falsy-mixed",
                Truthiness::Undetermined if self.is_non_null() => "empty-nonnull",
                Truthiness::Undetermined => "empty-mixed",
            }
        } else {
            match self.truthiness() {
                Truthiness::Truthy => "truthy-mixed",
                Truthiness::Falsy => "falsy-mixed",
                Truthiness::Undetermined if self.is_non_null() => "nonnull",
                Truthiness::Undetermined => "mixed",
            }
        };
        f.write_str(label)
    }
}
