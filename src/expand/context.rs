//! Per-call context for [`expand`](super::expand_with): contextual
//! class names (`self` / `static` / `parent`) and feature flags that
//! control how non-structural atoms are resolved.

use mago_atom::Atom;

/// Caller-controlled options for [`expand_with`](super::expand_with).
///
/// All fields default to "no resolution" — calling
/// [`expand`](super::expand) (the no-context wrapper) uses
/// `Default::default()`, which leaves contextual keywords and
/// conditional types unevaluated.
///
/// # Fields
///
/// - **`self_class`**, **`static_class`**, **`parent_class`** — the
///   class names that PHP's `self`, `static`, `$this`, and `parent`
///   resolve to in the current scope. When `None`, an `Object` atom
///   carrying the corresponding keyword passes through unchanged
///   (precision-loss-only fallback; the spec says ⊤, but keeping the
///   atom preserves more information for downstream resolution).
/// - **`eval_conditional`** — when `true`, `Conditional` atoms decide
///   their branch by running the lattice on the test (`subject <:
///   target`); when `false`, the atom is preserved but its operand
///   types are still expanded recursively.
#[derive(Debug, Clone, Copy, Default)]
pub struct ExpansionContext {
    pub self_class: Option<Atom>,
    pub static_class: Option<Atom>,
    pub parent_class: Option<Atom>,
    pub eval_conditional: bool,
}

impl ExpansionContext {
    #[must_use]
    pub const fn with_self_class(mut self, class: Atom) -> Self {
        self.self_class = Some(class);
        self
    }

    #[must_use]
    pub const fn with_static_class(mut self, class: Atom) -> Self {
        self.static_class = Some(class);
        self
    }

    #[must_use]
    pub const fn with_parent_class(mut self, class: Atom) -> Self {
        self.parent_class = Some(class);
        self
    }

    #[must_use]
    pub const fn with_eval_conditional(mut self, on: bool) -> Self {
        self.eval_conditional = on;
        self
    }
}
