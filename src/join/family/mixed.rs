//! Mixed-constraint joining: order-dependent state machine that
//! collapses any `Mixed` element with the surrounding union into a
//! single canonical mixed flavour.

use crate::ElementId;
use crate::ElementKind;
use crate::element::payload::MixedInfo;
use crate::element::payload::Truthiness;
use crate::element::payload::scalar::FloatInfo;
use crate::interner::interner;
use crate::predicates::element as pred;
use crate::prelude::NULL;

/// When any `Mixed` kind appears in the input, the result is a
/// single mixed element whose flavour is decided by walking the
/// input in original order:
///
/// - Vanilla `mixed` is the absorbing element: once seen, the result is
///   vanilla regardless of what follows.
/// - `truthy_mixed` / `falsy_mixed` / `nonnull_mixed` set their respective
///   flag if no contradiction has been seen yet (e.g. truthy seen after
///   any non-truthy non-mixed atom forces a generic mixed).
/// - Subsequent non-mixed atoms either strengthen the constraint (e.g.
///   truthy + truthy literal preserves truthy) or contradict it
///   (e.g. truthy + literal `"0"` collapses to nonnull).
///
/// Returns `None` when the input has no `Mixed` element (caller
/// proceeds with the regular join). Returns `Some(elem)` with a
/// single mixed element to emit.
pub(in crate::join) fn apply_mixed_constraint_join(elements: &[ElementId]) -> Option<ElementId> {
    let i = interner();

    let mut truthy: Option<bool> = None;
    let mut falsy: Option<bool> = None;
    let mut nonnull: Option<bool> = None;
    let mut generic = false;
    let mut has_mixed = false;
    let mut isset_from_loop: Option<bool> = None;

    for (idx, &el) in elements.iter().enumerate() {
        if el.kind() == ElementKind::Mixed {
            process_mixed(
                el,
                idx,
                elements,
                &mut truthy,
                &mut falsy,
                &mut nonnull,
                &mut generic,
                &mut has_mixed,
                &mut isset_from_loop,
            );
        } else {
            if generic {
                continue;
            }
            if falsy.unwrap_or(false) {
                if !pred::is_falsy(el) {
                    falsy = Some(false);
                    generic = true;
                }
                continue;
            }
            if truthy.unwrap_or(false) {
                if !pred::is_truthy(el) {
                    truthy = Some(false);
                    generic = true;
                }
                continue;
            }
            if nonnull.unwrap_or(false) && el == NULL {
                nonnull = Some(false);
                generic = true;
            }
        }
    }

    if !has_mixed {
        return None;
    }

    let final_nonnull = nonnull.unwrap_or(false);
    let final_truthy = truthy.unwrap_or(false);
    let final_falsy = falsy.unwrap_or(false);

    let truthiness = if final_truthy && !final_falsy {
        Truthiness::Truthy
    } else if final_falsy && !final_truthy {
        Truthiness::Falsy
    } else {
        Truthiness::Undetermined
    };

    // Truthy / falsy variants already encode their nullability
    // semantically; the explicit `is_non_null` flag only matters for
    // `non_null_mixed`.
    let info = match truthiness {
        Truthiness::Truthy => MixedInfo::EMPTY.with_truthiness(Truthiness::Truthy),
        Truthiness::Falsy => MixedInfo::EMPTY.with_truthiness(Truthiness::Falsy),
        Truthiness::Undetermined => MixedInfo::EMPTY.with_is_non_null(final_nonnull),
    };

    Some(i.intern_mixed(info))
}

#[allow(clippy::too_many_arguments)]
#[inline]
fn process_mixed(
    el: ElementId,
    idx: usize,
    elements: &[ElementId],
    truthy: &mut Option<bool>,
    falsy: &mut Option<bool>,
    nonnull: &mut Option<bool>,
    generic: &mut bool,
    has_mixed: &mut bool,
    isset_from_loop: &mut Option<bool>,
) {
    let info = *interner().get_mixed(el);

    if info.is_isset_from_loop() {
        if *generic {
            return;
        }
        if isset_from_loop.is_none() {
            *isset_from_loop = Some(true);
        }
        *has_mixed = true;
        return;
    }

    *has_mixed = true;

    let info_is_non_null = info.is_non_null() || info.truthiness() == Truthiness::Truthy;
    let is_vanilla = !info_is_non_null && !info.is_empty() && info.truthiness() == Truthiness::Undetermined;
    if is_vanilla {
        *falsy = Some(false);
        *truthy = Some(false);
        *isset_from_loop = Some(false);
        *generic = true;
        return;
    }

    if info.truthiness() == Truthiness::Truthy {
        if *generic {
            return;
        }
        *isset_from_loop = Some(false);

        if falsy.unwrap_or(false) {
            *falsy = Some(false);
            *generic = true;
            return;
        }

        if truthy.is_some() {
            return;
        }

        let has_non_truthy =
            elements_seen_so_far_any(elements, idx, |e| non_mixed_counts_for_truthy_check(e) && !pred::is_truthy(e));
        if has_non_truthy {
            *generic = true;
            return;
        }
        *truthy = Some(true);
    } else {
        *truthy = Some(false);
    }

    if info.truthiness() == Truthiness::Falsy {
        if *generic {
            return;
        }
        *isset_from_loop = Some(false);

        if truthy.unwrap_or(false) {
            *truthy = Some(false);
            *generic = true;
            return;
        }

        if falsy.is_some() {
            return;
        }

        let has_non_falsy =
            elements_seen_so_far_any(elements, idx, |e| non_mixed_counts_for_falsy_check(e) && !pred::is_falsy(e));
        if has_non_falsy {
            *generic = true;
            return;
        }
        *falsy = Some(true);
    } else {
        *falsy = Some(false);
    }

    if info_is_non_null {
        if *generic {
            return;
        }
        *isset_from_loop = Some(false);

        if elements_seen_so_far_any(elements, idx, |e| e == NULL) {
            *generic = true;
            return;
        }
        if falsy.unwrap_or(false) {
            *falsy = Some(false);
            *generic = true;
            return;
        }
        if nonnull.is_none() {
            *nonnull = Some(true);
        }
    } else {
        *nonnull = Some(false);
    }
}

/// Whether `el` (a non-mixed kind) counts as a value-types entry
/// that would contradict a `truthy_mixed` constraint. Integers and
/// float literals are excluded.
#[inline]
fn non_mixed_counts_for_truthy_check(el: ElementId) -> bool {
    match el.kind() {
        ElementKind::Int => false,
        ElementKind::Float => !matches!(*interner().get_float(el), FloatInfo::Literal(_)),
        _ => true,
    }
}

#[inline]
fn non_mixed_counts_for_falsy_check(el: ElementId) -> bool {
    non_mixed_counts_for_truthy_check(el)
}

#[inline]
fn elements_seen_so_far_any(elements: &[ElementId], upto: usize, predicate: impl Fn(ElementId) -> bool) -> bool {
    elements[..upto].iter().any(|&e| e.kind() != ElementKind::Mixed && predicate(e))
}
