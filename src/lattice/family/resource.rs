//! Resource family: `resource`, `open-resource`, `closed-resource`.
//!
//! `open` and `closed` both refine `resource`. They are not subtypes of each
//! other (an open resource is not a closed one, and vice versa).

use crate::ElementId;
use crate::ElementKind;
use crate::element::payload::ResourceInfo;
use crate::interner::interner;

/// `Open <: Resource` and `Closed <: Resource`. Reflexivity is the
/// dispatcher's job.
pub fn refines(input: ElementId, container: ElementId) -> bool {
    if input.kind() != ElementKind::Resource {
        return false;
    }

    let i = interner();
    let container_info = i.get_resource(container);
    let input_info = i.get_resource(input);
    matches!((input_info, container_info), (ResourceInfo::Open | ResourceInfo::Closed, ResourceInfo::Any))
}
