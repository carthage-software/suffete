//! `Iterable` family meet: `iterable<K1, V1> ∧ iterable<K2, V2>` is
//! `iterable<K1 ∧ K2, V1 ∧ V2>` (both axes covariant).

use crate::ElementId;
use crate::element::payload::IterableInfo;
use crate::interner::interner;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::world::World;

pub(in crate::meet) fn iterable_meet<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Option<ElementId> {
    let i = interner();
    let a_info = *i.get_iterable(a);
    let b_info = *i.get_iterable(b);

    let key_type = crate::meet::compute(a_info.key_type, b_info.key_type, world, options, report);
    let value_type = crate::meet::compute(a_info.value_type, b_info.value_type, world, options, report);

    let intersections = match (a_info.intersections, b_info.intersections) {
        (None, None) => None,
        (Some(id), None) | (None, Some(id)) => Some(id),
        (Some(a_id), Some(b_id)) if a_id == b_id => Some(a_id),
        (Some(a_id), Some(b_id)) => {
            let mut merged: Vec<ElementId> = i.get_element_list(a_id).to_vec();
            merged.extend_from_slice(i.get_element_list(b_id));
            merged.sort_unstable();
            merged.dedup();
            Some(i.intern_element_list(&merged))
        }
    };

    Some(i.intern_iterable(IterableInfo { key_type, value_type, intersections }))
}
