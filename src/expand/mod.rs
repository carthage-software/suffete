//! Type expansion: resolve non-structural type forms (`Alias`,
//! `Reference`, `Derived`, `Conditional`, contextual keywords) into
//! their structural definitions, per `type-system/generics.md` §7.
//!
//! [`expand`] takes a [`TypeId`] and a [`World`] and returns a new
//! `TypeId` whose tree contains no expandable atoms (modulo the rules
//! that the spec marks as "passes through unchanged" — see `§7.3` for
//! the per-form treatment).
//!
//! # Stages
//!
//! Stages land independently. Each new stage adds rules; the public
//! signature is monotone in result precision (a previously
//! pass-through atom becomes a real resolution).
//!
//! - **Stage 1 (current):** `Alias` resolution. Aliases declared on a
//!   class via `World::alias_body` are looked up and replaced by their
//!   recorded body, recursively expanded.
//! - **Stage 2:** `Reference` resolution + contextual keyword
//!   substitution (`self`, `static`, `parent`).
//! - **Stage 3:** `Derived` evaluation (`KeyOf`, `ValueOf`, `IndexAccess`,
//!   `PropertiesOf`, `IntMask`, etc.).
//! - **Stage 4:** `Conditional` evaluation.
//!
//! # Structural descent
//!
//! Per `§7.4`, expansion descends into every nested type — `Object`
//! type arguments, list / keyed-array / iterable element-types, sealed
//! known items, and class-like-string constraints. Element kinds whose
//! payloads do not (yet) carry nested types pass through unchanged.

use crate::ElementId;
use crate::ElementKind;
use crate::TypeId;
use crate::element::payload::ClassLikeStringInfo;
use crate::element::payload::ClassLikeStringSpecifier;
use crate::element::payload::IterableInfo;
use crate::element::payload::KeyedArrayInfo;
use crate::element::payload::KnownItemEntry;
use crate::element::payload::ListInfo;
use crate::element::payload::ObjectInfo;
use crate::interner::interner;
use crate::world::World;

/// Resolve every expandable atom inside `ty` against `world`. Returns
/// the same `TypeId` handle when nothing changed.
///
/// Stage 1 only resolves `Alias` atoms. Other expandable kinds
/// (`Reference`, `Conditional`, `Derived`) pass through unchanged but
/// their nested types are still descended in case they carry aliases.
pub fn expand<W: World>(ty: TypeId, world: &W) -> TypeId {
    let i = interner();
    let original = ty.as_ref();
    let mut new_elements: Vec<ElementId> = Vec::with_capacity(original.elements.len());
    let mut changed = false;

    for &elem in original.elements {
        match expand_element(elem, world) {
            Expansion::Unchanged => new_elements.push(elem),
            Expansion::Single(new_elem) => {
                changed = true;
                new_elements.push(new_elem);
            }
            Expansion::Many(elems) => {
                changed = true;
                new_elements.extend(elems);
            }
        }
    }

    if !changed {
        return ty;
    }

    let joined = crate::join::compute(&new_elements);
    i.intern_type(&joined, original.flags)
}

/// What `expand_element` returns. `Unchanged` is the common case;
/// `Single` covers in-place rewrites; `Many` covers an alias whose
/// body is a multi-element union and flat-merges into the surrounding
/// type.
enum Expansion {
    Unchanged,
    Single(ElementId),
    Many(Vec<ElementId>),
}

fn expand_element<W: World>(elem: ElementId, world: &W) -> Expansion {
    match elem.kind() {
        ElementKind::Alias => expand_alias(elem, world),
        ElementKind::Object => expand_object(elem, world),
        ElementKind::List => expand_list(elem, world),
        ElementKind::Array => expand_keyed_array(elem, world),
        ElementKind::Iterable => expand_iterable(elem, world),
        ElementKind::ClassLikeString => expand_class_like_string(elem, world),
        _ => Expansion::Unchanged,
    }
}

fn expand_alias<W: World>(elem: ElementId, world: &W) -> Expansion {
    let info = interner().get_alias(elem);
    let Some(body) = world.alias_body(info.class_name, info.alias_name) else {
        return Expansion::Unchanged;
    };

    let expanded = expand(body, world);
    let elements = expanded.as_ref().elements;
    if elements.len() == 1 { Expansion::Single(elements[0]) } else { Expansion::Many(elements.to_vec()) }
}

