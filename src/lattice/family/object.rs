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
//!
//! Intersection types (`Foo&Bar`) are handled by the Int-L / Int-R rules
//! from comparison.md §1.4.3: container intersections require the input
//! to refine every conjunct; input intersections require some conjunct to
//! refine the container.
//!
//! `static` and `$this` modality (comparison.md §1.4.4) is enforced
//! asymmetrically: a container marked `static` (or `$this`) accepts only
//! inputs that are at least as constrained on that flag.
//!
//! Structural narrowings (comparison.md §1.4.6):
//!
//! - `HasMethod(m)`: input is accepted iff it is itself `HasMethod(m)`,
//!   or a `Named(C)` (or descendant) where `Γ` confirms `C` declares /
//!   inherits `m`.
//! - `HasProperty(p)`: symmetric to `HasMethod`.
//! - `ObjectShape{props_out}`: shape-vs-shape uses the same rules as
//!   keyed arrays — every required-out key must be present (and
//!   required) in the input shape with a refining value, and a sealed
//!   container demands a sealed input. `Named(C)` refines an object
//!   shape iff every required property of the shape is declared on `C`
//!   with a refining declared type, queried via `World::class_property_type`.

use mago_atom::Atom;
use mago_atom::atom;

use crate::ElementId;
use crate::ElementKind;
use crate::FlowFlags;
use crate::TypeId;
use crate::element::payload::DefiningEntity;
use crate::element::payload::EnumInfo;
use crate::element::payload::GenericParameterInfo;
use crate::element::payload::KnownPropertyEntry;
use crate::element::payload::ObjectFlags;
use crate::element::payload::ObjectInfo;
use crate::element::payload::ObjectShapeFlags;
use crate::element::payload::ObjectShapeInfo;
use crate::interner::interner;
use crate::lattice::CoercionCauses;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::lattice::refines::refines as type_refines;
use crate::prelude::NON_EMPTY_STRING;
use crate::prelude::TYPE_MIXED;
use crate::template::substitute;
use crate::world::EnumBacking;
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

    // Container-intersection rule fires first: input fits `H & C₁ & … &
    // Cₙ` iff input fits the head AND each conjunct. Running this before
    // stripping input's intersections is what lets `D&B&A<int> <: B&D`
    // succeed via a per-conjunct check (each container conjunct is
    // matched by some input conjunct), instead of short-circuiting on
    // "no single input conjunct refines the whole container".
    if container.kind() == ElementKind::Object {
        let container_info = *i.get_object(container);
        if let Some(intersections_id) = container_info.intersections {
            let head = i.intern_object(ObjectInfo { intersections: None, ..container_info });
            if !element_refines_via_type(input, head, world, options, report) {
                return false;
            }
            for &conjunct in i.get_element_list(intersections_id) {
                if !element_refines_via_type(input, conjunct, world, options, report) {
                    return false;
                }
            }
            return true;
        }
    }

    // Container-intersection rule for HasMethod / HasProperty: same as
    // for Object — every conjunct (including the head) must match.
    if container.kind() == ElementKind::HasMethod {
        let container_info = *i.get_has_method(container);
        if let Some(intersections_id) = container_info.intersections {
            let head = i.intern_has_method(crate::element::payload::HasMethodInfo {
                method_name: container_info.method_name,
                intersections: None,
            });
            if !element_refines_via_type(input, head, world, options, report) {
                return false;
            }
            for &conjunct in i.get_element_list(intersections_id) {
                if !element_refines_via_type(input, conjunct, world, options, report) {
                    return false;
                }
            }
            return true;
        }
    }
    if container.kind() == ElementKind::HasProperty {
        let container_info = *i.get_has_property(container);
        if let Some(intersections_id) = container_info.intersections {
            let head = i.intern_has_property(crate::element::payload::HasPropertyInfo {
                property_name: container_info.property_name,
                intersections: None,
            });
            if !element_refines_via_type(input, head, world, options, report) {
                return false;
            }
            for &conjunct in i.get_element_list(intersections_id) {
                if !element_refines_via_type(input, conjunct, world, options, report) {
                    return false;
                }
            }
            return true;
        }
    }

    if input.kind() == ElementKind::Object {
        let input_info = *i.get_object(input);
        if let Some(intersections_id) = input_info.intersections {
            let head = i.intern_object(ObjectInfo { intersections: None, ..input_info });
            if element_refines_via_type(head, container, world, options, report) {
                return true;
            }

            for &conjunct in i.get_element_list(intersections_id) {
                if element_refines_via_type(conjunct, container, world, options, report) {
                    return true;
                }
            }
            return false;
        }
    }

    // Input-intersection rule for HasMethod / HasProperty: any conjunct
    // refining the container suffices.
    if input.kind() == ElementKind::HasMethod {
        let input_info = *i.get_has_method(input);
        if let Some(intersections_id) = input_info.intersections {
            let head = i.intern_has_method(crate::element::payload::HasMethodInfo {
                method_name: input_info.method_name,
                intersections: None,
            });
            if element_refines_via_type(head, container, world, options, report) {
                return true;
            }
            for &conjunct in i.get_element_list(intersections_id) {
                if element_refines_via_type(conjunct, container, world, options, report) {
                    return true;
                }
            }
            return false;
        }
    }
    if input.kind() == ElementKind::HasProperty {
        let input_info = *i.get_has_property(input);
        if let Some(intersections_id) = input_info.intersections {
            let head = i.intern_has_property(crate::element::payload::HasPropertyInfo {
                property_name: input_info.property_name,
                intersections: None,
            });
            if element_refines_via_type(head, container, world, options, report) {
                return true;
            }
            for &conjunct in i.get_element_list(intersections_id) {
                if element_refines_via_type(conjunct, container, world, options, report) {
                    return true;
                }
            }
            return false;
        }
    }

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

        (_, ElementKind::HasMethod) => {
            let container_info = i.get_has_method(container);
            refines_has_method(input, container_info.method_name, world)
        }

        (_, ElementKind::HasProperty) => {
            let container_info = i.get_has_property(container);
            refines_has_property(input, container_info.property_name, world)
        }

        (_, ElementKind::ObjectShape) => {
            let container_info = *i.get_object_shape(container);
            refines_object_shape(input, container_info, world, options, report)
        }

        _ => false,
    }
}

