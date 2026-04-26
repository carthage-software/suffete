//! Template / generic parameter metadata exposed by [`World`](super::World)
//! to consumers (the lattice, future narrowing operations, etc.) when
//! comparing or substituting generic types.

use mago_atom::Atom;

use crate::TypeId;

/// One template parameter of a generic class-like or function.
///
/// Variance is per-parameter and defaults to [`Invariant`](Variance::Invariant)
/// when the source provides no annotation. The class author opts into
/// [`Covariant`](Variance::Covariant) (`@template-covariant T`) or
/// [`Contravariant`](Variance::Contravariant) (`@template-contravariant T`)
/// only when the parameter is used exclusively in the corresponding position.
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
/// **Default is [`Invariant`](Variance::Invariant).** This is the only
/// sound default for a class whose template usage isn't analysed: a
/// generic mutable container (read AND write of `T`) is invariant, and
/// defaulting to anything looser is unsound.
///
/// Concrete example of why covariant-by-default is broken:
///
/// ```text
/// /** @template T */
/// class Cell {
///     /** @var T */ public $value;
///     /** @param T $v */ public function set($v) { $this->value = $v; }
///     /** @return T */ public function get() { return $this->value; }
/// }
///
/// /** @param Cell<scalar> $c */
/// function store_string(Cell $c) { $c->set('hi'); }
///
/// $cell = new Cell(42);          // Cell<int>
/// store_string($cell);           // accepted under Covariant default
/// return $cell->get() + 1;       // runtime: 'hi' + 1, type confusion
/// ```
///
/// With [`Invariant`](Variance::Invariant) as the default, `Cell<int>`
/// is NOT a subtype of `Cell<scalar>` — the unsoundness is rejected at
/// the call site. A library author who has audited their class for
/// covariant-only or contravariant-only usage opts in explicitly.
///
/// - [`Covariant`](Variance::Covariant): `Box<int> <: Box<scalar>` when
///   `int <: scalar`. Sound only when `T` appears exclusively in
///   producer (return / read-only) positions.
/// - [`Contravariant`](Variance::Contravariant): `Sink<scalar> <:
///   Sink<int>` when `int <: scalar`. Sound only when `T` appears
///   exclusively in consumer (parameter / write-only) positions.
/// - [`Invariant`](Variance::Invariant): the type argument must match
///   exactly (mutual subtyping). The default and only safe choice when
///   `T` is used both as input and output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Variance {
    Covariant,
    Contravariant,
    #[default]
    Invariant,
}
