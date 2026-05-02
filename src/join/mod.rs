#![allow(clippy::arithmetic_side_effects)]

//! Lattice join (least upper bound) of element multisets.
//!
//! [`compute`] takes a slice of [`ElementId`]s and returns the canonical
//! multiset that the corresponding union should hold. The pass is purely
//! structural: it inspects element identity and kind tags only, never
//! consults the lattice machinery, and so can run without any
//! subtype-driven information.
//!
//! In type-lattice terms, `compute(elements)` is the least upper bound
//! (join, ⊔) of the element multiset under the suffete subtype order.
//! Pairs with [`crate::meet`] (greatest lower bound, ⊓).
//!
//! # Why join is separate from interning
//!
//! The join preserves the subtype order. For any unions `A`, `B`:
//!
//! ```text
//! A ≤ B  ⟺  compute(A) ≤ B  ⟺  compute(A) ≤ compute(B)  ⟺  A ≤ compute(B)
//! ```
//!
//! That property is what lets the interner store unions in whatever shape
//! the caller hands in (sorted + deduplicated, but not otherwise canonical),
//! and the lattice answer refinement questions correctly on either side.
//! Calling [`compute`] is therefore an optional optimization for size and
//! readability, never a precondition for soundness.
//!
//! # What this pass does
//!
//! - Drops `never` when any non-`never` element exists; collapses an
//!   all-`never` multiset to `[never]`. Collapses `void | null` to `null`.
//! - Lets vanilla `mixed` absorb every other element.
//! - Merges `true ∨ false → bool`; lets `bool` absorb `true` / `false`.
//! - Lets `resource` absorb `open-resource` / `closed-resource`; merges
//!   `open-resource ∨ closed-resource → resource` when neither is dominated.
//! - Lets a same-kind dominator (`int`, `float`, `string`, `resource`,
//!   `callable`) absorb every other element of its kind.
//! - Lets `object` absorb the entire object family (named objects, enums,
//!   shapes, has-method, has-property).
//!
//! Family-specific payload merges (range merging, string-axis merging,
//! list / keyed-array element-type unions, scalar synthesis, mixed
//! constraint joining, literal-count and shape-count thresholds) live
//! in [`family`] and are gated per [`JoinOptions`] toggle.

mod family;

use crate::ElementId;
use crate::ElementKind;
use crate::lattice::CoercionCauses;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::lattice::element_refines;
use crate::prelude::BOOL;
use crate::prelude::CALLABLE;
use crate::prelude::CLOSED_RESOURCE;
use crate::prelude::FALSE;
use crate::prelude::FLOAT;
use crate::prelude::INT;
use crate::prelude::MIXED;
use crate::prelude::NEVER;
use crate::prelude::NULL;
use crate::prelude::OBJECT;
use crate::prelude::OPEN_RESOURCE;
use crate::prelude::RESOURCE;
use crate::prelude::STRING;
use crate::prelude::TRUE;
use crate::prelude::VOID;
use crate::world::NullWorld;

/// Compute the join (least upper bound) of a slice of elements with the
/// canonical preset.
///
/// [`JoinOptions::default`] enables payload-level merges (range merging,
/// string-axis merging, scalar synthesis, list and keyed-array
/// element-type unions), subtype-driven absorption, and the standard
/// 128 / 32 literal/shape thresholds. Use [`compute_with`] with
/// [`JoinOptions::structural`] for sort + dedup only.
///
/// Returns a freshly-allocated, sorted, deduplicated [`Vec`]. Empty
/// input collapses to `[NEVER]` so callers always receive a non-empty
/// multiset suitable for [`Type`] construction.
///
/// [`Type`]: crate::Type
#[inline]
#[must_use]
pub fn compute(elements: &[ElementId]) -> Vec<ElementId> {
    compute_with(elements, &JoinOptions::default())
}

