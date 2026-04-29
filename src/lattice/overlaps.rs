//! Overlap relation: `overlaps(a, b)` is `true` iff there exists a
//! runtime value `v` such that `v ∈ a ∩ b`.
//!
//! Symmetric: `overlaps(a, b) == overlaps(b, a)`. Distinct from
//! `refines`: `int<0,10>` and `int<5,15>` overlap (value 7 inhabits both)
//! without either refining the other. The type-returning meet (greatest
//! lower bound) lives in `crate::meet`.
//!
//! Strategy: distribute over union (any element pair on the two sides
//! that overlaps proves the whole types overlap), then for each element
//! pair fall through these rules in order:
//!
//! 1. Reflexivity / Top / Bot axioms.
//! 2. Generic-parameter projection — `T` overlaps `X` iff `T`'s constraint
//!    overlaps `X`.
//! 3. Subsumption — `a <: b` or `b <: a` implies overlap.
//! 4. Family-specific positive overlap rules (e.g. range overlap, the
//!    string/class-like-string crossing, narrowed-mixed conservatism).
//!
//! When none of those fire we report disjoint. The rule set is incomplete
//! by design — adding a positive rule never weakens correctness, since the
//! relation is monotone in true outcomes; missing rules only cost
//! precision (a downstream narrowing returns `never` instead of a real
//! overlap).

use crate::ElementId;
use crate::ElementKind;
use crate::FlowFlags;
use crate::TypeId;
use crate::element::payload::MixedInfo;
use crate::element::payload::Truthiness;
use crate::element::payload::scalar::IntInfo;
use crate::interner::interner;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::lattice::family::mixed as mixed_family;
use crate::lattice::refines::element_refines;
use crate::prelude::MIXED;
use crate::prelude::NEVER;
use crate::prelude::PLACEHOLDER;
use crate::world::Variance;
use crate::world::World;

pub fn overlaps<W: World>(
    a: TypeId,
    b: TypeId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let a_type = a.as_ref();
    let b_type = b.as_ref();

    a_type.elements.iter().any(|x| b_type.elements.iter().any(|y| element_overlaps(*x, *y, world, options, report)))
}

fn element_overlaps<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    if a == NEVER || b == NEVER {
        return false;
    }

    if is_uninhabited(a) || is_uninhabited(b) {
        return false;
    }

    if a == b {
        return true;
    }

    if a == MIXED || b == MIXED || a == PLACEHOLDER || b == PLACEHOLDER {
        return true;
    }

    if a.kind() == ElementKind::GenericParameter {
        let constraint = interner().get_generic_parameter(a).constraint;
        let other = interner().intern_type(&[b], FlowFlags::EMPTY);
        return overlaps(constraint, other, world, options, report);
    }
    if b.kind() == ElementKind::GenericParameter {
        let constraint = interner().get_generic_parameter(b).constraint;
        let other = interner().intern_type(&[a], FlowFlags::EMPTY);
        return overlaps(constraint, other, world, options, report);
    }

    if a.kind() == ElementKind::Object && b.kind() == ElementKind::Object {
        return object_overlap(a, b, world, options, report);
    }

    if a.kind() == ElementKind::String && b.kind() == ElementKind::String {
        return string_overlap(a, b, world, options, report);
    }

    if a.kind() == ElementKind::List && b.kind() == ElementKind::List {
        return list_overlap(a, b, world, options, report);
    }

    if a.kind() == ElementKind::Array && b.kind() == ElementKind::Array {
        return array_overlap(a, b, world, options, report);
    }

    // Two callables share at least the always-throwing function (its
    // return-type is `never`, which trivially satisfies any signature),
    // so they overlap regardless of declared return / parameter types.
    if a.kind() == ElementKind::Callable && b.kind() == ElementKind::Callable {
        return true;
    }

    // Iterables likewise share the empty iterator: `[]`, the empty
    // generator, etc. inhabit `iterable<K, V>` for any K, V.
    if a.kind() == ElementKind::Iterable && b.kind() == ElementKind::Iterable {
        return true;
    }

    // Member predicates (`has-method`, `has-property`) are open: a
    // class can declare both methods or both properties, so any pair
    // is compositionally satisfiable.
    if (a.kind() == ElementKind::HasMethod && b.kind() == ElementKind::HasMethod)
        || (a.kind() == ElementKind::HasProperty && b.kind() == ElementKind::HasProperty)
    {
        return true;
    }

    if element_refines(a, b, world, options, report) || element_refines(b, a, world, options, report) {
        return true;
    }

    family_overlap(a, b)
}

