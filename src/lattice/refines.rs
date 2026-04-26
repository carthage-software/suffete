//! Refinement (subtype) relation: `refines(a, b)` is `true` iff every value
//! of type `a` is also a value of type `b` (i.e. `a <: b`).

use crate::ElementId;
use crate::ElementKind;
use crate::TypeId;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::lattice::family;
use crate::prelude::FALSE;
use crate::prelude::MIXED;
use crate::prelude::NEVER;
use crate::prelude::NULL;
use crate::world::World;

/// `true` iff `a <: b` — every runtime value of type `a` is also a value of
/// type `b` (i.e. `a` is a refinement / narrowing of `b`).
///
/// Implements the universal axioms (refl / Bot / Top from spec §4.1, §4.2),
/// the union dispatch (Union-L / Union-R from §4.3), and the structural
/// scalar lattice (bool / int / float / string / class-like-string /
/// resource / array-key / numeric / scalar / object-any). Object hierarchy
/// queries flow through `codebase`; callable variance, array shape rules,
/// mixed-axis refinements, and template machinery layer in family by
/// family; what isn't implemented returns `false` conservatively.
pub fn refines<W: World>(
    a: TypeId,
    b: TypeId,
    codebase: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    if a == b && !options.ignore_null && !options.ignore_false {
        return true;
    }

    let a_type = a.as_ref();
    let b_type = b.as_ref();

    // Union-L / Union-R: every element of `a` (modulo any caller-requested
    // skips for `null` / `false`) must fit some element of `b`.
    a_type
        .elements
        .iter()
        .filter(|input| {
            let skipped = (options.ignore_null && **input == NULL) || (options.ignore_false && **input == FALSE);
            !skipped
        })
        .all(|input| {
            b_type.elements.iter().any(|container| element_refines(*input, *container, codebase, options, report))
        })
}

/// `true` iff `a :> b` — every value of type `b` is also a value of type `a`
/// (`a` generalizes `b`). Equivalent to `refines(b, a, codebase, options, report)`.
#[inline]
pub fn generalizes<W: World>(
    a: TypeId,
    b: TypeId,
    codebase: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    refines(b, a, codebase, options, report)
}

/// Decide whether one element refines another, ignoring flow flags.
///
/// Universal axioms first (refl, `never <: anything`, `anything <: mixed`),
/// then dispatch on the container's kind into a family-specific helper.
/// When the result is `false` and the input belongs to a "true-union" kind
/// (`mixed`, `array_key`, `bool`, `object_any`, `scalar`, `numeric`), the
/// `type_coerced` flag is set to record that the rejection was a narrowing,
/// not an out-of-family mismatch. `mixed` inputs additionally set
/// `type_coerced_from_nested_mixed`.
fn element_refines<W: World>(
    input: ElementId,
    container: ElementId,
    codebase: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    if input == container {
        return true;
    }

    if input == NEVER {
        return true;
    }

    if container == MIXED {
        return true;
    }

    let result = dispatch_refines(input, container, codebase, options, report);

    if !result && is_true_union_kind(input.kind()) {
        report.type_coerced = Some(true);
        if input.kind() == ElementKind::Mixed {
            report.type_coerced_from_nested_mixed = Some(true);
        }
    }

    result
}

/// `true` for kinds whose values inhabit multiple disjoint sub-families:
/// narrowing one of these to a concrete sub-form is the standard PHP
/// "type-coerced" pattern that the lattice records via `type_coerced`.
fn is_true_union_kind(kind: ElementKind) -> bool {
    matches!(
        kind,
        ElementKind::Mixed
            | ElementKind::ArrayKey
            | ElementKind::Bool
            | ElementKind::ObjectAny
            | ElementKind::Scalar
            | ElementKind::Numeric
    )
}

