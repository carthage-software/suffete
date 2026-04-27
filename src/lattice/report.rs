use std::ops::BitOr;
use std::ops::BitOrAssign;

use crate::TypeId;

/// Diagnostic output from the lattice operations
/// ([`refines`](crate::lattice::refines),
/// [`generalizes`](crate::lattice::generalizes),
/// [`overlaps`](crate::lattice::overlaps), [`crate::meet::compute`],
/// [`crate::subtract::compute`]).
///
/// Operations return `bool` / `TypeId`; this struct carries the *why*.
/// Callers pass `&mut LatticeReport` and read the fields after the call to
/// learn whether a coercion fired (and which kind), and what type the input
/// would have needed to be for the answer to flip cleanly.
///
/// Eight bytes wide (one byte of cause flags, niche-packed `Option<TypeId>`,
/// padding); pass by `&mut`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct LatticeReport {
    /// The set of coercion patterns the operation observed. Multiple flags
    /// may be set in a single call (e.g. a nested-mixed input that also
    /// triggered a true-union narrowing).
    pub causes: CoercionCauses,
    /// The smallest type that, substituted for the input, would have made
    /// the comparison succeed cleanly (no coercion). `None` when no rule
    /// could compute one.
    pub replacement: Option<TypeId>,
}

impl LatticeReport {
    /// A fresh report with no causes recorded and no replacement.
    #[inline]
    pub const fn new() -> Self {
        Self { causes: CoercionCauses::NONE, replacement: None }
    }

    /// Record a coercion cause without disturbing other fields.
    #[inline]
    pub fn add_cause(&mut self, cause: CoercionCauses) {
        self.causes |= cause;
    }

    /// Record a replacement type; subsequent calls overwrite. Use only
    /// when the rule has high confidence the replacement is the right
    /// fix-suggestion; otherwise leave it `None`.
    #[inline]
    pub fn set_replacement(&mut self, ty: TypeId) {
        self.replacement = Some(ty);
    }

    /// `true` iff at least one coercion cause was recorded.
    #[inline]
    pub const fn coerced(self) -> bool {
        self.causes.any()
    }
}

/// A bitset of coercion patterns the lattice observed during a comparison.
///
/// Each constant is a single bit; combine with `|` and test with
/// [`CoercionCauses::contains`]. Eight bits today, room for two more
/// without growing the storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CoercionCauses(u8);

impl CoercionCauses {
    /// Empty set.
    pub const NONE: Self = Self(0);

    /// The input contained a `mixed` somewhere that the container
    /// constrained: e.g. `array<string, mixed>` flowing into
    /// `array<string, int>`. Distinct from a top-level `mixed` because the
    /// programmer's mental model is "I have an array of values" rather
    /// than "I have a top-typed thing".
    pub const NESTED_MIXED: Self = Self(1 << 0);

    /// A "true union" element-kind (`mixed`, `array_key`, `bool`,
    /// `object_any`, `scalar`, `numeric`) was narrowed to one of its
    /// concrete subforms by the container. This is the standard PHP
    /// pattern: the input *could* be the right thing at runtime, but the
    /// type system can't prove it.
    pub const TRUE_UNION_NARROW: Self = Self(1 << 1);

    /// PHP's runtime would coerce the input to fit (e.g. `int -> float`).
    /// Distinct from the other causes because the coercion is silent at
    /// runtime — the programmer may not realise it happened.
    pub const PHP_RUNTIME_COERCE: Self = Self(1 << 2);

    /// A literal-shaped value was accepted where its general form was
    /// expected, or vice versa. Reserved for rules that promote
    /// `Literal(5)` into `int` or accept `int` into `LITERAL_INT` slots.
    pub const LITERAL_PROMOTED: Self = Self(1 << 3);

    /// A generic position was filled with its declared default rather
    /// than an explicit type-argument. Variance checks must skip the
    /// reverse direction for default-filled positions; see report §8.
    pub const TEMPLATE_DEFAULT: Self = Self(1 << 4);

    /// `object` (the unspecified-class element) was accepted where a
    /// concrete class was expected. Unsound in general; the consumer may
    /// want to warn.
    pub const OBJECT_ANY_DOWN: Self = Self(1 << 5);

    /// `true` iff no causes are set.
    #[inline]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// `true` iff at least one cause is set.
    #[inline]
    pub const fn any(self) -> bool {
        self.0 != 0
    }

    /// `true` iff every bit in `other` is set in `self`.
    #[inline]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Add the bits in `other` in-place.
    #[inline]
    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    /// Clear the bits in `other` in-place.
    #[inline]
    pub fn remove(&mut self, other: Self) {
        self.0 &= !other.0;
    }

    /// Convenience: the input contained a nested `mixed`.
    #[inline]
    pub const fn nested_mixed(self) -> bool {
        self.contains(Self::NESTED_MIXED)
    }

    /// Convenience: a true-union kind was narrowed.
    #[inline]
    pub const fn true_union_narrow(self) -> bool {
        self.contains(Self::TRUE_UNION_NARROW)
    }

    /// Convenience: PHP would coerce the input at runtime.
    #[inline]
    pub const fn php_runtime_coerce(self) -> bool {
        self.contains(Self::PHP_RUNTIME_COERCE)
    }

    /// Convenience: a literal was promoted to its general form.
    #[inline]
    pub const fn literal_promoted(self) -> bool {
        self.contains(Self::LITERAL_PROMOTED)
    }

    /// Convenience: a generic position was default-filled.
    #[inline]
    pub const fn template_default(self) -> bool {
        self.contains(Self::TEMPLATE_DEFAULT)
    }

    /// Convenience: `object` was accepted in a concrete-class slot.
    #[inline]
    pub const fn object_any_down(self) -> bool {
        self.contains(Self::OBJECT_ANY_DOWN)
    }
}

impl BitOr for CoercionCauses {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for CoercionCauses {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}
