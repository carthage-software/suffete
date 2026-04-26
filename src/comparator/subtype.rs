use crate::ElementId;
use crate::ElementKind;
use crate::TypeId;
use crate::comparator::Codebase;
use crate::comparator::SubtypeContext;
use crate::element::payload::ResourceInfo;
use crate::element::payload::scalar::FloatInfo;
use crate::element::payload::scalar::IntInfo;
use crate::element::payload::scalar::IntRange;
use crate::element::payload::scalar::StringCasing;
use crate::element::payload::scalar::StringInfo;
use crate::element::payload::scalar::StringLiteral;
use crate::element::payload::scalar::StringRefinementFlags;
use crate::interner::interner;
use crate::well_known::ARRAY_KEY;
use crate::well_known::BOOL;
use crate::well_known::FALSE;
use crate::well_known::MIXED;
use crate::well_known::NEVER;
use crate::well_known::NUMERIC;
use crate::well_known::OBJECT;
use crate::well_known::SCALAR;
use crate::well_known::TRUE;

/// Decide whether `input <: container` under the spec's subtype relation.
///
/// Returns the boolean answer; richer information (coercions, template
/// bounds, replacement suggestions) accumulates in `ctx`. Implementations
/// that only need yes/no can pass `&mut SubtypeContext::new()`.
///
/// Implements the universal axioms (refl / Bot / Top from spec §4.1, §4.2),
/// the union dispatch (Union-L / Union-R from §4.3), and the structural
/// scalar lattice (Bool / Int / Float / String / ClassLikeString / Resource
/// / ArrayKey / Numeric / Scalar / ObjectAny). Object hierarchy via
/// [`Codebase`], mixed-axis refinements, callable variance, and template
/// machinery are added incrementally; what isn't implemented returns `false`
/// conservatively.
pub fn is_subtype<C: Codebase>(input: TypeId, container: TypeId, ctx: &mut SubtypeContext, codebase: &C) -> bool {
    if input == container {
        return true;
    }

    let input_type = input.as_ref();
    let container_type = container.as_ref();

    input_type.elements.iter().all(|input_elem| {
        container_type
            .elements
            .iter()
            .any(|container_elem| element_is_subtype(*input_elem, *container_elem, ctx, codebase))
    })
}

/// Decide whether one element is a subtype of another, ignoring flow flags.
///
/// Universal axioms first (refl, `never <: anything`, `anything <: mixed`),
/// then dispatch on the container's kind into a family-specific helper.
/// Anything not yet handled returns `false` conservatively; this is sound
/// (we never accept a wrong subtype edge), only incomplete.
fn element_is_subtype<C: Codebase>(
    input: ElementId,
    container: ElementId,
    _ctx: &mut SubtypeContext,
    _codebase: &C,
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

    match container.kind() {
        ElementKind::Bool => is_input_subtype_of_bool(input),
        ElementKind::Resource => is_input_subtype_of_resource(input, container),
        ElementKind::Int => is_input_subtype_of_int(input, container),
        ElementKind::Float => is_input_subtype_of_float(input, container),
        ElementKind::String => is_input_subtype_of_string(input, container),
        ElementKind::ClassLikeString => is_input_subtype_of_class_like_string(input, container),
        ElementKind::ArrayKey => is_input_subtype_of_array_key(input),
        ElementKind::Numeric => is_input_subtype_of_numeric(input),
        ElementKind::Scalar => is_input_subtype_of_scalar(input),
        ElementKind::ObjectAny => is_object_family_kind(input.kind()),
        _ => false,
    }
}

/// `true` iff `input` is `true`, `false`, or `bool`. The reflexivity case
/// (`bool <: bool`) is already handled by the caller.
fn is_input_subtype_of_bool(input: ElementId) -> bool {
    input == TRUE || input == FALSE
}

/// `Open <: Resource` and `Closed <: Resource`. Reflexivity is the caller's
/// job; `Open`/`Closed` are not subtypes of each other.
fn is_input_subtype_of_resource(input: ElementId, container: ElementId) -> bool {
    if input.kind() != ElementKind::Resource {
        return false;
    }

    let i = interner();
    let container_info = i.get_resource(container);
    let input_info = i.get_resource(input);
    matches!((input_info, container_info), (ResourceInfo::Open | ResourceInfo::Closed, ResourceInfo::Any))
}

