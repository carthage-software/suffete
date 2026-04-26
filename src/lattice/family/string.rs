//! String family.
//!
//! Containers express constraints on three axes:
//!
//! - literal slot (`None` / `Unspecified` / `Value(v)`)
//! - casing (`Unspecified` / `Lowercase` / `Uppercase`)
//! - refinement flags (`is_non_empty`, `is_truthy`, `is_numeric`,
//!   `is_callable`)
//!
//! The input must satisfy *every* constraint the container imposes. Each
//! constraint is satisfied either by an equivalent constraint on the input,
//! or by the input being a literal value that structurally implies it
//! (e.g. `"abc"` is non-empty by inspection).
//!
//! Class-like-string inputs are also accepted here: class names are
//! non-empty and not `"0"`, so they satisfy `non-empty` and `truthy`. They
//! do not satisfy casing or `is_callable` constraints by default.

use crate::ElementId;
use crate::ElementKind;
use crate::element::payload::ClassLikeKind;
use crate::element::payload::scalar::StringCasing;
use crate::element::payload::scalar::StringInfo;
use crate::element::payload::scalar::StringLiteral;
use crate::element::payload::scalar::StringRefinementFlags;
use crate::interner::interner;

pub fn refines(input: ElementId, container: ElementId) -> bool {
    let i = interner();
    let container_info = *i.get_string(container);

    if input.kind() == ElementKind::ClassLikeString {
        return class_like_string_satisfies(i.get_class_like_string(input).kind, container_info);
    }

    if input.kind() != ElementKind::String {
        return false;
    }

    let input_info = *i.get_string(input);
    string_satisfies(input_info, container_info)
}

fn class_like_string_satisfies(_kind: ClassLikeKind, container: StringInfo) -> bool {
    if !literal_constraint_admits_class_like(container.literal) {
        return false;
    }

    if container.casing != StringCasing::Unspecified {
        return false;
    }

    !container.flags.is_callable()
}

fn literal_constraint_admits_class_like(literal: StringLiteral) -> bool {
    matches!(literal, StringLiteral::None)
}

fn string_satisfies(input: StringInfo, container: StringInfo) -> bool {
    satisfies_literal(input.literal, container.literal)
        && satisfies_casing(input, container.casing)
        && satisfies_flags(input, container.flags)
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
                StringLiteral::Value(v) => v.as_str().chars().all(|c| !c.is_ascii_uppercase()),
                _ => false,
            },
        },
        StringCasing::Uppercase => match input.casing {
            StringCasing::Uppercase => true,
            _ => match input.literal {
                StringLiteral::Value(v) => v.as_str().chars().all(|c| !c.is_ascii_lowercase()),
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

    if container_flags.is_callable() && !input.flags.is_callable() {
        return false;
    }

    true
}

pub(super) fn input_is_non_empty(input: StringInfo) -> bool {
    // Truthy, numeric, and callable all imply non-empty (the empty string
    // is none of those).
    if input.flags.is_non_empty() || input.flags.is_truthy() || input.flags.is_numeric() || input.flags.is_callable() {
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

pub(super) fn input_is_numeric(input: StringInfo) -> bool {
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
