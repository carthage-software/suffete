//! Explicit pairwise combiner cases. One test per pair.

mod combiner_common;

use combiner_common::*;
use suffete::ElementId;

fn check(input: Vec<ElementId>, expected: &[ElementId]) {
    let result = combine_default(input);
    let mut actual = result.clone();
    actual.sort();
    let mut e = expected.to_vec();
    e.sort();
    assert_eq!(actual, e, "got {result:?}, expected {expected:?}");
}

#[test]
fn p_true_true() {
    check(vec![t_true(), t_true()], &[t_true()]);
}
#[test]
fn p_true_false() {
    check(vec![t_true(), t_false()], &[t_bool()]);
}
#[test]
fn p_true_bool() {
    check(vec![t_true(), t_bool()], &[t_bool()]);
}
#[test]
fn p_false_true() {
    check(vec![t_false(), t_true()], &[t_bool()]);
}
#[test]
fn p_false_false() {
    check(vec![t_false(), t_false()], &[t_false()]);
}
#[test]
fn p_false_bool() {
    check(vec![t_false(), t_bool()], &[t_bool()]);
}
#[test]
fn p_bool_true() {
    check(vec![t_bool(), t_true()], &[t_bool()]);
}
#[test]
fn p_bool_false() {
    check(vec![t_bool(), t_false()], &[t_bool()]);
}
#[test]
fn p_bool_bool() {
    check(vec![t_bool(), t_bool()], &[t_bool()]);
}

#[test]
fn p_int_int() {
    check(vec![t_int(), t_int()], &[t_int()]);
}
#[test]
fn p_int_lit_0() {
    check(vec![t_int(), t_lit_int(0)], &[t_int()]);
}
#[test]
fn p_int_lit_1() {
    check(vec![t_int(), t_lit_int(1)], &[t_int()]);
}
#[test]
fn p_int_lit_neg() {
    check(vec![t_int(), t_lit_int(-1)], &[t_int()]);
}
#[test]
fn p_lit_0_int() {
    check(vec![t_lit_int(0), t_int()], &[t_int()]);
}
#[test]
fn p_lit_0_lit_0() {
    check(vec![t_lit_int(0), t_lit_int(0)], &[t_lit_int(0)]);
}
#[test]
fn p_lit_0_lit_1() {
    check(vec![t_lit_int(0), t_lit_int(1)], &[t_int_range(0, 1)]);
}
#[test]
fn p_lit_neg_lit_pos() {
    check(vec![t_lit_int(-1), t_lit_int(1)], &[t_lit_int(-1), t_lit_int(1)]);
}

#[test]
fn p_int_positive() {
    check(vec![t_int(), t_positive_int()], &[t_int()]);
}
#[test]
fn p_int_negative() {
    check(vec![t_int(), t_negative_int()], &[t_int()]);
}
#[test]
fn p_int_non_neg() {
    check(vec![t_int(), t_non_negative_int()], &[t_int()]);
}
#[test]
fn p_int_non_pos() {
    check(vec![t_int(), t_non_positive_int()], &[t_int()]);
}
#[test]
fn p_int_range() {
    check(vec![t_int(), t_int_range(0, 10)], &[t_int()]);
}

#[test]
fn p_positive_negative() {
    check(vec![t_positive_int(), t_negative_int()], &[t_negative_int(), t_positive_int()]);
}

#[test]
fn p_non_neg_non_pos() {
    check(vec![t_non_negative_int(), t_non_positive_int()], &[t_int()]);
}
#[test]
fn p_pos_lit_0() {
    check(vec![t_positive_int(), t_lit_int(0)], &[t_non_negative_int()]);
}
#[test]
fn p_neg_lit_0() {
    check(vec![t_negative_int(), t_lit_int(0)], &[t_non_positive_int()]);
}
#[test]
fn p_pos_lit_neg_1() {
    check(vec![t_positive_int(), t_lit_int(-1)], &[t_lit_int(-1), t_positive_int()]);
}
#[test]
fn p_range_overlap() {
    check(vec![t_int_range(0, 5), t_int_range(3, 10)], &[t_int_range(0, 10)]);
}
#[test]
fn p_range_adjacent() {
    check(vec![t_int_range(0, 5), t_int_range(6, 10)], &[t_int_range(0, 10)]);
}