/// Compute the join with caller-controlled extended rules per
/// [`JoinOptions`].
///
/// The structural pass runs first; each extended rule fires after,
/// gated on its own option, so the analyzer can pick the simplification
/// aggressiveness per call site.
#[inline]
#[must_use]
pub fn compute_with(elements: &[ElementId], options: &JoinOptions) -> Vec<ElementId> {
    if elements.is_empty() {
        return vec![NEVER];
    }

    if elements.iter().any(|e| e.kind() == ElementKind::Mixed)
        && let Some(mixed_result) = family::mixed::apply_mixed_constraint_join(elements)
    {
        return vec![mixed_result];
    }

    let mut out: Vec<ElementId> = if options.merge_string_axes
        && elements.iter().filter(|e| e.kind() == ElementKind::String).take(2).count() >= 2
    {
        family::string::apply_string_axis_merge_in_order(elements)
    } else {
        elements.to_vec()
    };
    out.sort_unstable();
    out.dedup();
    canonicalize(&mut out);

    if options.overwrite_empty_array {
        family::array::apply_overwrite_empty_array(&mut out);
    }
    if let Some(t) = options.int_literal_collapse_threshold {
        family::int::apply_int_literal_collapse(&mut out, t);
    }
    if let Some(t) = options.string_literal_collapse_threshold {
        family::string::apply_string_literal_collapse(&mut out, t);
    }
    if let Some(t) = options.float_literal_collapse_threshold {
        family::float::apply_float_literal_collapse(&mut out, t);
    }
    if let Some(t) = options.array_shape_collapse_threshold {
        family::array::apply_array_shape_collapse(&mut out, t);
    }
    if options.merge_int_ranges {
        family::int::apply_merge_int_ranges(&mut out);
    }
    if options.absorb_refinements {
        apply_subtype_absorption(&mut out);
    }
    if options.synthesise_scalar {
        family::scalar::apply_scalar_synthesis(&mut out);
    }
    if options.merge_array_shapes {
        family::array::apply_merge_array_shapes(&mut out);
    }
    if options.merge_list_element_types {
        family::list::apply_merge_list_element_types(&mut out);
    }
    if options.merge_keyed_array_params {
        family::array::apply_merge_keyed_array_params(&mut out);
    }
    if options.rewrite_int_keyed_to_list {
        family::array::apply_rewrite_int_keyed_to_list(&mut out);
    }

    out.sort_unstable();
    out.dedup();
    out
}

/// Caller-controlled toggles for [`compute_with`].
///
/// [`Default`] returns the canonical preset: every payload-level
/// merge rule on, with the standard 128-literal / 32-array
/// thresholds, so a plain [`compute`] call gives the lattice-canonical
/// form. Use [`JoinOptions::structural`] when you want a single rule
/// in isolation (typical for option-coverage tests) or to skip the
/// payload-level work for callers that only need sort + dedup +
/// same-kind dominator.
#[derive(Debug, Clone, Copy)]
pub struct JoinOptions {
    /// Merge adjacent integer literals and ranges into wider ranges
    /// (e.g. `0 | 1 | 2` → `int<0, 2>`). Touches Int-kind atoms only.
    pub merge_int_ranges: bool,
    /// When the union contains more than this many integer literals,
    /// drop them and add the general `int` form. `None` disables;
    /// `Some(0)` always collapses if any literals are present.
    pub int_literal_collapse_threshold: Option<u16>,
    /// When the union contains more than this many `string` literals,
    /// drop them and add the general `string` form.
    pub string_literal_collapse_threshold: Option<u16>,
    /// When the union contains more than this many `float` literals,
    /// drop them and add the general `float` form.
    pub float_literal_collapse_threshold: Option<u16>,
    /// When the union contains more than this many array shapes (keyed
    /// or list), collapse them to the general `array` form.
    pub array_shape_collapse_threshold: Option<u16>,
    /// Detect keyed-array shapes whose keys are `0..n-1` integers and
    /// rewrite them as `list` shapes.
    pub rewrite_int_keyed_to_list: bool,
    /// Merge multiple keyed-array shapes that share at least one key
    /// into a single shape with per-key value unions.
    pub merge_array_shapes: bool,
    /// Drop `EMPTY_ARRAY` from the union when another `Array` or `List`
    /// atom is present.
    pub overwrite_empty_array: bool,
    /// Apply subtype-driven absorption (refined int ranges, refined
    /// string axes, family hierarchy: numeric/scalar/array-key).
    pub absorb_refinements: bool,
    /// Merge same-kind strings via the AND-of-flags algebra (e.g.
    /// `lower | upper → string`, `non_empty | lit("") → string`,
    /// `truthy | lit("0") → string`). Compatible literals are absorbed
    /// into the merged refined form; incompatible literals stay separate.
    pub merge_string_axes: bool,
    /// Collapse `int | string | float | bool` to `scalar` once all four
    /// general primitives are present in the union.
    pub synthesise_scalar: bool,
    /// Merge multiple unsealed lists with the same non-empty flag into
    /// a single list whose element type is the union of theirs (e.g.
    /// `list<int> | list<string> → list<int|string>`).
    pub merge_list_element_types: bool,
    /// Same merge for unsealed keyed arrays (`array<K1, V1> | array<K2, V2>
    /// → array<K1|K2, V1|V2>`).
    pub merge_keyed_array_params: bool,
}

