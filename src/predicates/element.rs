//! Per-element truthiness / falsiness / literal classifiers.
//!
//! Each function returns a 2-state guarantee on a single element:
//!
//! - [`is_truthy`] ; every value the element admits is truthy.
//! - [`is_falsy`] ; every value the element admits is falsy.
//! - [`could_be_truthy`] ; at least one value could be truthy.
//! - [`could_be_falsy`] ; at least one value could be falsy.
//!
//! [`is_literal`] reports whether the element represents a single
//! literal value (used by [`crate::predicates::is_literal`] and
//! [`crate::predicates::is_constant_foldable`]).

use crate::ElementId;
use crate::ElementKind;
use crate::element::payload::FloatInfo;
use crate::element::payload::ResourceInfo;
use crate::element::payload::Truthiness;
use crate::element::payload::scalar::IntInfo;
use crate::element::payload::scalar::StringLiteral;
use crate::interner::interner;
use crate::prelude;

/// Every value of `elem` is guaranteed truthy at runtime.
#[inline]
pub(crate) fn is_truthy(elem: ElementId) -> bool {
    let i = interner();
    match elem.kind() {
        ElementKind::True => true,
        ElementKind::False => false,
        ElementKind::Bool => false,
        ElementKind::Null | ElementKind::Void | ElementKind::Never => false,

        ElementKind::Object
        | ElementKind::Enum
        | ElementKind::ObjectShape
        | ElementKind::HasMethod
        | ElementKind::HasProperty
        | ElementKind::ObjectAny
        | ElementKind::Callable => true,

        ElementKind::ClassLikeString => true,

        ElementKind::Resource => match i.get_resource(elem) {
            ResourceInfo::Open => true,
            ResourceInfo::Closed => false,
            ResourceInfo::Any => true,
        },

        ElementKind::Int => match i.get_int(elem) {
            IntInfo::Literal(n) => *n != 0,
            IntInfo::Range(range_id) => {
                let r = i.get_int_range(*range_id);
                match (r.lower(), r.upper()) {
                    (Some(lo), _) if lo > 0 => true,
                    (_, Some(hi)) if hi < 0 => true,
                    _ => false,
                }
            }
            _ => false,
        },

        ElementKind::Float => match i.get_float(elem) {
            FloatInfo::Literal(literal) => literal.value() != 0.0,
            _ => false,
        },

        ElementKind::String => {
            if elem == prelude::EMPTY_STRING {
                return false;
            }
            let info = i.get_string(elem);
            if info.flags.is_truthy() {
                return true;
            }
            match info.literal {
                StringLiteral::Value(value) => {
                    let s = value.as_str();
                    !s.is_empty() && s != "0"
                }
                _ => false,
            }
        }

        ElementKind::Array => {
            if elem == prelude::EMPTY_ARRAY {
                return false;
            }
            let info = i.get_array(elem);
            if info.flags.non_empty() {
                return true;
            }
            if let Some(known_id) = info.known_items {
                for entry in i.get_known_items(known_id) {
                    if !entry.optional {
                        return true;
                    }
                }
            }
            false
        }

        ElementKind::List => {
            let info = i.get_list(elem);
            if info.flags.non_empty() {
                return true;
            }
            if let Some(known_id) = info.known_elements {
                for entry in i.get_known_elements(known_id) {
                    if !entry.optional {
                        return true;
                    }
                }
            }
            false
        }

        ElementKind::Mixed => i.get_mixed(elem).truthiness() == Truthiness::Truthy,

        ElementKind::GenericParameter => {
            let info = i.get_generic_parameter(elem);
            let constraint = info.constraint.as_ref();
            !constraint.elements.is_empty() && constraint.elements.iter().all(|&el| is_truthy(el))
        }

        _ => false,
    }
}

/// Every value of `elem` is guaranteed falsy at runtime.
#[inline]
pub(crate) fn is_falsy(elem: ElementId) -> bool {
    let i = interner();
    match elem.kind() {
        ElementKind::True => false,
        ElementKind::False => true,
        ElementKind::Bool => false,
        ElementKind::Null | ElementKind::Void => true,
        ElementKind::Never => false,
        ElementKind::Object
        | ElementKind::Enum
        | ElementKind::ObjectShape
        | ElementKind::HasMethod
        | ElementKind::HasProperty
        | ElementKind::ObjectAny
        | ElementKind::Callable => false,
        ElementKind::ClassLikeString => false,
        ElementKind::Resource => matches!(i.get_resource(elem), ResourceInfo::Closed),
        ElementKind::Int => matches!(i.get_int(elem), IntInfo::Literal(0)),
        ElementKind::Float => matches!(i.get_float(elem), FloatInfo::Literal(literal) if literal.value() == 0.0),
        ElementKind::String => {
            if elem == prelude::EMPTY_STRING {
                return true;
            }

            match i.get_string(elem).literal {
                StringLiteral::Value(value) => value.as_str().is_empty(),
                _ => false,
            }
        }
        ElementKind::Array => {
            if elem == prelude::EMPTY_ARRAY {
                return true;
            }

            let info = i.get_array(elem);
            if info.known_items.is_some() {
                return false;
            }

            if let Some(k) = info.key_param
                && let Some(v) = info.value_param
                && k != crate::prelude::TYPE_NEVER
                && v != crate::prelude::TYPE_NEVER
            {
                return false;
            }

            !info.flags.non_empty()
        }
        ElementKind::List => {
            let info = i.get_list(elem);
            info.known_elements.is_none() && info.element_type == crate::prelude::TYPE_NEVER && !info.flags.non_empty()
        }
        ElementKind::Mixed => i.get_mixed(elem).truthiness() == Truthiness::Falsy,
        ElementKind::GenericParameter => {
            let info = i.get_generic_parameter(elem);
            let constraint = info.constraint.as_ref();
            !constraint.elements.is_empty() && constraint.elements.iter().all(|&el| is_falsy(el))
        }
        _ => false,
    }
}

/// At least one value of `elem` could be truthy. `never` and `void`
/// have no truthy values.
#[inline]
pub(super) fn could_be_truthy(elem: ElementId) -> bool {
    if elem.kind() == ElementKind::Never || elem.kind() == ElementKind::Void {
        return false;
    }

    !is_falsy(elem)
}

/// At least one value of `elem` could be falsy. `never` has no values
/// at all.
#[inline]
pub(super) fn could_be_falsy(elem: ElementId) -> bool {
    if elem.kind() == ElementKind::Never {
        return false;
    }

    !is_truthy(elem)
}

/// `true` iff `elem` represents a single literal value.
pub(super) fn is_literal(elem: ElementId) -> bool {
    let i = interner();
    match elem.kind() {
        ElementKind::True | ElementKind::False | ElementKind::Null | ElementKind::Void => true,
        ElementKind::Int => matches!(i.get_int(elem), IntInfo::Literal(_)),
        ElementKind::Float => matches!(i.get_float(elem), FloatInfo::Literal(_)),
        ElementKind::String => matches!(i.get_string(elem).literal, StringLiteral::Value(_)),
        _ => false,
    }
}
