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
//! A future `meet` (greatest lower bound, ⊓) module will pair with this
//! one when narrowing / intersection lands.
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
//! Refinement-driven absorptions (`int ∨ Literal(N) → int` once the lattice
//! decides the literal refines the dominator, range merging, class hierarchy
//! collapse, etc.) require the lattice and a codebase, and are not applied
//! here.

use std::num::NonZeroU32;

use crate::ElementId;
use crate::ElementKind;
use crate::TypeId;
use crate::element::payload::ArrayKey;
use crate::element::payload::FloatInfo;
use crate::element::payload::KeyedArrayFlags;
use crate::element::payload::KeyedArrayInfo;
use crate::element::payload::KnownElementEntry;
use crate::element::payload::KnownItemEntry;
use crate::element::payload::ListFlags;
use crate::element::payload::ListInfo;
use crate::element::payload::MixedInfo;
use crate::element::payload::Truthiness;
use crate::element::payload::scalar::IntInfo;
use crate::element::payload::scalar::StringLiteral;
use crate::interner::interner;
use crate::lattice::CoercionCauses;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::lattice::element_refines;
use crate::prelude::BOOL;
use crate::prelude::CALLABLE;
use crate::prelude::CLOSED_RESOURCE;
use crate::prelude::EMPTY_ARRAY;
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
use crate::prelude::TYPE_ARRAY_KEY;
use crate::prelude::TYPE_MIXED;
use crate::prelude::TYPE_NEVER;
use crate::prelude::VOID;
use crate::world::NullWorld;

/// Compute the join (least upper bound) of a slice of elements with the
/// default (purely-structural) options.
///
/// Returns a freshly-allocated, sorted, deduplicated [`Vec`] with the
/// canonicalization rules applied. Empty input collapses to `[NEVER]` so
/// callers always receive a non-empty multiset suitable for [`Type`]
/// construction.
///
/// [`Type`]: crate::Type
pub fn compute(elements: &[ElementId]) -> Vec<ElementId> {
    compute_with(elements, &JoinOptions::default())
}

/// Compute the join with caller-controlled extended rules per
/// [`JoinOptions`] (report §19). The structural pass runs first; each
/// extended rule fires after, gated on its own option, so the analyzer
/// can pick the simplification aggressiveness per call site.
pub fn compute_with(elements: &[ElementId], options: &JoinOptions) -> Vec<ElementId> {
    if elements.is_empty() {
        return vec![NEVER];
    }

    if elements.iter().any(|e| e.kind() == ElementKind::Mixed)
        && let Some(mixed_result) = apply_mixed_constraint_join(elements)
    {
        return vec![mixed_result];
    }

    let mut out: Vec<ElementId> = if options.merge_string_axes
        && elements.iter().filter(|e| e.kind() == ElementKind::String).take(2).count() >= 2
    {
        apply_string_axis_merge_in_order(elements)
    } else {
        elements.to_vec()
    };
    out.sort_unstable();
    out.dedup();
    canonicalize(&mut out);

    if options.overwrite_empty_array {
        apply_overwrite_empty_array(&mut out);
    }
    if let Some(t) = options.int_literal_collapse_threshold {
        apply_int_literal_collapse(&mut out, t);
    }
    if let Some(t) = options.string_literal_collapse_threshold {
        apply_string_literal_collapse(&mut out, t);
    }
    if let Some(t) = options.float_literal_collapse_threshold {
        apply_float_literal_collapse(&mut out, t);
    }
    if let Some(t) = options.array_shape_collapse_threshold {
        apply_array_shape_collapse(&mut out, t);
    }
    if options.merge_int_ranges {
        apply_merge_int_ranges(&mut out);
    }
    if options.absorb_refinements {
        apply_subtype_absorption(&mut out);
    }
    if options.synthesise_scalar {
        apply_scalar_synthesis(&mut out);
    }
    if options.merge_array_shapes {
        apply_merge_array_shapes(&mut out);
    }
    if options.merge_list_element_types {
        apply_merge_list_element_types(&mut out);
    }
    if options.merge_keyed_array_params {
        apply_merge_keyed_array_params(&mut out);
    }
    if options.rewrite_int_keyed_to_list {
        apply_rewrite_int_keyed_to_list(&mut out);
    }

    out.sort_unstable();
    out.dedup();
    out
}

