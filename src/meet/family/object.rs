//! Compositional object intersection. Two paths:
//!
//! - Different classes (or one with no shared `name`): glue them as a
//!   single intersection-bearing object (`Foo & Bar`), choosing the
//!   canonical-smallest participant as the head.
//! - Same class, different generic arguments: merge args pointwise
//!   under the world-declared variance. Invariant args meet (must
//!   agree); covariant args meet; contravariant args join. If any
//!   invariant slot meets to `never`, the whole intersection is
//!   uninhabitable and we return `None`.

use crate::ElementId;
use crate::ElementKind;
use crate::FlowFlags;
use crate::TypeId;
use crate::TypeListId;
use crate::element::payload::ObjectInfo;
use crate::interner::interner;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::prelude::TYPE_NEVER;
use crate::world::Variance;
use crate::world::World;

pub(in crate::meet) fn compose_object_intersection<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Option<ElementId> {
    let i = interner();
    let a_info = *i.get_object(a);
    let b_info = *i.get_object(b);

    let mut participants: Vec<ElementId> = Vec::new();
    participants.push(i.intern_object(ObjectInfo { intersections: None, ..a_info }));
    if let Some(id) = a_info.intersections {
        participants.extend_from_slice(i.get_element_list(id));
    }

    participants.push(i.intern_object(ObjectInfo { intersections: None, ..b_info }));
    if let Some(id) = b_info.intersections {
        participants.extend_from_slice(i.get_element_list(id));
    }

    let merged = merge_same_class_participants(participants, world, options, report)?;

    let mut object_parts: Vec<ElementId> = Vec::new();
    let mut other_parts: Vec<ElementId> = Vec::new();
    for elem in merged {
        if elem.kind() == ElementKind::Object {
            object_parts.push(elem);
        } else {
            other_parts.push(elem);
        }
    }

    if !single_inheritance_consistent(&object_parts, world) {
        return None;
    }

    object_parts.sort();
    object_parts.dedup();
    other_parts.sort();
    other_parts.dedup();

    let head_elem = object_parts.remove(0);
    let head_info = *i.get_object(head_elem);
    let mut conjuncts = object_parts;
    conjuncts.extend(other_parts);
    let intersections = if conjuncts.is_empty() { None } else { Some(i.intern_element_list(&conjuncts)) };
    let result = i.intern_object(ObjectInfo { intersections, ..head_info });
    Some(result)
}

/// `Foo & Bar` is inhabitable when one descends from the other (so a
/// concrete subclass satisfies both) or they could conceivably share
/// an interface. The world only exposes class ancestry, so the
/// pragmatic rule is: every pair of distinct nominal classes in the
/// intersection must be ancestor-related. Otherwise the composition
/// is uninhabitable under PHP's single-inheritance class graph.
fn single_inheritance_consistent<W: World>(objects: &[ElementId], world: &W) -> bool {
    let i = interner();
    for (idx, &a) in objects.iter().enumerate() {
        for &b in &objects[idx + 1..] {
            let a_name = i.get_object(a).name;
            let b_name = i.get_object(b).name;
            if a_name == b_name {
                continue;
            }
            if !(world.descends_from(a_name, b_name) || world.descends_from(b_name, a_name)) {
                return false;
            }
        }
    }
    true
}

fn merge_same_class_participants<W: World>(
    participants: Vec<ElementId>,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Option<Vec<ElementId>> {
    let i = interner();
    let mut out: Vec<ElementId> = Vec::with_capacity(participants.len());

    for elem in participants {
        if elem.kind() != ElementKind::Object {
            out.push(elem);
            continue;
        }

        let info = *i.get_object(elem);
        let mut absorbed = false;
        for slot in out.iter_mut() {
            if slot.kind() != ElementKind::Object {
                continue;
            }

            let existing = *i.get_object(*slot);
            if existing.name != info.name {
                continue;
            }

            let merged_args = merge_args(existing, info, world, options, report)?;
            *slot = i.intern_object(ObjectInfo {
                name: info.name,
                type_args: merged_args,
                intersections: None,
                flags: info.flags,
            });
            absorbed = true;
            break;
        }

        if !absorbed {
            out.push(elem);
        }
    }

    Some(out)
}

fn merge_args<W: World>(
    a: ObjectInfo,
    b: ObjectInfo,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Option<Option<TypeListId>> {
    let i = interner();
    match (a.type_args, b.type_args) {
        (None, None) => Some(None),
        // Bare class on one side carries the implicit `<mixed,…>`
        // default; the args-bearing side is strictly more specific, so
        // adopt it directly.
        (Some(id), None) | (None, Some(id)) => Some(Some(id)),
        (Some(a_id), Some(b_id)) => {
            let a_args: Vec<TypeId> = i.get_type_list(a_id).to_vec();
            let b_args: Vec<TypeId> = i.get_type_list(b_id).to_vec();
            if a_args.len() != b_args.len() {
                let arity = world.template_parameter_arity(a.name);
                if a_args.len() == arity {
                    return Some(Some(a_id));
                }

                if b_args.len() == arity {
                    return Some(Some(b_id));
                }

                return Some(Some(a_id));
            }

            let mut merged: Vec<TypeId> = Vec::with_capacity(a_args.len());
            for (idx, (&a_arg, &b_arg)) in a_args.iter().zip(b_args.iter()).enumerate() {
                let variance =
                    world.template_parameter_at(a.name, idx).map(|t| t.variance).unwrap_or(Variance::Invariant);
                let arg = match variance {
                    Variance::Covariant => crate::meet::compute(a_arg, b_arg, world, options, report),
                    Variance::Invariant => {
                        // Invariant slots require args to be mutually
                        // refining (i.e. value-equal). Mere non-empty
                        // intersection isn't enough: `B<int|enum>` and
                        // `B<int>` admit no shared B-instances when T is
                        // pinned exactly.
                        let a_refines_b = crate::lattice::refines(a_arg, b_arg, world, options, report);
                        let b_refines_a = crate::lattice::refines(b_arg, a_arg, world, options, report);
                        if !a_refines_b || !b_refines_a {
                            return None;
                        }
                        a_arg
                    }
                    Variance::Contravariant => {
                        let mut elems: Vec<ElementId> = a_arg.as_ref().elements.to_vec();
                        elems.extend_from_slice(b_arg.as_ref().elements);
                        i.intern_type(&elems, FlowFlags::EMPTY)
                    }
                };

                if matches!(variance, Variance::Covariant) && arg == TYPE_NEVER {
                    return None;
                }

                merged.push(arg);
            }

            Some(Some(i.intern_type_list(&merged)))
        }
    }
}
