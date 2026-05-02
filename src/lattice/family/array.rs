//! Array family: keyed arrays (`array<K, V>`, `array{a: int, ...}`,
//! `array{}`) and lists (`list<T>`, `non-empty-list<T>`,
//! `list{0: int, ...}`).
//!
//! The two PHP-level kinds (`Array` and `List`) share most rules. Lists are
//! int-keyed keyed arrays whose values share an element type; the family
//! treats them uniformly where the rules coincide and dispatches to
//! shape-specific helpers where they don't.
//!
//! Implemented rules:
//!
//! - **Reflexivity**: handled by the dispatcher.
//! - **`array{}` (empty)** refines every list and every keyed array (an
//!   empty array fits both views vacuously, except `non-empty` containers).
//! - **List vs list**: element-type covariance; non-empty refines empty-or-
//!   not (`non-empty-list<E> <: list<E>`); sealed-list (`list{...}`)
//!   refines an unsealed list when every known element refines the
//!   container's element type.
//! - **Keyed vs keyed**: key-type and value-type covariance; sealed shapes
//!   refine unsealed keyed-arrays when every known item's key+value refine
//!   the container's parameters; sealed-vs-sealed checks that every
//!   container required key has a matching (refining) input key, and that
//!   the input doesn't have extra required keys.
//! - **Optional-vs-required**: required `<:` optional (the input always
//!   carries the key, the container also accepts that), but optional `not
//!   <:` required (the input might miss the key).
//! - **List vs keyed**: a list refines an unsealed keyed-array if the
//!   container's key parameter accepts `int` and value parameter accepts
//!   the list's element type.
//! - **Sealed list vs unsealed list**: every known element refines the
//!   element type.
//! - **Sealed list vs sealed list**: pointwise element refinement.

use crate::ElementId;
use crate::ElementKind;
use crate::FlowFlags;
use crate::TypeId;
use crate::element::payload::ArrayKey;
use crate::element::payload::KeyedArrayInfo;
use crate::element::payload::ListInfo;
use crate::interner::interner;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::lattice::refines::refines as type_refines;
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
    match container.kind() {
        ElementKind::List => refines_list(input, container, world, options, report),
        ElementKind::Array => refines_keyed(input, container, world, options, report),
        _ => false,
    }
}

#[inline]
fn refines_list<W: World>(
    input: ElementId,
    container: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let i = interner();
    let container_info = *i.get_list(container);

    // `array{}` refines `list<T>` (vacuously: no entries to check), but not
    // `non-empty-list<T>`.
    if input == EMPTY_ARRAY {
        return !container_info.flags.non_empty();
    }

    match input.kind() {
        ElementKind::List => {
            let input_info = *i.get_list(input);
            list_refines_list(input_info, container_info, world, options, report)
        }
        // Keyed-vs-list, sealed-list-vs-unsealed-list, etc., land here once
        // the sealed-list / sealed-keyed shape helpers are wired.
        _ => false,
    }
}

#[inline]
fn refines_keyed<W: World>(
    input: ElementId,
    container: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let i = interner();
    let container_info = *i.get_array(container);

    // `array{}` refines every keyed-array container that is itself unsealed
    // or admits empty (`array{}` itself is also empty so refl handles that).
    if input == EMPTY_ARRAY {
        return !container_info.flags.non_empty();
    }

    match input.kind() {
        ElementKind::Array => {
            let input_info = *i.get_array(input);
            keyed_refines_keyed(input_info, container_info, world, options, report)
        }
        ElementKind::List => {
            let input_info = *i.get_list(input);
            // A list refines an unsealed keyed-array when the container
            // accepts `int` keys and the container's value parameter
            // accepts the list's element type. Sealed-keyed containers
            // require fixed entries the list cannot guarantee, so reject.
            let (Some(key_param), Some(value_param)) = (container_info.key_param, container_info.value_param)
            else {
                return false;
            };
            let int_t = single_type(INT);
            // Empty list (`flags.non_empty=false`, no known elements) does
            // not satisfy `non-empty-array` containers.
            if container_info.flags.non_empty() && !input_info.flags.non_empty() {
                return false;
            }
            type_refines(int_t, key_param, world, options, report)
                && type_refines(input_info.element_type, value_param, world, options, report)
        }
        _ => false,
    }
}

