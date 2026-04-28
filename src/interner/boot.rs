use crate::FlowFlags;
use crate::element::payload::CallableInfo;
use crate::element::payload::ClassLikeKind;
use crate::element::payload::ClassLikeStringInfo;
use crate::element::payload::ClassLikeStringSpecifier;
use crate::element::payload::IterableInfo;
use crate::element::payload::KeyedArrayFlags;
use crate::element::payload::KeyedArrayInfo;
use crate::element::payload::MixedInfo;
use crate::element::payload::ResourceInfo;
use crate::element::payload::StringCasing;
use crate::element::payload::StringInfo;
use crate::element::payload::StringLiteral;
use crate::element::payload::StringRefinementFlags;
use crate::element::payload::Truthiness;
use crate::element::payload::scalar::FloatInfo;
use crate::element::payload::scalar::IntInfo;
use crate::element::payload::scalar::IntRange;
use crate::interner::Interner;
use crate::prelude::*;

impl Interner {
    /// Construct a fully-booted interner with every well-known
    /// [`ElementId`](crate::ElementId) and [`TypeId`](crate::TypeId) constant
    /// pre-populated.
    ///
    /// Boot is deterministic: each insert lands at the slot the matching
    /// `pub const` constant claims. `debug_assert_eq!` calls verify the
    /// alignment in development builds, so reordering constants without
    /// reordering boot triggers an immediate panic.
    pub fn boot() -> Self {
        let i = Self::new();
        i.boot_atomic_elements();
        i.boot_singleton_types();
        i.boot_typed_elements();
        i.boot_pre_canonicalized_unions();
        i
    }

    fn boot_atomic_elements(&self) {
        self.boot_mixed_family();
        self.boot_int_family();
        self.boot_float_family();
        self.boot_string_family();
        self.boot_class_like_string_family();
        self.boot_resource_family();
        self.boot_empty_array();
        self.boot_callable();
    }

    fn boot_mixed_family(&self) {
        debug_assert_eq!(self.intern_mixed(MixedInfo::EMPTY), MIXED);
        debug_assert_eq!(self.intern_mixed(MixedInfo::EMPTY.with_is_non_null(true)), NON_NULL_MIXED);
        debug_assert_eq!(self.intern_mixed(MixedInfo::EMPTY.with_truthiness(Truthiness::Truthy)), TRUTHY_MIXED,);
        debug_assert_eq!(self.intern_mixed(MixedInfo::EMPTY.with_truthiness(Truthiness::Falsy)), FALSY_MIXED,);
        debug_assert_eq!(self.intern_mixed(MixedInfo::EMPTY.with_is_isset_from_loop(true)), ISSET_FROM_LOOP,);
    }

    fn boot_int_family(&self) {
        debug_assert_eq!(self.intern_int(IntInfo::Unspecified), INT);
        debug_assert_eq!(self.intern_int(self.range_int(Some(1), None)), POSITIVE_INT);
        debug_assert_eq!(self.intern_int(self.range_int(None, Some(-1))), NEGATIVE_INT);
        debug_assert_eq!(self.intern_int(self.range_int(None, Some(0))), NON_POSITIVE_INT);
        debug_assert_eq!(self.intern_int(self.range_int(Some(0), None)), NON_NEGATIVE_INT);
        debug_assert_eq!(self.intern_int(IntInfo::UnspecifiedLiteral), LITERAL_INT);
        debug_assert_eq!(self.intern_int(IntInfo::Literal(0)), INT_ZERO);
        debug_assert_eq!(self.intern_int(IntInfo::Literal(1)), INT_ONE);
        debug_assert_eq!(self.intern_int(IntInfo::Literal(-1)), INT_MINUS_ONE);
    }

    fn range_int(&self, lower: Option<i64>, upper: Option<i64>) -> IntInfo {
        IntInfo::Range(self.intern_int_range(IntRange::new(lower, upper)))
    }

    fn boot_float_family(&self) {
        debug_assert_eq!(self.intern_float(FloatInfo::Unspecified), FLOAT);
        debug_assert_eq!(self.intern_float(FloatInfo::UnspecifiedLiteral), LITERAL_FLOAT);
    }

