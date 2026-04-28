//! Generic-parameter family meet: narrow `T`'s constraint when an
//! assertion eliminates part of its declared bound.
//!
//! `T of X ∧ Y` is a fresh `T` whose constraint is `X ∩ Y` (computed
//! recursively via the lattice meet). When the narrowed constraint is
//! empty, the result is `None` (impossible — no value of T can satisfy
//! both the original bound and the assertion). When `T of X` already
//! refines `Y`, the subsumption rule in [`crate::meet`] short-circuits
//! before we get here, so this rule fires only for genuine narrowings.
//!
//! Same-`T` meets (`T of X ∧ T of Y`) intersect both constraints, since
//! both sides describe the same parameter under different bounds.

use crate::ElementId;
use crate::ElementKind;
use crate::element::payload::GenericParameterInfo;
use crate::interner::interner;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::world::World;

pub(in crate::meet) fn generic_parameter_meet<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Option<ElementId> {
    let i = interner();
    let (template, other_constraint) = match (a.kind(), b.kind()) {
        (ElementKind::GenericParameter, ElementKind::GenericParameter) => {
            let a_info = *i.get_generic_parameter(a);
            let b_info = *i.get_generic_parameter(b);
            if a_info.name != b_info.name || a_info.defining_entity != b_info.defining_entity {
                return None;
            }
            (a_info, b_info.constraint)
        }
        (ElementKind::GenericParameter, _) => {
            let a_info = *i.get_generic_parameter(a);
            let b_t = i.intern_type(&[b], crate::FlowFlags::EMPTY);
            (a_info, b_t)
        }
        (_, ElementKind::GenericParameter) => {
            let b_info = *i.get_generic_parameter(b);
            let a_t = i.intern_type(&[a], crate::FlowFlags::EMPTY);
            (b_info, a_t)
        }
        _ => return None,
    };

    let new_constraint = crate::meet::compute(template.constraint, other_constraint, world, options, report);
    if new_constraint == crate::prelude::TYPE_NEVER {
        return None;
    }
    Some(i.intern_generic_parameter(GenericParameterInfo { constraint: new_constraint, ..template }))
}
