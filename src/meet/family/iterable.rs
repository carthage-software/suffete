//! `Iterable` family meet: `iterable<K1, V1> ∧ iterable<K2, V2>` is
//! `iterable<K1 ∧ K2, V1 ∧ V2>` (both axes covariant).

use crate::ElementId;
use crate::element::payload::IterableInfo;
use crate::interner::interner;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::world::NullWorld;

pub(in crate::meet) fn iterable_meet(a: ElementId, b: ElementId) -> Option<ElementId> {
    let i = interner();
    let a_info = *i.get_iterable(a);
    let b_info = *i.get_iterable(b);

    let mut report = LatticeReport::new();
    let key_type =
        crate::meet::compute(a_info.key_type, b_info.key_type, &NullWorld, LatticeOptions::default(), &mut report);
    let value_type =
        crate::meet::compute(a_info.value_type, b_info.value_type, &NullWorld, LatticeOptions::default(), &mut report);

    let merged = IterableInfo { key_type, value_type, intersections: None };
    Some(i.intern_iterable(merged))
}