/// Object × Object overlap. Two named classes share values when:
///
/// - They are the same class with type-args compatible under each
///   parameter's variance (invariant slots must value-equal, covariant
///   slots must overlap).
/// - One descends from the other (the descendant subset overlaps the
///   ancestor).
///
/// Otherwise, in PHP's single-inheritance model, two unrelated nominal
/// classes cannot share a runtime instance, so we return `false`. This
/// is conservative: a future world surface for shared interfaces /
/// traits can lift the answer to `true`.
fn object_overlap<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let i = interner();
    let a_info = *i.get_object(a);
    let b_info = *i.get_object(b);

    if a_info.name == b_info.name {
        if let (Some(a_args_id), Some(b_args_id)) = (a_info.type_args, b_info.type_args) {
            let a_args = i.get_type_list(a_args_id);
            let b_args = i.get_type_list(b_args_id);
            if a_args.len() == b_args.len() {
                for (idx, (&a_arg, &b_arg)) in a_args.iter().zip(b_args.iter()).enumerate() {
                    let variance = world
                        .template_parameter_at(a_info.name, idx)
                        .map(|t| t.variance)
                        .unwrap_or(Variance::Invariant);
                    match variance {
                        Variance::Invariant => {
                            let a_refines_b = crate::lattice::refines(a_arg, b_arg, world, options, report);
                            let b_refines_a = crate::lattice::refines(b_arg, a_arg, world, options, report);
                            if !a_refines_b || !b_refines_a {
                                return false;
                            }
                        }
                        Variance::Covariant => {
                            if !overlaps(a_arg, b_arg, world, options, report) {
                                return false;
                            }
                        }
                        Variance::Contravariant => {}
                    }
                }
            }
        }
        return true;
    }

    world.descends_from(a_info.name, b_info.name) || world.descends_from(b_info.name, a_info.name)
}

/// `true` for atoms that are structurally non-NEVER but whose value
/// set is empty: `non-empty-list<never>`, `non-empty-array<…, never>`,
/// etc. The lattice can construct these but no runtime value inhabits
/// them, so `overlap` should treat them as bottom.
fn is_uninhabited(elem: ElementId) -> bool {
    let i = interner();
    match elem.kind() {
        ElementKind::List => {
            let info = *i.get_list(elem);
            info.flags.non_empty() && info.element_type == crate::prelude::TYPE_NEVER
        }
        ElementKind::Array => {
            let info = *i.get_array(elem);
            if !info.flags.non_empty() {
                return false;
            }
            match (info.key_param, info.value_param) {
                (Some(k), _) if k == crate::prelude::TYPE_NEVER => true,
                (_, Some(v)) if v == crate::prelude::TYPE_NEVER => true,
                _ => false,
            }
        }
        _ => false,
    }
}

/// `String × String` overlap: defer to the meet rule. Two refined
/// string axes (`numeric-string`, `lowercase-string`, etc.) admit a
/// non-empty intersection unless their literal/casing/flags are
/// jointly unsatisfiable, which `string_meet` already decides.
fn string_overlap<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let _ = (world, options, report);
    crate::meet::family::string::string_meet(a, b).is_some()
}

/// `list<X> ∩ list<Y>` shares the empty list `[]` only when neither
/// side requires non-empty. When at least one side requires non-empty,
/// the element types must overlap for any concrete value to inhabit
/// both sets.
fn list_overlap<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let i = interner();
    let a_info = *i.get_list(a);
    let b_info = *i.get_list(b);
    if !a_info.flags.non_empty() && !b_info.flags.non_empty() {
        return true;
    }

    overlaps(a_info.element_type, b_info.element_type, world, options, report)
}

/// `array<K,V> ∩ array<K',V'>` mirrors `list_overlap`: the empty
/// array `[]` is shared only when neither side demands non-empty.
fn array_overlap<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let i = interner();
    let a_info = *i.get_array(a);
    let b_info = *i.get_array(b);
    if !a_info.flags.non_empty() && !b_info.flags.non_empty() {
        return true;
    }

    match (a_info.key_param, b_info.key_param, a_info.value_param, b_info.value_param) {
        (Some(ak), Some(bk), Some(av), Some(bv)) => {
            overlaps(ak, bk, world, options, report) && overlaps(av, bv, world, options, report)
        }
        _ => true,
    }
}

fn family_overlap(a: ElementId, b: ElementId) -> bool {
    if a.kind() == ElementKind::Int && b.kind() == ElementKind::Int {
        return int_overlap(a, b);
    }

    if a.kind() == ElementKind::Mixed || b.kind() == ElementKind::Mixed {
        return mixed_overlap(a, b);
    }

    let pair = (a.kind(), b.kind());
    if matches!(
        pair,
        (ElementKind::String, ElementKind::ClassLikeString) | (ElementKind::ClassLikeString, ElementKind::String)
    ) {
        return string_class_like_string_overlap(a, b);
    }

    // Numeric strings inhabit both `numeric` and `string`.
    if matches!(pair, (ElementKind::Numeric, ElementKind::String) | (ElementKind::String, ElementKind::Numeric)) {
        return true;
    }

    false
}

