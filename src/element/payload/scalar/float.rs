use core::mem::size_of;

use ordered_float::OrderedFloat;

/// `float`, `literal-float`, and float literals (including `INF`, `NAN`).
///
/// `Literal` wraps `OrderedFloat<f64>` so the payload implements `Eq` and
/// `Hash`. NaN compares equal to NaN at the *type* level here; runtime
/// `is_nan` semantics are an analyzer concern, not the type system's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FloatInfo {
    Unspecified,
    UnspecifiedLiteral,
    Literal(LiteralFloat),
}

/// Newtype around `OrderedFloat<f64>` so the public API doesn't leak the
/// `ordered_float` crate at every callsite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LiteralFloat(pub OrderedFloat<f64>);

impl LiteralFloat {
    #[inline]
    #[must_use] 
    pub const fn new(value: f64) -> Self {
        Self(OrderedFloat(value))
    }

    #[inline]
    #[must_use] 
    pub const fn value(self) -> f64 {
        self.0.0
    }
}

const _: () = assert!(size_of::<FloatInfo>() <= 16, "size budget exceeded");
const _: () = assert!(size_of::<LiteralFloat>() == 8, "size budget exceeded");

impl core::fmt::Display for FloatInfo {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FloatInfo::Unspecified => f.write_str("float"),
            FloatInfo::UnspecifiedLiteral => f.write_str("literal-float"),
            FloatInfo::Literal(lit) => write!(f, "float({})", lit.value()),
        }
    }
}
