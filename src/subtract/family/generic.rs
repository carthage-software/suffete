//! Generic-parameter subtract: narrow `T`'s constraint by removing
//! the right-hand side from its bound.

use crate::ElementId;
use crate::ElementKind;
use crate::FlowFlags;
use crate::TypeId;
use crate::element::payload::GenericParameterInfo;
use crate::interner::interner;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::prelude::TYPE_NEVER;
use crate::world::World;

/// `(T of X) \ Y`: narrow `T`'s constraint by removing `Y` from its
/// bound. When the new constraint is empty (every value of `T` was in
/// `Y`), the result is `[]` (impossible). When the same-`T` rule fires
/// (`(T of X) \ (T of Y) → T of (X \ Y)`), both sides agree on the
/// parameter identity. Otherwise the rhs is treated as a plain type
/// the constraint must shed.
pub(in crate::subtract) fn generic_parameter_minus<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Option<Vec<ElementId>> {
    let i = interner();
    let a_info = *i.get_generic_parameter(a);

    let other_constraint: TypeId = if b.kind() == ElementKind::GenericParameter {
        let b_info = *i.get_generic_parameter(b);
        if a_info.name != b_info.name || a_info.defining_entity != b_info.defining_entity {
            return None;
        }
        b_info.constraint
    } else {
        i.intern_type(&[b], FlowFlags::EMPTY)
    };

    let new_constraint = crate::subtract::compute(a_info.constraint, other_constraint, world, options, report);
    if new_constraint == TYPE_NEVER {
        return Some(Vec::new());
    }
    let narrowed = i.intern_generic_parameter(GenericParameterInfo { constraint: new_constraint, ..a_info });
    Some(vec![narrowed])
}
