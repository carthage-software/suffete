#![allow(clippy::shadow_unrelated)]

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

// The `debug_assert_eq!` calls below pin every well-known constant to the slot
// it must land in ; the assert message would just restate the comparison.
#[allow(clippy::missing_assert_message)]
impl Interner {
    /// Construct a fully-booted interner with every well-known
    /// [`ElementId`](crate::ElementId) and [`TypeId`](crate::TypeId) constant
    /// pre-populated.
    ///
    /// Boot is deterministic: each insert lands at the slot the matching
    /// `pub const` constant claims. `debug_assert_eq!` calls verify the
    /// alignment in development builds, so reordering constants without
    /// reordering boot triggers an immediate panic.
    #[inline]
    #[must_use]
    pub fn boot() -> Self {
        let i = Self::new();
        i.boot_atomic_elements();
        i.boot_singleton_types();
        i.boot_typed_elements();
        i.boot_pre_canonicalized_unions();
        i
    }

    #[inline]
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

    #[inline]
    fn boot_mixed_family(&self) {
        {
            let booted = self.intern_mixed(MixedInfo::EMPTY);
            debug_assert_eq!(booted, MIXED);

            let booted = self.intern_mixed(MixedInfo::EMPTY.with_is_non_null(true));
            debug_assert_eq!(booted, NON_NULL_MIXED);

            let booted = self.intern_mixed(MixedInfo::EMPTY.with_truthiness(Truthiness::Truthy));
            debug_assert_eq!(booted, TRUTHY_MIXED);

            let booted = self.intern_mixed(MixedInfo::EMPTY.with_truthiness(Truthiness::Falsy));
            debug_assert_eq!(booted, FALSY_MIXED);

            let booted = self.intern_mixed(MixedInfo::EMPTY.with_is_isset_from_loop(true));
            debug_assert_eq!(booted, ISSET_FROM_LOOP);
        }
    }

    #[inline]
    fn boot_int_family(&self) {
        let booted = self.intern_int(IntInfo::Unspecified);
        debug_assert_eq!(booted, INT);

        let booted = self.intern_int(self.range_int(Some(1), None));
        debug_assert_eq!(booted, POSITIVE_INT);

        let booted = self.intern_int(self.range_int(None, Some(-1)));
        debug_assert_eq!(booted, NEGATIVE_INT);

        let booted = self.intern_int(self.range_int(None, Some(0)));
        debug_assert_eq!(booted, NON_POSITIVE_INT);

        let booted = self.intern_int(self.range_int(Some(0), None));
        debug_assert_eq!(booted, NON_NEGATIVE_INT);

        let booted = self.intern_int(IntInfo::UnspecifiedLiteral);
        debug_assert_eq!(booted, LITERAL_INT);

        let booted = self.intern_int(IntInfo::Literal(0));
        debug_assert_eq!(booted, INT_ZERO);

        let booted = self.intern_int(IntInfo::Literal(1));
        debug_assert_eq!(booted, INT_ONE);

        let booted = self.intern_int(IntInfo::Literal(-1));
        debug_assert_eq!(booted, INT_MINUS_ONE);
    }

    #[inline]
    fn range_int(&self, lower: Option<i64>, upper: Option<i64>) -> IntInfo {
        IntInfo::Range(self.intern_int_range(IntRange::new(lower, upper)))
    }

    #[inline]
    fn boot_float_family(&self) {
        let booted = self.intern_float(FloatInfo::Unspecified);
        debug_assert_eq!(booted, FLOAT);

        let booted = self.intern_float(FloatInfo::UnspecifiedLiteral);
        debug_assert_eq!(booted, LITERAL_FLOAT);
    }

    #[inline]
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

        let booted = self.intern_string(plain);
        debug_assert_eq!(booted, STRING);

        let booted = self.intern_string(with_flags(non_empty));
        debug_assert_eq!(booted, NON_EMPTY_STRING);

        let booted = self.intern_string(with_flags(truthy));
        debug_assert_eq!(booted, TRUTHY_STRING);

