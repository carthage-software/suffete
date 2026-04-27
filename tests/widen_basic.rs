//! Widening: `widen::scalars` (full family generalisation) and
//! `widen::literals` (literal-only generalisation, preserves
//! user-declared narrowings).

mod comparator_common;

use comparator_common::*;

use suffete::ElementId;
use suffete::TypeId;
use suffete::element::payload::ClassLikeKind;
use suffete::element::payload::ClassLikeStringInfo;
use suffete::element::payload::ClassLikeStringSpecifier;
use suffete::element::payload::StringCasing;
use suffete::element::payload::StringInfo;
use suffete::element::payload::StringLiteral;
use suffete::element::payload::StringRefinementFlags;
use suffete::interner::interner;
use suffete::prelude;
use suffete::widen;

fn t_string_with(literal: StringLiteral, casing: StringCasing, flags: StringRefinementFlags) -> ElementId {
    interner().intern_string(StringInfo { literal, casing, flags })
}

fn t_class_string_lit(name: &str, kind: ClassLikeKind) -> ElementId {
    interner().intern_class_like_string(ClassLikeStringInfo {
        kind,
        specifier: ClassLikeStringSpecifier::Literal { value: mago_atom::atom(name) },
    })
}

fn t_class_string_any(kind: ClassLikeKind) -> ElementId {
    interner().intern_class_like_string(ClassLikeStringInfo { kind, specifier: ClassLikeStringSpecifier::Any })
}

fn ty_of(elem: ElementId) -> TypeId {
    TypeId::singleton(elem)
}

#[test]
fn scalars_widens_int_literal_to_int() {
    let result = widen::scalars(u(t_lit_int(42)));
    assert_eq!(result, prelude::TYPE_INT);
}

#[test]
fn scalars_widens_int_range_to_int() {
    let result = widen::scalars(u(t_int_range(0, 10)));
    assert_eq!(result, prelude::TYPE_INT);
}

#[test]
fn scalars_widens_positive_int_to_int() {
    let result = widen::scalars(u(t_positive_int()));
    assert_eq!(result, prelude::TYPE_INT);
}

#[test]
fn scalars_widens_float_literal_to_float() {
    let result = widen::scalars(u(t_lit_float(1.5)));
    assert_eq!(result, prelude::TYPE_FLOAT);
}

#[test]
fn scalars_widens_string_literal_to_string() {
    let result = widen::scalars(u(t_lit_string("foo")));
    assert_eq!(result, prelude::TYPE_STRING);
}

#[test]
fn scalars_widens_non_empty_string_to_string() {
    let result = widen::scalars(u(t_non_empty_string()));
    assert_eq!(result, prelude::TYPE_STRING);
}

#[test]
fn scalars_widens_truthy_string_to_string() {
    let result = widen::scalars(u(t_truthy_string()));
    assert_eq!(result, prelude::TYPE_STRING);
}

#[test]
fn scalars_widens_true_and_false_to_bool() {
    assert_eq!(widen::scalars(u(t_true())), prelude::TYPE_BOOL);
    assert_eq!(widen::scalars(u(t_false())), prelude::TYPE_BOOL);
}

#[test]
fn scalars_widens_class_like_string_to_any_of_same_kind() {
    let lit = t_class_string_lit("Foo", ClassLikeKind::Class);
    let result = widen::scalars(ty_of(lit));
    assert_eq!(result, ty_of(t_class_string_any(ClassLikeKind::Class)));
}

#[test]
fn scalars_preserves_class_like_string_kind() {
    let interface_lit = t_class_string_lit("Foo", ClassLikeKind::Interface);
    let result = widen::scalars(ty_of(interface_lit));
    assert_eq!(result, ty_of(t_class_string_any(ClassLikeKind::Interface)));
}

#[test]
fn scalars_leaves_resource_alone() {
    let result = widen::scalars(u(t_open_resource()));
    let expected = u(t_open_resource());
    assert_eq!(result, expected);
}

#[test]
fn scalars_descends_into_list_element() {
    let list = u(t_list(u(t_lit_int(42)), false));
    let result = widen::scalars(list);
    let expected = u(t_list(prelude::TYPE_INT, false));
    assert_eq!(result, expected);
}

