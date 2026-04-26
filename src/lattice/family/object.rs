//! Object family: `object` (the dominator), named objects (`Foo`),
//! enums and enum cases, object shapes, has-method / has-property
//! narrowings.
//!
//! `object` (the `ObjectAny` kind) accepts any object-family element. Named
//! objects, enums, shapes, etc., need codebase queries to decide hierarchy
//! relations and are not implemented here yet.

use crate::ElementId;
use crate::ElementKind;

/// Container is `object` (`ObjectAny`): accept anything in the object
/// family.
pub fn refines_object_any(input: ElementId, _container: ElementId) -> bool {
    is_object_family_kind(input.kind())
}

/// Container is `Named(Foo)`, `Enum(E, _)`, `ObjectShape(...)`, etc.
/// Requires codebase queries; not implemented yet.
pub fn refines(_input: ElementId, _container: ElementId) -> bool {
    false
}

pub(crate) fn is_object_family_kind(kind: ElementKind) -> bool {
    matches!(
        kind,
        ElementKind::Object
            | ElementKind::Enum
            | ElementKind::ObjectShape
            | ElementKind::HasMethod
            | ElementKind::HasProperty
            | ElementKind::ObjectAny
    )
}