/// Caller-controlled toggles for [`compute_with`] (report §19). All
/// fields default to "off" so [`compute`] keeps its purely-structural
/// behaviour.
#[derive(Debug, Clone, Copy, Default)]
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

impl JoinOptions {
    #[must_use]
    pub const fn with_merge_int_ranges(mut self, on: bool) -> Self {
        self.merge_int_ranges = on;
        self
    }

    #[must_use]
    pub const fn with_int_literal_collapse_threshold(mut self, threshold: u16) -> Self {
        self.int_literal_collapse_threshold = Some(threshold);
        self
    }

    #[must_use]
    pub const fn with_string_literal_collapse_threshold(mut self, threshold: u16) -> Self {
        self.string_literal_collapse_threshold = Some(threshold);
        self
    }

    #[must_use]
    pub const fn with_float_literal_collapse_threshold(mut self, threshold: u16) -> Self {
        self.float_literal_collapse_threshold = Some(threshold);
        self
    }

    #[must_use]
    pub const fn with_array_shape_collapse_threshold(mut self, threshold: u16) -> Self {
        self.array_shape_collapse_threshold = Some(threshold);
        self
    }

    #[must_use]
    pub const fn with_rewrite_int_keyed_to_list(mut self, on: bool) -> Self {
        self.rewrite_int_keyed_to_list = on;
        self
    }

    #[must_use]
    pub const fn with_merge_array_shapes(mut self, on: bool) -> Self {
        self.merge_array_shapes = on;
        self
    }

    #[must_use]
    pub const fn with_overwrite_empty_array(mut self, on: bool) -> Self {
        self.overwrite_empty_array = on;
        self
    }

    #[must_use]
    pub const fn with_absorb_refinements(mut self, on: bool) -> Self {
        self.absorb_refinements = on;
        self
    }

    #[must_use]
    pub const fn with_merge_string_axes(mut self, on: bool) -> Self {
        self.merge_string_axes = on;
        self
    }

    #[must_use]
    pub const fn with_synthesise_scalar(mut self, on: bool) -> Self {
        self.synthesise_scalar = on;
        self
    }

    #[must_use]
    pub const fn with_merge_list_element_types(mut self, on: bool) -> Self {
        self.merge_list_element_types = on;
        self
    }

    #[must_use]
    pub const fn with_merge_keyed_array_params(mut self, on: bool) -> Self {
        self.merge_keyed_array_params = on;
        self
    }
}