impl Default for JoinOptions {
    /// The canonical preset: every payload-level merge / absorption rule
    /// enabled, standard literal thresholds (128 ints / 128 strings /
    /// 128 floats / 32 array shapes), `overwrite_empty_array` and
    /// `rewrite_int_keyed_to_list` left off (they change the *shape* of
    /// the output, not just collapse equivalent forms, so they remain
    /// opt-in).
    #[inline]
    fn default() -> Self {
        Self {
            merge_int_ranges: true,
            int_literal_collapse_threshold: Some(128),
            string_literal_collapse_threshold: Some(128),
            float_literal_collapse_threshold: Some(128),
            array_shape_collapse_threshold: Some(32),
            rewrite_int_keyed_to_list: false,
            merge_array_shapes: true,
            overwrite_empty_array: false,
            absorb_refinements: true,
            merge_string_axes: true,
            synthesise_scalar: true,
            merge_list_element_types: true,
            merge_keyed_array_params: true,
        }
    }
}

impl JoinOptions {
    /// All payload-level rules off, all thresholds disabled. The
    /// resulting [`compute_with`] call performs only the structural
    /// canonicalisation (sort, dedup, same-kind dominator absorption,
    /// `void | null → null`, `true | false → bool`). Useful for testing
    /// a single rule in isolation.
    #[inline]
    #[must_use]
    pub const fn structural() -> Self {
        Self {
            merge_int_ranges: false,
            int_literal_collapse_threshold: None,
            string_literal_collapse_threshold: None,
            float_literal_collapse_threshold: None,
            array_shape_collapse_threshold: None,
            rewrite_int_keyed_to_list: false,
            merge_array_shapes: false,
            overwrite_empty_array: false,
            absorb_refinements: false,
            merge_string_axes: false,
            synthesise_scalar: false,
            merge_list_element_types: false,
            merge_keyed_array_params: false,
        }
    }

    #[must_use]
    #[inline]
    pub const fn with_merge_int_ranges(mut self, on: bool) -> Self {
        self.merge_int_ranges = on;
        self
    }

    #[must_use]
    #[inline]
    pub const fn with_int_literal_collapse_threshold(mut self, threshold: u16) -> Self {
        self.int_literal_collapse_threshold = Some(threshold);
        self
    }

    #[must_use]
    #[inline]
    pub const fn with_string_literal_collapse_threshold(mut self, threshold: u16) -> Self {
        self.string_literal_collapse_threshold = Some(threshold);
        self
    }

