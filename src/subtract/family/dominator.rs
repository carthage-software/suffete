//! True-union dominator subtract: split `scalar` / `numeric` /
//! `array-key` into their constituents when the right-hand side
//! lands inside one of the sub-families.

use crate::ElementId;
use crate::ElementKind;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::world::World;

/// Fan out a true-union dominator (`scalar`, `numeric`, `array-key`)
/// when the right-hand side is a member of one of its sub-families.
/// The dominator's value-set is the disjoint union of its members;
/// subtracting splits the dominator into its constituents and
/// delegates the in-family subtraction to the recursive call.
///
/// `scalar = bool | int | float | string`,
/// `numeric = int | float | numeric-string`,
/// `array-key = int | string`.
pub(in crate::subtract) fn true_union_minus<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Option<Vec<ElementId>> {
    use crate::prelude::ARRAY_KEY;
    use crate::prelude::BOOL;
    use crate::prelude::FLOAT;
    use crate::prelude::INT;
    use crate::prelude::NUMERIC;
    use crate::prelude::NUMERIC_STRING;
    use crate::prelude::SCALAR;
    use crate::prelude::STRING;

    let members: &[ElementId] = if a == SCALAR {
        &[BOOL, INT, FLOAT, STRING]
    } else if a == NUMERIC {
        &[INT, FLOAT, NUMERIC_STRING]
    } else if a == ARRAY_KEY {
        &[INT, STRING]
    } else {
        return None;
    };

    // Only fan out when `b` lands inside one of the sub-families.
    // Otherwise we'd needlessly re-emit the dominator's constituents
    // for an unrelated subtraction (e.g. `scalar \ Foo`).
    if !members.iter().any(|m| dominator_member_covers(*m, b)) {
        return None;
    }

    let mut pieces: Vec<ElementId> = Vec::with_capacity(members.len());
    for &m in members {
        for piece in crate::subtract::atom_minus(m, b, world, options, report) {
            pieces.push(piece);
        }
    }
    Some(pieces)
}

/// `true` iff member `m` and `b` share at least one runtime axis,
/// so splitting the dominator into its members lets the per-member
/// subtract drop or narrow some pieces. Same-axis pairs (`int \ int`)
/// and subsuming dominators (`array-key \ numeric` where `int` is
/// in both) both qualify.
#[inline]
const fn dominator_member_covers(m: ElementId, b: ElementId) -> bool {
    matches!(
        (m.kind(), b.kind()),
        (ElementKind::Bool, ElementKind::Bool | ElementKind::True | ElementKind::False)
            | (ElementKind::Int, ElementKind::Int)
            | (ElementKind::Float, ElementKind::Float)
            | (ElementKind::String, ElementKind::String | ElementKind::ClassLikeString)
            | (ElementKind::Int, ElementKind::Numeric | ElementKind::Scalar | ElementKind::ArrayKey)
            | (ElementKind::Float, ElementKind::Numeric | ElementKind::Scalar)
            | (ElementKind::Bool, ElementKind::Scalar)
            | (ElementKind::String, ElementKind::Numeric | ElementKind::Scalar | ElementKind::ArrayKey)
    )
}
