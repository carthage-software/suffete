//! Iterable family: `iterable<K, V>` and intersected forms.
//!
//! `iterable` accepts:
//!
//! - other `iterable<K', V'>` when `K' <: K` and `V' <: V` (key + value
//!   covariance ; iterables are read-only at the type level so values are
//!   covariant; PHP doesn't model write positions on `iterable`)
//! - `list<E>` when `E <: V` and `array-key <: K` (lists key by `int <:
//!   array-key`)
//! - keyed arrays when their key/value parameters refine the container's
//! - empty array when the container's value side accepts nothing extra
//!   (vacuously true: empty has no entries)
//!
//! `iterable` does NOT accept generic `\Traversable` named-objects yet
//! because that requires world-driven hierarchy queries.

use crate::ElementId;
use crate::ElementKind;
use crate::TypeId;
use crate::interner::interner;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::lattice::refines::refines as type_refines_outer;
use crate::prelude::ARRAY_KEY;
use crate::prelude::EMPTY_ARRAY;
use crate::prelude::INT;
use crate::world::World;

#[inline]
pub fn refines<W: World>(
    input: ElementId,
    container: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let i = interner();
    let container_info = *i.get_iterable(container);

    // Empty array always fits an iterable container (no entries to violate
    // the value/key constraints).
    if input == EMPTY_ARRAY {
        return true;
    }

    match input.kind() {
        ElementKind::Iterable => {
            let input_info = *i.get_iterable(input);
            type_refines(input_info.key_type, container_info.key_type, world, options, report)
                && type_refines(input_info.value_type, container_info.value_type, world, options, report)
        }
        ElementKind::List => {
            let input_info = *i.get_list(input);
            let int_t = single_type(INT);
            type_refines(int_t, container_info.key_type, world, options, report)
                && type_refines(input_info.element_type, container_info.value_type, world, options, report)
        }
        ElementKind::Array => {
            let input_info = *i.get_array(input);
            let key = input_info.key_param.unwrap_or_else(|| single_type(ARRAY_KEY));
            let value = input_info.value_param.unwrap_or(container_info.value_type);
            type_refines(key, container_info.key_type, world, options, report)
                && type_refines(value, container_info.value_type, world, options, report)
        }
        _ => false,
    }
}

#[inline]
fn type_refines<W: World>(
    a: TypeId,
    b: TypeId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    type_refines_outer(a, b, world, options, report)
}

#[inline]
fn single_type(element: ElementId) -> TypeId {
    interner().intern_type(&[element], crate::FlowFlags::EMPTY)
}
