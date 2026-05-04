//! String-family join: axis merging and literal-count collapse.

use crate::ElementId;
use crate::ElementKind;
use crate::element::payload::scalar::StringCasing;
use crate::element::payload::scalar::StringInfo;
use crate::element::payload::scalar::StringLiteral;
use crate::interner::interner;
use crate::prelude::STRING;

/// Merge same-kind string elements via AND-of-flags algebra. When
/// multiple strings are present, this folds them into a single
/// general/refined string plus the surviving incompatible literals.
///
/// The merge rules:
///
/// - `lower_string | upper_string` → `string` (casing collapses to
///   `Unspecified`).
/// - `non_empty_string | lit("")` → `string` (empty literal forces
///   `is_non_empty` and `is_truthy` and `is_numeric` off).
/// - `truthy_string | lit("0")` → `truthy_string`, `lit("0")` (literal
///   "0" is incompatible with truthy → kept separate).
/// - `numeric_string | lit("123")` → `numeric_string` (compatible
///   literal absorbed).
/// - `numeric_string | lit("abc")` → `numeric_string`, `lit("abc")`
///   (non-numeric literal stays separate).
pub(in crate::join) fn apply_string_axis_merge_in_order(elements: &[ElementId]) -> Vec<ElementId> {
    let i = interner();

    let mut other: Vec<ElementId> = Vec::with_capacity(elements.len());
    let mut general: Option<StringInfo> = None;
    let mut literals: Vec<mago_atom::Atom> = Vec::new();

    for &el in elements {
        if el.kind() != ElementKind::String {
            other.push(el);
            continue;
        }
        let info = *i.get_string(el);
        if let StringLiteral::Value(value) = info.literal {
            if let Some(ref mut existing) = general {
                let lit_value = value.as_str();
                let incompatible = (existing.flags.is_numeric() && !str_is_numeric(lit_value))
                    || (existing.flags.is_truthy() && (lit_value.is_empty() || lit_value == "0"))
                    || (existing.flags.is_non_empty() && lit_value.is_empty())
                    || (existing.casing == StringCasing::Lowercase
                        && lit_value.chars().any(|c| c.is_ascii_uppercase()))
                    || (existing.casing == StringCasing::Uppercase
                        && lit_value.chars().any(|c| c.is_ascii_lowercase()));
                if incompatible {
                    literals.push(value);
                } else {
                    *existing = combine_string_info(*existing, info);
                }
            } else {
                literals.push(value);
            }
            continue;
        }

        match general {
            None => {
                let mut new_info = info;
                if new_info.flags.is_truthy()
                    || new_info.flags.is_non_empty()
                    || new_info.flags.is_numeric()
                    || new_info.casing != StringCasing::Unspecified
                {
                    let mut keep_literals: Vec<mago_atom::Atom> = Vec::new();
                    let mut hit_empty = false;
                    for atom in &literals {
                        let value = atom.as_str();
                        if value.is_empty() {
                            new_info.flags =
                                new_info.flags.with_is_non_empty(false).with_is_truthy(false).with_is_numeric(false);
                            hit_empty = true;
                            break;
                        }
                        if value == "0" {
                            new_info.flags = new_info.flags.with_is_truthy(false);
                        }
                        if new_info.flags.is_numeric() && !str_is_numeric(value) {
                            keep_literals.push(*atom);
                            continue;
                        }

                        let literal_casing_is_incompatible = match new_info.casing {
                            StringCasing::Lowercase if value.chars().any(|c| c.is_ascii_uppercase()) => true,
                            StringCasing::Uppercase if value.chars().any(|c| c.is_ascii_lowercase()) => true,
                            _ => false,
                        };

                        if literal_casing_is_incompatible {
                            keep_literals.push(*atom);
                            continue;
                        }

                        new_info.flags =
                            new_info.flags.with_is_numeric(new_info.flags.is_numeric() && str_is_numeric(value));
                        new_info.casing = match new_info.casing {
                            StringCasing::Lowercase => StringCasing::Lowercase,
                            StringCasing::Uppercase => StringCasing::Uppercase,
                            _ => StringCasing::Unspecified,
                        };
                    }

                    if hit_empty {
                        new_info.casing = StringCasing::Unspecified;
                    }

                    literals = keep_literals;
                }

                general = Some(new_info);
            }
            Some(ref mut existing) => {
                *existing = combine_string_info(*existing, info);
            }
        }
    }

    let mut new_strings: Vec<ElementId> =
        literals.into_iter().map(|atom| ElementId::string_literal(atom.as_str())).collect();
    if let Some(info) = general {
        new_strings.push(i.intern_string(info));
    }

    other.extend(new_strings);
    other
}

#[inline]
fn combine_string_info(a: StringInfo, b: StringInfo) -> StringInfo {
    let literal = match (a.literal, b.literal) {
        (StringLiteral::Value(v1), StringLiteral::Value(v2)) => {
            if v1 == v2 {
                StringLiteral::Value(v2)
            } else {
                StringLiteral::Unspecified
            }
        }
        (StringLiteral::Unspecified, _) | (_, StringLiteral::Unspecified) => StringLiteral::Unspecified,
        _ => StringLiteral::None,
    };
    let casing = match (a.casing, b.casing) {
        (StringCasing::Lowercase, StringCasing::Lowercase) => StringCasing::Lowercase,
        (StringCasing::Uppercase, StringCasing::Uppercase) => StringCasing::Uppercase,
        _ => StringCasing::Unspecified,
    };
    StringInfo { literal, casing, flags: a.flags.and(b.flags) }
}

#[inline]
fn str_is_numeric(s: &str) -> bool {
    s.parse::<i64>().is_ok() || s.parse::<f64>().is_ok()
}

/// Drop string literals and add the broad `string` form when the
/// literal count exceeds `threshold`.
pub(in crate::join) fn apply_string_literal_collapse(elements: &mut Vec<ElementId>, threshold: u16) {
    if crate::element::simd::contains(elements, STRING) {
        return;
    }
    let i = interner();
    let count = elements
        .iter()
        .filter(|e| e.kind() == ElementKind::String && matches!(i.get_string(**e).literal, StringLiteral::Value(_)))
        .count();
    if count as u32 <= u32::from(threshold) {
        return;
    }
    elements
        .retain(|e| !(e.kind() == ElementKind::String && matches!(i.get_string(*e).literal, StringLiteral::Value(_))));
    let pos = elements.binary_search(&STRING).unwrap_or_else(|p| p);
    elements.insert(pos, STRING);
}
