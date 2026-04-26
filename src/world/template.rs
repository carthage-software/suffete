//! Template / generic parameter metadata exposed by [`World`](super::World)
//! to consumers (the lattice, future narrowing operations, etc.) when
//! comparing or substituting generic types.

use mago_atom::Atom;

use crate::TypeId;

/// One template parameter of a generic class-like or function.
///
/// Variance is per-parameter: PHP analyzers default to covariant in the
/// value position (Psalm-style), with `@template-contravariant T` flipping
/// the direction. Strict invariance (`@template-invariant T`) requires
/// both directions when comparing type arguments.
///
/// `upper_bound` is the `@template T of Foo` constraint, if any. `None`
/// means unbounded (`mixed`-equivalent).
#[derive(Debug, Clone)]
pub struct TemplateParameter {
    pub name: Atom,
    pub variance: Variance,
    pub upper_bound: Option<TypeId>,
}

/// How a template parameter behaves in the value position when comparing
/// generic types.
///
/// PHP analyzers default to [`Covariant`](Variance::Covariant): given
/// `Box<int>` and `Box<scalar>`, the lattice accepts `Box<int> <:
/// Box<scalar>` because `int <: scalar`. [`Contravariant`](Variance::Contravariant)
/// flips this; [`Invariant`](Variance::Invariant) requires equality (or
/// mutual subtyping) on the type argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Variance {
    #[default]
    Covariant,
    Contravariant,
    Invariant,
}
