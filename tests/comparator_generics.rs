//! Generic-object comparison: type-argument variance, ancestor
//! specialisation, multi-level chains, and the Cell<T> soundness check.

mod comparator_common;

use comparator_common::*;
use suffete::world::Variance;

#[test]
fn box_int_in_box_int_reflexive() {
    let mut w = MockWorld::new();
    w.with_templates("Box", &[("T", Variance::Invariant)]);

    let lhs = t_generic_named("Box", vec![u(t_int())]);
    let rhs = t_generic_named("Box", vec![u(t_int())]);
    assert!(atomic_is_contained(lhs, rhs, &w));
}

#[test]
fn box_int_not_in_box_scalar_invariant_default() {
    // Box<T> with no @template-* annotation defaults to Invariant.
    // This is the soundness fix: Box<int> NOT <: Box<scalar>.
    let mut w = MockWorld::new();
    w.with_templates("Box", &[("T", Variance::Invariant)]);

    let lhs = t_generic_named("Box", vec![u(t_int())]);
    let rhs = t_generic_named("Box", vec![u(t_scalar())]);
    assert!(!atomic_is_contained(lhs, rhs, &w));
    assert!(!atomic_is_contained(rhs, lhs, &w));
}

#[test]
fn container_int_in_container_scalar_when_covariant() {
    // Explicit @template-covariant T allows the widening.
    let mut w = MockWorld::new();
    w.with_templates("Container", &[("T", Variance::Covariant)]);

    let lhs = t_generic_named("Container", vec![u(t_int())]);
    let rhs = t_generic_named("Container", vec![u(t_scalar())]);
    assert!(atomic_is_contained(lhs, rhs, &w));
    // The reverse is not a subtype.
    assert!(!atomic_is_contained(rhs, lhs, &w));
}

#[test]
fn sink_scalar_in_sink_int_when_contravariant() {
    // Explicit @template-contravariant T flips the direction.
    let mut w = MockWorld::new();
    w.with_templates("Sink", &[("T", Variance::Contravariant)]);

    let lhs = t_generic_named("Sink", vec![u(t_scalar())]);
    let rhs = t_generic_named("Sink", vec![u(t_int())]);
    assert!(atomic_is_contained(lhs, rhs, &w));
    assert!(!atomic_is_contained(rhs, lhs, &w));
}

#[test]
fn cell_int_not_in_cell_scalar_exploit_rejected() {
    // The full Cell<T> soundness regression: T is used as both setter
    // parameter (contravariant) and getter return (covariant), so the
    // sound variance is invariant. With Invariant default, the exploit
    // is rejected.
    let mut w = MockWorld::new();
    w.with_templates("Cell", &[("T", Variance::Invariant)]);

    let cell_int = t_generic_named("Cell", vec![u(t_int())]);
    let cell_scalar = t_generic_named("Cell", vec![u(t_scalar())]);

    assert!(
        !atomic_is_contained(cell_int, cell_scalar, &w),
        "Cell<int> must NOT refine Cell<scalar> — defaulting to covariant would let store_string() be called on a Cell<int>",
    );
}

#[test]
fn descendant_with_concrete_parent_specialisation() {
    // class A<T>; class B extends A<string>;
    // B <: A<string> ✓
    // B NOT <: A<int>
    let mut w = MockWorld::new();
    w.with_templates("A", &[("T", Variance::Invariant)]);
    w.declare("B");
    w.with_extended("B", "A", vec![u(t_string())]);

    let b = t_named("B");
    let a_string = t_generic_named("A", vec![u(t_string())]);
    let a_int = t_generic_named("A", vec![u(t_int())]);

    assert!(atomic_is_contained(b, a_string, &w));
    assert!(!atomic_is_contained(b, a_int, &w));
}

#[test]
fn descendant_threading_template() {
    // class A<T>; class B<T> extends A<T>;
    // B<int> <: A<int> ✓
    // B<int> NOT <: A<string>
    let mut w = MockWorld::new();
    w.with_templates("A", &[("T", Variance::Invariant)]);
    w.with_templates("B", &[("T", Variance::Invariant)]);
    // B passes its own T (a template ref) to A's T.
    w.with_extended("B", "A", vec![u(t_template("B", "T"))]);

    let b_int = t_generic_named("B", vec![u(t_int())]);
    let a_int = t_generic_named("A", vec![u(t_int())]);
    let a_string = t_generic_named("A", vec![u(t_string())]);

    assert!(atomic_is_contained(b_int, a_int, &w));
    assert!(!atomic_is_contained(b_int, a_string, &w));
}