        let booted = self.intern_string(with_casing(StringCasing::Lowercase));
        debug_assert_eq!(booted, LOWERCASE_STRING);

        let booted = self.intern_string(with_casing(StringCasing::Uppercase));
        debug_assert_eq!(booted, UPPERCASE_STRING);

        let booted = self.intern_string(with_both(StringCasing::Lowercase, non_empty));
        debug_assert_eq!(booted, NON_EMPTY_LOWERCASE_STRING);

        let booted = self.intern_string(with_both(StringCasing::Uppercase, non_empty));
        debug_assert_eq!(booted, NON_EMPTY_UPPERCASE_STRING);

        let booted = self.intern_string(with_both(StringCasing::Lowercase, truthy));
        debug_assert_eq!(booted, TRUTHY_LOWERCASE_STRING);

        let booted = self.intern_string(with_both(StringCasing::Uppercase, truthy));
        debug_assert_eq!(booted, TRUTHY_UPPERCASE_STRING);

        let booted = self.intern_string(with_flags(numeric));
        debug_assert_eq!(booted, NUMERIC_STRING);

        let booted = self.intern_string(with_flags(truthy_numeric));
        debug_assert_eq!(booted, TRUTHY_NUMERIC_STRING);

        let booted = self.intern_string(with_flags(callable));
        debug_assert_eq!(booted, CALLABLE_STRING);

        let booted = self.intern_string(with_both(StringCasing::Lowercase, callable));
        debug_assert_eq!(booted, LOWERCASE_CALLABLE_STRING);

        let booted = self.intern_string(with_both(StringCasing::Uppercase, callable));
        debug_assert_eq!(booted, UPPERCASE_CALLABLE_STRING);

        let unspecified_literal = StringInfo { literal: StringLiteral::Unspecified, ..plain };
        let non_empty_unspecified_literal =
            StringInfo { literal: StringLiteral::Unspecified, flags: non_empty, ..plain };
        let empty_literal = StringInfo { literal: StringLiteral::Value(mago_atom::atom("")), ..plain };

        let booted = self.intern_string(unspecified_literal);
        debug_assert_eq!(booted, LITERAL_STRING);

        let booted = self.intern_string(non_empty_unspecified_literal);
        debug_assert_eq!(booted, NON_EMPTY_LITERAL_STRING);

