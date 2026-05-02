#![allow(clippy::arithmetic_side_effects)]

//! Class-like-string family.
//!
//! `class-string`, `interface-string`, `enum-string`, `trait-string`,
//! plus refined forms (`class-string<Foo>`, `class-string<T>`,
//! `class-string<T of B>`, the literal `"App\\Foo"` typed as a class-string).
//!
//! Distinct kinds are disjoint: `class-string` is not a subtype of
//! `interface-string`, etc. Within a kind, the rule is "input fits
//! container iff the class the input names refines the class the
//! container expects". A literal class-string and a refined
//! `class-string<C>` therefore both reduce to "compare the named
//! object against the constraint", which routes through the regular
//! object-family lattice (so all the world's ancestry / generic-arg
//! / variance rules apply).
//!
//! Cross-kind: a regular `String` input whose literal value is a valid
//! PHP class name is also accepted as a class-string, mirroring how
//! the runtime treats `"\\App\\Foo"` interchangeably with
//! `App\Foo::class`.

use crate::ElementId;
use crate::ElementKind;
use crate::FlowFlags;
use crate::TypeId;
use crate::element::payload::ClassLikeKind;
use crate::element::payload::ClassLikeStringSpecifier;
use crate::element::payload::ObjectFlags;
use crate::element::payload::ObjectInfo;
use crate::element::payload::scalar::StringLiteral;
use crate::interner::interner;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::world::World;

#[inline]
pub fn refines<W: World>(
    input: ElementId,
    container: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let i = interner();
    let container_info = *i.get_class_like_string(container);

    if matches!(container_info.specifier, ClassLikeStringSpecifier::Any) {
        return matches_kind(input, container_info.kind, world);
    }

    if !matches_kind(input, container_info.kind, world) {
        return false;
    }

    let Some(container_target) = represented_type(container, world) else {
        return false;
    };

    let Some(input_target) = input_represented_type(input, world) else {
        return false;
    };

    crate::lattice::refines(input_target, container_target, world, options, report)
}

#[inline]
fn matches_kind<W: World>(input: ElementId, container_kind: ClassLikeKind, world: &W) -> bool {
    if input.kind() == ElementKind::String {
        let info = interner().get_string(input);
        let StringLiteral::Value(value) = info.literal else { return false };
        if !is_valid_class_name(value.as_str()) {
            return false;
        }
        // If the world classifies the name, require an exact kind match.
        // When the world is silent, accept: an unknown name stays
        // permissive (open-world).
        return match world.class_like_kind(value) {
            Some(k) => k == container_kind,
            None => true,
        };
    }
    if input.kind() != ElementKind::ClassLikeString {
        return false;
    }
    interner().get_class_like_string(input).kind == container_kind
}

#[inline]
fn represented_type<W: World>(elem: ElementId, world: &W) -> Option<TypeId> {
    let info = *interner().get_class_like_string(elem);
    match info.specifier {
        ClassLikeStringSpecifier::Any => None,
        ClassLikeStringSpecifier::Literal { value } => Some(name_as_object_type(value, info.kind, world)),
        ClassLikeStringSpecifier::OfType { constraint } | ClassLikeStringSpecifier::Generic { constraint } => {
            Some(constraint)
        }
    }
}

#[inline]
fn input_represented_type<W: World>(input: ElementId, world: &W) -> Option<TypeId> {
    if input.kind() == ElementKind::ClassLikeString {
        return represented_type(input, world);
    }
    if input.kind() != ElementKind::String {
        return None;
    }
    let info = interner().get_string(input);
    let StringLiteral::Value(value) = info.literal else { return None };
    if !is_valid_class_name(value.as_str()) {
        return None;
    }
    let kind = kind_from_world(value, world);
    Some(name_as_object_type(value, kind, world))
}

#[inline]
fn name_as_object_type<W: World>(name: mago_atom::Atom, kind: ClassLikeKind, _world: &W) -> TypeId {
    let i = interner();
    let element = match kind {
        ClassLikeKind::Enum => ElementId::enum_any(name.as_str()),
        ClassLikeKind::Class | ClassLikeKind::Interface | ClassLikeKind::Trait => {
            i.intern_object(ObjectInfo { name, type_args: None, intersections: None, flags: ObjectFlags::default() })
        }
    };
    i.intern_type(&[element], FlowFlags::EMPTY)
}

#[inline]
fn kind_from_world<W: World>(name: mago_atom::Atom, world: &W) -> ClassLikeKind {
    world.class_like_kind(name).unwrap_or(ClassLikeKind::Class)
}

/// Validate that `s` is a syntactically well-formed PHP class name
/// (`Foo`, `\Foo`, `Foo\Bar`, `App\Service\Logger`, …). Used to reject
/// arbitrary string literals that don't look like class names before
/// treating them as class-strings.
#[inline]
fn is_valid_class_name(s: &str) -> bool {
    let bytes = s.as_bytes();
    let len = bytes.len();
    if len == 0 || bytes[len - 1] == b'\\' {
        return false;
    }
    let mut i = usize::from(bytes[0] == b'\\');
    if i >= len {
        return false;
    }
    let mut part_start = true;
    while i < len {
        let b = bytes[i];
        #[allow(clippy::else_if_without_else)]
        if b == b'\\' {
            if part_start {
                return false;
            }
            part_start = true;
        } else if part_start {
            if !(b.is_ascii_alphabetic() || b == b'_') {
                return false;
            }
            part_start = false;
        } else if !(b.is_ascii_alphanumeric() || b == b'_' || b >= 0x80) {
            return false;
        }
        i += 1;
    }
    !part_start
}