/// Merge same-kind string elements via AND-of-flags algebra. When
/// multiple strings are present, this folds them into a single
/// general/refined string plus the surviving incompatible literals.
///
/// The merge follows mago's behaviour:
///
/// - `lower_string | upper_string` → `string` (casing collapses to
///   `Unspecified`).
/// - `non_empty_string | lit("")` → `string` (empty literal forces
///   `is_non_empty` and `is_truthy` and `is_numeric` off).
/// - `truthy_string | lit("0")` → `truthy_string`, `lit("0")` (literal
///   "0" is incompatible with truthy → kept separate).
/// - `numeric_string | lit("123")` → `numeric_string` (compatible
///   literal absorbed).
/// - `numeric_string | lit("abc")` → `numeric_string`, `lit("abc")`
///   (non-numeric literal stays separate).
fn apply_string_axis_merge_in_order(elements: &[ElementId]) -> Vec<ElementId> {
    use crate::element::payload::scalar::StringCasing;
    use crate::element::payload::scalar::StringInfo;

    let i = interner();

    let mut other: Vec<ElementId> = Vec::with_capacity(elements.len());
    let mut general: Option<StringInfo> = None;
    let mut literals: Vec<mago_atom::Atom> = Vec::new();

    for &el in elements.iter() {
        if el.kind() != ElementKind::String {
            other.push(el);
            continue;
        }
        let info = *i.get_string(el);
        if let StringLiteral::Value(value) = info.literal {
            if let Some(ref mut existing) = general {
                let lit_value = value.as_str();
                let incompatible = (existing.flags.is_numeric() && !str_is_numeric(lit_value))
                    || (existing.flags.is_truthy() && (lit_value.is_empty() || lit_value == "0"))
                    || (existing.flags.is_non_empty() && lit_value.is_empty())
                    || (existing.casing == StringCasing::Lowercase
                        && lit_value.chars().any(|c| c.is_ascii_uppercase()))
                    || (existing.casing == StringCasing::Uppercase
                        && lit_value.chars().any(|c| c.is_ascii_lowercase()));
                if incompatible {
                    literals.push(value);
                } else {
                    *existing = combine_string_info(*existing, info);
                }
            } else {
                literals.push(value);
            }
            continue;
        }

        match general {
            None => {
                let mut new_info = info;
                if new_info.flags.is_truthy()
                    || new_info.flags.is_non_empty()
                    || new_info.flags.is_numeric()
                    || new_info.casing != StringCasing::Unspecified
                {
                    let mut keep_literals: Vec<mago_atom::Atom> = Vec::new();
                    let mut hit_empty = false;
                    for atom in &literals {
                        let value = atom.as_str();
                        if value.is_empty() {
                            new_info.flags =
                                new_info.flags.with_is_non_empty(false).with_is_truthy(false).with_is_numeric(false);
                            hit_empty = true;
                            break;
                        }
                        if value == "0" {
                            new_info.flags = new_info.flags.with_is_truthy(false);
                        }
                        if new_info.flags.is_numeric() && !str_is_numeric(value) {
                            keep_literals.push(*atom);
                            continue;
                        }
                        new_info.flags =
                            new_info.flags.with_is_numeric(new_info.flags.is_numeric() && str_is_numeric(value));
                        new_info.casing = match new_info.casing {
                            StringCasing::Lowercase if value.chars().all(|c| !c.is_ascii_uppercase()) => {
                                StringCasing::Lowercase
                            }
                            StringCasing::Uppercase if value.chars().all(|c| !c.is_ascii_lowercase()) => {
                                StringCasing::Uppercase
                            }
                            _ => StringCasing::Unspecified,
                        };
                    }
                    if hit_empty {
                        new_info.casing = StringCasing::Unspecified;
                    }
                    literals = keep_literals;
                }
                general = Some(new_info);
            }
            Some(ref mut existing) => {
                *existing = combine_string_info(*existing, info);
            }
        }
    }

    let mut new_strings: Vec<ElementId> =
        literals.into_iter().map(|atom| ElementId::string_literal(atom.as_str())).collect();
    if let Some(info) = general {
        new_strings.push(i.intern_string(info));
    }

    other.extend(new_strings);
    other
}

fn combine_string_info(
    a: crate::element::payload::scalar::StringInfo,
    b: crate::element::payload::scalar::StringInfo,
) -> crate::element::payload::scalar::StringInfo {
    use crate::element::payload::scalar::StringCasing;
    use crate::element::payload::scalar::StringInfo;
    use crate::element::payload::scalar::StringLiteral;
    let literal = match (a.literal, b.literal) {
        (StringLiteral::Value(v1), StringLiteral::Value(v2)) => {
            if v1 == v2 {
                StringLiteral::Value(v2)
            } else {
                StringLiteral::Unspecified
            }
        }
        (StringLiteral::Unspecified, _) | (_, StringLiteral::Unspecified) => StringLiteral::Unspecified,
        _ => StringLiteral::None,
    };
    let casing = match (a.casing, b.casing) {
        (StringCasing::Lowercase, StringCasing::Lowercase) => StringCasing::Lowercase,
        (StringCasing::Uppercase, StringCasing::Uppercase) => StringCasing::Uppercase,
        _ => StringCasing::Unspecified,
    };
    StringInfo { literal, casing, flags: a.flags.and(b.flags) }
}

fn str_is_numeric(s: &str) -> bool {
    s.parse::<i64>().is_ok() || s.parse::<f64>().is_ok()
}