fn refines_has_method<W: World>(input: ElementId, method: Atom, world: &W) -> bool {
    let i = interner();
    match input.kind() {
        ElementKind::HasMethod => i.get_has_method(input).method_name == method,
        ElementKind::Object => world.class_has_method(i.get_object(input).name, method),
        ElementKind::Enum => world.class_has_method(i.get_enum(input).name, method),
        ElementKind::ObjectShape => true,
        _ => false,
    }
}

fn refines_has_property<W: World>(input: ElementId, property: Atom, world: &W) -> bool {
    let i = interner();
    match input.kind() {
        ElementKind::HasProperty => i.get_has_property(input).property_name == property,
        ElementKind::Object => world.class_property_type(i.get_object(input).name, property).is_some(),
        ElementKind::Enum => {
            let info = i.get_enum(input);
            enum_property_present(info.name, property, world)
        }
        ElementKind::ObjectShape => true,
        _ => false,
    }
}

/// Built-in enum properties: `name` is always present (any enum case has
/// one); `value` is present only on backed enums.
fn enum_property_present<W: World>(enum_name: Atom, property: Atom, world: &W) -> bool {
    if property == atom("name") {
        return true;
    }
    if property == atom("value") {
        return matches!(world.enum_backing(enum_name), Some(EnumBacking::Backed(_)));
    }
    false
}

