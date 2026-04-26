use suffete::Element;
use suffete::ElementId;
use suffete::TypeId;
use suffete::element::payload::scalar::IntInfo;
use suffete::well_known;

#[test]
fn int_literal_zero_dedupes_against_well_known() {
    let zero = ElementId::int_literal(0);
    assert_eq!(zero, well_known::INT_ZERO, "int_literal(0) must dedupe to INT_ZERO");

    let one = ElementId::int_literal(1);
    assert_eq!(one, well_known::INT_ONE);

    let minus_one = ElementId::int_literal(-1);
    assert_eq!(minus_one, well_known::INT_MINUS_ONE);
}

#[test]
fn int_literal_arbitrary_values_round_trip() {
    let id = ElementId::int_literal(1729);
    assert!(matches!(id.view(), Element::Int(IntInfo::Literal(1729))));
}

#[test]
fn int_range_dedupes_against_well_known_positive_int() {
    let positive = ElementId::int_range(Some(1), None);
    assert_eq!(positive, well_known::POSITIVE_INT);

    let negative = ElementId::int_range(None, Some(-1));
    assert_eq!(negative, well_known::NEGATIVE_INT);
}

#[test]
fn float_literal_round_trips() {
    let id = ElementId::float_literal(2.5);
    let Element::Float(info) = id.view() else {
        panic!("expected Float variant");
    };
    use suffete::element::payload::scalar::FloatInfo;
    let FloatInfo::Literal(lit) = info else {
        panic!("expected Literal variant");
    };
    assert!((lit.value() - 2.5).abs() < 1e-9);
}

#[test]
fn string_literal_round_trips_with_correct_atom() {
    let foo = ElementId::string_literal("foo");
    let Element::String(info) = foo.view() else {
        panic!("expected String variant");
    };
    use suffete::element::payload::scalar::StringLiteral;
    let StringLiteral::Value(atom) = info.literal else {
        panic!("expected Value variant");
    };
    assert_eq!(atom.as_str(), "foo");
}

#[test]
fn type_singleton_dedupes_against_well_known_type_int() {
    let t = TypeId::singleton(well_known::INT);
    assert_eq!(t, well_known::TYPE_INT);
}

#[test]
fn type_union_of_int_and_float_dedupes_against_well_known() {
    let t = TypeId::union(&[well_known::INT, well_known::FLOAT]);
    assert_eq!(t, well_known::TYPE_INT_OR_FLOAT);
}

#[test]
fn type_int_literal_builds_singleton_with_correct_atom() {
    let t = TypeId::int_literal(42);
    let view = t.as_ref();
    assert_eq!(view.elements.len(), 1);
    assert!(matches!(view.elements[0].view(), Element::Int(IntInfo::Literal(42))));
}

#[test]
fn type_string_literal_builds_singleton_with_correct_atom() {
    let t = TypeId::string_literal("hello");
    let view = t.as_ref();
    assert_eq!(view.elements.len(), 1);
    let Element::String(info) = view.elements[0].view() else {
        panic!("expected String variant");
    };
    use suffete::element::payload::scalar::StringLiteral;
    let StringLiteral::Value(atom) = info.literal else {
        panic!("expected Value variant");
    };
    assert_eq!(atom.as_str(), "hello");
}