#[test]
fn p_range_disjoint() {
    check(vec![t_int_range(0, 5), t_int_range(10, 15)], &[t_int_range(0, 5), t_int_range(10, 15)]);
}

#[test]
fn p_from_to_overlap() {
    check(vec![t_int_from(0), t_int_to(5)], &[t_int()]);
}
#[test]
fn p_from_to_adjacent() {
    check(vec![t_int_from(5), t_int_to(4)], &[t_int()]);
}

#[test]
fn p_from_to_disjoint() {
    check(vec![t_int_from(10), t_int_to(0)], &[t_int_from(10), t_int_to(0)]);
}

#[test]
fn p_from_lit_extends() {
    check(vec![t_int_from(5), t_lit_int(4)], &[t_int_from(4)]);
}
#[test]
fn p_to_lit_extends() {
    check(vec![t_int_to(5), t_lit_int(6)], &[t_int_to(6)]);
}

#[test]
fn p_string_string() {
    check(vec![t_string(), t_string()], &[t_string()]);
}
#[test]
fn p_string_lit_empty() {
    check(vec![t_string(), t_lit_string("")], &[t_string()]);
}
#[test]
fn p_string_lit_hi() {
    check(vec![t_string(), t_lit_string("hi")], &[t_string()]);
}
#[test]
fn p_string_lit_0() {
    check(vec![t_string(), t_lit_string("0")], &[t_string()]);
}
#[test]
fn p_lit_string_string() {
    check(vec![t_lit_string("hi"), t_string()], &[t_string()]);
}
#[test]
fn p_lit_a_lit_b() {
    check(vec![t_lit_string("a"), t_lit_string("b")], &[t_lit_string("a"), t_lit_string("b")]);
}
#[test]
fn p_lit_a_lit_a() {
    check(vec![t_lit_string("a"), t_lit_string("a")], &[t_lit_string("a")]);
}
#[test]
fn p_lit_uppercase_kept() {
    check(vec![t_lit_string("Hello"), t_lit_string("hello")], &[t_lit_string("Hello"), t_lit_string("hello")]);
}

#[test]
fn p_string_non_empty() {
    check(vec![t_string(), t_non_empty_string()], &[t_string()]);
}
#[test]
fn p_non_empty_string() {
    check(vec![t_non_empty_string(), t_string()], &[t_string()]);
}
#[test]
fn p_non_empty_lit_hi() {
    check(vec![t_non_empty_string(), t_lit_string("hi")], &[t_non_empty_string()]);
}
#[test]
fn p_non_empty_lit_0() {
    check(vec![t_non_empty_string(), t_lit_string("0")], &[t_non_empty_string()]);
}
#[test]
fn p_non_empty_lit_empty() {
    check(vec![t_non_empty_string(), t_lit_string("")], &[t_non_empty_string(), t_lit_string("")]);
}