/// Merge multiple unsealed lists with the same `non_empty` flag into a
/// single list whose element type is the type-union of theirs. Sealed
/// lists (those with `known_elements`) and lists with differing
/// `non_empty` flags are left alone.
fn apply_merge_list_element_types(elements: &mut Vec<ElementId>) {
    let i = interner();
    let mut groups: std::collections::HashMap<bool, (Vec<usize>, Vec<crate::ElementId>)> = Default::default();
    for (idx, &el) in elements.iter().enumerate() {
        if el.kind() != ElementKind::List {
            continue;
        }
        let info = *i.get_list(el);
        if info.known_elements.is_some() {
            continue;
        }
        groups.entry(info.flags.non_empty()).or_default().0.push(idx);
        groups.get_mut(&info.flags.non_empty()).unwrap().1.push(el);
    }

    let mut to_remove: std::collections::BTreeSet<usize> = Default::default();
    for (non_empty, (indices, _)) in &groups {
        if indices.len() < 2 {
            continue;
        }
        let mut merged_elements: Vec<ElementId> = Vec::new();
        for &idx in indices {
            let info = *i.get_list(elements[idx]);
            merged_elements.extend_from_slice(info.element_type.as_ref().elements);
        }
        let merged = compute(&merged_elements);
        let union_ty = i.intern_type(&merged, crate::FlowFlags::EMPTY);
        let merged_list = ElementId::list(union_ty, *non_empty);
        elements[indices[0]] = merged_list;
        for &idx in &indices[1..] {
            to_remove.insert(idx);
        }
    }

    if to_remove.is_empty() {
        return;
    }
    let mut idx = 0;
    elements.retain(|_| {
        let keep = !to_remove.contains(&idx);
        idx += 1;
        keep
    });
}

/// Merge multiple unsealed keyed arrays with the same `non_empty` flag
/// into a single keyed array with unioned key+value parameters. Sealed
/// keyed arrays (with `known_items`) are left to `apply_merge_array_shapes`.
fn apply_merge_keyed_array_params(elements: &mut Vec<ElementId>) {
    let i = interner();
    let mut groups: std::collections::HashMap<bool, Vec<usize>> = Default::default();
    for (idx, &el) in elements.iter().enumerate() {
        if el.kind() != ElementKind::Array {
            continue;
        }
        let info = *i.get_array(el);
        if info.known_items.is_some() || info.key_param.is_none() || info.value_param.is_none() {
            continue;
        }
        groups.entry(info.flags.non_empty()).or_default().push(idx);
    }

    let mut to_remove: std::collections::BTreeSet<usize> = Default::default();
    for (non_empty, indices) in &groups {
        if indices.len() < 2 {
            continue;
        }
        let mut key_elements: Vec<ElementId> = Vec::new();
        let mut value_elements: Vec<ElementId> = Vec::new();
        for &idx in indices {
            let info = *i.get_array(elements[idx]);
            key_elements.extend_from_slice(info.key_param.unwrap().as_ref().elements);
            value_elements.extend_from_slice(info.value_param.unwrap().as_ref().elements);
        }
        let key_canon = compute(&key_elements);
        let value_canon = compute(&value_elements);
        let key_ty = i.intern_type(&key_canon, crate::FlowFlags::EMPTY);
        let value_ty = i.intern_type(&value_canon, crate::FlowFlags::EMPTY);
        let merged_array = ElementId::keyed_unsealed(key_ty, value_ty, *non_empty);
        elements[indices[0]] = merged_array;
        for &idx in &indices[1..] {
            to_remove.insert(idx);
        }
    }

    if to_remove.is_empty() {
        return;
    }
    let mut idx = 0;
    elements.retain(|_| {
        let keep = !to_remove.contains(&idx);
        idx += 1;
        keep
    });
}

/// When the union contains all four general primitives (`int`, `string`,
/// `float`, `bool`), collapse them to `scalar`. Refined / literal forms
/// alone don't trigger the collapse: only the general unspecified `int`
/// counts (mirroring mago's `integers.any(is_unspecified)`). Other scalar
/// elements (literals, refinements, class-like-strings) remain
/// independent and are left to subtype absorption.
fn apply_scalar_synthesis(elements: &mut Vec<ElementId>) {
    let has_int = elements.contains(&INT);
    let has_string = elements.contains(&STRING);
    let has_float = elements.contains(&FLOAT);
    let has_bool = elements.contains(&BOOL);
    if !(has_int && has_string && has_float && has_bool) {
        return;
    }

    use crate::prelude::SCALAR;
    elements.retain(|e| *e != INT && *e != STRING && *e != FLOAT && *e != BOOL);
    let pos = elements.binary_search(&SCALAR).unwrap_or_else(|p| p);
    elements.insert(pos, SCALAR);
}

