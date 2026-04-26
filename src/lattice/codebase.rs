use mago_atom::Atom;

/// The information the type lattice needs from the surrounding world.
///
/// Implementations supply class-hierarchy lookups, member existence checks,
/// constant resolutions, alias bodies, and template-parameter metadata. The
/// trait is deliberately narrow: only the information the lattice rules
/// consult appears here. Implementations come from outside this crate (an
/// analyser like Mago, a language server, mock fixtures for tests).
///
/// Methods are added to this trait as rule families are implemented. The
/// current set is the minimum the axiom-only / scalar-lattice rules need
/// (which is nothing); object hierarchy, callable variance, and template
/// queries will all add their own methods.
pub trait Codebase {
    /// `true` iff `child` is the same class-like as `parent`, or extends /
    /// implements / uses-as-trait it (transitively). Implementations are
    /// expected to precompute the ancestor closure so this query is O(1).
    fn is_subclass_of(&self, child: Atom, parent: Atom) -> bool;
}

/// A no-op `Codebase` for lattice queries that don't consult Γ.
///
/// Every lookup returns the empty / negative answer. Suitable when the input
/// types contain only scalar / trivial elements and no object / generic /
/// reference machinery would be exercised.
pub struct NullCodebase;

impl Codebase for NullCodebase {
    fn is_subclass_of(&self, _child: Atom, _parent: Atom) -> bool {
        false
    }
}
