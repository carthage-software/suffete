//! Bound reconciliation: pick the relevant bounds for a template
//! parameter from a [`TemplateState`] and union them into the
//! parameter's *witness*: the type the parameter resolves to in the
//! current call context.
//!
//! # Algorithm
//!
//! Given the bounds collected for a single template parameter, sorted
//! by ascending appearance depth (`d`):
//!
//! 1. The *baseline depth* `d_0` is the depth of the shallowest bound.
//! 2. Bounds at depth `d_0` are always relevant.
//! 3. A deeper bound (`d > d_0`) is included **only** when:
//!    - some bound seen so far carried the equality marker
//!      (i.e. came through an invariant generic position), **and**
//!    - the deeper bound's argument offset matches the baseline's
//!      offset.
//!
//! The relevant bounds' types are then unioned via [`crate::join`].
//! When no bounds were collected, the materialisation falls back to
//! the parameter's constraint (`κ(T)`).
//!
//! # Equality marker (current first-cut approximation)
//!
//! `equality_marker` is "set when the bound was collected through an
//! invariant generic position". We approximate that by treating
//! [`BoundKind::Equality`] as the marker: sound when the invariant
//! position is the position itself, but precision-loose when
//! invariance happens *higher* in the structural walk and the deeper
//! bound is recorded at a covariant position. Tightening this
//! requires propagating a marker flag through the standin walk; left
//! as a TODO.

use crate::TypeId;

use super::standin::Bound;
use super::standin::BoundKind;
use super::standin::TemplateKey;
use super::standin::TemplateState;

/// Run depth-based selection on a list of bounds and return the
/// unioned witness type. Returns `None` when the bound list is
/// empty so the caller can fall back to the parameter's constraint.
///
/// `bounds` is taken as a slice; ordering doesn't matter; this
/// function sorts internally.
#[inline]
#[must_use]
pub fn reconcile(bounds: &[Bound]) -> Option<TypeId> {
    if bounds.is_empty() {
        return None;
    }

    let mut sorted: Vec<Bound> = bounds.to_vec();
    sorted.sort_by_key(|b| b.depth);

    let baseline_depth = sorted[0].depth;
    let baseline_offset = sorted[0].argument_offset;

    let mut seen_equality = false;
    let mut relevant: Vec<TypeId> = Vec::new();

    for bound in sorted {
        if bound.depth == baseline_depth {
            if matches!(bound.kind, BoundKind::Equality) {
                seen_equality = true;
            }
            relevant.push(bound.ty);
            continue;
        }

        // Deeper-than-baseline bound.
        if seen_equality && bound.argument_offset == baseline_offset {
            if matches!(bound.kind, BoundKind::Equality) {
                // Marker stays sticky for any further bounds.
                seen_equality = true;
            }
            relevant.push(bound.ty);
        }
        // Otherwise discard.
    }

    let mut elements = Vec::new();
    for ty in relevant {
        elements.extend_from_slice(ty.as_ref().elements);
    }
    Some(TypeId::union(&elements))
}

impl TemplateState {
    /// Materialise `key`'s witness from its collected bounds. Returns
    /// `fallback` (typically the parameter's constraint or `mixed`)
    /// when no bound was recorded.
    #[inline]
    #[must_use]
    pub fn witness(&self, key: TemplateKey, fallback: TypeId) -> TypeId {
        reconcile(self.bounds_for(key)).unwrap_or(fallback)
    }
}