/// Mixed-constraint joining: order-dependent state machine ported from
/// mago's combiner. When any `Mixed` kind appears in the input, the
/// result is a single mixed element whose flavour is decided by walking
/// the input in original order:
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
/// Returns `None` when the input has no `Mixed` element (caller proceeds
/// with the regular join). Returns `Some(elem)` with a single mixed
/// element to emit.
fn apply_mixed_constraint_join(elements: &[ElementId]) -> Option<ElementId> {
    use crate::predicates::element as pred;

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
                continue;
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

    // Normalise to suffete's canonical mixed slots: truthy/falsy variants
    // already encode their nullability semantically, so the explicit
    // `is_non_null` flag only matters for `non_null_mixed`.
    let info = match truthiness {
        Truthiness::Truthy => MixedInfo::EMPTY.with_truthiness(Truthiness::Truthy),
        Truthiness::Falsy => MixedInfo::EMPTY.with_truthiness(Truthiness::Falsy),
        Truthiness::Undetermined => MixedInfo::EMPTY.with_is_non_null(final_nonnull),
    };

    Some(i.intern_mixed(info))
}

#[allow(clippy::too_many_arguments)]
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
    use crate::predicates::element as pred;

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

/// Whether `el` (a non-mixed kind) is checked by mago's
/// `has_non_truthy` filter (i.e. counts as a value-types entry that
/// would contradict a `truthy_mixed` constraint). Mago skips integers
/// and float literals from this check (they live in `combination.integers`
/// / `combination.literal_floats`, not `combination.value_types`).
fn non_mixed_counts_for_truthy_check(el: ElementId) -> bool {
    match el.kind() {
        ElementKind::Int => false,
        ElementKind::Float => !matches!(*interner().get_float(el), FloatInfo::Literal(_)),
        _ => true,
    }
}

fn non_mixed_counts_for_falsy_check(el: ElementId) -> bool {
    non_mixed_counts_for_truthy_check(el)
}

/// Returns `true` if any non-mixed element in `elements[..upto]`
/// satisfies `predicate`.
fn elements_seen_so_far_any(elements: &[ElementId], upto: usize, predicate: impl Fn(ElementId) -> bool) -> bool {
    elements[..upto].iter().any(|&e| e.kind() != ElementKind::Mixed && predicate(e))
}

/// Drop any element absorbed by another structurally-larger element in
/// the same multiset (`a <: b` ⇒ drop `a`). Uses the lattice's element
/// refinement check with [`NullWorld`](crate::world::NullWorld), so only
/// purely-structural rules fire. Coercion-driven refinements (e.g.
/// `int <: float` via PHP's runtime int-to-float coercion) do **not**
/// drive absorption: keeping `int|float` distinct preserves the
/// information that the value was originally typed as `int`.
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

fn apply_int_literal_collapse(elements: &mut Vec<ElementId>, threshold: u16) {
    if elements.contains(&INT) {
        return;
    }

    let i = interner();
    let count = elements
        .iter()
        .filter(|e| e.kind() == ElementKind::Int && matches!(i.get_int(**e), IntInfo::Literal(_)))
        .count();

    if count as u32 <= u32::from(threshold) {
        return;
    }

    elements.retain(|e| !(e.kind() == ElementKind::Int && matches!(i.get_int(*e), IntInfo::Literal(_))));
    let pos = elements.binary_search(&INT).unwrap_or_else(|p| p);
    elements.insert(pos, INT);
}

fn apply_float_literal_collapse(elements: &mut Vec<ElementId>, threshold: u16) {
    use crate::element::payload::scalar::FloatInfo;
    if elements.contains(&FLOAT) {
        return;
    }

    let i = interner();
    let count = elements
        .iter()
        .filter(|e| e.kind() == ElementKind::Float && matches!(i.get_float(**e), FloatInfo::Literal(_)))
        .count();

    if count as u32 <= u32::from(threshold) {
        return;
    }

    elements.retain(|e| !(e.kind() == ElementKind::Float && matches!(i.get_float(*e), FloatInfo::Literal(_))));
    let pos = elements.binary_search(&FLOAT).unwrap_or_else(|p| p);
    elements.insert(pos, FLOAT);
}