        let booted = self.intern_string(empty_literal);
        debug_assert_eq!(booted, EMPTY_STRING);
    }

    #[inline]
    fn boot_class_like_string_family(&self) {
        let make = |kind: ClassLikeKind| ClassLikeStringInfo { kind, specifier: ClassLikeStringSpecifier::Any };

        let booted = self.intern_class_like_string(make(ClassLikeKind::Class));
        debug_assert_eq!(booted, CLASS_STRING);

        let booted = self.intern_class_like_string(make(ClassLikeKind::Interface));
        debug_assert_eq!(booted, INTERFACE_STRING);

        let booted = self.intern_class_like_string(make(ClassLikeKind::Enum));
        debug_assert_eq!(booted, ENUM_STRING);

        let booted = self.intern_class_like_string(make(ClassLikeKind::Trait));
        debug_assert_eq!(booted, TRAIT_STRING);
    }

    #[inline]
    fn boot_resource_family(&self) {
        let booted = self.intern_resource(ResourceInfo::Any);
        debug_assert_eq!(booted, RESOURCE);

        let booted = self.intern_resource(ResourceInfo::Open);
        debug_assert_eq!(booted, OPEN_RESOURCE);

        let booted = self.intern_resource(ResourceInfo::Closed);
        debug_assert_eq!(booted, CLOSED_RESOURCE);
    }

    #[inline]
    fn boot_empty_array(&self) {
        let empty =
            KeyedArrayInfo { key_param: None, value_param: None, known_items: None, flags: KeyedArrayFlags::default() };

        let booted = self.intern_array(empty);
        debug_assert_eq!(booted, EMPTY_ARRAY);
    }

    #[inline]
    fn boot_callable(&self) {
        let booted = self.intern_callable(CallableInfo::Any);
        debug_assert_eq!(booted, CALLABLE);
    }

    #[inline]
    fn boot_singleton_types(&self) {
        let booted = self.intern_type(&[NULL], FlowFlags::EMPTY);
        debug_assert_eq!(booted, TYPE_NULL);

        let booted = self.intern_type(&[NEVER], FlowFlags::EMPTY);
        debug_assert_eq!(booted, TYPE_NEVER);

        let booted = self.intern_type(&[VOID], FlowFlags::EMPTY);
        debug_assert_eq!(booted, TYPE_VOID);

        let booted = self.intern_type(&[MIXED], FlowFlags::EMPTY);
        debug_assert_eq!(booted, TYPE_MIXED);

        let booted = self.intern_type(&[BOOL], FlowFlags::EMPTY);
        debug_assert_eq!(booted, TYPE_BOOL);

        let booted = self.intern_type(&[TRUE], FlowFlags::EMPTY);
        debug_assert_eq!(booted, TYPE_TRUE);

        let booted = self.intern_type(&[FALSE], FlowFlags::EMPTY);
        debug_assert_eq!(booted, TYPE_FALSE);

        let booted = self.intern_type(&[INT], FlowFlags::EMPTY);
        debug_assert_eq!(booted, TYPE_INT);

        let booted = self.intern_type(&[FLOAT], FlowFlags::EMPTY);
        debug_assert_eq!(booted, TYPE_FLOAT);

        let booted = self.intern_type(&[STRING], FlowFlags::EMPTY);
        debug_assert_eq!(booted, TYPE_STRING);

        let booted = self.intern_type(&[OBJECT], FlowFlags::EMPTY);
        debug_assert_eq!(booted, TYPE_OBJECT);

        let booted = self.intern_type(&[SCALAR], FlowFlags::EMPTY);
        debug_assert_eq!(booted, TYPE_SCALAR);

        let booted = self.intern_type(&[NUMERIC], FlowFlags::EMPTY);
        debug_assert_eq!(booted, TYPE_NUMERIC);

        let booted = self.intern_type(&[ARRAY_KEY], FlowFlags::EMPTY);
        debug_assert_eq!(booted, TYPE_ARRAY_KEY);

        let booted = self.intern_type(&[CALLABLE], FlowFlags::EMPTY);
        debug_assert_eq!(booted, TYPE_CALLABLE);
    }

    #[inline]
    fn boot_typed_elements(&self) {
        let iterable_mixed_mixed = IterableInfo { key_type: TYPE_MIXED, value_type: TYPE_MIXED };
        let booted = self.intern_iterable(iterable_mixed_mixed);
        debug_assert_eq!(booted, ITERABLE_MIXED_MIXED);

        let array_key_mixed = KeyedArrayInfo {
            key_param: Some(TYPE_ARRAY_KEY),
            value_param: Some(TYPE_MIXED),
            known_items: None,
            flags: KeyedArrayFlags::default(),
        };

        let booted = self.intern_array(array_key_mixed);
        debug_assert_eq!(booted, ARRAY_KEY_MIXED);
    }

    #[inline]
    fn boot_pre_canonicalized_unions(&self) {
        let booted = self.intern_type(&[INT, FLOAT], FlowFlags::EMPTY);
        debug_assert_eq!(booted, TYPE_INT_OR_FLOAT);

        let booted = self.intern_type(&[INT, STRING], FlowFlags::EMPTY);
        debug_assert_eq!(booted, TYPE_INT_OR_STRING);

        let booted = self.intern_type(&[NULL, SCALAR], FlowFlags::EMPTY);
        debug_assert_eq!(booted, TYPE_NULL_OR_SCALAR);

        let booted = self.intern_type(&[NULL, STRING], FlowFlags::EMPTY);
        debug_assert_eq!(booted, TYPE_NULL_OR_STRING);

        let booted = self.intern_type(&[NULL, INT], FlowFlags::EMPTY);
        debug_assert_eq!(booted, TYPE_NULL_OR_INT);

        let booted = self.intern_type(&[NULL, FLOAT], FlowFlags::EMPTY);
        debug_assert_eq!(booted, TYPE_NULL_OR_FLOAT);

        let booted = self.intern_type(&[NULL, OBJECT], FlowFlags::EMPTY);
        debug_assert_eq!(booted, TYPE_NULL_OR_OBJECT);

        let booted = self.intern_type(&[INT_MINUS_ONE, INT_ZERO, INT_ONE], FlowFlags::EMPTY);
        debug_assert_eq!(booted, TYPE_MINUS_ONE_ZERO_ONE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interner::interner;

    #[test]
    #[inline]
    fn type_int_resolves_to_singleton_int_union() {
        let t = TYPE_INT.as_ref();
        assert_eq!(t.elements, &[INT]);
        assert_eq!(TYPE_INT.flags(), FlowFlags::EMPTY);
    }

    #[test]
    #[inline]
    fn type_null_or_string_elements_are_in_canonical_order() {
        let t = TYPE_NULL_OR_STRING.as_ref();
        assert_eq!(t.elements.len(), 2);
        assert_eq!(t.elements[0], NULL);
        assert_eq!(t.elements[1], STRING);
    }

    #[test]
    #[inline]
    fn type_minus_one_zero_one_elements_are_sorted_by_int_slot() {
        let t = TYPE_MINUS_ONE_ZERO_ONE.as_ref();
        assert_eq!(t.elements, &[INT_ZERO, INT_ONE, INT_MINUS_ONE]);
    }

    #[test]
    #[inline]
    fn well_known_int_payloads_resolve_correctly() {
        let i = interner();
        assert_eq!(i.get_int(INT), &IntInfo::Unspecified);
        assert_eq!(i.get_int(LITERAL_INT), &IntInfo::UnspecifiedLiteral);
        assert_eq!(i.get_int(INT_ZERO), &IntInfo::Literal(0));
        assert_eq!(i.get_int(INT_ONE), &IntInfo::Literal(1));
        assert_eq!(i.get_int(INT_MINUS_ONE), &IntInfo::Literal(-1));
    }

    #[test]
    #[inline]
    fn well_known_resource_payloads_resolve_correctly() {
        let i = interner();
        assert_eq!(i.get_resource(RESOURCE), &ResourceInfo::Any);
        assert_eq!(i.get_resource(OPEN_RESOURCE), &ResourceInfo::Open);
        assert_eq!(i.get_resource(CLOSED_RESOURCE), &ResourceInfo::Closed);
    }

    #[test]
    #[inline]
    fn well_known_class_like_string_payloads_resolve_correctly() {
        let i = interner();
        assert_eq!(i.get_class_like_string(CLASS_STRING).kind, ClassLikeKind::Class);
        assert_eq!(i.get_class_like_string(INTERFACE_STRING).kind, ClassLikeKind::Interface);
        assert_eq!(i.get_class_like_string(ENUM_STRING).kind, ClassLikeKind::Enum);
        assert_eq!(i.get_class_like_string(TRAIT_STRING).kind, ClassLikeKind::Trait);
    }

    #[test]
    #[inline]
    fn empty_array_resolves_to_sealed_no_known_items() {
        let info = interner().get_array(EMPTY_ARRAY);
        assert!(info.is_sealed());
        assert!(info.known_items.is_none());
    }

    #[test]
    #[inline]
    fn array_key_mixed_uses_well_known_type_ids() {
        let info = interner().get_array(ARRAY_KEY_MIXED);
        assert_eq!(info.key_param, Some(TYPE_ARRAY_KEY));
        assert_eq!(info.value_param, Some(TYPE_MIXED));
    }

    #[test]
    #[inline]
    fn iterable_mixed_mixed_uses_well_known_type_ids() {
        let info = interner().get_iterable(ITERABLE_MIXED_MIXED);
        assert_eq!(info.key_type, TYPE_MIXED);
        assert_eq!(info.value_type, TYPE_MIXED);
    }
}