fn dispatch_refines<W: World>(
    input: ElementId,
    container: ElementId,
    codebase: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    match container.kind() {
        ElementKind::Bool => family::bool::refines(input, container),
        ElementKind::Resource => family::resource::refines(input, container),
        ElementKind::Int => family::int::refines(input, container),
        ElementKind::Float => family::float::refines(input, container),
        ElementKind::String => family::string::refines(input, container),
        ElementKind::ClassLikeString => family::class_like_string::refines(input, container),
        ElementKind::ArrayKey => family::array_key::refines(input, container),
        ElementKind::Numeric => family::numeric::refines(input, container),
        ElementKind::Scalar => family::scalar::refines(input, container),
        ElementKind::ObjectAny => family::object::refines_object_any(input, container),
        ElementKind::Object
        | ElementKind::Enum
        | ElementKind::ObjectShape
        | ElementKind::HasMethod
        | ElementKind::HasProperty => family::object::refines(input, container, codebase),
        ElementKind::Array | ElementKind::List => family::array::refines(input, container, codebase, options, report),
        ElementKind::Iterable => family::iterable::refines(input, container, codebase, options, report),
        ElementKind::Callable => family::callable::refines(input, container),
        ElementKind::Mixed => family::mixed::refines(input, container),
        ElementKind::GenericParameter => family::generic::refines(input, container),
        ElementKind::Variable
        | ElementKind::Reference
        | ElementKind::MemberReference
        | ElementKind::GlobalReference
        | ElementKind::Alias
        | ElementKind::Conditional
        | ElementKind::Derived => family::reference::refines(input, container),
        ElementKind::Null
        | ElementKind::Never
        | ElementKind::Void
        | ElementKind::Placeholder
        | ElementKind::True
        | ElementKind::False => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ElementId;
    use crate::FlowFlags;
    use crate::interner::interner;
    use crate::prelude::ARRAY_KEY;
    use crate::prelude::BOOL;
    use crate::prelude::CALLABLE_STRING;
    use crate::prelude::CLASS_STRING;
    use crate::prelude::CLOSED_RESOURCE;
    use crate::prelude::ENUM_STRING;
    use crate::prelude::FALSE;
    use crate::prelude::FLOAT;
    use crate::prelude::INT;
    use crate::prelude::INTERFACE_STRING;
    use crate::prelude::LITERAL_FLOAT;
    use crate::prelude::LITERAL_INT;
    use crate::prelude::LITERAL_STRING;
    use crate::prelude::LOWERCASE_STRING;
    use crate::prelude::NEGATIVE_INT;
    use crate::prelude::NON_EMPTY_STRING;
    use crate::prelude::NULL;
    use crate::prelude::NUMERIC;
    use crate::prelude::NUMERIC_STRING;
    use crate::prelude::OPEN_RESOURCE;
    use crate::prelude::POSITIVE_INT;
    use crate::prelude::RESOURCE;
    use crate::prelude::SCALAR;
    use crate::prelude::STRING;
    use crate::prelude::TRUE;
    use crate::prelude::TRUTHY_STRING;
    use crate::prelude::TYPE_ARRAY_KEY;
    use crate::prelude::TYPE_BOOL;
    use crate::prelude::TYPE_FLOAT;
    use crate::prelude::TYPE_INT;
    use crate::prelude::TYPE_INT_OR_FLOAT;
    use crate::prelude::TYPE_INT_OR_STRING;
    use crate::prelude::TYPE_MIXED;
    use crate::prelude::TYPE_NEVER;
    use crate::prelude::TYPE_NULL;
    use crate::prelude::TYPE_NUMERIC;
    use crate::prelude::TYPE_SCALAR;
    use crate::prelude::TYPE_STRING;
    use crate::prelude::UPPERCASE_STRING;
    use crate::world::NullWorld;

    fn check(input: TypeId, container: TypeId) -> bool {
        let mut report = LatticeReport::new();
        refines(input, container, &NullWorld, LatticeOptions::default(), &mut report)
    }

    fn check_elem(input: ElementId, container: ElementId) -> bool {
        let i = interner();
        let it = i.intern_type(&[input], FlowFlags::EMPTY);
        let ct = i.intern_type(&[container], FlowFlags::EMPTY);
        check(it, ct)
    }

    #[test]
    fn reflexivity_holds_for_well_known_types() {
        assert!(check(TYPE_INT, TYPE_INT));
        assert!(check(TYPE_NULL, TYPE_NULL));
        assert!(check(TYPE_NEVER, TYPE_NEVER));
        assert!(check(TYPE_MIXED, TYPE_MIXED));
        assert!(check(TYPE_INT_OR_STRING, TYPE_INT_OR_STRING));
    }

    #[test]
    fn bot_axiom_never_refines_anything() {
        assert!(check(TYPE_NEVER, TYPE_INT));
        assert!(check(TYPE_NEVER, TYPE_NULL));
        assert!(check(TYPE_NEVER, TYPE_MIXED));
        assert!(check(TYPE_NEVER, TYPE_INT_OR_STRING));
    }

    #[test]
    fn top_axiom_anything_refines_vanilla_mixed() {
        assert!(check(TYPE_INT, TYPE_MIXED));
        assert!(check(TYPE_NULL, TYPE_MIXED));
        assert!(check(TYPE_STRING, TYPE_MIXED));
        assert!(check(TYPE_INT_OR_STRING, TYPE_MIXED));
    }

    #[test]
    fn bool_family_refines_bool() {
        assert!(check_elem(TRUE, BOOL));
        assert!(check_elem(FALSE, BOOL));
        assert!(!check_elem(TRUE, FALSE));
        assert!(!check_elem(FALSE, TRUE));
        assert!(!check_elem(BOOL, TRUE));
        assert!(!check_elem(BOOL, FALSE));
    }

    #[test]
    fn resource_family_refines_resource() {
        assert!(check_elem(OPEN_RESOURCE, RESOURCE));
        assert!(check_elem(CLOSED_RESOURCE, RESOURCE));
        assert!(!check_elem(OPEN_RESOURCE, CLOSED_RESOURCE));
        assert!(!check_elem(CLOSED_RESOURCE, OPEN_RESOURCE));
        assert!(!check_elem(RESOURCE, OPEN_RESOURCE));
    }

    #[test]
    fn int_dominator_absorbs_subforms() {
        assert!(check_elem(POSITIVE_INT, INT));
        assert!(check_elem(NEGATIVE_INT, INT));
        assert!(check_elem(LITERAL_INT, INT));
        assert!(check_elem(ElementId::int_literal(42), INT));
        assert!(check_elem(ElementId::int_literal(-1), INT));
    }

    #[test]
    fn int_literal_in_range() {
        let r = ElementId::int_range(Some(0), Some(10));
        assert!(check_elem(ElementId::int_literal(0), r));
        assert!(check_elem(ElementId::int_literal(5), r));
        assert!(check_elem(ElementId::int_literal(10), r));
        assert!(!check_elem(ElementId::int_literal(-1), r));
        assert!(!check_elem(ElementId::int_literal(11), r));
    }

    #[test]
    fn int_range_in_range() {
        let outer = ElementId::int_range(Some(0), Some(100));
        let inner = ElementId::int_range(Some(10), Some(20));
        assert!(check_elem(inner, outer));
        assert!(!check_elem(outer, inner));
    }

    #[test]
    fn int_open_range_subsumes_closed() {
        let from_zero = ElementId::int_range(Some(0), None);
        let bounded = ElementId::int_range(Some(5), Some(10));
        assert!(check_elem(bounded, from_zero));
        assert!(!check_elem(from_zero, bounded));
    }

    #[test]
    fn int_unspec_literal_accepts_concrete_literals() {
        assert!(check_elem(ElementId::int_literal(42), LITERAL_INT));
        assert!(check_elem(ElementId::int_literal(-1), LITERAL_INT));
        assert!(check_elem(LITERAL_INT, LITERAL_INT));
        assert!(!check_elem(INT, LITERAL_INT));
    }

    #[test]
    fn float_dominator_absorbs_subforms() {
        assert!(check_elem(LITERAL_FLOAT, FLOAT));
        assert!(check_elem(ElementId::float_literal(1.5), FLOAT));
        assert!(check_elem(ElementId::float_literal(-2.5), FLOAT));
    }

    #[test]
    fn float_unspec_literal_accepts_concrete_literals() {
        assert!(check_elem(ElementId::float_literal(1.5), LITERAL_FLOAT));
        assert!(check_elem(LITERAL_FLOAT, LITERAL_FLOAT));
        assert!(!check_elem(FLOAT, LITERAL_FLOAT));
    }

    #[test]
    fn string_dominator_absorbs_subforms() {
        assert!(check_elem(NON_EMPTY_STRING, STRING));
        assert!(check_elem(NUMERIC_STRING, STRING));
        assert!(check_elem(LOWERCASE_STRING, STRING));
        assert!(check_elem(UPPERCASE_STRING, STRING));
        assert!(check_elem(TRUTHY_STRING, STRING));
        assert!(check_elem(LITERAL_STRING, STRING));
        assert!(check_elem(ElementId::string_literal("hi"), STRING));
        assert!(check_elem(ElementId::string_literal(""), STRING));
    }

    #[test]
    fn string_literal_satisfies_non_empty() {
        assert!(check_elem(ElementId::string_literal("hi"), NON_EMPTY_STRING));
        assert!(check_elem(ElementId::string_literal("0"), NON_EMPTY_STRING));
        assert!(!check_elem(ElementId::string_literal(""), NON_EMPTY_STRING));
    }

    #[test]
    fn string_literal_satisfies_truthy() {
        assert!(check_elem(ElementId::string_literal("hi"), TRUTHY_STRING));
        assert!(check_elem(ElementId::string_literal("1"), TRUTHY_STRING));
        assert!(!check_elem(ElementId::string_literal(""), TRUTHY_STRING));
        assert!(!check_elem(ElementId::string_literal("0"), TRUTHY_STRING));
    }

    #[test]
    fn string_literal_satisfies_numeric() {
        assert!(check_elem(ElementId::string_literal("123"), NUMERIC_STRING));
        assert!(check_elem(ElementId::string_literal("-1"), NUMERIC_STRING));
        assert!(check_elem(ElementId::string_literal("1.5"), NUMERIC_STRING));
        assert!(!check_elem(ElementId::string_literal("hi"), NUMERIC_STRING));
        assert!(!check_elem(ElementId::string_literal(""), NUMERIC_STRING));
    }

    #[test]
    fn string_literal_satisfies_casing() {
        assert!(check_elem(ElementId::string_literal("hello"), LOWERCASE_STRING));
        assert!(!check_elem(ElementId::string_literal("Hello"), LOWERCASE_STRING));
        assert!(check_elem(ElementId::string_literal("HELLO"), UPPERCASE_STRING));
        assert!(!check_elem(ElementId::string_literal("Hello"), UPPERCASE_STRING));
        // Case-neutral characters satisfy both.
        assert!(check_elem(ElementId::string_literal("123"), LOWERCASE_STRING));
        assert!(check_elem(ElementId::string_literal("123"), UPPERCASE_STRING));
    }

    #[test]
    fn truthy_string_refines_non_empty_string() {
        assert!(check_elem(TRUTHY_STRING, NON_EMPTY_STRING));
    }

    #[test]
    fn callable_string_does_not_refine_numeric_or_lowercase_by_default() {
        assert!(check_elem(CALLABLE_STRING, STRING));
        assert!(!check_elem(CALLABLE_STRING, NUMERIC_STRING));
        assert!(!check_elem(CALLABLE_STRING, LOWERCASE_STRING));
    }

    #[test]
    fn class_like_string_refines_string() {
        assert!(check_elem(CLASS_STRING, STRING));
        assert!(check_elem(INTERFACE_STRING, STRING));
        assert!(check_elem(ENUM_STRING, STRING));
    }

    #[test]
    fn distinct_class_like_kinds_are_not_subtypes() {
        assert!(!check_elem(CLASS_STRING, INTERFACE_STRING));
        assert!(!check_elem(INTERFACE_STRING, CLASS_STRING));
        assert!(!check_elem(CLASS_STRING, ENUM_STRING));
    }

    #[test]
    fn array_key_absorbs_int_string_class_string() {
        assert!(check_elem(INT, ARRAY_KEY));
        assert!(check_elem(STRING, ARRAY_KEY));
        assert!(check_elem(CLASS_STRING, ARRAY_KEY));
        assert!(check_elem(ElementId::int_literal(42), ARRAY_KEY));
        assert!(check_elem(ElementId::string_literal("k"), ARRAY_KEY));
    }

    #[test]
    fn array_key_does_not_absorb_float_or_bool() {
        assert!(!check_elem(FLOAT, ARRAY_KEY));
        assert!(!check_elem(BOOL, ARRAY_KEY));
        assert!(!check_elem(NULL, ARRAY_KEY));
    }

    #[test]
    fn numeric_absorbs_int_float_and_numeric_string() {
        assert!(check_elem(INT, NUMERIC));
        assert!(check_elem(FLOAT, NUMERIC));
        assert!(check_elem(NUMERIC_STRING, NUMERIC));
        assert!(check_elem(ElementId::int_literal(5), NUMERIC));
        assert!(check_elem(ElementId::float_literal(1.5), NUMERIC));
        assert!(check_elem(ElementId::string_literal("123"), NUMERIC));
    }

    #[test]
    fn numeric_does_not_absorb_general_string() {
        assert!(!check_elem(STRING, NUMERIC));
        assert!(!check_elem(ElementId::string_literal("hi"), NUMERIC));
        assert!(!check_elem(BOOL, NUMERIC));
    }

    #[test]
    fn scalar_absorbs_all_scalar_families() {
        assert!(check_elem(INT, SCALAR));
        assert!(check_elem(FLOAT, SCALAR));
        assert!(check_elem(STRING, SCALAR));
        assert!(check_elem(BOOL, SCALAR));
        assert!(check_elem(TRUE, SCALAR));
        assert!(check_elem(FALSE, SCALAR));
        assert!(check_elem(ARRAY_KEY, SCALAR));
        assert!(check_elem(NUMERIC, SCALAR));
        assert!(check_elem(CLASS_STRING, SCALAR));
        assert!(check_elem(ElementId::int_literal(0), SCALAR));
    }

    #[test]
    fn scalar_does_not_absorb_null_or_resource() {
        assert!(!check_elem(NULL, SCALAR));
        assert!(!check_elem(RESOURCE, SCALAR));
    }

    #[test]
    fn union_left_every_input_element_must_fit_some_container_element() {
        assert!(check(TYPE_INT_OR_STRING, TYPE_MIXED));
        assert!(check(TYPE_INT_OR_FLOAT, TYPE_INT_OR_FLOAT));
        assert!(!check(TYPE_INT_OR_STRING, TYPE_INT));
    }

    #[test]
    fn union_right_singleton_input_fits_member_of_union() {
        assert!(check(TYPE_INT, TYPE_INT_OR_STRING));
        assert!(check(TYPE_STRING, TYPE_INT_OR_STRING));
        assert!(!check(TYPE_FLOAT, TYPE_INT_OR_STRING));
    }

    #[test]
    fn unrelated_types_do_not_refine() {
        assert!(!check(TYPE_INT, TYPE_STRING));
        assert!(!check(TYPE_FLOAT, TYPE_STRING));
        assert!(!check(TYPE_NULL, TYPE_INT));
    }

    #[test]
    fn int_refines_float_via_php_coercion() {
        // PHP implicitly coerces int -> float; the lattice records this as a
        // refinement edge for the general `float` container.
        assert!(check(TYPE_INT, TYPE_FLOAT));
    }

    #[test]
    fn fresh_int_literal_refines_int() {
        let lit = ElementId::int_literal(42);
        let lit_type = interner().intern_type(&[lit], FlowFlags::EMPTY);
        assert!(check(lit_type, TYPE_INT));
        assert!(check(lit_type, TYPE_MIXED));
    }

    #[test]
    fn input_int_or_float_fits_int_or_float_via_union_dispatch() {
        let int_or_float = interner().intern_type(&[INT, FLOAT], FlowFlags::EMPTY);
        assert!(check(int_or_float, TYPE_INT_OR_FLOAT));

        let with_string = interner().intern_type(&[INT, FLOAT, STRING], FlowFlags::EMPTY);
        assert!(!check(with_string, TYPE_INT_OR_FLOAT));
    }

    #[test]
    fn int_or_string_refines_array_key() {
        assert!(check(TYPE_INT_OR_STRING, TYPE_ARRAY_KEY));
    }

    #[test]
    fn int_or_float_refines_numeric() {
        assert!(check(TYPE_INT_OR_FLOAT, TYPE_NUMERIC));
    }

    #[test]
    fn int_or_string_or_float_or_bool_refines_scalar() {
        let id = interner().intern_type(&[INT, STRING, FLOAT, BOOL], FlowFlags::EMPTY);
        assert!(check(id, TYPE_SCALAR));
    }

    #[test]
    fn nullable_int_refines_nullable_array_key() {
        let nullable_int = interner().intern_type(&[NULL, INT], FlowFlags::EMPTY);
        let nullable_ak = interner().intern_type(&[NULL, ARRAY_KEY], FlowFlags::EMPTY);
        assert!(check(nullable_int, nullable_ak));
    }

    #[test]
    fn type_bool_refines_scalar() {
        assert!(check(TYPE_BOOL, TYPE_SCALAR));
    }

    #[test]
    fn generalizes_is_inverse_of_refines() {
        let mut r = LatticeReport::new();
        assert!(generalizes(TYPE_MIXED, TYPE_INT, &NullWorld, LatticeOptions::default(), &mut r));
        assert!(generalizes(TYPE_INT, TYPE_NEVER, &NullWorld, LatticeOptions::default(), &mut r));
        assert!(!generalizes(TYPE_INT, TYPE_FLOAT, &NullWorld, LatticeOptions::default(), &mut r));
    }
}