#[test]
fn p_lit_empty_non_empty() {
    check(vec![t_lit_string(""), t_non_empty_string()], &[t_string()]);
}
#[test]
fn p_numeric_string() {
    check(vec![t_numeric_string(), t_string()], &[t_string()]);
}
#[test]
fn p_numeric_lit_123() {
    check(vec![t_numeric_string(), t_lit_string("123")], &[t_numeric_string()]);
}
#[test]
fn p_numeric_lit_abc() {
    check(vec![t_numeric_string(), t_lit_string("abc")], &[t_numeric_string(), t_lit_string("abc")]);
}
#[test]
fn p_lower_lit_hi_lower() {
    check(vec![t_lower_string(), t_lit_string("hi")], &[t_lower_string()]);
}
#[test]
fn p_lower_lit_hi_upper() {
    check(vec![t_lower_string(), t_lit_string("HI")], &[t_lower_string(), t_lit_string("HI")]);
}
#[test]
fn p_upper_lit_hi_upper() {
    check(vec![t_upper_string(), t_lit_string("HI")], &[t_upper_string()]);
}
#[test]
fn p_upper_lit_hi_lower() {
    check(vec![t_upper_string(), t_lit_string("hi")], &[t_lit_string("hi"), t_upper_string()]);
}
#[test]
fn p_truthy_lit_hi_lower() {
    check(vec![t_truthy_string(), t_lit_string("hi")], &[t_truthy_string()]);
}
#[test]
fn p_truthy_lit_0() {
    check(vec![t_truthy_string(), t_lit_string("0")], &[t_lit_string("0"), t_truthy_string()]);
}
#[test]
fn p_truthy_lit_empty() {
    check(vec![t_truthy_string(), t_lit_string("")], &[t_lit_string(""), t_truthy_string()]);
}
#[test]
fn p_lower_upper() {
    check(vec![t_lower_string(), t_upper_string()], &[t_string()]);
}
#[test]
fn p_non_empty_truthy() {
    check(vec![t_non_empty_string(), t_truthy_string()], &[t_non_empty_string()]);
}
#[test]
fn p_truthy_non_empty() {
    check(vec![t_truthy_string(), t_non_empty_string()], &[t_non_empty_string()]);
}
#[test]
fn p_non_empty_lower() {
    check(vec![t_non_empty_string(), t_lower_string()], &[t_string()]);
}

#[test]
fn p_float_float() {
    check(vec![t_float(), t_float()], &[t_float()]);
}
#[test]
fn p_float_lit() {
    check(vec![t_float(), t_lit_float(1.5)], &[t_float()]);
}
#[test]
fn p_lit_float() {
    check(vec![t_lit_float(1.5), t_float()], &[t_float()]);
}
#[test]
fn p_lit_lit_float() {
    check(vec![t_lit_float(1.5), t_lit_float(1.5)], &[t_lit_float(1.5)]);
}
#[test]
fn p_lit_lit_float_distinct() {
    check(vec![t_lit_float(1.0), t_lit_float(2.0)], &[t_lit_float(1.0), t_lit_float(2.0)]);
}

#[test]
fn p_int_string() {
    check(vec![t_int(), t_string()], &[t_int(), t_string()]);
}
#[test]
fn p_int_float() {
    check(vec![t_int(), t_float()], &[t_float(), t_int()]);
}
#[test]
fn p_int_bool() {
    check(vec![t_int(), t_bool()], &[t_bool(), t_int()]);
}
#[test]
fn p_string_float() {
    check(vec![t_string(), t_float()], &[t_float(), t_string()]);
}
#[test]
fn p_string_bool() {
    check(vec![t_string(), t_bool()], &[t_bool(), t_string()]);
}
#[test]
fn p_float_bool() {
    check(vec![t_float(), t_bool()], &[t_bool(), t_float()]);
}

#[test]
fn p_numeric_int() {
    check(vec![t_numeric(), t_int()], &[t_numeric()]);
}
#[test]
fn p_int_numeric() {
    check(vec![t_int(), t_numeric()], &[t_numeric()]);
}
#[test]
fn p_numeric_float() {
    check(vec![t_numeric(), t_float()], &[t_numeric()]);
}
#[test]
fn p_float_numeric() {
    check(vec![t_float(), t_numeric()], &[t_numeric()]);
}
#[test]
fn p_numeric_lit_int() {
    check(vec![t_numeric(), t_lit_int(5)], &[t_numeric()]);
}
#[test]
fn p_lit_int_numeric() {
    check(vec![t_lit_int(5), t_numeric()], &[t_numeric()]);
}

