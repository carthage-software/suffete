//! Class-like-string family: `class-string`, `interface-string`,
//! `enum-string`, `trait-string`, plus refined forms (`class-string<Foo>`,
//! the literal `"App\\Foo"` typed as a class-string).
//!
//! Distinct kinds are disjoint: `class-string` is not a subtype of
//! `interface-string`, etc. Within a kind, the `Any` specifier accepts
//! anything; literal specifiers only fit themselves (refl).

use crate::ElementId;
use crate::ElementKind;
use crate::element::payload::ClassLikeStringSpecifier;
use crate::interner::interner;

pub fn refines(input: ElementId, container: ElementId) -> bool {
    if input.kind() != ElementKind::ClassLikeString {
        return false;
    }

    let i = interner();
    let container_info = *i.get_class_like_string(container);
    let input_info = *i.get_class_like_string(input);

    if input_info.kind != container_info.kind {
        return false;
    }

    matches!(
        (input_info.specifier, container_info.specifier),
        (_, ClassLikeStringSpecifier::Any)
            | (ClassLikeStringSpecifier::Literal { .. }, ClassLikeStringSpecifier::Literal { .. })
    )
}