fn apply_array_shape_collapse(elements: &mut Vec<ElementId>, threshold: u16) {
    let shape_count = elements
        .iter()
        .filter(|e| matches!(e.kind(), ElementKind::Array | ElementKind::List) && **e != EMPTY_ARRAY)
        .count();

    if shape_count as u32 <= u32::from(threshold) {
        return;
    }

    let i = interner();
    elements.retain(|e| !(matches!(e.kind(), ElementKind::Array | ElementKind::List) && *e != EMPTY_ARRAY));
    let general = i.intern_array(KeyedArrayInfo {
        key_param: Some(TYPE_ARRAY_KEY),
        value_param: Some(TYPE_MIXED),
        known_items: None,
        flags: KeyedArrayFlags::default(),
    });

    let pos = elements.binary_search(&general).unwrap_or_else(|p| p);
    elements.insert(pos, general);
}

fn apply_overwrite_empty_array(elements: &mut Vec<ElementId>) {
    let has_other_array =
        elements.iter().any(|e| *e != EMPTY_ARRAY && matches!(e.kind(), ElementKind::Array | ElementKind::List));
    if has_other_array {
        elements.retain(|e| *e != EMPTY_ARRAY);
    }
}

fn apply_string_literal_collapse(elements: &mut Vec<ElementId>, threshold: u16) {
    if elements.contains(&STRING) {
        return;
    }
    let i = interner();
    let count = elements
        .iter()
        .filter(|e| e.kind() == ElementKind::String && matches!(i.get_string(**e).literal, StringLiteral::Value(_)))
        .count();
    if count as u32 <= u32::from(threshold) {
        return;
    }
    elements
        .retain(|e| !(e.kind() == ElementKind::String && matches!(i.get_string(*e).literal, StringLiteral::Value(_))));
    let pos = elements.binary_search(&STRING).unwrap_or_else(|p| p);
    elements.insert(pos, STRING);
}

/// Merge adjacent integer literals and bounded ranges into wider
/// ranges. Untouched `IntInfo` variants (`Unspecified`,
/// `UnspecifiedLiteral`) are dominators / virtual forms and stay as-is.
fn apply_merge_int_ranges(elements: &mut Vec<ElementId>) {
    let i = interner();
    let mut intervals: Vec<(Option<i64>, Option<i64>)> = Vec::new();
    let mut other: Vec<ElementId> = Vec::with_capacity(elements.len());
    for &el in elements.iter() {
        if el.kind() != ElementKind::Int {
            other.push(el);
            continue;
        }
        match *i.get_int(el) {
            IntInfo::Literal(n) => intervals.push((Some(n), Some(n))),
            IntInfo::Range(rid) => {
                let r = *i.get_int_range(rid);
                intervals.push((r.lower(), r.upper()));
            }
            _ => other.push(el),
        }
    }

    if intervals.is_empty() {
        return;
    }

    intervals.sort_by(|a, b| match (a.0, b.0) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, _) => std::cmp::Ordering::Less,
        (_, None) => std::cmp::Ordering::Greater,
        (Some(x), Some(y)) => x.cmp(&y),
    });

    let mut merged: Vec<(Option<i64>, Option<i64>)> = Vec::with_capacity(intervals.len());
    for r in intervals {
        if let Some(last) = merged.last_mut() {
            let adjacent = match (last.1, r.0) {
                (None, _) => true,
                (Some(_), None) => true,
                (Some(lu), Some(rl)) => lu.checked_add(1).is_some_and(|n| n >= rl),
            };
            if adjacent {
                last.1 = match (last.1, r.1) {
                    (None, _) | (_, None) => None,
                    (Some(a), Some(b)) => Some(a.max(b)),
                };
                continue;
            }
        }
        merged.push(r);
    }

    let mut new_elements: Vec<ElementId> = other;
    for (lo, hi) in merged {
        let elem = match (lo, hi) {
            (None, None) => INT,
            (Some(l), Some(h)) if l == h => ElementId::int_literal(l),
            _ => ElementId::int_range(lo, hi),
        };
        new_elements.push(elem);
    }
    *elements = new_elements;
}

