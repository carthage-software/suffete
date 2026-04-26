//! Object family: `object` (the dominator), named objects (`Foo`),
//! enums and enum cases, object shapes, has-method / has-property
//! narrowings.
//!
//! `object` (the `ObjectAny` kind) accepts any object-family element. Named
//! objects use [`Codebase::is_subclass_of`] for hierarchy queries; enums
//! match by name (no enum-vs-enum hierarchy in PHP); enum cases narrow
//! their owning enum but not other enums or cases.

use crate::ElementId;
use crate::ElementKind;
use crate::interner::interner;
use crate::lattice::Codebase;

/// Container is `object` (`ObjectAny`): accept anything in the object
/// family.
pub fn refines_object_any(input: ElementId, _container: ElementId) -> bool {
    is_object_family_kind(input.kind())
}

/// Refinement for `Object | Enum | ObjectShape | HasMethod | HasProperty`
/// containers. Hierarchy queries flow through [`Codebase::is_subclass_of`].
pub fn refines<C: Codebase>(input: ElementId, container: ElementId, codebase: &C) -> bool {
    let i = interner();

    match (input.kind(), container.kind()) {
        // Named-vs-named: codebase decides.
        (ElementKind::Object, ElementKind::Object) => {
            let input_info = i.get_object(input);
            let container_info = i.get_object(container);
            codebase.is_subclass_of(input_info.name, container_info.name)
        }

        // Enum-vs-enum: same enum (refl handled upstream); different enums
        // are disjoint.
        (ElementKind::Enum, ElementKind::Enum) => {
            let input_info = i.get_enum(input);
            let container_info = i.get_enum(container);
            // Same enum name, container has no case constraint: input fits
            // (whether case-narrowed or not).
            input_info.name == container_info.name && container_info.case.is_none()
        }

        // Named-vs-enum or enum-vs-named: PHP enums and classes don't share
        // a hierarchy here (enums implement interfaces, but the object
        // family handles those as named objects). Distinct.
        (ElementKind::Object, ElementKind::Enum) | (ElementKind::Enum, ElementKind::Object) => false,

        _ => false,
    }
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
