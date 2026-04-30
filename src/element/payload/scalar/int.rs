use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::mem::size_of;

use crate::handle::define_handle;
use crate::interner::interner;

define_handle! {
    /// Handle to an interned [`IntRange`]. Pulled out so [`IntInfo`] itself
    /// stays small. Most ranges in real worlds are well-known
    /// (`positive-int`, `non-zero-int`, etc.) and dedupe to one entry.
    IntRangeId
}

/// `int`, `literal-int`, integer literals, and bounded integer ranges.
///
/// `Range` carries an [`IntRangeId`] handle (not the bounds inline) so this
/// enum stays at 16 bytes per slot. Most ranges in real worlds are
/// well-known (`positive-int`, …) and dedupe to one entry in the
/// `IntRange` interner. Open complements like `int \ int(0)` are
/// expressed via the universal [`NegatedInfo`](crate::element::payload::NegatedInfo)
/// machinery (`int & !int(0)`) rather than a dedicated variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum IntInfo {
    Unspecified,
    UnspecifiedLiteral,
    Literal(i64),
    Range(IntRangeId),
}

/// A bounded integer range. Either bound may be open (±∞), recorded in
/// [`BoundFlags`]. When a bound is open, its accompanying value field is
/// canonically zeroed by the constructor so structural equality stays sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntRange {
    lower_value: i64,
    upper_value: i64,
    bounds: BoundFlags,
}

impl IntRange {
    /// Construct a range. `None` for either bound means open (±∞). The unused
    /// value field is canonicalized to `0` so two ranges with the same
    /// effective bounds always compare equal.
    #[inline]
    pub const fn new(lower: Option<i64>, upper: Option<i64>) -> Self {
        let mut bounds = BoundFlags::EMPTY;
        let lower_value = match lower {
            Some(v) => {
                bounds = bounds.with_has_lower(true);
                v
            }
            None => 0,
        };
        let upper_value = match upper {
            Some(v) => {
                bounds = bounds.with_has_upper(true);
                v
            }
            None => 0,
        };
        Self { lower_value, upper_value, bounds }
    }

    /// `Some(v)` if the range has a lower bound, `None` if open.
    #[inline]
    pub const fn lower(self) -> Option<i64> {
        if self.bounds.has_lower() { Some(self.lower_value) } else { None }
    }

    /// `Some(v)` if the range has an upper bound, `None` if open.
    #[inline]
    pub const fn upper(self) -> Option<i64> {
        if self.bounds.has_upper() { Some(self.upper_value) } else { None }
    }

    #[inline]
    pub const fn bounds(self) -> BoundFlags {
        self.bounds
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct BoundFlags(u8);

impl BoundFlags {
    pub const EMPTY: Self = Self(0);

    const HAS_LOWER: u8 = 1 << 0;
    const HAS_UPPER: u8 = 1 << 1;

    #[inline]
    pub const fn has_lower(self) -> bool {
        self.0 & Self::HAS_LOWER != 0
    }

    #[inline]
    pub const fn has_upper(self) -> bool {
        self.0 & Self::HAS_UPPER != 0
    }

    #[inline]
    #[must_use]
    pub const fn with_has_lower(self, on: bool) -> Self {
        Self(if on { self.0 | Self::HAS_LOWER } else { self.0 & !Self::HAS_LOWER })
    }

    #[inline]
    #[must_use]
    pub const fn with_has_upper(self, on: bool) -> Self {
        Self(if on { self.0 | Self::HAS_UPPER } else { self.0 & !Self::HAS_UPPER })
    }
}

const _: () = assert!(size_of::<IntInfo>() <= 16);
const _: () = assert!(size_of::<IntRange>() <= 24);

impl Display for IntInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            IntInfo::Unspecified => f.write_str("int"),
            IntInfo::UnspecifiedLiteral => f.write_str("literal-int"),
            IntInfo::Literal(n) => write!(f, "int({n})"),
            IntInfo::Range(rid) => Display::fmt(interner().get_int_range(*rid), f),
        }
    }
}

impl Display for IntRange {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match (self.lower(), self.upper()) {
            (Some(1), None) => f.write_str("positive-int"),
            (Some(0), None) => f.write_str("non-negative-int"),
            (Some(lo), None) => write!(f, "int<{lo}, max>"),
            (None, Some(-1)) => f.write_str("negative-int"),
            (None, Some(0)) => f.write_str("non-positive-int"),
            (None, Some(hi)) => write!(f, "int<min, {hi}>"),
            (Some(lo), Some(hi)) => write!(f, "int<{lo}, {hi}>"),
            (None, None) => f.write_str("int"),
        }
    }
}