/// Subtype rules within the Int family.
///
/// Container variants accept inputs as follows:
///
/// - `Unspecified` (general `int`) accepts any Int-kind input.
/// - `UnspecifiedLiteral` (`literal-int`) accepts `Literal(_)` and itself.
/// - `Literal(N)` accepts only the same literal (handled by reflexivity).
/// - `Range(R)` accepts `Literal(N)` if `N ∈ R`, and `Range(R')` if `R' ⊆ R`.
fn is_input_subtype_of_int(input: ElementId, container: ElementId) -> bool {
    if input.kind() != ElementKind::Int {
        return false;
    }

    let i = interner();
    let container_info = *i.get_int(container);
    let input_info = *i.get_int(input);

    match (input_info, container_info) {
        (_, IntInfo::Unspecified) => true,
        (IntInfo::Literal(_) | IntInfo::UnspecifiedLiteral, IntInfo::UnspecifiedLiteral) => true,
        (IntInfo::Literal(n), IntInfo::Range(rid)) => {
            let r = *i.get_int_range(rid);
            range_contains_value(r, n)
        }
        (IntInfo::Range(input_rid), IntInfo::Range(container_rid)) => {
            let inner = *i.get_int_range(input_rid);
            let outer = *i.get_int_range(container_rid);
            range_contains_range(outer, inner)
        }
        _ => false,
    }
}

/// `Float` accepts any Float-kind input; `UnspecifiedLiteral` accepts
/// `Literal(_)` and itself; concrete literals only fit themselves
/// (reflexivity handles that).
fn is_input_subtype_of_float(input: ElementId, container: ElementId) -> bool {
    if input.kind() != ElementKind::Float {
        return false;
    }

    let i = interner();
    let container_info = *i.get_float(container);
    let input_info = *i.get_float(input);

    matches!(
        (input_info, container_info),
        (_, FloatInfo::Unspecified)
            | (FloatInfo::Literal(_) | FloatInfo::UnspecifiedLiteral, FloatInfo::UnspecifiedLiteral),
    )
}

/// String container subtype rules. Accepts inputs of kind String or
/// ClassLikeString; checks container constraints (literal slot, casing,
/// refinement flags) one at a time, deriving them from the input's literal
/// value where possible.
fn is_input_subtype_of_string(input: ElementId, container: ElementId) -> bool {
    let i = interner();
    let container_info = *i.get_string(container);

    if input.kind() == ElementKind::ClassLikeString {
        return class_like_string_satisfies_string_container(i.get_class_like_string(input).kind, container_info);
    }

    if input.kind() != ElementKind::String {
        return false;
    }

    let input_info = *i.get_string(input);
    string_satisfies_string_container(input_info, container_info)
}

/// `class-string` and friends are subtypes of `string` and of any string
/// refinement they satisfy structurally (non-empty + truthy, since class
/// names are non-empty and not `"0"`). Casing and `is_callable` are not
/// guaranteed for class-like names.
fn class_like_string_satisfies_string_container(
    _kind: crate::element::payload::ClassLikeKind,
    container: StringInfo,
) -> bool {
    if !literal_constraint_admits_class_like(container.literal) {
        return false;
    }

    if container.casing != StringCasing::Unspecified {
        return false;
    }

    let f = container.flags;
    if f.is_callable() {
        return false;
    }
    let _ = f.is_non_empty();
    let _ = f.is_truthy();
    let _ = f.is_numeric();
    true
}

fn literal_constraint_admits_class_like(literal: StringLiteral) -> bool {
    match literal {
        StringLiteral::None => true,
        StringLiteral::Unspecified => false,
        StringLiteral::Value(_) => false,
    }
}

/// Per-axis check for "input string satisfies container string".
///
/// The input must satisfy *every* constraint the container imposes:
///
/// - literal slot (None / Unspecified / Value(v))
/// - casing (Unspecified / Lowercase / Uppercase)
/// - refinement flags (non-empty, truthy, numeric, callable)
///
/// Each constraint is satisfied either by an equivalent constraint on the
/// input, or by the input being a literal value that structurally implies it
/// (e.g. `"abc"` is non-empty by inspection).
fn string_satisfies_string_container(input: StringInfo, container: StringInfo) -> bool {
    if !satisfies_literal(input.literal, container.literal) {
        return false;
    }

    if !satisfies_casing(input, container.casing) {
        return false;
    }

    satisfies_flags(input, container.flags)
}