#[test]
fn descendant_with_template_in_nested_position() {
    // class A<T>; class B<T> extends A<list<T>>;
    // B<int> <: A<list<int>> ✓
    let mut w = MockWorld::new();
    w.with_templates("A", &[("T", Variance::Invariant)]);
    w.with_templates("B", &[("T", Variance::Invariant)]);

    // B passes list<B::T> to A's T.
    let list_of_b_t = u(t_list(u(t_template("B", "T")), false));
    w.with_extended("B", "A", vec![list_of_b_t]);

    let b_int = t_generic_named("B", vec![u(t_int())]);
    let a_list_int = t_generic_named("A", vec![u(t_list(u(t_int()), false))]);
    let a_list_string = t_generic_named("A", vec![u(t_list(u(t_string()), false))]);

    assert!(atomic_is_contained(b_int, a_list_int, &w));
    assert!(!atomic_is_contained(b_int, a_list_string, &w));
}

#[test]
fn multi_level_chain_with_explicit_grandparent_link() {
    // class A<T>; class B<T> extends A<T>; class C<U> extends B<U>;
    // The MockWorld requires explicit `with_extended(C, A, ...)` since
    // it doesn't compose the chain itself; tests must precompute, the
    // way a real analyzer would.
    let mut w = MockWorld::new();
    w.with_templates("A", &[("T", Variance::Invariant)]);
    w.with_templates("B", &[("T", Variance::Invariant)]);
    w.with_templates("C", &[("U", Variance::Invariant)]);
    w.with_extended("B", "A", vec![u(t_template("B", "T"))]);
    w.with_extended("C", "B", vec![u(t_template("C", "U"))]);
    // Composed: C passes its U all the way through to A.
    w.with_extended("C", "A", vec![u(t_template("C", "U"))]);

    let c_int = t_generic_named("C", vec![u(t_int())]);
    let a_int = t_generic_named("A", vec![u(t_int())]);
    let a_string = t_generic_named("A", vec![u(t_string())]);

    assert!(atomic_is_contained(c_int, a_int, &w));
    assert!(!atomic_is_contained(c_int, a_string, &w));
}

#[test]
fn covariant_descendant_widening() {
    // Covariant container: B<int> <: A<scalar> via covariance + concrete
    // parent specialisation.
    let mut w = MockWorld::new();
    w.with_templates("A", &[("T", Variance::Covariant)]);
    w.with_templates("B", &[("T", Variance::Covariant)]);
    w.with_extended("B", "A", vec![u(t_template("B", "T"))]);

    let b_int = t_generic_named("B", vec![u(t_int())]);
    let a_scalar = t_generic_named("A", vec![u(t_scalar())]);

    assert!(atomic_is_contained(b_int, a_scalar, &w));
}

#[test]
fn unrelated_classes_disjoint_with_args() {
    // Foo<int> and Bar<int> are unrelated; neither descends from the
    // other regardless of type args.
    let mut w = MockWorld::new();
    w.with_templates("Foo", &[("T", Variance::Invariant)]);
    w.with_templates("Bar", &[("T", Variance::Invariant)]);

    let foo_int = t_generic_named("Foo", vec![u(t_int())]);
    let bar_int = t_generic_named("Bar", vec![u(t_int())]);

    assert!(!atomic_is_contained(foo_int, bar_int, &w));
    assert!(!atomic_is_contained(bar_int, foo_int, &w));
}

#[test]
fn non_generic_named_into_non_generic_descendant() {
    // No type parameters anywhere — nominal subtyping is sufficient.
    let mut w = MockWorld::new();
    w.add_edge("Dog", "Animal");

    let dog = t_named("Dog");
    let animal = t_named("Animal");
    assert!(atomic_is_contained(dog, animal, &w));
    assert!(!atomic_is_contained(animal, dog, &w));
}

#[test]
fn descendant_reaches_generic_ancestor_without_own_templates() {
    // class A<T>; class StringList extends A<string>; (StringList non-generic)
    let mut w = MockWorld::new();
    w.with_templates("A", &[("T", Variance::Invariant)]);
    w.declare("StringList");
    w.with_extended("StringList", "A", vec![u(t_string())]);

    let sl = t_named("StringList");
    let a_string = t_generic_named("A", vec![u(t_string())]);
    let a_int = t_generic_named("A", vec![u(t_int())]);

    assert!(atomic_is_contained(sl, a_string, &w));
    assert!(!atomic_is_contained(sl, a_int, &w));
}
