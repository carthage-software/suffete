//! Mixed family. Container is a `mixed` atom carrying axis flags
//! (`is_non_null`, truthiness, `is_empty`, `is_isset_from_loop`).
//!
//! Vanilla `mixed` is handled by the universal Top axiom in
//! [`crate::lattice::refines::element_refines`]; this family fires only
//! for narrowed mixed containers. The input refines the container iff
//! every axis the container constrains is implied by the input — either
//! because the input is a `mixed` carrying at least the same flags, or
//! because the input's element kind structurally guarantees the property
//! (e.g. an `int` is non-null, a `Named` object is truthy, `EMPTY_STRING`
//! is falsy).

use crate::ElementId;
use crate::ElementKind;
use crate::element::payload::Truthiness;
use crate::element::payload::scalar::FloatInfo;
use crate::element::payload::scalar::IntInfo;
use crate::element::payload::scalar::StringLiteral;
use crate::interner::interner;

pub fn refines(input: ElementId, container: ElementId) -> bool {
    if container.kind() != ElementKind::Mixed {
        return false;
    }

    let container_info = *interner().get_mixed(container);

    if container_info.is_non_null() && !is_non_null(input) {
        return false;
    }

    match container_info.truthiness() {
        Truthiness::Truthy => {
            if truthiness_of(input) != Truthiness::Truthy {
                return false;
            }
        }
        Truthiness::Falsy => {
            if truthiness_of(input) != Truthiness::Falsy {
                return false;
            }
        }
        Truthiness::Undetermined => {}
    }

    if container_info.is_empty() && truthiness_of(input) != Truthiness::Falsy {
        return false;
    }

    // `isset_from_loop` is an analysis-internal marker (a value that flows
    // through a loop body). Only an input that already carries the marker
    // satisfies a container demanding it.
    if container_info.is_isset_from_loop() {
        if input.kind() != ElementKind::Mixed {
            return false;
        }
        if !interner().get_mixed(input).is_isset_from_loop() {
            return false;
        }
    }

    true
}

/// `true` iff `input` cannot be `null`.
fn is_non_null(input: ElementId) -> bool {
    match input.kind() {
        ElementKind::Null | ElementKind::Void => false,
        ElementKind::Mixed => {
            let info = interner().get_mixed(input);
            info.is_non_null() || info.truthiness() == Truthiness::Truthy
        }
        ElementKind::GenericParameter => {
            let info = interner().get_generic_parameter(input);
            let constraint = info.constraint.as_ref();
            constraint.elements.iter().all(|&el| is_non_null(el))
        }
        _ => true,
    }
}

/// Best-known truthiness of `input` as a single value. Returns
/// [`Truthiness::Undetermined`] when both possibilities remain open.
fn truthiness_of(input: ElementId) -> Truthiness {
    match input.kind() {
        ElementKind::True => Truthiness::Truthy,
        ElementKind::False | ElementKind::Null => Truthiness::Falsy,
        ElementKind::Bool => Truthiness::Undetermined,

        ElementKind::ObjectAny | ElementKind::Object | ElementKind::Enum | ElementKind::Resource => Truthiness::Truthy,

        ElementKind::ClassLikeString => Truthiness::Truthy,

        ElementKind::Int => match interner().get_int(input) {
            IntInfo::Literal(0) => Truthiness::Falsy,
            IntInfo::Literal(_) => Truthiness::Truthy,
            IntInfo::Range(range_id) => {
                let range = interner().get_int_range(*range_id);
                match (range.lower(), range.upper()) {
                    (Some(lo), _) if lo > 0 => Truthiness::Truthy,
                    (_, Some(hi)) if hi < 0 => Truthiness::Truthy,
                    _ => Truthiness::Undetermined,
                }
            }
            _ => Truthiness::Undetermined,
        },

        ElementKind::Float => match interner().get_float(input) {
            FloatInfo::Literal(literal) => {
                if literal.value() == 0.0 {
                    Truthiness::Falsy
                } else {
                    Truthiness::Truthy
                }
            }
            _ => Truthiness::Undetermined,
        },

        ElementKind::String => {
            if input == crate::prelude::EMPTY_STRING {
                return Truthiness::Falsy;
            }
            let info = interner().get_string(input);
            if info.flags.is_truthy() {
                return Truthiness::Truthy;
            }
            match info.literal {
                StringLiteral::Value(value) => {
                    let s = value.as_str();
                    if s.is_empty() || s == "0" { Truthiness::Falsy } else { Truthiness::Truthy }
                }
                _ => Truthiness::Undetermined,
            }
        }

        ElementKind::Array => {
            if input == crate::prelude::EMPTY_ARRAY {
                return Truthiness::Falsy;
            }
            let info = interner().get_array(input);
            if info.flags.non_empty() { Truthiness::Truthy } else { Truthiness::Undetermined }
        }
        ElementKind::List => {
            let info = interner().get_list(input);
            if info.flags.non_empty() { Truthiness::Truthy } else { Truthiness::Undetermined }
        }

        ElementKind::Mixed => interner().get_mixed(input).truthiness(),

        ElementKind::GenericParameter => {
            let info = interner().get_generic_parameter(input);
            let constraint = info.constraint.as_ref();
            let mut acc: Option<Truthiness> = None;
            for &el in constraint.elements.iter() {
                let t = truthiness_of(el);
                acc = Some(match acc {
                    None => t,
                    Some(prev) if prev == t => prev,
                    _ => return Truthiness::Undetermined,
                });
            }
            acc.unwrap_or(Truthiness::Undetermined)
        }

        _ => Truthiness::Undetermined,
    }
}