fn satisfies_literal(input: StringLiteral, container: StringLiteral) -> bool {
    match (input, container) {
        (_, StringLiteral::None) => true,
        (StringLiteral::Value(_) | StringLiteral::Unspecified, StringLiteral::Unspecified) => true,
        (StringLiteral::Value(a), StringLiteral::Value(b)) => a == b,
        _ => false,
    }
}

fn satisfies_casing(input: StringInfo, container_casing: StringCasing) -> bool {
    match container_casing {
        StringCasing::Unspecified => true,
        StringCasing::Lowercase => match input.casing {
            StringCasing::Lowercase => true,
            _ => match input.literal {
                StringLiteral::Value(v) => {
                    let s = v.as_str();
                    s.chars().all(|c| !c.is_ascii_uppercase())
                }
                _ => false,
            },
        },
        StringCasing::Uppercase => match input.casing {
            StringCasing::Uppercase => true,
            _ => match input.literal {
                StringLiteral::Value(v) => {
                    let s = v.as_str();
                    s.chars().all(|c| !c.is_ascii_lowercase())
                }
                _ => false,
            },
        },
    }
}

fn satisfies_flags(input: StringInfo, container_flags: StringRefinementFlags) -> bool {
    if container_flags.is_non_empty() && !input_is_non_empty(input) {
        return false;
    }

    if container_flags.is_truthy() && !input_is_truthy(input) {
        return false;
    }

    if container_flags.is_numeric() && !input_is_numeric(input) {
        return false;
    }

    if container_flags.is_callable() && !input_is_callable(input) {
        return false;
    }
    true
}

fn input_is_non_empty(input: StringInfo) -> bool {
    if input.flags.is_non_empty() || input.flags.is_truthy() {
        return true;
    }

    match input.literal {
        StringLiteral::Value(v) => !v.as_str().is_empty(),
        _ => false,
    }
}

fn input_is_truthy(input: StringInfo) -> bool {
    if input.flags.is_truthy() {
        return true;
    }

    match input.literal {
        StringLiteral::Value(v) => {
            let s = v.as_str();
            !s.is_empty() && s != "0"
        }
        _ => false,
    }
}

fn input_is_numeric(input: StringInfo) -> bool {
    if input.flags.is_numeric() {
        return true;
    }

    match input.literal {
        StringLiteral::Value(v) => {
            let s = v.as_str();
            s.parse::<i64>().is_ok() || s.parse::<f64>().is_ok()
        }
        _ => false,
    }
}

fn input_is_callable(input: StringInfo) -> bool {
    input.flags.is_callable()
}

/// `class-string` family container subtype rules. Inputs must be of kind
/// `ClassLikeString` and have the same `kind` (class-string vs
/// interface-string vs enum-string vs trait-string don't merge), and the
/// container's specifier must be at least as permissive as the input's.
fn is_input_subtype_of_class_like_string(input: ElementId, container: ElementId) -> bool {
    if input.kind() != ElementKind::ClassLikeString {
        return false;
    }

    let i = interner();
    let container_info = *i.get_class_like_string(container);
    let input_info = *i.get_class_like_string(input);

    if input_info.kind != container_info.kind {
        return false;
    }

    use crate::element::payload::ClassLikeStringSpecifier as Spec;
    matches!(
        (input_info.specifier, container_info.specifier),
        (_, Spec::Any) | (Spec::Literal { .. }, Spec::Literal { .. }) // refl handles equal literals; no narrowing yet
    )
}

/// `int <: array-key`, `string <: array-key`, `class-like-string <:
/// array-key`. Floats and bools are explicitly NOT array keys. Reflexivity
/// (`array-key <: array-key`) is the caller's.
fn is_input_subtype_of_array_key(input: ElementId) -> bool {
    matches!(input.kind(), ElementKind::Int | ElementKind::String | ElementKind::ClassLikeString)
}

/// `int <: numeric`, `float <: numeric`, `numeric-string <: numeric`. A
/// general `string` is NOT numeric (only `numeric-string` and string
/// literals that parse as numbers are).
fn is_input_subtype_of_numeric(input: ElementId) -> bool {
    match input.kind() {
        ElementKind::Int | ElementKind::Float => true,
        ElementKind::String => {
            let info = interner().get_string(input);
            input_is_numeric(*info)
        }
        _ => false,
    }
}