    #[must_use]
    #[inline]
    pub const fn with_float_literal_collapse_threshold(mut self, threshold: u16) -> Self {
        self.float_literal_collapse_threshold = Some(threshold);
        self
    }

    #[must_use]
    #[inline]
    pub const fn with_array_shape_collapse_threshold(mut self, threshold: u16) -> Self {
        self.array_shape_collapse_threshold = Some(threshold);
        self
    }

    #[must_use]
    #[inline]
    pub const fn with_rewrite_int_keyed_to_list(mut self, on: bool) -> Self {
        self.rewrite_int_keyed_to_list = on;
        self
    }

    #[must_use]
    #[inline]
    pub const fn with_merge_array_shapes(mut self, on: bool) -> Self {
        self.merge_array_shapes = on;
        self
    }

    #[must_use]
    #[inline]
    pub const fn with_overwrite_empty_array(mut self, on: bool) -> Self {
        self.overwrite_empty_array = on;
        self
    }

    #[must_use]
    #[inline]
    pub const fn with_absorb_refinements(mut self, on: bool) -> Self {
        self.absorb_refinements = on;
        self
    }

    #[must_use]
    #[inline]
    pub const fn with_merge_string_axes(mut self, on: bool) -> Self {
        self.merge_string_axes = on;
        self
    }

    #[must_use]
    #[inline]
    pub const fn with_synthesise_scalar(mut self, on: bool) -> Self {
        self.synthesise_scalar = on;
        self
    }

    #[must_use]
    #[inline]
    pub const fn with_merge_list_element_types(mut self, on: bool) -> Self {
        self.merge_list_element_types = on;
        self
    }

    #[must_use]
    #[inline]
    pub const fn with_merge_keyed_array_params(mut self, on: bool) -> Self {
        self.merge_keyed_array_params = on;
        self
    }
}

/// Drop any element absorbed by another structurally-larger element in
/// the same multiset (`a <: b` ⇒ drop `a`). Uses the lattice's element
/// refinement check with [`NullWorld`], so only purely-structural rules
/// fire. Coercion-driven refinements (e.g. `int <: float` via PHP's
/// runtime int-to-float coercion) do **not** drive absorption: keeping
/// `int|float` distinct preserves the information that the value was
/// originally typed as `int`.
#[inline]
fn apply_subtype_absorption(elements: &mut Vec<ElementId>) {
    if elements.len() < 2 {
        return;
    }

    let world = NullWorld;
    let opts = LatticeOptions::default();
    let mut absorbed = vec![false; elements.len()];

    for i in 0..elements.len() {
        if absorbed[i] {
            continue;
        }

        for j in 0..elements.len() {
            if i == j || absorbed[j] {
                continue;
            }
            let mut report = LatticeReport::new();
            if element_refines(elements[i], elements[j], &world, opts, &mut report)
                && !report.causes.contains(CoercionCauses::PHP_RUNTIME_COERCE)
            {
                absorbed[i] = true;
                break;
            }
        }
    }

    let mut idx = 0;
    elements.retain(|_| {
        let keep = !absorbed[idx];
        idx += 1;
        keep
    });
}