/// Detect keyed-array atoms whose `known_items` use contiguous integer
/// keys `0..n-1` (and whose key/value rest types are absent or
/// list-compatible) and rewrite them as `List` atoms.
fn apply_rewrite_int_keyed_to_list(elements: &mut [ElementId]) {
    let i = interner();
    for el in elements.iter_mut() {
        if el.kind() != ElementKind::Array {
            continue;
        }
        let info = *i.get_array(*el);
        if info.key_param.is_some() {
            continue;
        }
        let Some(known_id) = info.known_items else {
            continue;
        };
        let entries = i.get_known_items(known_id);
        let mut indexed: Vec<(i64, &KnownItemEntry)> = Vec::with_capacity(entries.len());
        let mut all_int = true;
        for entry in entries {
            match entry.key {
                ArrayKey::Int(n) => indexed.push((n, entry)),
                _ => {
                    all_int = false;
                    break;
                }
            }
        }
        if !all_int {
            continue;
        }
        indexed.sort_by_key(|(n, _)| *n);
        if !(0..indexed.len()).all(|idx| indexed[idx].0 == idx as i64) {
            continue;
        }

        let known_elements: Vec<KnownElementEntry> = indexed
            .iter()
            .map(|(n, entry)| KnownElementEntry { index: *n as u32, value: entry.value, optional: entry.optional })
            .collect();
        let known_count = NonZeroU32::new(known_elements.len() as u32);
        let list_info = ListInfo {
            element_type: info.value_param.unwrap_or(TYPE_NEVER),
            known_elements: Some(i.intern_known_elements(&known_elements)),
            known_count,
            flags: ListFlags::default().with_non_empty(info.flags.non_empty()),
        };
        *el = i.intern_list(list_info);
    }
}

/// When the union contains multiple keyed-array atoms that share at
/// least one literal key, fold them into a single shape whose value at
/// every shared key is the union of the source values.
fn apply_merge_array_shapes(elements: &mut Vec<ElementId>) {
    let i = interner();
    let mut shapes: Vec<usize> = elements
        .iter()
        .enumerate()
        .filter_map(|(idx, el)| {
            (el.kind() == ElementKind::Array && i.get_array(*el).known_items.is_some()).then_some(idx)
        })
        .collect();

    if shapes.len() < 2 {
        return;
    }

    let head_idx = shapes.remove(0);
    let head_info = *i.get_array(elements[head_idx]);
    let head_entries = i.get_known_items(head_info.known_items.unwrap()).to_vec();

    let mut new_known: Vec<KnownItemEntry> = head_entries.clone();
    let mut absorbed: Vec<usize> = Vec::new();
    let mut accumulated_non_empty = head_info.flags.non_empty();

    for &shape_idx in &shapes {
        let other = *i.get_array(elements[shape_idx]);
        if other.key_param != head_info.key_param || other.value_param != head_info.value_param {
            continue;
        }
        let Some(other_known_id) = other.known_items else { continue };
        let other_entries = i.get_known_items(other_known_id);
        let shares_key = other_entries.iter().any(|o| new_known.iter().any(|e| e.key == o.key));
        if !shares_key {
            continue;
        }

        for o_entry in other_entries {
            if let Some(existing) = new_known.iter_mut().find(|e| e.key == o_entry.key) {
                let mut elems: Vec<ElementId> = existing.value.as_ref().elements.to_vec();
                elems.extend_from_slice(o_entry.value.as_ref().elements);
                existing.value = TypeId::union(&elems);
                existing.optional = existing.optional || o_entry.optional;
            } else {
                new_known.push(*o_entry);
            }
        }
        accumulated_non_empty = accumulated_non_empty || other.flags.non_empty();
        absorbed.push(shape_idx);
    }

    if absorbed.is_empty() {
        return;
    }

    new_known.sort_by_key(|e| e.key);
    let merged_info = KeyedArrayInfo {
        known_items: Some(i.intern_known_items(&new_known)),
        flags: KeyedArrayFlags::default().with_non_empty(accumulated_non_empty),
        ..head_info
    };
    elements[head_idx] = i.intern_array(merged_info);

    let mut absorbed_set: std::collections::BTreeSet<usize> = absorbed.into_iter().collect();
    let mut idx = 0;
    elements.retain(|_| {
        let keep = !absorbed_set.remove(&idx);
        idx += 1;
        keep
    });
}