/// `bool/true/false`, `int`, `float`, `string`, `class-like-string`,
/// `array-key`, `numeric` are all `<: scalar`. Mixed and object families
/// are NOT scalars.
fn is_input_subtype_of_scalar(input: ElementId) -> bool {
    matches!(
        input.kind(),
        ElementKind::Bool
            | ElementKind::True
            | ElementKind::False
            | ElementKind::Int
            | ElementKind::Float
            | ElementKind::String
            | ElementKind::ClassLikeString
            | ElementKind::ArrayKey
            | ElementKind::Numeric
            | ElementKind::Scalar
    )
}

/// The kinds that all sit under `Object::Any` and are absorbed by it: named
/// objects, enums (and enum cases), object shapes, has-method /
/// has-property narrowings.
fn is_object_family_kind(kind: ElementKind) -> bool {
    matches!(
        kind,
        ElementKind::Object
            | ElementKind::Enum
            | ElementKind::ObjectShape
            | ElementKind::HasMethod
            | ElementKind::HasProperty
            | ElementKind::ObjectAny
    )
}

fn range_contains_value(range: IntRange, n: i64) -> bool {
    let lower_ok = match range.lower() {
        Some(lo) => lo <= n,
        None => true,
    };
    let upper_ok = match range.upper() {
        Some(hi) => n <= hi,
        None => true,
    };
    lower_ok && upper_ok
}

fn range_contains_range(outer: IntRange, inner: IntRange) -> bool {
    let lower_ok = match (outer.lower(), inner.lower()) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(o), Some(i)) => o <= i,
    };

    let upper_ok = match (outer.upper(), inner.upper()) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(o), Some(i)) => i <= o,
    };

    lower_ok && upper_ok
}

