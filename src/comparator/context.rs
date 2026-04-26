/// Side-effect accumulator for [`is_subtype`](crate::comparator::is_subtype).
///
/// `is_subtype` returns a plain `bool`; richer information (coercion edges,
/// template bounds, replacement type suggestions, etc.) is collected here as
/// the comparator walks. The caller decides what to do with it (suppress
/// warnings, feed bounds into inference, surface diagnostics).
///
/// New fields are added incrementally as each comparator rule family lands.
/// The current set is the minimum the axiom-only comparator needs.
#[derive(Debug, Clone, Copy, Default)]
pub struct SubtypeContext {
    /// Set when the answer was "yes, but with a coercion" rather than a clean
    /// subtype edge. The most common cause is `int <: float` outside an
    /// assertion context, but every rule family contributes its own coercion
    /// patterns.
    pub type_coerced: bool,
}

impl SubtypeContext {
    /// A fresh context with every field zeroed.
    #[inline]
    pub const fn new() -> Self {
        Self { type_coerced: false }
    }
}
