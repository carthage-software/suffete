//! `String` family meet: union-of-constraints algebra plus the
//! `numeric ∧ string` cross-kind crossing.

use crate::ElementId;
use crate::ElementKind;
use crate::element::payload::scalar::StringCasing;
use crate::element::payload::scalar::StringInfo;
use crate::element::payload::scalar::StringLiteral;
use crate::element::payload::scalar::StringRefinementFlags;
use crate::interner::interner;

/// Intersect two `String` atoms. The result has every flag present in
/// either side (OR-merge), the casing constraint of either when the
/// other is unspecified (AND-merge), and a literal value when only one
/// side pins one. Opposite fixed casings collapse to `lit("")` (the
/// only string satisfying both); literal-vs-flag and literal-vs-casing
/// incompatibilities collapse to `None` (disjoint).
pub(in crate::meet) fn string_meet(a: ElementId, b: ElementId) -> Option<ElementId> {
    let i = interner();
    let a_info = *i.get_string(a);
    let b_info = *i.get_string(b);

    let opposite_casings =
        matches!((a_info.casing, b_info.casing), (StringCasing::Lowercase, StringCasing::Uppercase) | (StringCasing::Uppercase, StringCasing::Lowercase));

    let casing = match (a_info.casing, b_info.casing) {
        (StringCasing::Lowercase, StringCasing::Lowercase) => StringCasing::Lowercase,
        (StringCasing::Uppercase, StringCasing::Uppercase) => StringCasing::Uppercase,
        (StringCasing::Unspecified, c) | (c, StringCasing::Unspecified) => c,
        // Opposite fixed casings: no surviving casing constraint; the
        // remaining values must contain neither lowercase nor uppercase
        // ASCII chars. Suffete doesn't model that constraint directly,
        // so we erase the casing axis and let the literal-vs-flags /
        // literal-vs-casing checks below decide the precise outcome.
        _ => StringCasing::Unspecified,
    };

    let flags = a_info.flags.or(b_info.flags);

    let literal = match (a_info.literal, b_info.literal) {
        (StringLiteral::Value(va), StringLiteral::Value(vb)) => {
            if va == vb {
                StringLiteral::Value(va)
            } else {
                return None;
            }
        }
        (StringLiteral::Value(v), StringLiteral::Unspecified)
        | (StringLiteral::Unspecified, StringLiteral::Value(v)) => StringLiteral::Value(v),
        (StringLiteral::Value(v), StringLiteral::None) | (StringLiteral::None, StringLiteral::Value(v)) => {
            StringLiteral::Value(v)
        }
        (StringLiteral::Unspecified, _) | (_, StringLiteral::Unspecified) => StringLiteral::Unspecified,
        (StringLiteral::None, StringLiteral::None) => StringLiteral::None,
    };

    // Opposite-casing collapse: when neither side fixed a literal value,
    // the only string in both sets is `""`. With a literal on one side,
    // fall through — the literal-vs-flags / literal-vs-casing checks
    // below will reject it iff the literal violates the casing it carries.
    if opposite_casings && matches!(literal, StringLiteral::None | StringLiteral::Unspecified) {
        return Some(ElementId::string_literal(""));
    }

    let merged = StringInfo { literal, casing, flags };
    if !literal_satisfies_flags(merged.literal, merged.flags) {
        return None;
    }
    if opposite_casings
        && let StringLiteral::Value(v) = merged.literal
    {
        let s = v.as_str();
        // Literal must satisfy BOTH original casings — i.e. carry no
        // ASCII letters at all.
        if s.chars().any(|c| c.is_ascii_alphabetic()) {
            return None;
        }
    }
    if !literal_satisfies_casing(merged.literal, merged.casing) {
        return None;
    }
    Some(i.intern_string(merged))
}

/// `numeric ∧ string` is the set of strings whose value parses as a
/// number — i.e. the `numeric-string` refinement, preserving any
/// casing / literal / flags already on the string side.
pub(in crate::meet) fn numeric_string_meet(a: ElementId, b: ElementId) -> Option<ElementId> {
    let i = interner();
    let string_atom = if a.kind() == ElementKind::String { a } else { b };
    let string_info = *i.get_string(string_atom);

    let merged = StringInfo {
        literal: string_info.literal,
        casing: string_info.casing,
        flags: string_info.flags.with_is_numeric(true),
    };
    if !literal_satisfies_flags(merged.literal, merged.flags) {
        return None;
    }
    Some(i.intern_string(merged))
}

fn literal_satisfies_flags(literal: StringLiteral, flags: StringRefinementFlags) -> bool {
    let StringLiteral::Value(v) = literal else { return true };
    let s = v.as_str();
    if flags.is_non_empty() && s.is_empty() {
        return false;
    }
    if flags.is_truthy() && (s.is_empty() || s == "0") {
        return false;
    }
    if flags.is_numeric() && !(s.parse::<i64>().is_ok() || s.parse::<f64>().is_ok()) {
        return false;
    }
    true
}

fn literal_satisfies_casing(literal: StringLiteral, casing: StringCasing) -> bool {
    let StringLiteral::Value(v) = literal else { return true };
    let s = v.as_str();
    match casing {
        StringCasing::Unspecified => true,
        StringCasing::Lowercase => !s.chars().any(|c| c.is_ascii_uppercase()),
        StringCasing::Uppercase => !s.chars().any(|c| c.is_ascii_lowercase()),
    }
}
