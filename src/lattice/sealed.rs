//! Sealed-class lattice rules: when a named class is declared sealed
//! (its set of direct inheritors is closed by the language engine), the
//! lattice can prove identities that open-world reasoning cannot reach.

use mago_atom::Atom;

use crate::ElementId;
use crate::ElementKind;
use crate::FlowFlags;
use crate::TypeId;
use crate::element::payload::ObjectFlags;
use crate::element::payload::ObjectInfo;
use crate::interner::interner;
use crate::lattice;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::prelude;
use crate::world::World;

/// The result of asking "what survives of `H`'s sealed cover after
/// these negation conjuncts filter out some inheritors?".
#[derive(Debug, Clone)]
pub(crate) enum SealedResidual {
    NotSealed,
    FullyCovered,
    Surviving(Vec<ElementId>),
}

const DEPTH_CAP: usize = 16;

pub(crate) fn compute_residual<W: World>(
    head: ElementId,
    negated_inners: &[TypeId],
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> SealedResidual {
    if head.kind() != ElementKind::Object {
        return SealedResidual::NotSealed;
    }

    let i = interner();
    let head_info = *i.get_object(head);
    let class_name = head_info.name;

    let Some(inheritors) = world.sealed_direct_inheritors(class_name) else {
        return SealedResidual::NotSealed;
    };

    let mut visited: Vec<Atom> = Vec::with_capacity(8);
    visited.push(class_name);

    compute_residual_impl(head, negated_inners, inheritors, world, options, report, &mut visited, 0)
}

#[inline]
fn compute_residual_impl<W: World>(
    head: ElementId,
    negated_inners: &[TypeId],
    inheritors: &[Atom],
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
    visited: &mut Vec<Atom>,
    depth: usize,
) -> SealedResidual {
    if depth > DEPTH_CAP {
        return SealedResidual::NotSealed;
    }

    let i = interner();
    let head_info = *i.get_object(head);
    let mut surviving: Vec<ElementId> = Vec::new();

    for &si in inheritors {
        let si_elem = build_inheritor_element(head_info, si, world);

        let covered = negated_inners.iter().any(|n| {
            let si_type = i.intern_type(&[si_elem], FlowFlags::EMPTY);
            lattice::refines(si_type, *n, world, options, report)
        });

        if covered {
            continue;
        }

        if let Some(grandchildren) = world.sealed_direct_inheritors(si) {
            if visited.contains(&si) {
                // A cycle in the sealing graph is unresolvable. Bail
                // up to the caller as `NotSealed` rather than emitting
                // self-referential survivors ; otherwise downstream
                // refines / overlaps consumers loop forever asking the
                // same question.
                return SealedResidual::NotSealed;
            }
            visited.push(si);

            #[allow(clippy::arithmetic_side_effects)]
            let sub = compute_residual_impl(
                si_elem,
                negated_inners,
                grandchildren,
                world,
                options,
                report,
                visited,
                depth + 1,
            );

            visited.pop();

            match sub {
                SealedResidual::FullyCovered => {}
                // Recursion bailed (cycle or depth cap). Don't try to
                // synthesize a partial cover ; propagate the give-up
                // signal so callers fall back to non-sealed reasoning
                // rather than looping on a self-referential survivor.
                SealedResidual::NotSealed => return SealedResidual::NotSealed,
                SealedResidual::Surviving(children) => surviving.extend(children),
            }
        } else {
            surviving.push(si_elem);
        }
    }

    if surviving.is_empty() { SealedResidual::FullyCovered } else { SealedResidual::Surviving(surviving) }
}

#[inline]
fn build_inheritor_element<W: World>(head_info: ObjectInfo, si: Atom, world: &W) -> ElementId {
    let i = interner();
    let arity = world.template_parameter_arity(si);

    let type_args = if let Some(type_args) = head_info.type_args
        && arity != 0
    {
        let head_args = i.get_type_list(type_args);
        let mut projected: Vec<TypeId> = Vec::with_capacity(arity);
        for pos in 0..arity {
            let arg = world
                .inherited_template_argument(si, head_info.name, pos)
                .unwrap_or_else(|| head_args.get(pos).copied().unwrap_or(prelude::TYPE_MIXED));
            projected.push(arg);
        }
        Some(i.intern_type_list(&projected))
    } else {
        None
    };

    let info = ObjectInfo { name: si, type_args, flags: ObjectFlags::default() };
    i.intern_object(info)
}