#[test]
fn p_ak_int() {
    check(vec![t_array_key(), t_int()], &[t_array_key()]);
}
#[test]
fn p_int_ak() {
    check(vec![t_int(), t_array_key()], &[t_array_key()]);
}
#[test]
fn p_ak_string() {
    check(vec![t_array_key(), t_string()], &[t_array_key()]);
}
#[test]
fn p_string_ak() {
    check(vec![t_string(), t_array_key()], &[t_array_key()]);
}
#[test]
fn p_ak_float() {
    check(vec![t_array_key(), t_float()], &[t_array_key(), t_float()]);
}
#[test]
fn p_ak_bool() {
    check(vec![t_array_key(), t_bool()], &[t_array_key(), t_bool()]);
}
#[test]
fn p_ak_null() {
    check(vec![t_array_key(), null()], &[t_array_key(), null()]);
}

#[test]
fn p_scalar_int() {
    check(vec![t_scalar(), t_int()], &[t_scalar()]);
}
#[test]
fn p_int_scalar() {
    check(vec![t_int(), t_scalar()], &[t_scalar()]);
}
#[test]
fn p_scalar_string() {
    check(vec![t_scalar(), t_string()], &[t_scalar()]);
}
#[test]
fn p_string_scalar() {
    check(vec![t_string(), t_scalar()], &[t_scalar()]);
}
#[test]
fn p_scalar_float() {
    check(vec![t_scalar(), t_float()], &[t_scalar()]);
}
#[test]
fn p_float_scalar() {
    check(vec![t_float(), t_scalar()], &[t_scalar()]);
}
#[test]
fn p_scalar_numeric() {
    check(vec![t_scalar(), t_numeric()], &[t_scalar()]);
}
#[test]
fn p_numeric_scalar() {
    check(vec![t_numeric(), t_scalar()], &[t_scalar()]);
}
#[test]
fn p_scalar_ak() {
    check(vec![t_scalar(), t_array_key()], &[t_scalar()]);
}
#[test]
fn p_ak_scalar() {
    check(vec![t_array_key(), t_scalar()], &[t_scalar()]);
}
#[test]
fn p_scalar_bool() {
    check(vec![t_scalar(), t_bool()], &[t_scalar()]);
}
#[test]
fn p_bool_scalar() {
    check(vec![t_bool(), t_scalar()], &[t_scalar()]);
}
#[test]
fn p_scalar_true() {
    check(vec![t_scalar(), t_true()], &[t_scalar()]);
}
#[test]
fn p_true_scalar() {
    check(vec![t_true(), t_scalar()], &[t_scalar()]);
}
#[test]
fn p_scalar_false() {
    check(vec![t_scalar(), t_false()], &[t_scalar()]);
}
#[test]
fn p_false_scalar() {
    check(vec![t_false(), t_scalar()], &[t_scalar()]);
}

#[test]
fn p_null_null() {
    check(vec![null(), null()], &[null()]);
}
#[test]
fn p_void_void() {
    check(vec![void(), void()], &[void()]);
}
#[test]
fn p_never_never() {
    check(vec![never(), never()], &[never()]);
}
#[test]
fn p_null_void() {
    check(vec![null(), void()], &[null()]);
}
#[test]
fn p_void_null() {
    check(vec![void(), null()], &[null()]);
}
#[test]
fn p_null_never() {
    check(vec![null(), never()], &[null()]);
}
#[test]
fn p_never_null() {
    check(vec![never(), null()], &[null()]);
}
#[test]
fn p_void_never() {
    check(vec![void(), never()], &[void()]);
}
#[test]
fn p_never_void() {
    check(vec![never(), void()], &[void()]);
}
#[test]
fn p_null_int() {
    check(vec![null(), t_int()], &[t_int(), null()]);
}
#[test]
fn p_void_int() {
    check(vec![void(), t_int()], &[t_int(), void()]);
}
#[test]
fn p_never_int() {
    check(vec![never(), t_int()], &[t_int()]);
}
#[test]
fn p_null_object() {
    check(vec![null(), t_object_any()], &[null(), t_object_any()]);
}
#[test]
fn p_void_object() {
    check(vec![void(), t_object_any()], &[t_object_any(), void()]);
}
#[test]
fn p_never_object() {
    check(vec![never(), t_object_any()], &[t_object_any()]);
}
#[test]
fn p_null_resource() {
    check(vec![null(), t_resource()], &[null(), t_resource()]);
}
#[test]
fn p_void_resource() {
    check(vec![void(), t_resource()], &[t_resource(), void()]);
}
#[test]
fn p_never_resource() {
    check(vec![never(), t_resource()], &[t_resource()]);
}