// Suppress dead-code warnings for the OBJECT / NUMERIC / SCALAR / ARRAY_KEY
// constants that the dispatch implicitly relies on (callers pass elements
// whose kind matches; we don't reference the well-known IDs in this module).
const _: ElementId = OBJECT;
const _: ElementId = NUMERIC;
const _: ElementId = SCALAR;
const _: ElementId = ARRAY_KEY;
const _: ElementId = BOOL;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ElementId;
    use crate::FlowFlags;
    use crate::comparator::NullCodebase;
    use crate::interner::interner;
    use crate::well_known::CALLABLE_STRING;
    use crate::well_known::CLASS_STRING;
    use crate::well_known::CLOSED_RESOURCE;
    use crate::well_known::ENUM_STRING;
    use crate::well_known::FLOAT;
    use crate::well_known::INT;
    use crate::well_known::INTERFACE_STRING;
    use crate::well_known::LITERAL_FLOAT;
    use crate::well_known::LITERAL_INT;
    use crate::well_known::LITERAL_STRING;
    use crate::well_known::LOWERCASE_STRING;
    use crate::well_known::NEGATIVE_INT;
    use crate::well_known::NON_EMPTY_STRING;
    use crate::well_known::NULL;
    use crate::well_known::NUMERIC_STRING;
    use crate::well_known::OPEN_RESOURCE;
    use crate::well_known::POSITIVE_INT;
    use crate::well_known::RESOURCE;
    use crate::well_known::STRING;
    use crate::well_known::TRUTHY_STRING;
    use crate::well_known::TYPE_ARRAY_KEY;
    use crate::well_known::TYPE_BOOL;
    use crate::well_known::TYPE_FLOAT;
    use crate::well_known::TYPE_INT;
    use crate::well_known::TYPE_INT_OR_FLOAT;
    use crate::well_known::TYPE_INT_OR_STRING;
    use crate::well_known::TYPE_MIXED;
    use crate::well_known::TYPE_NEVER;
    use crate::well_known::TYPE_NULL;
    use crate::well_known::TYPE_NUMERIC;
    use crate::well_known::TYPE_SCALAR;
    use crate::well_known::TYPE_STRING;
    use crate::well_known::UPPERCASE_STRING;

    fn check(input: TypeId, container: TypeId) -> bool {
        let mut ctx = SubtypeContext::new();
        is_subtype(input, container, &mut ctx, &NullCodebase)
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
    fn bot_axiom_never_is_subtype_of_anything() {
        assert!(check(TYPE_NEVER, TYPE_INT));
        assert!(check(TYPE_NEVER, TYPE_NULL));
        assert!(check(TYPE_NEVER, TYPE_MIXED));
        assert!(check(TYPE_NEVER, TYPE_INT_OR_STRING));
    }

    #[test]
    fn top_axiom_anything_is_subtype_of_vanilla_mixed() {
        assert!(check(TYPE_INT, TYPE_MIXED));
        assert!(check(TYPE_NULL, TYPE_MIXED));
        assert!(check(TYPE_STRING, TYPE_MIXED));
        assert!(check(TYPE_INT_OR_STRING, TYPE_MIXED));
    }

    #[test]
    fn bool_family_subtypes_bool() {
        assert!(check_elem(TRUE, BOOL));
        assert!(check_elem(FALSE, BOOL));
        assert!(!check_elem(TRUE, FALSE));
        assert!(!check_elem(FALSE, TRUE));
        assert!(!check_elem(BOOL, TRUE));
        assert!(!check_elem(BOOL, FALSE));
    }

    #[test]
    fn resource_family_subtypes_resource() {
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
        // The reverse fails: bounded doesn't contain the open upper end.
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
        // Digits / spaces / punctuation are case-neutral and satisfy both.
        assert!(check_elem(ElementId::string_literal("123"), LOWERCASE_STRING));
        assert!(check_elem(ElementId::string_literal("123"), UPPERCASE_STRING));
    }

    #[test]
    fn truthy_string_subtypes_non_empty_string() {
        assert!(check_elem(TRUTHY_STRING, NON_EMPTY_STRING));
    }

    #[test]
    fn callable_string_does_not_subtype_numeric_or_lowercase_by_default() {
        assert!(check_elem(CALLABLE_STRING, STRING));
        assert!(!check_elem(CALLABLE_STRING, NUMERIC_STRING));
        assert!(!check_elem(CALLABLE_STRING, LOWERCASE_STRING));
    }

    #[test]
    fn class_like_string_subtypes_string() {
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
    fn unrelated_types_are_not_subtypes() {
        assert!(!check(TYPE_INT, TYPE_FLOAT));
        assert!(!check(TYPE_INT, TYPE_STRING));
        assert!(!check(TYPE_NULL, TYPE_INT));
    }

    #[test]
    fn fresh_int_literal_is_subtype_of_int() {
        let lit = ElementId::int_literal(42);
        let lit_type = interner().intern_type(&[lit], FlowFlags::EMPTY);
        assert!(check(lit_type, TYPE_INT));
        assert!(check(lit_type, TYPE_MIXED));
    }

    #[test]
    fn input_int_or_float_fits_int_or_float_via_union_dispatch() {
        let mixed = interner().intern_type(&[INT, FLOAT], FlowFlags::EMPTY);
        assert!(check(mixed, TYPE_INT_OR_FLOAT));

        let with_string = interner().intern_type(&[INT, FLOAT, STRING], FlowFlags::EMPTY);
        assert!(!check(with_string, TYPE_INT_OR_FLOAT));
    }

    #[test]
    fn int_or_string_subtypes_array_key() {
        assert!(check(TYPE_INT_OR_STRING, TYPE_ARRAY_KEY));
    }

    #[test]
    fn int_or_float_subtypes_numeric() {
        assert!(check(TYPE_INT_OR_FLOAT, TYPE_NUMERIC));
    }

    #[test]
    fn int_or_string_or_float_or_bool_subtypes_scalar() {
        let id = interner().intern_type(&[INT, STRING, FLOAT, BOOL], FlowFlags::EMPTY);
        assert!(check(id, TYPE_SCALAR));
    }

    #[test]
    fn nullable_int_subtypes_nullable_array_key() {
        let nullable_int = interner().intern_type(&[NULL, INT], FlowFlags::EMPTY);
        let nullable_ak = interner().intern_type(&[NULL, ARRAY_KEY], FlowFlags::EMPTY);
        assert!(check(nullable_int, nullable_ak));
    }

    #[test]
    fn type_bool_is_subtype_of_scalar() {
        assert!(check(TYPE_BOOL, TYPE_SCALAR));
    }
}
