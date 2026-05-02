use core::ops::BitOr;
use core::ops::BitOrAssign;

use crate::ElementId;
use crate::TypeId;
use crate::template::Bound;
use crate::template::TemplateKey;

/// Diagnostic output from the lattice operations.
///
/// Produced by [`refines`](crate::lattice::refines),
/// [`generalizes`](crate::lattice::generalizes),
/// [`overlaps`](crate::lattice::overlaps), [`crate::meet::compute`],
/// and [`crate::subtract::compute`].
///
/// Operations return `bool` / `TypeId`; this struct carries the *why*.
/// Callers pass `&mut LatticeReport` and read the fields after the call.
///
/// Empty `bounds` (the default) is allocation-free; the alloc is paid only
/// when a rule pushes a bound.
#[derive(Debug, Clone, Default)]
pub struct LatticeReport {
    /// The set of coercion patterns the operation observed. Multiple flags
    /// may be set in a single call (e.g. a nested-mixed input that also
    /// triggered a true-union narrowing).
    pub causes: CoercionCauses,
    /// The smallest *type* that, substituted for the input, would have made
    /// the comparison succeed cleanly (no coercion). `None` when no rule
    /// could compute one. Use this when reporting on the whole union.
    pub replacement: Option<TypeId>,
    /// The single problematic *element* in the input, when only one atom
    /// of a wider union was at fault. The reconciler swaps just this atom
    /// rather than rebuilding the entire union. `None` when the issue is
    /// at the union level (or no replacement is computable).
    pub replacement_element: Option<ElementId>,
    /// Bounds on free template parameters that surfaced during the
    /// comparison itself (distinct from a separate inference pass through
    /// [`crate::template::standin`]). Each entry tags which template the
    /// bound applies to and whether it is a Lower / Upper / Equality
    /// constraint. Empty in the common case.
    pub bounds: Vec<(TemplateKey, Bound)>,
}

impl LatticeReport {
    /// A fresh report with no causes, no replacements, and no bounds.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a coercion cause without disturbing other fields.
    #[inline]
    pub fn add_cause(&mut self, cause: CoercionCauses) {
        self.causes |= cause;
    }

    /// Record a union-level replacement; subsequent calls overwrite.
    #[inline]
    pub const fn set_replacement(&mut self, ty: TypeId) {
        self.replacement = Some(ty);
    }

    /// Record an element-level replacement; subsequent calls overwrite.
    #[inline]
    pub const fn set_replacement_element(&mut self, elem: ElementId) {
        self.replacement_element = Some(elem);
    }

    /// Record a bound surfaced for `key`.
    #[inline]
    pub fn push_bound(&mut self, key: TemplateKey, bound: Bound) {
        self.bounds.push((key, bound));
    }

    /// `true` iff at least one coercion cause was recorded.
    #[inline]
    #[must_use]
    pub const fn coerced(&self) -> bool {
        self.causes.any()
    }
}

/// A bitset of coercion patterns the lattice observed during a comparison.
///
/// Each constant is a single bit; combine with `|` and test with
/// [`CoercionCauses::contains`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CoercionCauses(u8);

impl CoercionCauses {
    /// Empty set.
    pub const NONE: Self = Self(0);

    /// The input contained a `mixed` somewhere that the container
    /// constrained: e.g. `array<string, mixed>` flowing into
    /// `array<string, int>`. Distinct from a top-level `mixed` because
    /// the programmer's mental model is "I have an array of values"
    /// rather than "I have a top-typed thing".
    pub const NESTED_MIXED: Self = Self(1 << 0);

    /// The input was a generic parameter whose constraint is `mixed`
    /// (`@template T` with no `of` clause, or `@template T of mixed`).
    /// Distinct from [`NESTED_MIXED`](Self::NESTED_MIXED): the fix
    /// suggestion is "tighten the template constraint", not "stop
    /// shoving `mixed` into the container".
    pub const FROM_AS_MIXED: Self = Self(1 << 1);

    /// A "true union" element-kind (`mixed`, `array_key`, `bool`,
    /// `object_any`, `scalar`, `numeric`) was narrowed to one of its
    /// concrete subforms by the container. The standard PHP pattern: the
    /// input *could* be the right thing at runtime, but the type system
    /// can't prove it.
    pub const TRUE_UNION_NARROW: Self = Self(1 << 2);

    /// PHP's runtime would coerce the input to fit (e.g. `int -> float`).
    /// Distinct from the other causes because the coercion is silent at
    /// runtime: the programmer may not realise it happened.
    pub const PHP_RUNTIME_COERCE: Self = Self(1 << 3);

    /// A literal-shaped value was accepted where its general form was
    /// expected, or vice versa. Reserved for rules that promote
    /// `Literal(5)` into `int` or accept `int` into `LITERAL_INT` slots.
    pub const LITERAL_PROMOTED: Self = Self(1 << 4);

    /// A generic position was filled with its declared default rather
    /// than an explicit type-argument. Variance checks must skip the
    /// reverse direction for default-filled positions.
    pub const TEMPLATE_DEFAULT: Self = Self(1 << 5);

    /// `object` (the unspecified-class element) was accepted where a
    /// concrete class was expected. Unsound in general; the consumer may
    /// want to warn.
    pub const OBJECT_ANY_DOWN: Self = Self(1 << 6);

    /// `true` iff no causes are set.
    #[inline]
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// `true` iff at least one cause is set.
    #[inline]
    #[must_use]
    pub const fn any(self) -> bool {
        self.0 != 0
    }

    /// `true` iff every bit in `other` is set in `self`.
    #[inline]
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Add the bits in `other` in-place.
    #[inline]
    pub const fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    /// Clear the bits in `other` in-place.
    #[inline]
    pub const fn remove(&mut self, other: Self) {
        self.0 &= !other.0;
    }

    /// Convenience: the input contained a nested `mixed`.
    #[inline]
    #[must_use]
    pub const fn nested_mixed(self) -> bool {
        self.contains(Self::NESTED_MIXED)
    }

    /// Convenience: the input was a generic parameter constrained to
    /// `mixed`.
    #[inline]
    #[must_use]
    pub const fn from_as_mixed(self) -> bool {
        self.contains(Self::FROM_AS_MIXED)
    }

    /// Convenience: a true-union kind was narrowed.
    #[inline]
    #[must_use]
    pub const fn true_union_narrow(self) -> bool {
        self.contains(Self::TRUE_UNION_NARROW)
    }

    /// Convenience: PHP would coerce the input at runtime.
    #[inline]
    #[must_use]
    pub const fn php_runtime_coerce(self) -> bool {
        self.contains(Self::PHP_RUNTIME_COERCE)
    }

    /// Convenience: a literal was promoted to its general form.
    #[inline]
    #[must_use]
    pub const fn literal_promoted(self) -> bool {
        self.contains(Self::LITERAL_PROMOTED)
    }

    /// Convenience: a generic position was default-filled.
    #[inline]
    #[must_use]
    pub const fn template_default(self) -> bool {
        self.contains(Self::TEMPLATE_DEFAULT)
    }

    /// Convenience: `object` was accepted in a concrete-class slot.
    #[inline]
    #[must_use]
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
