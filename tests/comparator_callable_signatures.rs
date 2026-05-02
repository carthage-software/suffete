#![allow(
    clippy::absolute_paths,
    clippy::missing_docs_in_private_items,
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::tests_outside_test_module,
    clippy::missing_assert_message,
    clippy::std_instead_of_alloc,
    clippy::std_instead_of_core
)]

mod comparator_common;

use comparator_common::*;

#[test]
fn signature_reflexive() {
    let cb = empty_world();
    let sig = t_callable(&[u(t_int())], u(t_string()));
    assert!(atomic_is_contained(sig, sig, &cb));
}

#[test]
fn return_covariance_holds() {
    let cb = empty_world();
    let returns_lit = t_callable(&[], ui(42));
    let returns_int = t_callable(&[], u(t_int()));
    assert!(atomic_is_contained(returns_lit, returns_int, &cb));
}

#[test]
fn return_covariance_failure() {
    let cb = empty_world();
    let returns_int = t_callable(&[], u(t_int()));
    let returns_lit = t_callable(&[], ui(42));
    assert!(!atomic_is_contained(returns_int, returns_lit, &cb));
}

#[test]
fn return_widens_into_mixed() {
    let cb = empty_world();
    let returns_int = t_callable(&[], u(t_int()));
    let returns_mixed = t_callable(&[], suffete::prelude::TYPE_MIXED);
    assert!(atomic_is_contained(returns_int, returns_mixed, &cb));
}

#[test]
fn parameter_contravariance_holds() {
    let cb = empty_world();
    let takes_int = t_callable(&[u(t_int())], u(t_int()));
    let takes_lit = t_callable(&[ui(42)], u(t_int()));
    assert!(atomic_is_contained(takes_int, takes_lit, &cb));
}

#[test]
fn parameter_contravariance_failure() {
    let cb = empty_world();
    let takes_lit = t_callable(&[ui(42)], u(t_int()));
    let takes_int = t_callable(&[u(t_int())], u(t_int()));
    assert!(!atomic_is_contained(takes_lit, takes_int, &cb));
}

#[test]
fn parameter_contravariance_widens_via_mixed() {
    let cb = empty_world();
    let takes_mixed = t_callable(&[suffete::prelude::TYPE_MIXED], u(t_int()));
    let takes_int = t_callable(&[u(t_int())], u(t_int()));
    assert!(atomic_is_contained(takes_mixed, takes_int, &cb));
}

#[test]
fn arity_mismatch_more_required_input() {
    let cb = empty_world();
    let takes_two = t_callable(&[u(t_int()), u(t_string())], u(t_int()));
    let takes_one = t_callable(&[u(t_int())], u(t_int()));
    assert!(!atomic_is_contained(takes_two, takes_one, &cb));
}

#[test]
fn arity_mismatch_more_required_container() {
    let cb = empty_world();
    let takes_one = t_callable(&[u(t_int())], u(t_int()));
    let takes_two = t_callable(&[u(t_int()), u(t_string())], u(t_int()));
    assert!(!atomic_is_contained(takes_one, takes_two, &cb));
}

#[test]
fn input_with_default_satisfies_smaller_arity_container() {
    let cb = empty_world();
    let opt =
        t_callable_sig(&[(u(t_int()), false, false, false), (u(t_string()), true, false, false)], u(t_int()), false);
    let one = t_callable(&[u(t_int())], u(t_int()));
    assert!(atomic_is_contained(opt, one, &cb));
}

#[test]
fn pure_container_rejects_impure_input() {
    let cb = empty_world();
    let impure = t_callable_sig(&[(u(t_int()), false, false, false)], u(t_int()), false);
    let pure = t_callable_sig(&[(u(t_int()), false, false, false)], u(t_int()), true);
    assert!(!atomic_is_contained(impure, pure, &cb));
}

#[test]
fn pure_input_satisfies_pure_container() {
    let cb = empty_world();
    let pure = t_callable_sig(&[(u(t_int()), false, false, false)], u(t_int()), true);
    assert!(atomic_is_contained(pure, pure, &cb));
}

#[test]
fn pure_input_satisfies_impure_container() {
    let cb = empty_world();
    let pure = t_callable_sig(&[(u(t_int()), false, false, false)], u(t_int()), true);
    let impure = t_callable_sig(&[(u(t_int()), false, false, false)], u(t_int()), false);
    assert!(atomic_is_contained(pure, impure, &cb));
}

#[test]
fn variadic_input_absorbs_extra_container_param() {
    let cb = empty_world();
    let variadic_in = t_callable_sig(&[(u(t_int()), false, false, true)], u(t_int()), false);
    let two_int = t_callable(&[u(t_int()), u(t_int())], u(t_int()));
    assert!(atomic_is_contained(variadic_in, two_int, &cb));
}

#[test]
fn variadic_container_requires_variadic_input() {
    let cb = empty_world();
    let one_in = t_callable(&[u(t_int())], u(t_int()));
    let variadic_out = t_callable_sig(&[(u(t_int()), false, false, true)], u(t_int()), false);
    assert!(!atomic_is_contained(one_in, variadic_out, &cb));
}

#[test]
fn variadic_to_variadic_with_contravariant_type() {
    let cb = empty_world();
    let in_takes_mixed = t_callable_sig(&[(suffete::prelude::TYPE_MIXED, false, false, true)], u(t_int()), false);
    let out_takes_int = t_callable_sig(&[(u(t_int()), false, false, true)], u(t_int()), false);
    assert!(atomic_is_contained(in_takes_mixed, out_takes_int, &cb));
}

#[test]
fn unspecified_container_accepts_any_signature() {
    let cb = empty_world();
    let specific = t_callable(&[u(t_int())], u(t_int()));
    assert!(atomic_is_contained(specific, t_callable_mixed(), &cb));
}

#[test]
fn unspecified_input_does_not_refine_specific_container() {
    let cb = empty_world();
    let specific = t_callable(&[u(t_int())], u(t_int()));
    assert!(!atomic_is_contained(t_callable_mixed(), specific, &cb));
}

#[test]
fn closure_refines_signature_with_compatible_shape() {
    let cb = empty_world();
    use suffete::element::payload::CallableInfo;
    use suffete::element::payload::Signature;
    use suffete::element::payload::SignatureFlags;
    use suffete::interner::interner;
    let i = interner();
    let sig = i.intern_signature(Signature {
        parameters: None,
        return_type: u(t_int()),
        throws: None,
        flags: SignatureFlags::EMPTY,
    });
    let closure = i.intern_callable(CallableInfo::Closure(sig));
    let signature = i.intern_callable(CallableInfo::Signature(sig));
    assert!(atomic_is_contained(closure, signature, &cb));
    assert!(!atomic_is_contained(signature, closure, &cb));
}

#[test]
fn any_callable_does_not_refine_specific() {
    let cb = empty_world();
    let specific = t_callable(&[u(t_int())], u(t_int()));
    assert!(!atomic_is_contained(t_callable_any(), specific, &cb));
}

#[test]
fn anything_refines_any_callable() {
    let cb = empty_world();
    let specific = t_callable(&[u(t_int())], u(t_int()));
    assert!(atomic_is_contained(specific, t_callable_any(), &cb));
    assert!(atomic_is_contained(t_callable_mixed(), t_callable_any(), &cb));
}
