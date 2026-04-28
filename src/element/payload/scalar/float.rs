use std::mem::size_of;

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
    pub const fn new(value: f64) -> Self {
        Self(OrderedFloat(value))
    }

    #[inline]
    pub const fn value(self) -> f64 {
        self.0.0
    }
}

const _: () = assert!(size_of::<FloatInfo>() <= 16);
const _: () = assert!(size_of::<LiteralFloat>() == 8);