#[test]
fn scalars_descends_into_object_type_args() {
    let generic = u(t_generic_named("Box", vec![u(t_lit_int(42))]));
    let result = widen::scalars(generic);
    let expected = u(t_generic_named("Box", vec![prelude::TYPE_INT]));
    assert_eq!(result, expected);
}

#[test]
fn literals_widens_int_literal_to_int() {
    assert_eq!(widen::literals(u(t_lit_int(42))), prelude::TYPE_INT);
    assert_eq!(widen::literals(u(t_lit_int(0))), prelude::TYPE_INT);
    assert_eq!(widen::literals(u(t_lit_int(-1))), prelude::TYPE_INT);
}

#[test]
fn literals_widens_unspecified_literal_int_to_int() {
    let lit_int = u(t_int_unspec_lit());
    assert_eq!(widen::literals(lit_int), prelude::TYPE_INT);
}

#[test]
fn literals_preserves_int_range() {
    let r = u(t_int_range(0, 10));
    assert_eq!(widen::literals(r), r);
}

#[test]
fn literals_preserves_positive_int_dominator() {
    let p = u(t_positive_int());
    assert_eq!(widen::literals(p), p);
}

#[test]
fn literals_widens_float_literal_to_float() {
    assert_eq!(widen::literals(u(t_lit_float(1.5))), prelude::TYPE_FLOAT);
    assert_eq!(widen::literals(u(t_lit_float(0.0))), prelude::TYPE_FLOAT);
}

#[test]
fn literals_widens_unspecified_literal_float_to_float() {
    let lf = u(t_unspec_lit_float());
    assert_eq!(widen::literals(lf), prelude::TYPE_FLOAT);
}

#[test]
fn literals_widens_true_to_bool() {
    assert_eq!(widen::literals(u(t_true())), prelude::TYPE_BOOL);
    assert_eq!(widen::literals(u(t_false())), prelude::TYPE_BOOL);
}

#[test]
fn literals_widens_string_to_non_empty_truthy_lowercase() {
    let result = widen::literals(u(t_lit_string("foo")));
    let expected = ty_of(t_string_with(
        StringLiteral::None,
        StringCasing::Lowercase,
        StringRefinementFlags::EMPTY.with_is_non_empty(true).with_is_truthy(true),
    ));
    assert_eq!(result, expected);
}

#[test]
fn literals_widens_uppercase_string_correctly() {
    let result = widen::literals(u(t_lit_string("FOO")));
    let expected = ty_of(t_string_with(
        StringLiteral::None,
        StringCasing::Uppercase,
        StringRefinementFlags::EMPTY.with_is_non_empty(true).with_is_truthy(true),
    ));
    assert_eq!(result, expected);
}

#[test]
fn literals_widens_mixed_case_string_without_casing() {
    let result = widen::literals(u(t_lit_string("FooBar")));
    let expected = ty_of(t_string_with(
        StringLiteral::None,
        StringCasing::Unspecified,
        StringRefinementFlags::EMPTY.with_is_non_empty(true).with_is_truthy(true),
    ));
    assert_eq!(result, expected);
}

#[test]
fn literals_widens_zero_string_to_non_empty_non_truthy() {
    let result = widen::literals(u(t_lit_string("0")));
    let expected = ty_of(t_string_with(
        StringLiteral::None,
        StringCasing::Unspecified,
        StringRefinementFlags::EMPTY.with_is_non_empty(true).with_is_numeric(true),
    ));
    assert_eq!(result, expected);
}

#[test]
fn literals_widens_numeric_string() {
    let result = widen::literals(u(t_lit_string("42")));
    let expected = ty_of(t_string_with(
        StringLiteral::None,
        StringCasing::Unspecified,
        StringRefinementFlags::EMPTY.with_is_non_empty(true).with_is_truthy(true).with_is_numeric(true),
    ));
    assert_eq!(result, expected);
}

#[test]
fn literals_widens_negative_numeric_string() {
    let result = widen::literals(u(t_lit_string("-1")));
    let expected = ty_of(t_string_with(
        StringLiteral::None,
        StringCasing::Unspecified,
        StringRefinementFlags::EMPTY.with_is_non_empty(true).with_is_truthy(true).with_is_numeric(true),
    ));
    assert_eq!(result, expected);
}