fn refines_object_shape<W: World>(
    input: ElementId,
    container: ObjectShapeInfo,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let i = interner();
    match input.kind() {
        ElementKind::ObjectShape => {
            let input_info = *i.get_object_shape(input);
            shape_refines_shape(input_info, container, world, options, report)
        }
        ElementKind::Object => {
            let input_info = *i.get_object(input);
            named_refines_shape(input_info.name, container, world, options, report)
        }
        ElementKind::Enum => {
            let info = *i.get_enum(input);
            match build_enum_shape(info, world) {
                Some(shape) => shape_refines_shape(shape, container, world, options, report),
                None => false,
            }
        }
        _ => false,
    }
}

/// Synthesize the structural shape of an enum case: `name` is always a
/// `non-empty-string` (or the literal case name when narrowed to a
/// specific case), and `value` is the backing type for backed enums.
/// The shape is sealed because enum cases expose no other properties.
///
/// Returns `None` when the world doesn't know the enum's backing — the
/// caller treats that as "can't prove refinement" and rejects.
fn build_enum_shape<W: World>(info: EnumInfo, world: &W) -> Option<ObjectShapeInfo> {
    let i = interner();
    let backing = world.enum_backing(info.name)?;

    let name_type = match info.case {
        Some(case_name) => i.intern_type(&[ElementId::string_literal(case_name.as_str())], FlowFlags::EMPTY),
        None => i.intern_type(&[NON_EMPTY_STRING], FlowFlags::EMPTY),
    };

    let mut props = Vec::with_capacity(2);
    props.push(KnownPropertyEntry { name: atom("name"), value: name_type, optional: false });
    if let EnumBacking::Backed(value_type) = backing {
        props.push(KnownPropertyEntry { name: atom("value"), value: value_type, optional: false });
    }

    Some(ObjectShapeInfo {
        known_properties: Some(i.intern_known_properties(&props)),
        intersections: None,
        flags: ObjectShapeFlags::default().with_sealed(true),
    })
}

/// Shape-vs-shape rule from comparison.md §1.4.6, mirroring the keyed-
/// array rule: every required key in the container must be present
/// (required) in the input with a refining value, a sealed container
/// demands a sealed input, and the input may not introduce required keys
/// the container does not list when sealed.
fn shape_refines_shape<W: World>(
    input: ObjectShapeInfo,
    container: ObjectShapeInfo,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let i = interner();
    let in_props = input.known_properties.map(|id| i.get_known_properties(id)).unwrap_or(&[]);
    let out_props = container.known_properties.map(|id| i.get_known_properties(id)).unwrap_or(&[]);

    if container.flags.sealed() && !input.flags.sealed() {
        return false;
    }

    for out in out_props {
        match in_props.iter().find(|p| p.name == out.name) {
            Some(in_entry) => {
                if !out.optional && in_entry.optional {
                    return false;
                }
                if !type_refines(in_entry.value, out.value, world, options, report) {
                    return false;
                }
            }
            None => {
                if !out.optional {
                    return false;
                }
            }
        }
    }

    if container.flags.sealed() {
        for in_entry in in_props {
            if !out_props.iter().any(|p| p.name == in_entry.name) {
                return false;
            }
        }
    }

    true
}

/// `Named(C) <: object{p1: T1, p2: T2, ...}` iff `Γ` records every
/// required property `pi` on `C` (or an ancestor) with a declared type
/// that refines `Ti`. Optional container properties impose no
/// requirement when missing on `C`.
fn named_refines_shape<W: World>(
    class: mago_atom::Atom,
    container: ObjectShapeInfo,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let i = interner();
    let out_props = container.known_properties.map(|id| i.get_known_properties(id)).unwrap_or(&[]);

    for out in out_props {
        match world.class_property_type(class, out.name) {
            Some(declared) => {
                if !type_refines(declared, out.value, world, options, report) {
                    return false;
                }
            }
            None => {
                if !out.optional {
                    return false;
                }
            }
        }
    }

    true
}

