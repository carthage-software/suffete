/// Side-effect accumulator for the lattice operations
/// ([`refines`](crate::lattice::refines),
/// [`generalizes`](crate::lattice::generalizes),
/// [`intersects`](crate::lattice::intersects)).
///
/// The operations return plain `bool`s; richer information (coercion edges,
/// template bounds, replacement type suggestions, etc.) is collected here as
/// the lattice walks. The caller decides what to do with it (suppress
/// warnings, feed bounds into inference, surface diagnostics).
///
/// New fields are added incrementally as each rule family lands. The current
/// set is the minimum the axiom-only / scalar-lattice rules need.
#[derive(Debug, Clone, Copy, Default)]
pub struct LatticeContext {
    /// Set when the refinement answer was "yes, but with a coercion" rather
    /// than a clean subtype edge. The most common cause is `int <: float`
    /// outside an assertion context, but every rule family contributes its
    /// own coercion patterns.
    pub type_coerced: bool,
}

impl LatticeContext {
    /// A fresh context with every field zeroed.
    #[inline]
    pub const fn new() -> Self {
        Self { type_coerced: false }
    }
}