/// Apply the structural canonicalization rules. `elements` must be sorted
/// and deduplicated on entry; sorted order is preserved on exit.
#[inline]
fn canonicalize(elements: &mut Vec<ElementId>) {
    if elements.contains(&MIXED) {
        elements.clear();
        elements.push(MIXED);
        return;
    }

    let has_non_never = elements.iter().any(|e| *e != NEVER);
    if has_non_never {
        elements.retain(|e| *e != NEVER);
    }

    if elements.contains(&VOID) && elements.contains(&NULL) {
        elements.retain(|e| *e != VOID);
    }

    let has_bool = elements.contains(&BOOL);
    let has_true = elements.contains(&TRUE);
    let has_false = elements.contains(&FALSE);

    #[allow(clippy::else_if_without_else)]
    if has_bool {
        elements.retain(|e| *e != TRUE && *e != FALSE);
    } else if has_true && has_false {
        elements.retain(|e| *e != TRUE && *e != FALSE);
        let pos = elements.binary_search(&BOOL).unwrap_or_else(|p| p);
        elements.insert(pos, BOOL);
    }

    let has_open_resource = elements.contains(&OPEN_RESOURCE);
    let has_closed_resource = elements.contains(&CLOSED_RESOURCE);
    let has_resource = elements.contains(&RESOURCE);
    if has_open_resource && has_closed_resource && !has_resource {
        elements.retain(|e| *e != OPEN_RESOURCE && *e != CLOSED_RESOURCE);
        let pos = elements.binary_search(&RESOURCE).unwrap_or_else(|p| p);
        elements.insert(pos, RESOURCE);
    }

    apply_same_kind_dominator(elements, INT);
    apply_same_kind_dominator(elements, FLOAT);
    apply_same_kind_dominator(elements, STRING);
    apply_same_kind_dominator(elements, RESOURCE);
    apply_same_kind_dominator(elements, CALLABLE);

    if elements.contains(&OBJECT) {
        elements.retain(|e| *e == OBJECT || !is_object_family_kind(e.kind()));
    }
}

/// If `dominator` is in `elements`, drop every other element of the same
/// kind (the dominator is the unrefined / top-of-its-family form).
#[inline]
fn apply_same_kind_dominator(elements: &mut Vec<ElementId>, dominator: ElementId) {
    if !elements.contains(&dominator) {
        return;
    }

    let kind = dominator.kind();
    elements.retain(|e| *e == dominator || e.kind() != kind);
}

