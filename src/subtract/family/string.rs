//! `String \ String` axis-narrowing rules.

use crate::ElementId;
use crate::element::payload::scalar::StringCasing;
use crate::element::payload::scalar::StringLiteral;
use crate::element::payload::scalar::StringRefinementFlags;
use crate::interner::interner;

/// `String \ String` for axis-narrowing cases.
///
/// - Two distinct string literals: subtract is identity (the literal
///   sets are disjoint, but our `overlaps` returns `true` due to the
///   broader `String` family rules; we keep `a` unchanged here so the
///   distributive fold still terminates correctly).
/// - Equal literals: collapse to bottom.
/// - General string `\` non-empty / truthy string: only the empty
///   string `""` survives.
pub(in crate::subtract) fn string_minus(a: ElementId, b: ElementId) -> Option<Vec<ElementId>> {
    let i = interner();
    let a_info = *i.get_string(a);
    let b_info = *i.get_string(b);

    if let StringLiteral::Value(av) = a_info.literal
        && let StringLiteral::Value(bv) = b_info.literal
        && av == bv
    {
        return Some(Vec::new());
    }

    let a_is_general = matches!(a_info.literal, StringLiteral::None | StringLiteral::Unspecified)
        && a_info.flags == StringRefinementFlags::EMPTY
        && matches!(a_info.casing, StringCasing::Unspecified);

    // `general-string \ broad-non-empty-string` collapses to `""`;
    // a specific literal removes only one value and has no
    // canonical complement form so subtract stays identity.
    let b_is_broad = matches!(b_info.literal, StringLiteral::None | StringLiteral::Unspecified);
    let b_requires_non_empty = b_info.flags.is_non_empty() || b_info.flags.is_truthy();
    if a_is_general && b_is_broad && b_requires_non_empty {
        return Some(vec![ElementId::string_literal("")]);
    }

    None
}
