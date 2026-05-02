#![allow(
    clippy::absolute_paths,
    clippy::missing_docs_in_private_items,
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::tests_outside_test_module,
    clippy::missing_assert_message,
    clippy::std_instead_of_alloc,
    clippy::std_instead_of_core,
)]

mod comparator_common;

use comparator_common::*;

#[test]
fn smoke_int_int() {
    assert_subtype(u(t_int()), u(t_int()));
}

#[test]
fn smoke_lit_int_in_int() {
    assert_subtype(u(t_lit_int(5)), u(t_int()));
}

#[test]
fn smoke_int_not_in_string() {
    assert_not_subtype(u(t_int()), u(t_string()));
}

#[test]
fn smoke_int_in_mixed() {
    assert_subtype(u(t_int()), u(mixed()));
}

#[test]
fn smoke_never_in_anything() {
    assert_subtype(u(never()), u(t_int()));
    assert_subtype(u(never()), u(t_string()));
    assert_subtype(u(never()), u(mixed()));
}