fn element_refines_via_type<W: World>(
    input: ElementId,
    container: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let i = interner();
    let it = i.intern_type(&[input], FlowFlags::EMPTY);
    let ct = i.intern_type(&[container], FlowFlags::EMPTY);
    type_refines(it, ct, world, options, report)
}

fn refines_named_named<W: World>(
    input: ObjectInfo,
    container: ObjectInfo,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    if !world.descends_from(input.name, container.name) {
        return false;
    }

    if !modality_satisfied(input.flags, container.flags) {
        return false;
    }

    // TODO(algorithmic gap, tests/algorithmic_gaps.rs::gap_arity_zero_class_with_explicit_args_reduces_to_bare):
    // when the world declares zero template parameters for either
    // side but the atom carries explicit `type_args`, the args are
    // syntactically invalid and should be dropped (reducing to the
    // bare class). Today refines / overlaps / meet treat them
    // inconsistently. Canonicalising the atom at intern time is the
    // cleanest fix.

    let container_args: Vec<TypeId> = match container.type_args {
        Some(id) => interner().get_type_list(id).to_vec(),
        None => default_fill_template_args(container.name, world),
    };
    if container_args.is_empty() {
        return true;
    }

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
        let constraint =
            world.template_parameter_at(input_name, position).and_then(|p| p.upper_bound).unwrap_or(TYPE_MIXED);
        return Some(mark_default_filled(constraint));
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

/// Compare a single type-argument pair under the container parameter's
/// declared variance. A default-fill marker on either side bypasses the
/// check and records [`CoercionCauses::TEMPLATE_DEFAULT`] so the consumer
/// can warn about the unpinned position.
fn compare_with_variance<W: World>(
    input: TypeId,
    container: TypeId,
    variance: Variance,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    if container.flags().from_template_default() && !matches!(variance, Variance::Contravariant) {
        report.add_cause(CoercionCauses::TEMPLATE_DEFAULT);
        return true;
    }

    if input.flags().from_template_default() && matches!(variance, Variance::Contravariant) {
        report.add_cause(CoercionCauses::TEMPLATE_DEFAULT);
        return true;
    }

    match variance {
        Variance::Covariant => type_refines(input, container, world, options, report),
        Variance::Contravariant => type_refines(container, input, world, options, report),
        Variance::Invariant => {
            type_refines(input, container, world, options, report)
                && type_refines(container, input, world, options, report)
        }
    }
}

/// Stamp `ty` with [`FlowFlags::from_template_default`]. The flag rides
/// with the [`TypeId`] wherever it is later nested, so the variance check
/// sees it even several layers deep.
fn mark_default_filled(ty: TypeId) -> TypeId {
    if ty.flags().from_template_default() {
        return ty;
    }
    ty.with_flags(ty.flags().with_from_template_default(true))
}

/// Build a positional list of default-filled type-arguments for `class`,
/// one per declared template parameter. Each entry is the parameter's
/// upper bound (or `mixed`) stamped with
/// [`FlowFlags::from_template_default`]. Empty when `class` has no
/// declared templates.
fn default_fill_template_args<W: World>(class: Atom, world: &W) -> Vec<TypeId> {
    let arity = world.template_parameter_arity(class);
    (0..arity)
        .map(|position| {
            let constraint =
                world.template_parameter_at(class, position).and_then(|p| p.upper_bound).unwrap_or(TYPE_MIXED);
            mark_default_filled(constraint)
        })
        .collect()
}

/// `static<C>` accepts only `static` or `$this`; `$this<C>` accepts only
/// `$this`. A plain `Named(C)` refines neither, because the late-static
/// modality is a stronger guarantee than nominal identity. Inputs more
/// specific than the container's modality are accepted (`$this <: static`).
fn modality_satisfied(input: ObjectFlags, container: ObjectFlags) -> bool {
    if container.is_this() && !input.is_this() {
        return false;
    }
    if container.is_static() && !(input.is_static() || input.is_this()) {
        return false;
    }
    true
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
