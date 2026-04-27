//! Two scalar-widening modes for [`TypeId`].
//!
//! - [`scalars`]: replace **every** scalar narrowing — literal *and*
//!   user-declared — with the family dominator. `42` / `int<0,10>` /
//!   `positive-int` → `int`; `"foo"` / `non-empty-string` / `truthy-
//!   numeric-string` → `string`; `1.5` → `float`; `true` / `false` →
//!   `bool`. `class-string<Foo>` / `class-string<of: T>` →
//!   `class-string<Any>` (preserves the `Class`/`Interface`/
//!   `Enum`/`Trait` kind).
//!
//! - [`literals`]: replace literal scalar values with their tightest
//!   non-literal refinement; preserve user-declared narrowings.
//!   `42` / `0` / `-1` / `literal-int` → `int`. `1.5` /
//!   `literal-float` → `float`. `true` / `false` → `bool`. `"foo"` →
//!   `non-empty-truthy-lowercase-string` (every refinement bit the
//!   literal value satisfies; empty string `""` → `string`).
//!   `class-string<Foo>` → `class-string<Any>` of the same kind.
//!   Ranges, refinement flags, casing, and any non-literal form pass
//!   through unchanged.
//!
//! Both modes descend through every nested-type carrier via
//! [`crate::transform::map`].

use crate::ElementId;
use crate::ElementKind;
use crate::TypeId;
use crate::element::payload::ClassLikeStringInfo;
use crate::element::payload::ClassLikeStringSpecifier;
use crate::element::payload::FloatInfo;
use crate::element::payload::IntInfo;
use crate::element::payload::StringCasing;
use crate::element::payload::StringInfo;
use crate::element::payload::StringLiteral;
use crate::element::payload::StringRefinementFlags;
use crate::interner::interner;
use crate::prelude::BOOL;
use crate::prelude::FLOAT;
use crate::prelude::INT;
use crate::prelude::STRING;
use crate::transform;

/// Replace every scalar narrowing with its family dominator.
pub fn scalars(ty: TypeId) -> TypeId {
    transform::map(ty, widen_element_scalar)
}

/// Replace literal scalar values with their tightest non-literal
/// refinement; preserve user-declared narrowings.
pub fn literals(ty: TypeId) -> TypeId {
    transform::map(ty, widen_element_literal)
}

fn widen_element_scalar(elem: ElementId) -> ElementId {
    match elem.kind() {
        ElementKind::Int => INT,
        ElementKind::Float => FLOAT,
        ElementKind::String => STRING,
        ElementKind::True | ElementKind::False => BOOL,
        ElementKind::ClassLikeString => widen_class_like_string_to_any(elem),
        _ => elem,
    }
}

fn widen_element_literal(elem: ElementId) -> ElementId {
    match elem.kind() {
        ElementKind::Int => widen_int_literal(elem),
        ElementKind::Float => widen_float_literal(elem),
        ElementKind::String => widen_string_literal(elem),
        ElementKind::True | ElementKind::False => BOOL,
        ElementKind::ClassLikeString => widen_class_like_string_to_any(elem),
        _ => elem,
    }
}

fn widen_int_literal(elem: ElementId) -> ElementId {
    match interner().get_int(elem) {
        IntInfo::Literal(_) | IntInfo::UnspecifiedLiteral => INT,
        _ => elem,
    }
}

fn widen_float_literal(elem: ElementId) -> ElementId {
    match interner().get_float(elem) {
        FloatInfo::Literal(_) | FloatInfo::UnspecifiedLiteral => FLOAT,
        _ => elem,
    }
}

/// Build a non-literal `String` element capturing every PHP-correct
/// refinement bit the literal value satisfies. Empty strings collapse
/// to `STRING` naturally (every inferred flag is false → matches the
/// well-known dominator's interned shape).
fn widen_string_literal(elem: ElementId) -> ElementId {
    let i = interner();
    let info = *i.get_string(elem);
    match info.literal {
        StringLiteral::None => elem,
        StringLiteral::Unspecified => i.intern_string(StringInfo { literal: StringLiteral::None, ..info }),
        StringLiteral::Value(value) => {
            let s = value.as_str();
            let is_numeric = str_is_numeric(s);
            let is_non_empty = is_numeric || !s.is_empty();
            let is_truthy = is_non_empty && s != "0";

            let flags = StringRefinementFlags::EMPTY
                .with_is_numeric(is_numeric)
                .with_is_non_empty(is_non_empty)
                .with_is_truthy(is_truthy);
            let casing = infer_casing(s);

            i.intern_string(StringInfo { literal: StringLiteral::None, casing, flags })
        }
    }
}

fn widen_class_like_string_to_any(elem: ElementId) -> ElementId {
    let i = interner();
    let info = *i.get_class_like_string(elem);
    match info.specifier {
        ClassLikeStringSpecifier::Any => elem,
        _ => i.intern_class_like_string(ClassLikeStringInfo {
            specifier: ClassLikeStringSpecifier::Any,
            ..info
        }),
    }
}

/// Lowercase only when at least one lowercase letter is present and
/// no uppercase letter is. Uppercase symmetric. Strings with no
/// letters at all (digits, punctuation) get [`StringCasing::Unspecified`]
/// to avoid claiming a casing the string does not exhibit.
fn infer_casing(s: &str) -> StringCasing {
    let has_upper = s.chars().any(|c| c.is_uppercase());
    let has_lower = s.chars().any(|c| c.is_lowercase());
    match (has_upper, has_lower) {
        (false, true) => StringCasing::Lowercase,
        (true, false) => StringCasing::Uppercase,
        _ => StringCasing::Unspecified,
    }
}

/// Checks if a string is numeric according to PHP's definition.
///
/// Trims leading/trailing whitespace, strips a leading sign, removes
/// leading zeros, and uses `f64`'s parser for the remainder.
fn str_is_numeric(input: &str) -> bool {
    let mut maybe_numeric = input.trim();
    if maybe_numeric.is_empty() {
        return false;
    }

    if maybe_numeric.starts_with('+') || maybe_numeric.starts_with('-') {
        maybe_numeric = &maybe_numeric[1..];

        if maybe_numeric.is_empty() {
            return false;
        }
    }

    maybe_numeric = maybe_numeric.trim_start_matches('0');
    if maybe_numeric.is_empty() {
        return true;
    }

    maybe_numeric.parse::<f64>().is_ok()
}