fn expand_object<W: World>(elem: ElementId, world: &W) -> Expansion {
    let i = interner();
    let info = *i.get_object(elem);
    let Some(args_id) = info.type_args else {
        return Expansion::Unchanged;
    };

    let args = i.get_type_list(args_id);
    let new_args: Vec<TypeId> = args.iter().map(|&a| expand(a, world)).collect();
    if new_args.iter().zip(args.iter()).all(|(n, o)| n == o) {
        return Expansion::Unchanged;
    }

    let new_args_id = i.intern_type_list(&new_args);
    Expansion::Single(i.intern_object(ObjectInfo { type_args: Some(new_args_id), ..info }))
}

fn expand_list<W: World>(elem: ElementId, world: &W) -> Expansion {
    let i = interner();
    let info = *i.get_list(elem);
    let new_element_type = expand(info.element_type, world);

    let new_known = info.known_elements.map(|id| {
        let entries = i.get_known_elements(id);
        let new_entries: Vec<_> = entries
            .iter()
            .map(|entry| crate::element::payload::KnownElementEntry { value: expand(entry.value, world), ..*entry })
            .collect();
        let unchanged = new_entries.iter().zip(entries.iter()).all(|(n, o)| n.value == o.value);
        if unchanged { (id, false) } else { (i.intern_known_elements(&new_entries), true) }
    });

    let known_changed = new_known.is_some_and(|(_, ch)| ch);
    if new_element_type == info.element_type && !known_changed {
        return Expansion::Unchanged;
    }

    Expansion::Single(i.intern_list(ListInfo {
        element_type: new_element_type,
        known_elements: new_known.map(|(id, _)| id),
        ..info
    }))
}

fn expand_keyed_array<W: World>(elem: ElementId, world: &W) -> Expansion {
    let i = interner();
    let info = *i.get_array(elem);
    let new_key = info.key_param.map(|t| expand(t, world));
    let new_value = info.value_param.map(|t| expand(t, world));

    let new_known = info.known_items.map(|id| {
        let entries = i.get_known_items(id);
        let new_entries: Vec<KnownItemEntry> =
            entries.iter().map(|entry| KnownItemEntry { value: expand(entry.value, world), ..*entry }).collect();
        let unchanged = new_entries.iter().zip(entries.iter()).all(|(n, o)| n.value == o.value);
        if unchanged { (id, false) } else { (i.intern_known_items(&new_entries), true) }
    });

    let key_unchanged = new_key == info.key_param;
    let value_unchanged = new_value == info.value_param;
    let known_unchanged = new_known.is_none_or(|(_, ch)| !ch);
    if key_unchanged && value_unchanged && known_unchanged {
        return Expansion::Unchanged;
    }

    Expansion::Single(i.intern_array(KeyedArrayInfo {
        key_param: new_key,
        value_param: new_value,
        known_items: new_known.map(|(id, _)| id),
        ..info
    }))
}

fn expand_iterable<W: World>(elem: ElementId, world: &W) -> Expansion {
    let i = interner();
    let info = *i.get_iterable(elem);
    let new_key = expand(info.key_type, world);
    let new_value = expand(info.value_type, world);
    if new_key == info.key_type && new_value == info.value_type {
        return Expansion::Unchanged;
    }
    Expansion::Single(i.intern_iterable(IterableInfo { key_type: new_key, value_type: new_value, ..info }))
}

fn expand_class_like_string<W: World>(elem: ElementId, world: &W) -> Expansion {
    let i = interner();
    let info = *i.get_class_like_string(elem);
    let new_specifier = match info.specifier {
        ClassLikeStringSpecifier::OfType { constraint } => {
            let new_constraint = expand(constraint, world);
            if new_constraint == constraint {
                return Expansion::Unchanged;
            }
            ClassLikeStringSpecifier::OfType { constraint: new_constraint }
        }
        ClassLikeStringSpecifier::Generic { constraint } => {
            let new_constraint = expand(constraint, world);
            if new_constraint == constraint {
                return Expansion::Unchanged;
            }
            ClassLikeStringSpecifier::Generic { constraint: new_constraint }
        }
        _ => return Expansion::Unchanged,
    };
    Expansion::Single(i.intern_class_like_string(ClassLikeStringInfo { specifier: new_specifier, ..info }))
}