/// Apply the structural canonicalization rules. `elements` must be sorted
/// and deduplicated on entry; sorted order is preserved on exit.
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
fn is_object_family_kind(kind: ElementKind) -> bool {
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
    use crate::prelude::NULL;
    use crate::prelude::TYPE_BOOL;
    use crate::prelude::TYPE_INT_OR_STRING;
    use crate::prelude::TYPE_MIXED;

    #[test]
    fn empty_yields_never() {
        assert_eq!(compute(&[]), vec![NEVER]);
    }

    #[test]
    fn sorts_and_dedupes() {
        let a = ElementId::int_literal(99);
        let b = ElementId::int_literal(100);
        let r1 = compute(&[a, b]);
        let r2 = compute(&[b, a]);
        let r3 = compute(&[a, b, a, b, a]);
        assert_eq!(r1, r2);
        assert_eq!(r1, r3);
        assert_eq!(r1.len(), 2);
    }

    #[test]
    fn never_is_dropped_when_other_elements_exist() {
        assert_eq!(compute(&[NEVER, INT]), vec![INT]);
    }

    #[test]
    fn never_alone_is_preserved() {
        assert_eq!(compute(&[NEVER]), vec![NEVER]);
    }

    #[test]
    fn void_alone_is_preserved() {
        assert_eq!(compute(&[VOID]), vec![VOID]);
    }

    #[test]
    fn void_is_kept_when_other_elements_exist() {
        let mut out = compute(&[VOID, INT]);
        out.sort();
        let mut expected = vec![INT, VOID];
        expected.sort();
        assert_eq!(out, expected);
    }

    #[test]
    fn void_and_never_together_keeps_void() {
        assert_eq!(compute(&[VOID, NEVER]), vec![VOID]);
    }

    #[test]
    fn true_or_false_merges_to_bool() {
        assert_eq!(compute(&[TRUE, FALSE]), vec![BOOL]);
    }

    #[test]
    fn bool_absorbs_true_and_false() {
        assert_eq!(compute(&[BOOL, TRUE]), vec![BOOL]);
        assert_eq!(compute(&[BOOL, FALSE]), vec![BOOL]);
        assert_eq!(compute(&[BOOL, TRUE, FALSE]), vec![BOOL]);
    }

    #[test]
    fn vanilla_mixed_absorbs_everything_else() {
        assert_eq!(compute(&[MIXED, INT, STRING, NEVER]), vec![MIXED]);
    }

    #[test]
    fn open_or_closed_resource_merges_to_resource() {
        assert_eq!(compute(&[OPEN_RESOURCE, CLOSED_RESOURCE]), vec![RESOURCE]);
    }

    #[test]
    fn resource_absorbs_open_and_closed() {
        assert_eq!(compute(&[RESOURCE, OPEN_RESOURCE]), vec![RESOURCE]);
        assert_eq!(compute(&[RESOURCE, CLOSED_RESOURCE]), vec![RESOURCE]);
        assert_eq!(compute(&[RESOURCE, OPEN_RESOURCE, CLOSED_RESOURCE]), vec![RESOURCE]);
    }

    #[test]
    fn unrelated_elements_are_preserved() {
        let mut out = compute(&[INT, STRING]);
        out.sort();
        let mut expected = vec![INT, STRING];
        expected.sort();
        assert_eq!(out, expected);
    }

    #[test]
    fn null_and_array_key_kept_separate() {
        let mut out = compute(&[NULL, ARRAY_KEY]);
        out.sort();
        let mut expected = vec![NULL, ARRAY_KEY];
        expected.sort();
        assert_eq!(out, expected);
    }

    #[test]
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
    fn join_compute_then_union_collapses_to_well_known_handles() {
        let collapsed_bool = TypeId::union(&compute(&[TRUE, FALSE]));
        assert_eq!(collapsed_bool, TYPE_BOOL);

        let collapsed_mixed = TypeId::union(&compute(&[MIXED, INT, STRING]));
        assert_eq!(collapsed_mixed, TYPE_MIXED);
    }

    #[test]
    fn intern_type_does_not_canonicalize() {
        let i = interner();
        let raw = i.intern_type(&[TRUE, FALSE], FlowFlags::EMPTY);
        assert_eq!(raw.as_ref().elements, &[TRUE, FALSE]);
        assert_ne!(raw, TYPE_BOOL);
    }
}