    fn boot_string_family(&self) {
        let plain = StringInfo {
            literal: StringLiteral::None,
            casing: StringCasing::Unspecified,
            flags: StringRefinementFlags::EMPTY,
        };
        let with_casing = |c: StringCasing| StringInfo { casing: c, ..plain };
        let with_flags = |f: StringRefinementFlags| StringInfo { flags: f, ..plain };
        let with_both = |c: StringCasing, f: StringRefinementFlags| StringInfo {
            casing: c,
            flags: f,
            literal: StringLiteral::None,
        };

        let non_empty = StringRefinementFlags::EMPTY.with_is_non_empty(true);
        let truthy = StringRefinementFlags::EMPTY.with_is_truthy(true).with_is_non_empty(true);
        let numeric = StringRefinementFlags::EMPTY.with_is_numeric(true);
        let truthy_numeric = numeric.with_is_truthy(true).with_is_non_empty(true);
        let callable = StringRefinementFlags::EMPTY.with_is_callable(true).with_is_truthy(true).with_is_non_empty(true);

        debug_assert_eq!(self.intern_string(plain), STRING);
        debug_assert_eq!(self.intern_string(with_flags(non_empty)), NON_EMPTY_STRING);
        debug_assert_eq!(self.intern_string(with_flags(truthy)), TRUTHY_STRING);
        debug_assert_eq!(self.intern_string(with_casing(StringCasing::Lowercase)), LOWERCASE_STRING);
        debug_assert_eq!(self.intern_string(with_casing(StringCasing::Uppercase)), UPPERCASE_STRING);
        debug_assert_eq!(self.intern_string(with_both(StringCasing::Lowercase, non_empty)), NON_EMPTY_LOWERCASE_STRING,);
        debug_assert_eq!(self.intern_string(with_both(StringCasing::Uppercase, non_empty)), NON_EMPTY_UPPERCASE_STRING,);
        debug_assert_eq!(self.intern_string(with_both(StringCasing::Lowercase, truthy)), TRUTHY_LOWERCASE_STRING,);
        debug_assert_eq!(self.intern_string(with_both(StringCasing::Uppercase, truthy)), TRUTHY_UPPERCASE_STRING,);
        debug_assert_eq!(self.intern_string(with_flags(numeric)), NUMERIC_STRING);
        debug_assert_eq!(self.intern_string(with_flags(truthy_numeric)), TRUTHY_NUMERIC_STRING);
        debug_assert_eq!(self.intern_string(with_flags(callable)), CALLABLE_STRING);
        debug_assert_eq!(self.intern_string(with_both(StringCasing::Lowercase, callable)), LOWERCASE_CALLABLE_STRING,);
        debug_assert_eq!(self.intern_string(with_both(StringCasing::Uppercase, callable)), UPPERCASE_CALLABLE_STRING,);

        let unspecified_literal = StringInfo { literal: StringLiteral::Unspecified, ..plain };
        let non_empty_unspecified_literal =
            StringInfo { literal: StringLiteral::Unspecified, flags: non_empty, ..plain };
        let empty_literal = StringInfo { literal: StringLiteral::Value(mago_atom::atom("")), ..plain };

        debug_assert_eq!(self.intern_string(unspecified_literal), LITERAL_STRING);
        debug_assert_eq!(self.intern_string(non_empty_unspecified_literal), NON_EMPTY_LITERAL_STRING);
        debug_assert_eq!(self.intern_string(empty_literal), EMPTY_STRING);
    }

    fn boot_class_like_string_family(&self) {
        let make = |kind: ClassLikeKind| ClassLikeStringInfo { kind, specifier: ClassLikeStringSpecifier::Any };
        debug_assert_eq!(self.intern_class_like_string(make(ClassLikeKind::Class)), CLASS_STRING);
        debug_assert_eq!(self.intern_class_like_string(make(ClassLikeKind::Interface)), INTERFACE_STRING);
        debug_assert_eq!(self.intern_class_like_string(make(ClassLikeKind::Enum)), ENUM_STRING);
        debug_assert_eq!(self.intern_class_like_string(make(ClassLikeKind::Trait)), TRAIT_STRING);
    }

