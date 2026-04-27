mod comparator_common;

use comparator_common::*;
use mago_atom::atom;
use suffete::TypeId;
use suffete::hierarchy::HierarchyBuilder;
use suffete::world::Variance;

fn build_with_template_world(
    edges: &[(&str, &str, Vec<TypeId>)],
    templates: &[(&str, &str)],
) -> suffete::hierarchy::Hierarchy {
    let mut w = MockWorld::new();
    for (class, t) in templates {
        w.with_templates(class, &[(t, Variance::Invariant)]);
    }
    let mut b = HierarchyBuilder::new();
    for (child, parent, args) in edges {
        b.add_edge(atom(child), atom(parent), args.clone());
    }
    b.build(&w)
}

#[test]
fn direct_edge_returns_registered_args() {
    let h = build_with_template_world(&[("B", "A", vec![u(t_string())])], &[("A", "T")]);
    assert_eq!(h.arg(atom("B"), atom("A"), 0), Some(u(t_string())));
}

#[test]
fn missing_pair_returns_none() {
    let h = build_with_template_world(&[("B", "A", vec![u(t_string())])], &[("A", "T")]);
    assert_eq!(h.arg(atom("B"), atom("Other"), 0), None);
    assert_eq!(h.arg(atom("Foo"), atom("A"), 0), None);
}

#[test]
fn transitive_two_step_concrete_args_compose() {
    // class A<T>; class B extends A<string>; class C extends B;
    // C should compose to passing string to A.
    let mut w = MockWorld::new();
    w.with_templates("A", &[("T", Variance::Invariant)]);

    let mut b = HierarchyBuilder::new();
    b.add_edge(atom("B"), atom("A"), vec![u(t_string())]);
    b.add_edge(atom("C"), atom("B"), vec![]);
    let h = b.build(&w);

    assert_eq!(h.arg(atom("C"), atom("A"), 0), Some(u(t_string())));
}

#[test]
fn transitive_two_step_template_threading() {
    // class A<T>; class B<U> extends A<U>; class C<V> extends B<V>;
    // C should pass V (its own template) to A.
    let mut w = MockWorld::new();
    w.with_templates("A", &[("T", Variance::Invariant)]);
    w.with_templates("B", &[("U", Variance::Invariant)]);
    w.with_templates("C", &[("V", Variance::Invariant)]);

    let b_template = u(t_template("B", "U"));
    let c_template = u(t_template("C", "V"));

    let mut hb = HierarchyBuilder::new();
    hb.add_edge(atom("B"), atom("A"), vec![b_template]);
    hb.add_edge(atom("C"), atom("B"), vec![c_template]);
    let h = hb.build(&w);

    // C → A composed: should be C's own V threaded through.
    assert_eq!(h.arg(atom("C"), atom("A"), 0), Some(c_template));
}

#[test]
fn transitive_three_step_chain() {
    // class A<T>; class B<U> extends A<U>; class C<V> extends B<V>;
    // class D extends C<int>.
    // D → A should compose to int.
    let mut w = MockWorld::new();
    w.with_templates("A", &[("T", Variance::Invariant)]);
    w.with_templates("B", &[("U", Variance::Invariant)]);
    w.with_templates("C", &[("V", Variance::Invariant)]);

    let mut hb = HierarchyBuilder::new();
    hb.add_edge(atom("B"), atom("A"), vec![u(t_template("B", "U"))]);
    hb.add_edge(atom("C"), atom("B"), vec![u(t_template("C", "V"))]);
    hb.add_edge(atom("D"), atom("C"), vec![u(t_int())]);
    let h = hb.build(&w);

    assert_eq!(h.arg(atom("D"), atom("A"), 0), Some(u(t_int())));
    assert_eq!(h.arg(atom("D"), atom("B"), 0), Some(u(t_int())));
    assert_eq!(h.arg(atom("D"), atom("C"), 0), Some(u(t_int())));
}

#[test]
fn nested_template_in_parent_args_substitutes() {
    // class A<T>; class B<U> extends A<list<U>>;
    // class C<V> extends B<V>.
    // C → A should compose to list<V>.
    let mut w = MockWorld::new();
    w.with_templates("A", &[("T", Variance::Invariant)]);
    w.with_templates("B", &[("U", Variance::Invariant)]);
    w.with_templates("C", &[("V", Variance::Invariant)]);

    let list_of_b_u = u(t_list(u(t_template("B", "U")), false));
    let list_of_c_v = u(t_list(u(t_template("C", "V")), false));

    let mut hb = HierarchyBuilder::new();
    hb.add_edge(atom("B"), atom("A"), vec![list_of_b_u]);
    hb.add_edge(atom("C"), atom("B"), vec![u(t_template("C", "V"))]);
    let h = hb.build(&w);

    assert_eq!(h.arg(atom("C"), atom("A"), 0), Some(list_of_c_v));
}

#[test]
fn args_returns_full_slice() {
    // class A<T, U>; class B extends A<string, int>.
    let mut w = MockWorld::new();
    w.with_templates("A", &[("T", Variance::Invariant), ("U", Variance::Invariant)]);

    let mut hb = HierarchyBuilder::new();
    hb.add_edge(atom("B"), atom("A"), vec![u(t_string()), u(t_int())]);
    let h = hb.build(&w);

    let args = h.args(atom("B"), atom("A")).unwrap();
    assert_eq!(args, &[u(t_string()), u(t_int())]);
}

#[test]
fn iter_yields_every_pair() {
    let mut w = MockWorld::new();
    w.with_templates("A", &[("T", Variance::Invariant)]);
    let mut hb = HierarchyBuilder::new();
    hb.add_edge(atom("B"), atom("A"), vec![u(t_int())]);
    hb.add_edge(atom("C"), atom("B"), vec![]);
    let h = hb.build(&w);

    let pairs: Vec<(mago_atom::Atom, mago_atom::Atom)> = h.iter().map(|(k, _)| k).collect();
    assert!(pairs.contains(&(atom("B"), atom("A"))));
    assert!(pairs.contains(&(atom("C"), atom("B"))));
    assert!(pairs.contains(&(atom("C"), atom("A"))));
}

#[test]
fn last_added_edge_wins_on_duplicate() {
    let mut w = MockWorld::new();
    w.with_templates("A", &[("T", Variance::Invariant)]);
    let mut hb = HierarchyBuilder::new();
    hb.add_edge(atom("B"), atom("A"), vec![u(t_int())]);
    hb.add_edge(atom("B"), atom("A"), vec![u(t_string())]);
    let h = hb.build(&w);
    assert_eq!(h.arg(atom("B"), atom("A"), 0), Some(u(t_string())));
}
