/// Diagnostic output from the lattice operations
/// ([`refines`](crate::lattice::refines),
/// [`generalizes`](crate::lattice::generalizes),
/// [`intersects`](crate::lattice::intersects)).
///
/// The operations return a `bool`; this struct carries the *why*. Callers
/// pass `&mut LatticeReport` and read the fields after the call to learn
/// whether a `false` answer was a clean rejection or a coercion-style
/// near-miss, whether a `true` answer involved an implicit narrowing,
/// and so on.
///
/// All fields are `Option<bool>` so "not relevant for this query" (None)
/// is distinguishable from a concrete answer. New fields land as each
/// rule family contributes its own coercion patterns.
///
/// `LatticeReport` is `Copy` and tiny; pass by `&mut` for mutation.
#[derive(Debug, Clone, Copy, Default)]
pub struct LatticeReport {
    /// `Some(true)` when the answer involved an implicit coercion (e.g.
    /// `int -> float`, parent-class -> child-class via narrowing). `None`
    /// when the rule never had occasion to consider coercion.
    pub type_coerced: Option<bool>,
    /// `Some(true)` when the coercion narrowed an unrefined form to a
    /// literal (e.g. `int` accepted as `Literal(5)` in some contexts).
    pub type_coerced_to_literal: Option<bool>,
    /// `Some(true)` when the coercion stemmed from a nested `mixed`
    /// (e.g. `array<string, mixed>` flowing into `array<string, int>`).
    pub type_coerced_from_nested_mixed: Option<bool>,
}

impl LatticeReport {
    /// A fresh report with every field unset.
    #[inline]
    pub const fn new() -> Self {
        Self { type_coerced: None, type_coerced_to_literal: None, type_coerced_from_nested_mixed: None }
    }
}