    fn boot_resource_family(&self) {
        debug_assert_eq!(self.intern_resource(ResourceInfo::Any), RESOURCE);
        debug_assert_eq!(self.intern_resource(ResourceInfo::Open), OPEN_RESOURCE);
        debug_assert_eq!(self.intern_resource(ResourceInfo::Closed), CLOSED_RESOURCE);
    }

    fn boot_empty_array(&self) {
        let empty =
            KeyedArrayInfo { key_param: None, value_param: None, known_items: None, flags: KeyedArrayFlags::default() };
        debug_assert_eq!(self.intern_array(empty), EMPTY_ARRAY);
    }

    fn boot_callable(&self) {
        debug_assert_eq!(self.intern_callable(CallableInfo::Any), CALLABLE);
    }

    fn boot_singleton_types(&self) {
        debug_assert_eq!(self.intern_type(&[NULL], FlowFlags::EMPTY), TYPE_NULL);
        debug_assert_eq!(self.intern_type(&[NEVER], FlowFlags::EMPTY), TYPE_NEVER);
        debug_assert_eq!(self.intern_type(&[VOID], FlowFlags::EMPTY), TYPE_VOID);
        debug_assert_eq!(self.intern_type(&[MIXED], FlowFlags::EMPTY), TYPE_MIXED);
        debug_assert_eq!(self.intern_type(&[BOOL], FlowFlags::EMPTY), TYPE_BOOL);
        debug_assert_eq!(self.intern_type(&[TRUE], FlowFlags::EMPTY), TYPE_TRUE);
        debug_assert_eq!(self.intern_type(&[FALSE], FlowFlags::EMPTY), TYPE_FALSE);
        debug_assert_eq!(self.intern_type(&[INT], FlowFlags::EMPTY), TYPE_INT);
        debug_assert_eq!(self.intern_type(&[FLOAT], FlowFlags::EMPTY), TYPE_FLOAT);
        debug_assert_eq!(self.intern_type(&[STRING], FlowFlags::EMPTY), TYPE_STRING);
        debug_assert_eq!(self.intern_type(&[OBJECT], FlowFlags::EMPTY), TYPE_OBJECT);
        debug_assert_eq!(self.intern_type(&[SCALAR], FlowFlags::EMPTY), TYPE_SCALAR);
        debug_assert_eq!(self.intern_type(&[NUMERIC], FlowFlags::EMPTY), TYPE_NUMERIC);
        debug_assert_eq!(self.intern_type(&[ARRAY_KEY], FlowFlags::EMPTY), TYPE_ARRAY_KEY);
        debug_assert_eq!(self.intern_type(&[CALLABLE], FlowFlags::EMPTY), TYPE_CALLABLE);
    }

    fn boot_typed_elements(&self) {
        let iterable_mixed_mixed = IterableInfo { key_type: TYPE_MIXED, value_type: TYPE_MIXED, intersections: None };
        debug_assert_eq!(self.intern_iterable(iterable_mixed_mixed), ITERABLE_MIXED_MIXED);

        let array_key_mixed = KeyedArrayInfo {
            key_param: Some(TYPE_ARRAY_KEY),
            value_param: Some(TYPE_MIXED),
            known_items: None,
            flags: KeyedArrayFlags::default(),
        };
        debug_assert_eq!(self.intern_array(array_key_mixed), ARRAY_KEY_MIXED);
    }