/// `String` × `ClassLikeString`: they overlap iff some string value
/// inhabits both. A class-like-string is always non-empty and (as a
/// PHP class name) carries no chars outside `[A-Za-z_0-9\\]`. A
/// literal string side rules out the overlap if its value isn't a
/// valid class name; a literal class-string side rules it out if its
/// fixed name conflicts with the string's literal/casing constraints.
fn string_class_like_string_overlap(a: ElementId, b: ElementId) -> bool {
    let i = interner();
    let (string_atom, class_atom) = if a.kind() == ElementKind::String { (a, b) } else { (b, a) };
    let s = *i.get_string(string_atom);
    if let crate::element::payload::scalar::StringLiteral::Value(value) = s.literal {
        return is_valid_class_name(value.as_str());
    }
    // The string side has no literal — overlap depends only on the
    // class-string. Any class-string is non-empty and "class-name"-shaped,
    // which always intersects general / refined string forms.
    let _ = class_atom;
    true
}

fn is_valid_class_name(s: &str) -> bool {
    let bytes = s.as_bytes();
    let len = bytes.len();
    if len == 0 || bytes[len - 1] == b'\\' {
        return false;
    }
    let mut i = usize::from(bytes[0] == b'\\');
    if i >= len {
        return false;
    }
    let mut part_start = true;
    while i < len {
        let b = bytes[i];
        if b == b'\\' {
            if part_start {
                return false;
            }
            part_start = true;
        } else if part_start {
            if !(b.is_ascii_alphabetic() || b == b'_') {
                return false;
            }
            part_start = false;
        } else if !(b.is_ascii_alphanumeric() || b == b'_' || b >= 0x80) {
            return false;
        }
        i += 1;
    }
    !part_start
}

/// Narrowed-mixed overlap: each side's axis flags must be jointly
/// satisfiable by some runtime value the other side admits. Vanilla
/// `mixed` is already absorbed by the Top axiom, so at least one side
/// here carries a non-trivial axis.
fn mixed_overlap(a: ElementId, b: ElementId) -> bool {
    let (mixed, other) = if a.kind() == ElementKind::Mixed { (a, b) } else { (b, a) };
    if !mixed_axes_compatible(*interner().get_mixed(mixed), other) {
        return false;
    }
    if other.kind() == ElementKind::Mixed && !mixed_axes_compatible(*interner().get_mixed(other), mixed) {
        return false;
    }
    true
}

fn mixed_axes_compatible(info: MixedInfo, other: ElementId) -> bool {
    if info.is_non_null() && !mixed_family::is_non_null(other) {
        return false;
    }
    let other_truth = mixed_family::truthiness_of(other);
    match info.truthiness() {
        Truthiness::Truthy if other_truth == Truthiness::Falsy => return false,
        Truthiness::Falsy if other_truth == Truthiness::Truthy => return false,
        _ => {}
    }
    if info.is_empty() && other_truth == Truthiness::Truthy {
        return false;
    }
    true
}

/// Intervals (with absorption: `INT` and `LITERAL_INT` are unbounded) on
/// either side overlap iff `max(lo_a, lo_b) ≤ min(hi_a, hi_b)`. An open
/// bound on either side is treated as `±∞`.
fn int_overlap(a: ElementId, b: ElementId) -> bool {
    let i = interner();
    let (al, au) = int_bounds(*i.get_int(a));
    let (bl, bu) = int_bounds(*i.get_int(b));

    let lo = match (al, bl) {
        (Some(x), Some(y)) => Some(x.max(y)),
        (Some(x), None) | (None, Some(x)) => Some(x),
        (None, None) => None,
    };
    let hi = match (au, bu) {
        (Some(x), Some(y)) => Some(x.min(y)),
        (Some(x), None) | (None, Some(x)) => Some(x),
        (None, None) => None,
    };

    match (lo, hi) {
        (Some(l), Some(h)) => l <= h,
        _ => true,
    }
}

fn int_bounds(info: IntInfo) -> (Option<i64>, Option<i64>) {
    match info {
        IntInfo::Unspecified | IntInfo::UnspecifiedLiteral => (None, None),
        IntInfo::Literal(n) => (Some(n), Some(n)),
        IntInfo::Range(range_id) => {
            let r = interner().get_int_range(range_id);
            (r.lower(), r.upper())
        }
    }
}