#[test]
fn literals_widens_decimal_numeric_string() {
    let result = widen::literals(u(t_lit_string("1.5")));
    let expected = ty_of(t_string_with(
        StringLiteral::None,
        StringCasing::Unspecified,
        StringRefinementFlags::EMPTY.with_is_non_empty(true).with_is_truthy(true).with_is_numeric(true),
    ));
    assert_eq!(result, expected);
}

#[test]
fn literals_widens_empty_string_to_string_dominator() {
    let result = widen::literals(u(t_lit_string("")));
    assert_eq!(result, prelude::TYPE_STRING);
}

#[test]
fn literals_preserves_non_empty_string() {
    let nes = u(t_non_empty_string());
    assert_eq!(widen::literals(nes), nes);
}

#[test]
fn literals_preserves_truthy_string() {
    let ts = u(t_truthy_string());
    assert_eq!(widen::literals(ts), ts);
}

#[test]
fn literals_widens_class_like_string_literal_to_any() {
    let lit = t_class_string_lit("Foo", ClassLikeKind::Class);
    let result = widen::literals(ty_of(lit));
    assert_eq!(result, ty_of(t_class_string_any(ClassLikeKind::Class)));
}

#[test]
fn literals_descends_into_list_element() {
    let list = u(t_list(u(t_lit_int(42)), false));
    let result = widen::literals(list);
    let expected = u(t_list(prelude::TYPE_INT, false));
    assert_eq!(result, expected);
}

#[test]
fn literals_descends_into_keyed_array_value() {
    let arr = u(t_keyed_unsealed(prelude::TYPE_STRING, u(t_lit_int(42)), false));
    let result = widen::literals(arr);
    let expected = u(t_keyed_unsealed(prelude::TYPE_STRING, prelude::TYPE_INT, false));
    assert_eq!(result, expected);
}

#[test]
fn literals_descends_into_iterable_value() {
    let iter = u(t_iterable(prelude::TYPE_STRING, u(t_lit_int(42))));
    let result = widen::literals(iter);
    let expected = u(t_iterable(prelude::TYPE_STRING, prelude::TYPE_INT));
    assert_eq!(result, expected);
}

#[test]
fn literals_descends_into_object_type_args() {
    let generic = u(t_generic_named("Box", vec![u(t_lit_int(42))]));
    let result = widen::literals(generic);
    let expected = u(t_generic_named("Box", vec![prelude::TYPE_INT]));
    assert_eq!(result, expected);
}

#[test]
fn literals_descends_into_generic_parameter_constraint() {
    use mago_atom::atom;
    use suffete::element::payload::DefiningEntity;
    let param = ElementId::generic_parameter("T", DefiningEntity::ClassLike(atom("Foo")), u(t_lit_int(42)));
    let ty = u(param);
    let result = widen::literals(ty);
    let expected_param = ElementId::generic_parameter("T", DefiningEntity::ClassLike(atom("Foo")), prelude::TYPE_INT);
    let expected = u(expected_param);
    assert_eq!(result, expected);
}

#[test]
fn scalars_descends_into_generic_parameter_constraint() {
    use mago_atom::atom;
    use suffete::element::payload::DefiningEntity;
    let param = ElementId::generic_parameter("T", DefiningEntity::ClassLike(atom("Foo")), u(t_non_empty_string()));
    let ty = u(param);
    let result = widen::scalars(ty);
    let expected_param =
        ElementId::generic_parameter("T", DefiningEntity::ClassLike(atom("Foo")), prelude::TYPE_STRING);
    let expected = u(expected_param);
    assert_eq!(result, expected);
}

#[test]
fn scalars_no_op_returns_same_handle() {
    let already_general = prelude::TYPE_INT;
    assert_eq!(widen::scalars(already_general), already_general);
}

#[test]
fn literals_no_op_returns_same_handle() {
    let already_general = prelude::TYPE_INT;
    assert_eq!(widen::literals(already_general), already_general);
}

#[test]
fn literals_widens_union_of_literals_to_int() {
    let union = u_many(vec![t_lit_int(1), t_lit_int(2), t_lit_int(3)]);
    let result = widen::literals(union);
    assert_eq!(result, prelude::TYPE_INT);
}

#[test]
fn scalars_widens_union_of_refinements_to_int() {
    let union = u_many(vec![t_positive_int(), t_negative_int(), t_lit_int(0)]);
    let result = widen::scalars(union);
    assert_eq!(result, prelude::TYPE_INT);
}