    fn boot_pre_canonicalized_unions(&self) {
        debug_assert_eq!(self.intern_type(&[INT, FLOAT], FlowFlags::EMPTY), TYPE_INT_OR_FLOAT);
        debug_assert_eq!(self.intern_type(&[INT, STRING], FlowFlags::EMPTY), TYPE_INT_OR_STRING);
        debug_assert_eq!(self.intern_type(&[NULL, SCALAR], FlowFlags::EMPTY), TYPE_NULL_OR_SCALAR);
        debug_assert_eq!(self.intern_type(&[NULL, STRING], FlowFlags::EMPTY), TYPE_NULL_OR_STRING);
        debug_assert_eq!(self.intern_type(&[NULL, INT], FlowFlags::EMPTY), TYPE_NULL_OR_INT);
        debug_assert_eq!(self.intern_type(&[NULL, FLOAT], FlowFlags::EMPTY), TYPE_NULL_OR_FLOAT);
        debug_assert_eq!(self.intern_type(&[NULL, OBJECT], FlowFlags::EMPTY), TYPE_NULL_OR_OBJECT);
        debug_assert_eq!(
            self.intern_type(&[INT_MINUS_ONE, INT_ZERO, INT_ONE], FlowFlags::EMPTY),
            TYPE_MINUS_ONE_ZERO_ONE,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interner::interner;

    #[test]
    fn type_int_resolves_to_singleton_int_union() {
        let t = TYPE_INT.as_ref();
        assert_eq!(t.elements, &[INT]);
        assert_eq!(TYPE_INT.flags(), FlowFlags::EMPTY);
    }

    #[test]
    fn type_null_or_string_elements_are_in_canonical_order() {
        let t = TYPE_NULL_OR_STRING.as_ref();
        assert_eq!(t.elements.len(), 2);
        assert_eq!(t.elements[0], NULL);
        assert_eq!(t.elements[1], STRING);
    }

    #[test]
    fn type_minus_one_zero_one_elements_are_sorted_by_int_slot() {
        let t = TYPE_MINUS_ONE_ZERO_ONE.as_ref();
        assert_eq!(t.elements, &[INT_ZERO, INT_ONE, INT_MINUS_ONE]);
    }

    #[test]
    fn well_known_int_payloads_resolve_correctly() {
        let i = interner();
        assert_eq!(i.get_int(INT), &IntInfo::Unspecified);
        assert_eq!(i.get_int(LITERAL_INT), &IntInfo::UnspecifiedLiteral);
        assert_eq!(i.get_int(INT_ZERO), &IntInfo::Literal(0));
        assert_eq!(i.get_int(INT_ONE), &IntInfo::Literal(1));
        assert_eq!(i.get_int(INT_MINUS_ONE), &IntInfo::Literal(-1));
    }

    #[test]
    fn well_known_resource_payloads_resolve_correctly() {
        let i = interner();
        assert_eq!(i.get_resource(RESOURCE), &ResourceInfo::Any);
        assert_eq!(i.get_resource(OPEN_RESOURCE), &ResourceInfo::Open);
        assert_eq!(i.get_resource(CLOSED_RESOURCE), &ResourceInfo::Closed);
    }

    #[test]
    fn well_known_class_like_string_payloads_resolve_correctly() {
        let i = interner();
        assert_eq!(i.get_class_like_string(CLASS_STRING).kind, ClassLikeKind::Class);
        assert_eq!(i.get_class_like_string(INTERFACE_STRING).kind, ClassLikeKind::Interface);
        assert_eq!(i.get_class_like_string(ENUM_STRING).kind, ClassLikeKind::Enum);
        assert_eq!(i.get_class_like_string(TRAIT_STRING).kind, ClassLikeKind::Trait);
    }

    #[test]
    fn empty_array_resolves_to_sealed_no_known_items() {
        let info = interner().get_array(EMPTY_ARRAY);
        assert!(info.is_sealed());
        assert!(info.known_items.is_none());
    }

    #[test]
    fn array_key_mixed_uses_well_known_type_ids() {
        let info = interner().get_array(ARRAY_KEY_MIXED);
        assert_eq!(info.key_param, Some(TYPE_ARRAY_KEY));
        assert_eq!(info.value_param, Some(TYPE_MIXED));
    }

    #[test]
    fn iterable_mixed_mixed_uses_well_known_type_ids() {
        let info = interner().get_iterable(ITERABLE_MIXED_MIXED);
        assert_eq!(info.key_type, TYPE_MIXED);
        assert_eq!(info.value_type, TYPE_MIXED);
    }
}
