//! Object family: `object` (the dominator), named objects (`Foo`),
//! enums and enum cases, object shapes, has-method / has-property
//! narrowings.
//!
//! Implements the nominal subtype check from `comparison.md` plus the
//! type-argument comparison from `generics.md` §5 (specialisation): for
//! same-class containers, walk type arguments by position with the
//! container's variance; for descendant containers, resolve the inherited
//! arguments via [`World::inherited_template_argument`], substitute
//! `child`'s actual arguments through them, and then compare positionally
//! with the container's variance.

use crate::ElementId;
use crate::ElementKind;
use crate::TypeId;
use crate::element::payload::DefiningEntity;
use crate::element::payload::GenericParameterInfo;
use crate::interner::interner;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::lattice::refines::refines as type_refines;
use crate::prelude::TYPE_MIXED;
use crate::substitute::substitute;
use crate::world::TemplateParameter;
use crate::world::Variance;
use crate::world::World;

/// Container is `object` (`ObjectAny`): accept anything in the object
/// family.
pub fn refines_object_any(input: ElementId, _container: ElementId) -> bool {
    is_object_family_kind(input.kind())
}

/// Refinement for `Object | Enum | ObjectShape | HasMethod | HasProperty`
/// containers.
pub fn refines<W: World>(
    input: ElementId,
    container: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let i = interner();

    match (input.kind(), container.kind()) {
        (ElementKind::Object, ElementKind::Object) => {
            let input_info = *i.get_object(input);
            let container_info = *i.get_object(container);
            refines_named_named(input_info, container_info, world, options, report)
        }

        // Enum-vs-enum: same enum name, container has no case constraint.
        (ElementKind::Enum, ElementKind::Enum) => {
            let input_info = i.get_enum(input);
            let container_info = i.get_enum(container);
            input_info.name == container_info.name && container_info.case.is_none()
        }

        // Enums and named-objects don't cross — enums implement interfaces
        // but those flow as named objects (separate dispatch branch).
        (ElementKind::Object, ElementKind::Enum) | (ElementKind::Enum, ElementKind::Object) => false,

        _ => false,
    }
}

fn refines_named_named<W: World>(
    input: crate::element::payload::ObjectInfo,
    container: crate::element::payload::ObjectInfo,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    if !world.descends_from(input.name, container.name) {
        return false;
    }

    // Container takes no type arguments: nominal check is sufficient.
    let Some(container_args_id) = container.type_args else {
        return true;
    };

    let container_args: Vec<TypeId> = interner().get_type_list(container_args_id).to_vec();
    let input_actual_args: Vec<TypeId> =
        input.type_args.map(|id| interner().get_type_list(id).to_vec()).unwrap_or_default();
    let same_class = input.name == container.name;

    for (position, &container_arg) in container_args.iter().enumerate() {
        let input_arg = input_argument_for_container_position(
            input.name,
            &input_actual_args,
            container.name,
            position,
            same_class,
            world,
        );
        let Some(input_arg) = input_arg else {
            return false;
        };

        let variance = world
            .template_parameter_at(container.name, position)
            .map(|p: TemplateParameter| p.variance)
            .unwrap_or_default();

        if !compare_with_variance(input_arg, container_arg, variance, world, options, report) {
            return false;
        }
    }

    true
}

/// Resolve "what does `input` pass for `container`'s template at
/// `position`", expressed in suffete's type universe and free of any
/// remaining references to `input`'s own templates.
///
/// Same-class case: the input's positional argument, or its constraint /
/// `mixed` when no argument was supplied at the use site (the spec's
/// "partial application" path).
///
/// Strict-descendant case: query [`World::inherited_template_argument`]
/// for the chain-resolved type (in `input`'s template namespace), then
/// substitute `input`'s actual arguments into any `GenericParameter`
/// references that name `input`'s own templates.
fn input_argument_for_container_position<W: World>(
    input_name: mago_atom::Atom,
    input_actual_args: &[TypeId],
    container_name: mago_atom::Atom,
    position: usize,
    same_class: bool,
    world: &W,
) -> Option<TypeId> {
    if same_class {
        if let Some(&arg) = input_actual_args.get(position) {
            return Some(arg);
        }
        // No argument supplied at this position: fall back to the
        // template's constraint, defaulting to `mixed` when the world
        // doesn't know the parameter.
        return Some(
            world.template_parameter_at(input_name, position).and_then(|p| p.upper_bound).unwrap_or(TYPE_MIXED),
        );
    }

    let inherited = world.inherited_template_argument(input_name, container_name, position)?;
    let input_entity = interner().intern_defining_entity(DefiningEntity::ClassLike(input_name));

    Some(substitute(inherited, &|info: &GenericParameterInfo| -> Option<TypeId> {
        if info.defining_entity != input_entity {
            return None;
        }
        let pos = world.template_parameter_index(input_name, info.name)?;
        input_actual_args.get(pos).copied()
    }))
}

fn compare_with_variance<W: World>(
    input: TypeId,
    container: TypeId,
    variance: Variance,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    match variance {
        Variance::Covariant => type_refines(input, container, world, options, report),
        Variance::Contravariant => type_refines(container, input, world, options, report),
        Variance::Invariant => {
            type_refines(input, container, world, options, report)
                && type_refines(container, input, world, options, report)
        }
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