#[test]
fn p_mixed_int() {
    check(vec![mixed(), t_int()], &[mixed()]);
}
#[test]
fn p_int_mixed() {
    check(vec![t_int(), mixed()], &[mixed()]);
}
#[test]
fn p_mixed_string() {
    check(vec![mixed(), t_string()], &[mixed()]);
}
#[test]
fn p_mixed_object() {
    check(vec![mixed(), t_object_any()], &[mixed()]);
}
#[test]
fn p_mixed_array() {
    check(vec![mixed(), t_empty_array()], &[mixed()]);
}
#[test]
fn p_mixed_null() {
    check(vec![mixed(), null()], &[mixed()]);
}
#[test]
fn p_mixed_void() {
    check(vec![mixed(), void()], &[mixed()]);
}
#[test]
fn p_mixed_never() {
    check(vec![mixed(), never()], &[mixed()]);
}
#[test]
fn p_mixed_resource() {
    check(vec![mixed(), t_resource()], &[mixed()]);
}
#[test]
fn p_mixed_mixed() {
    check(vec![mixed(), mixed()], &[mixed()]);
}

#[test]
fn p_cs_cs() {
    check(vec![t_class_string(), t_class_string()], &[t_class_string()]);
}
#[test]
fn p_is_is() {
    check(vec![t_interface_string(), t_interface_string()], &[t_interface_string()]);
}
#[test]
fn p_es_es() {
    check(vec![t_enum_string(), t_enum_string()], &[t_enum_string()]);
}
#[test]
fn p_ts_ts() {
    check(vec![t_trait_string(), t_trait_string()], &[t_trait_string()]);
}
#[test]
fn p_cs_is() {
    check(vec![t_class_string(), t_interface_string()], &[t_class_string(), t_interface_string()]);
}
#[test]
fn p_cs_es() {
    check(vec![t_class_string(), t_enum_string()], &[t_class_string(), t_enum_string()]);
}
#[test]
fn p_cs_string() {
    check(vec![t_class_string(), t_string()], &[t_string()]);
}
#[test]
fn p_string_cs() {
    check(vec![t_string(), t_class_string()], &[t_string()]);
}
#[test]
fn p_cs_ak() {
    check(vec![t_class_string(), t_array_key()], &[t_array_key()]);
}
#[test]
fn p_cs_scalar() {
    check(vec![t_class_string(), t_scalar()], &[t_scalar()]);
}
#[test]
fn p_cs_int() {
    check(vec![t_class_string(), t_int()], &[t_class_string(), t_int()]);
}
#[test]
fn p_cs_null() {
    check(vec![t_class_string(), null()], &[t_class_string(), null()]);
}
#[test]
fn p_cs_never() {
    check(vec![t_class_string(), never()], &[t_class_string()]);
}

#[test]
fn p_res_res() {
    check(vec![t_resource(), t_resource()], &[t_resource()]);
}
#[test]
fn p_open_open() {
    check(vec![t_open_resource(), t_open_resource()], &[t_open_resource()]);
}
#[test]
fn p_closed_closed() {
    check(vec![t_closed_resource(), t_closed_resource()], &[t_closed_resource()]);
}
#[test]
fn p_open_closed() {
    check(vec![t_open_resource(), t_closed_resource()], &[t_resource()]);
}
#[test]
fn p_closed_open() {
    check(vec![t_closed_resource(), t_open_resource()], &[t_resource()]);
}
#[test]
fn p_res_open() {
    check(vec![t_resource(), t_open_resource()], &[t_resource()]);
}
#[test]
fn p_res_closed() {
    check(vec![t_resource(), t_closed_resource()], &[t_resource()]);
}
#[test]
fn p_open_res() {
    check(vec![t_open_resource(), t_resource()], &[t_resource()]);
}
#[test]
fn p_closed_res() {
    check(vec![t_closed_resource(), t_resource()], &[t_resource()]);
}