/// `true` for the kinds that all sit under `Object::Any` and are absorbed by
/// it: named objects, enums (including specific cases), object shapes,
/// has-method / has-property narrowings.
#[inline]
const fn is_object_family_kind(kind: ElementKind) -> bool {
    matches!(
        kind,
        ElementKind::Object
            | ElementKind::Enum
            | ElementKind::ObjectShape
            | ElementKind::HasMethod
            | ElementKind::HasProperty
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FlowFlags;
    use crate::TypeId;
    use crate::interner::interner;
    use crate::prelude::ARRAY_KEY;
    use crate::prelude::TYPE_BOOL;
    use crate::prelude::TYPE_INT_OR_STRING;
    use crate::prelude::TYPE_MIXED;

    #[test]
    #[inline]
    fn empty_yields_never() {
        assert_eq!(compute(&[]), vec![NEVER]);
    }

    #[test]
    #[inline]
    fn sorts_and_dedupes() {
        // Use the structural-only preset so the merge passes don't
        // collapse the adjacent literals into a range.
        let a = ElementId::int_literal(99);
        let b = ElementId::int_literal(100);
        let opts = JoinOptions::structural();
        let r1 = compute_with(&[a, b], &opts);
        let r2 = compute_with(&[b, a], &opts);
        let r3 = compute_with(&[a, b, a, b, a], &opts);
        assert_eq!(r1, r2);
        assert_eq!(r1, r3);
        assert_eq!(r1.len(), 2);
    }

    #[test]
    #[inline]
    fn never_is_dropped_when_other_elements_exist() {
        assert_eq!(compute(&[NEVER, INT]), vec![INT]);
    }

    #[test]
    #[inline]
    fn never_alone_is_preserved() {
        assert_eq!(compute(&[NEVER]), vec![NEVER]);
    }

    #[test]
    #[inline]
    fn void_alone_is_preserved() {
        assert_eq!(compute(&[VOID]), vec![VOID]);
    }

    #[test]
    #[inline]
    fn void_is_kept_when_other_elements_exist() {
        let mut out = compute(&[VOID, INT]);
        out.sort();
        let mut expected = vec![INT, VOID];
        expected.sort();
        assert_eq!(out, expected);
    }

    #[test]
    #[inline]
    fn void_and_never_together_keeps_void() {
        assert_eq!(compute(&[VOID, NEVER]), vec![VOID]);
    }

    #[test]
    #[inline]
    fn true_or_false_merges_to_bool() {
        assert_eq!(compute(&[TRUE, FALSE]), vec![BOOL]);
    }

    #[test]
    #[inline]
    fn bool_absorbs_true_and_false() {
        assert_eq!(compute(&[BOOL, TRUE]), vec![BOOL]);
        assert_eq!(compute(&[BOOL, FALSE]), vec![BOOL]);
        assert_eq!(compute(&[BOOL, TRUE, FALSE]), vec![BOOL]);
    }

    #[test]
    #[inline]
    fn vanilla_mixed_absorbs_everything_else() {
        assert_eq!(compute(&[MIXED, INT, STRING, NEVER]), vec![MIXED]);
    }

    #[test]
    #[inline]
    fn open_or_closed_resource_merges_to_resource() {
        assert_eq!(compute(&[OPEN_RESOURCE, CLOSED_RESOURCE]), vec![RESOURCE]);
    }

    #[test]
    #[inline]
    fn resource_absorbs_open_and_closed() {
        assert_eq!(compute(&[RESOURCE, OPEN_RESOURCE]), vec![RESOURCE]);
        assert_eq!(compute(&[RESOURCE, CLOSED_RESOURCE]), vec![RESOURCE]);
        assert_eq!(compute(&[RESOURCE, OPEN_RESOURCE, CLOSED_RESOURCE]), vec![RESOURCE]);
    }

    #[test]
    #[inline]
    fn unrelated_elements_are_preserved() {
        let mut out = compute(&[INT, STRING]);
        out.sort();
        let mut expected = vec![INT, STRING];
        expected.sort();
        assert_eq!(out, expected);
    }

    #[test]
    #[inline]
    fn null_and_array_key_kept_separate() {
        let mut out = compute(&[NULL, ARRAY_KEY]);
        out.sort();
        let mut expected = vec![NULL, ARRAY_KEY];
        expected.sort();
        assert_eq!(out, expected);
    }

    #[test]
    #[inline]
    fn type_id_union_does_not_apply_join_rules() {
        // `TypeId::union` only sort+dedups via the interner; it does
        // not run the merges in `join::compute`. Callers wanting the
        // collapsed form route through `join::compute` explicitly.
        let pair = TypeId::union(&[TRUE, FALSE]);
        assert_ne!(pair, TYPE_BOOL);
        assert_eq!(pair.as_ref().elements, &[TRUE, FALSE]);

        let with_mixed = TypeId::union(&[MIXED, INT, STRING]);
        assert_ne!(with_mixed, TYPE_MIXED);
        assert_eq!(with_mixed.as_ref().elements.len(), 3);

        // Sort+dedup still happens, so unions of distinct elements
        // canonical to the well-known handle when slot order matches.
        let int_or_string = TypeId::union(&[INT, STRING]);
        assert_eq!(int_or_string, TYPE_INT_OR_STRING);
    }

    #[test]
    #[inline]
    fn join_compute_then_union_collapses_to_well_known_handles() {
        let collapsed_bool = TypeId::union(&compute(&[TRUE, FALSE]));
        assert_eq!(collapsed_bool, TYPE_BOOL);

        let collapsed_mixed = TypeId::union(&compute(&[MIXED, INT, STRING]));
        assert_eq!(collapsed_mixed, TYPE_MIXED);
    }

    #[test]
    #[inline]
    fn intern_type_does_not_canonicalize() {
        let i = interner();
        let raw = i.intern_type(&[TRUE, FALSE], FlowFlags::EMPTY);
        assert_eq!(raw.as_ref().elements, &[TRUE, FALSE]);
        assert_ne!(raw, TYPE_BOOL);
    }
}