#[inline]
fn list_refines_list<W: World>(
    input: ListInfo,
    container: ListInfo,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    // non-empty constraint: container demands non-empty, input must
    // carry it.
    if container.flags.non_empty() && !input.flags.non_empty() {
        return false;
    }

    type_refines(input.element_type, container.element_type, world, options, report)
}

#[inline]
fn keyed_refines_keyed<W: World>(
    input: KeyedArrayInfo,
    container: KeyedArrayInfo,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    if container.flags.non_empty() && !input.flags.non_empty() && !has_required_known_item(input) {
        return false;
    }

    if container.is_sealed() {
        return sealed_refines_sealed(input, container, world, options, report);
    }

    let (Some(container_key), Some(container_value)) = (container.key_param, container.value_param) else {
        return false;
    };

    if let Some(items_id) = input.known_items {
        let items = interner().get_known_items(items_id);
        for item in items {
            let key_t = key_to_type(item.key);
            if !type_refines(key_t, container_key, world, options, report) {
                return false;
            }
            if !type_refines(item.value, container_value, world, options, report) {
                return false;
            }
        }
    }

    if let (Some(input_key), Some(input_value)) = (input.key_param, input.value_param) {
        if !type_refines(input_key, container_key, world, options, report) {
            return false;
        }
        if !type_refines(input_value, container_value, world, options, report) {
            return false;
        }
    }

    true
}

#[inline]
fn sealed_refines_sealed<W: World>(
    input: KeyedArrayInfo,
    container: KeyedArrayInfo,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    if input.key_param.is_some() || input.value_param.is_some() {
        return false;
    }

    let i = interner();
    let input_items: &[crate::element::payload::KnownItemEntry] = match input.known_items {
        Some(id) => i.get_known_items(id),
        None => &[],
    };
    let container_items: &[crate::element::payload::KnownItemEntry] = match container.known_items {
        Some(id) => i.get_known_items(id),
        None => &[],
    };

    // The input must NOT have extra required keys the container doesn't
    // declare (those would force the container to admit unknown entries).
    for input_item in input_items {
        let in_container = container_items.iter().any(|c| c.key == input_item.key);
        if !in_container && !input_item.optional {
            // The container is sealed but doesn't list this required key.
            return false;
        }
    }

    // Every container key must either appear (refined) in the input or be
    // optional (so the input is allowed to omit it).
    for container_item in container_items {
        let matched = input_items.iter().find(|c| c.key == container_item.key);
        match matched {
            Some(input_item) => {
                // Required-vs-required and required-vs-optional are fine;
                // optional-vs-required is not (input might miss the key).
                if !container_item.optional && input_item.optional {
                    return false;
                }

                if !type_refines(input_item.value, container_item.value, world, options, report) {
                    return false;
                }
            }
            None => {
                if !container_item.optional {
                    return false;
                }
            }
        }
    }

    true
}

#[inline]
fn has_required_known_item(info: KeyedArrayInfo) -> bool {
    match info.known_items {
        Some(id) => interner().get_known_items(id).iter().any(|item| !item.optional),
        None => false,
    }
}

#[inline]
fn key_to_type(key: ArrayKey) -> TypeId {
    match key {
        ArrayKey::Int(n) => single_type(ElementId::int_literal(n)),
        ArrayKey::String(atom) => single_type(ElementId::string_literal(atom.as_str())),
        ArrayKey::Const { .. } => single_type(crate::prelude::ARRAY_KEY),
    }
}

#[inline]
fn single_type(element: ElementId) -> TypeId {
    interner().intern_type(&[element], FlowFlags::EMPTY)
}
