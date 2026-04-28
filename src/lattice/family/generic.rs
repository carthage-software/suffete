//! Generic-parameter family. Comparison rules for the case where the
//! *container* is a `T` (a `@template` reference).
//!
//! Rules from comparison.md §1.9:
//!
//! - **Same-T**: `T_a <: T_b` when both sides name the same parameter
//!   declared by the same defining entity AND `a`'s constraint refines
//!   `b`'s. Constraints can differ when one side has been narrowed by an
//!   earlier assertion (`T of (int|string)` narrowed to `T of int`); the
//!   narrower side refines the wider but not vice versa.
//! - **Inherited-T**: `T_C <: T_D` when `C` extends `D` and the parameter
//!   is transferred along the extension. Not yet supported by the world
//!   surface — recorded as a TODO when a relevant query lands.
//!
//! The dual rule (input is `T`, container is non-generic, refined through
//! `T`'s constraint) lives in [`crate::lattice::refines::element_refines`]
//! because it must fire before the container-kind dispatch.

use crate::ElementId;
use crate::ElementKind;
use crate::interner::interner;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::world::World;

pub fn refines<W: World>(
    input: ElementId,
    container: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    if container.kind() != ElementKind::GenericParameter {
        return false;
    }

    if input.kind() != ElementKind::GenericParameter {
        return false;
    }

    let i = interner();
    let input_info = i.get_generic_parameter(input);
    let container_info = i.get_generic_parameter(container);

    if input_info.name != container_info.name || input_info.defining_entity != container_info.defining_entity {
        return false;
    }

    crate::lattice::refines(input_info.constraint, container_info.constraint, world, options, report)
}