#[test]
fn p_obj_obj() {
    check(vec![t_object_any(), t_object_any()], &[t_object_any()]);
}
#[test]
fn p_obj_foo() {
    check(vec![t_object_any(), t_named("Foo")], &[t_object_any()]);
}
#[test]
fn p_foo_obj() {
    check(vec![t_named("Foo"), t_object_any()], &[t_object_any()]);
}
#[test]
fn p_foo_foo() {
    check(vec![t_named("Foo"), t_named("Foo")], &[t_named("Foo")]);
}
#[test]
fn p_foo_bar() {
    check(vec![t_named("Foo"), t_named("Bar")], &[t_named("Bar"), t_named("Foo")]);
}
#[test]
fn p_e_e() {
    check(vec![t_enum("E"), t_enum("E")], &[t_enum("E")]);
}
#[test]
fn p_e_f() {
    check(vec![t_enum("E"), t_enum("F")], &[t_enum("E"), t_enum("F")]);
}
#[test]
fn p_e_ea() {
    check(vec![t_enum("E"), t_enum_case("E", "A")], &[t_enum("E")]);
}
#[test]
fn p_ea_ea() {
    check(vec![t_enum_case("E", "A"), t_enum_case("E", "A")], &[t_enum_case("E", "A")]);
}
#[test]
fn p_ea_eb() {
    check(vec![t_enum_case("E", "A"), t_enum_case("E", "B")], &[t_enum_case("E", "A"), t_enum_case("E", "B")]);
}

#[test]
fn p_arr_empty() {
    check(vec![t_empty_array()], &[t_empty_array()]);
}
#[test]
fn p_arr_empty_empty() {
    check(vec![t_empty_array(), t_empty_array()], &[t_empty_array()]);
}
#[test]
fn p_arr_list_int() {
    use suffete::prelude::TYPE_INT;
    check(vec![t_list(TYPE_INT, false)], &[t_list(TYPE_INT, false)]);
}
#[test]
fn p_arr_list_int_x2() {
    use suffete::prelude::TYPE_INT;
    check(vec![t_list(TYPE_INT, false), t_list(TYPE_INT, false)], &[t_list(TYPE_INT, false)]);
}
#[test]
fn p_arr_list_int_string() {
    use suffete::FlowFlags;
    use suffete::interner::interner;
    use suffete::prelude::TYPE_INT;
    use suffete::prelude::TYPE_STRING;
    let int_or_string = interner().intern_type(&[suffete::prelude::INT, suffete::prelude::STRING], FlowFlags::EMPTY);
    check(vec![t_list(TYPE_INT, false), t_list(TYPE_STRING, false)], &[t_list(int_or_string, false)]);
}
#[test]
fn p_arr_ne_list_int() {
    use suffete::prelude::TYPE_INT;
    check(vec![t_list(TYPE_INT, true)], &[t_list(TYPE_INT, true)]);
}
#[test]
fn p_arr_ne_list_x2() {
    use suffete::prelude::TYPE_INT;
    check(vec![t_list(TYPE_INT, true), t_list(TYPE_INT, true)], &[t_list(TYPE_INT, true)]);
}
#[test]
fn p_arr_ne_with_e() {
    use suffete::prelude::TYPE_INT;
    check(vec![t_list(TYPE_INT, true), t_list(TYPE_INT, false)], &[t_list(TYPE_INT, false)]);
}
#[test]
fn p_arr_empty_with_list() {
    use suffete::prelude::TYPE_INT;
    check(vec![t_empty_array(), t_list(TYPE_INT, false)], &[t_list(TYPE_INT, false)]);
}
#[test]
fn p_arr_list_with_empty() {
    use suffete::prelude::TYPE_INT;
    check(vec![t_list(TYPE_INT, false), t_empty_array()], &[t_list(TYPE_INT, false)]);
}
