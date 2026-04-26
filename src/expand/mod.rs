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
//! - **Stage 1:** `Alias` resolution.
//! - **Stage 2:** `Reference` resolution (`SymbolReference`,
//!   `MemberReference`, `GlobalReference`).
//! - **Stage 3 (current):** `Derived` evaluation. `KeyOf`, `ValueOf`,
//!   `IndexAccess`, `IntMask`, `IntMaskOf`, and `TemplateType` resolve
//!   structurally; `PropertiesOf` and `New` pass through until they
//!   gain dedicated `World` queries (property enumeration and
//!   constructor-signature lookup).
//! - **Stage 4:** Contextual keyword substitution (`self`, `static`,
//!   `parent`) and `Conditional` evaluation.
//!
//! # Structural descent
//!
//! Per `§7.4`, expansion descends into every nested type — `Object`
//! type arguments, list / keyed-array / iterable element-types, sealed
//! known items, and class-like-string constraints. Element kinds whose
//! payloads do not (yet) carry nested types pass through unchanged.

use mago_atom::Atom;

use crate::ElementId;
use crate::ElementKind;
use crate::TypeId;
use crate::element::payload::ArrayKey;
use crate::element::payload::ClassLikeStringInfo;
use crate::element::payload::ClassLikeStringSpecifier;
use crate::element::payload::DerivedInfo;
use crate::element::payload::IterableInfo;
use crate::element::payload::KeyedArrayInfo;
use crate::element::payload::KnownItemEntry;
use crate::element::payload::ListInfo;
use crate::element::payload::NameSelector;
use crate::element::payload::ObjectFlags;
use crate::element::payload::ObjectInfo;
use crate::element::payload::scalar::IntInfo;
use crate::interner::interner;
use crate::prelude::NON_NEGATIVE_INT;
use crate::prelude::TYPE_INT;
use crate::prelude::TYPE_MIXED;
use crate::prelude::TYPE_NEVER;
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
        ElementKind::Reference => expand_reference(elem, world),
        ElementKind::MemberReference => expand_member_reference(elem, world),
        ElementKind::GlobalReference => expand_global_reference(elem, world),
        ElementKind::Derived => expand_derived(elem, world),
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

/// `SymbolReference("Foo", type_args, intersections)` is, semantically,
/// the same value-set as `Object("Foo", ...)`. Convert it; type args
/// and intersection conjuncts are recursively expanded so a reference
/// like `Foo<MyAlias>` resolves to `Object("Foo", [<expanded alias>])`.
fn expand_reference<W: World>(elem: ElementId, world: &W) -> Expansion {
    let i = interner();
    let info = *i.get_reference(elem);

    let new_args = info.type_args.map(|id| {
        let args = i.get_type_list(id);
        let expanded: Vec<TypeId> = args.iter().map(|&a| expand(a, world)).collect();
        i.intern_type_list(&expanded)
    });

    let new_intersections = info.intersections.map(|id| {
        let conjuncts = i.get_element_list(id);
        let expanded: Vec<ElementId> = conjuncts
            .iter()
            .flat_map(|&c| match expand_element(c, world) {
                Expansion::Unchanged => vec![c],
                Expansion::Single(e) => vec![e],
                Expansion::Many(es) => es,
            })
            .collect();
        i.intern_element_list(&expanded)
    });

    Expansion::Single(i.intern_object(ObjectInfo {
        name: info.name,
        type_args: new_args,
        intersections: new_intersections,
        flags: ObjectFlags::default(),
    }))
}

/// `Foo::CONST` (an `Identifier` selector on a `MemberReference`)
/// resolves to the constant's declared type via
/// [`World::class_constant_type`]. Other selectors (wildcard / prefix /
/// suffix) need a constant-enumeration query and pass through
/// unchanged for now.
fn expand_member_reference<W: World>(elem: ElementId, world: &W) -> Expansion {
    let info = interner().get_member_reference(elem);
    let NameSelector::Identifier(constant) = info.selector else {
        return Expansion::Unchanged;
    };
    let Some(body) = world.class_constant_type(info.class_like_name, constant) else {
        return Expansion::Unchanged;
    };
    let expanded = expand(body, world);
    let elements = expanded.as_ref().elements;
    if elements.len() == 1 { Expansion::Single(elements[0]) } else { Expansion::Many(elements.to_vec()) }
}

/// A global constant reference resolves through
/// [`World::global_constant_type`]. Wildcard selectors pass through.
fn expand_global_reference<W: World>(elem: ElementId, world: &W) -> Expansion {
    let info = interner().get_global_reference(elem);
    let NameSelector::Identifier(name) = info.selector else {
        return Expansion::Unchanged;
    };
    let Some(body) = world.global_constant_type(name) else {
        return Expansion::Unchanged;
    };
    let expanded = expand(body, world);
    let elements = expanded.as_ref().elements;
    if elements.len() == 1 { Expansion::Single(elements[0]) } else { Expansion::Many(elements.to_vec()) }
}

fn expand_derived<W: World>(elem: ElementId, world: &W) -> Expansion {
    let info = *interner().get_derived(elem);
    match info {
        DerivedInfo::KeyOf(target) => derived_to_expansion(eval_key_of(target, world)),
        DerivedInfo::ValueOf(target) => derived_to_expansion(eval_value_of(target, world)),
        DerivedInfo::IndexAccess { target, index } => derived_to_expansion(eval_index_access(target, index, world)),
        DerivedInfo::IntMask(operands) => derived_to_expansion(eval_int_mask(operands, world)),
        DerivedInfo::IntMaskOf(target) => derived_to_expansion(eval_int_mask_of(target, world)),
        DerivedInfo::TemplateType { object: _, class_name, template_name } => {
            match eval_template_type(class_name, template_name, world) {
                Some(t) => derived_to_expansion(t),
                None => Expansion::Unchanged,
            }
        }
        DerivedInfo::PropertiesOf { .. } | DerivedInfo::New(_) => Expansion::Unchanged,
    }
}

fn derived_to_expansion(ty: TypeId) -> Expansion {
    let elements = ty.as_ref().elements;
    if elements.is_empty() {
        Expansion::Single(crate::prelude::NEVER)
    } else if elements.len() == 1 {
        Expansion::Single(elements[0])
    } else {
        Expansion::Many(elements.to_vec())
    }
}

/// `key-of<τ>` per spec §7.3: keys admissible by `τ`.
///
/// - `list`: sealed length `n` yields the integer range `[0, n-1]`;
///   unsealed yields the union of literal known indices and
///   `non-negative-int`.
/// - `array<K, V>` (keyed): the literal known keys joined with `K` when
///   the shape is unsealed.
/// - `iterable<K, V>`: `K` directly.
/// - Anything else (including unions of unhandled atoms): `mixed`.
fn eval_key_of<W: World>(target: TypeId, world: &W) -> TypeId {
    let target = expand(target, world);
    let elems = target.as_ref().elements;
    if elems.len() != 1 {
        return TYPE_MIXED;
    }
    let only = elems[0];
    let i = interner();
    match only.kind() {
        ElementKind::List => {
            let info = *i.get_list(only);
            let mut keys: Vec<ElementId> = Vec::new();
            if let Some(known_id) = info.known_elements {
                for entry in i.get_known_elements(known_id) {
                    keys.push(ElementId::int_literal(entry.index as i64));
                }
            }
            match info.known_count {
                Some(n) => {
                    let n = n.get() as i64;
                    keys.push(ElementId::int_range(Some(0), Some(n - 1)));
                }
                None => {
                    keys.push(NON_NEGATIVE_INT);
                }
            }
            TypeId::union(&keys)
        }
        ElementKind::Array => {
            let info = *i.get_array(only);
            let mut keys: Vec<ElementId> = Vec::new();
            if let Some(known_id) = info.known_items {
                for entry in i.get_known_items(known_id) {
                    if let Some(k) = array_key_to_element(entry.key) {
                        keys.push(k);
                    }
                }
            }
            if let Some(k_param) = info.key_param {
                for &el in k_param.as_ref().elements {
                    keys.push(el);
                }
            }
            if keys.is_empty() { TYPE_MIXED } else { TypeId::union(&keys) }
        }
        ElementKind::Iterable => i.get_iterable(only).key_type,
        _ => TYPE_MIXED,
    }
}

/// `value-of<τ>` per spec §7.3: values admissible by `τ`.
fn eval_value_of<W: World>(target: TypeId, world: &W) -> TypeId {
    let target = expand(target, world);
    let elems = target.as_ref().elements;
    if elems.len() != 1 {
        return TYPE_MIXED;
    }
    let only = elems[0];
    let i = interner();
    match only.kind() {
        ElementKind::List => {
            let info = *i.get_list(only);
            let mut values: Vec<ElementId> = Vec::new();
            if let Some(known_id) = info.known_elements {
                for entry in i.get_known_elements(known_id) {
                    values.extend_from_slice(entry.value.as_ref().elements);
                }
            }
            values.extend_from_slice(info.element_type.as_ref().elements);
            TypeId::union(&values)
        }
        ElementKind::Array => {
            let info = *i.get_array(only);
            let mut values: Vec<ElementId> = Vec::new();
            if let Some(known_id) = info.known_items {
                for entry in i.get_known_items(known_id) {
                    values.extend_from_slice(entry.value.as_ref().elements);
                }
            }
            if let Some(v_param) = info.value_param {
                values.extend_from_slice(v_param.as_ref().elements);
            }
            if values.is_empty() { TYPE_MIXED } else { TypeId::union(&values) }
        }
        ElementKind::Iterable => i.get_iterable(only).value_type,
        _ => TYPE_MIXED,
    }
}

/// `τ[κ]` per spec §7.3.
fn eval_index_access<W: World>(target: TypeId, index: TypeId, world: &W) -> TypeId {
    let target = expand(target, world);
    let index = expand(index, world);
    let target_elems = target.as_ref().elements;
    let index_elems = index.as_ref().elements;
    if target_elems.len() != 1 || index_elems.len() != 1 {
        return TYPE_MIXED;
    }
    let only = target_elems[0];
    let key_elem = index_elems[0];
    let i = interner();
    match only.kind() {
        ElementKind::Array => {
            let info = *i.get_array(only);
            if let Some(known_id) = info.known_items
                && let Some(literal_key) = element_to_array_key(key_elem)
            {
                for entry in i.get_known_items(known_id) {
                    if entry.key == literal_key {
                        return entry.value;
                    }
                }
            }
            info.value_param.unwrap_or(TYPE_NEVER)
        }
        ElementKind::List => {
            let info = *i.get_list(only);
            if let Some(idx) = literal_int(key_elem)
                && idx >= 0
                && let Some(known_id) = info.known_elements
            {
                for entry in i.get_known_elements(known_id) {
                    if entry.index as i64 == idx {
                        return entry.value;
                    }
                }
            }
            info.element_type
        }
        ElementKind::Iterable => i.get_iterable(only).value_type,
        _ => TYPE_MIXED,
    }
}

/// `int-mask<n_1, n_2, …>`: every distinct bitwise OR of subsets of the
/// operand literal integers, presented as integer literals. Operands
/// that don't reduce to a single integer literal cause the mask to fall
/// back to `mixed`.
fn eval_int_mask<W: World>(operands: crate::TypeListId, world: &W) -> TypeId {
    let i = interner();
    let raw = i.get_type_list(operands);
    let mut literals: Vec<i64> = Vec::with_capacity(raw.len());
    for &operand in raw {
        let expanded = expand(operand, world);
        let elems = expanded.as_ref().elements;
        if elems.len() != 1 {
            return TYPE_MIXED;
        }
        match literal_int(elems[0]) {
            Some(v) => literals.push(v),
            None => return TYPE_MIXED,
        }
    }
    int_mask_union(&literals)
}

/// `int-mask-of<τ>`: `τ` expands to a union of integer literals, then
/// the mask of those literals.
fn eval_int_mask_of<W: World>(target: TypeId, world: &W) -> TypeId {
    let expanded = expand(target, world);
    let mut literals: Vec<i64> = Vec::new();
    for &el in expanded.as_ref().elements {
        match literal_int(el) {
            Some(v) => literals.push(v),
            None => return TYPE_MIXED,
        }
    }
    int_mask_union(&literals)
}

fn int_mask_union(literals: &[i64]) -> TypeId {
    let n = literals.len();
    if n == 0 {
        return TypeId::union(&[ElementId::int_literal(0)]);
    }
    if n > 16 {
        // 2^16 = 65,536 subsets is the practical ceiling; beyond that
        // the mask widens to `int` to keep the operation cheap.
        return TYPE_INT;
    }
    let total = 1u32 << n;
    let mut values: std::collections::BTreeSet<i64> = std::collections::BTreeSet::new();
    for mask in 0..total {
        let mut acc: i64 = 0;
        for (bit, &lit) in literals.iter().enumerate() {
            if (mask >> bit) & 1 == 1 {
                acc |= lit;
            }
        }
        values.insert(acc);
    }
    let elems: Vec<ElementId> = values.into_iter().map(ElementId::int_literal).collect();
    TypeId::union(&elems)
}

/// `template-type<$object, C, T>`: `T`'s constraint as declared on `C`.
/// The `$object` operand exists for the analyser-side instantiation
/// context and is ignored here. Returns `None` when the class or the
/// template name cannot be extracted from their `TypeId` operands, so
/// the caller leaves the form unexpanded.
fn eval_template_type<W: World>(class_name: TypeId, template_name: TypeId, world: &W) -> Option<TypeId> {
    let class = single_object_or_reference_name(class_name)?;
    let template = single_string_literal_atom(template_name)?;
    let position = world.template_parameter_index(class, template)?;
    let parameter = world.template_parameter_at(class, position)?;
    Some(expand(parameter.upper_bound.unwrap_or(TYPE_MIXED), world))
}

fn single_object_or_reference_name(ty: TypeId) -> Option<Atom> {
    let elems = ty.as_ref().elements;
    if elems.len() != 1 {
        return None;
    }
    let i = interner();
    match elems[0].kind() {
        ElementKind::Object => Some(i.get_object(elems[0]).name),
        ElementKind::Reference => Some(i.get_reference(elems[0]).name),
        _ => None,
    }
}

fn single_string_literal_atom(ty: TypeId) -> Option<Atom> {
    let elems = ty.as_ref().elements;
    if elems.len() != 1 || elems[0].kind() != ElementKind::String {
        return None;
    }
    use crate::element::payload::scalar::StringLiteral;
    match interner().get_string(elems[0]).literal {
        StringLiteral::Value(atom) => Some(atom),
        _ => None,
    }
}

fn array_key_to_element(key: ArrayKey) -> Option<ElementId> {
    match key {
        ArrayKey::Int(n) => Some(ElementId::int_literal(n)),
        ArrayKey::String(atom) => Some(ElementId::string_literal(atom.as_str())),
        ArrayKey::Const { .. } => None,
    }
}

fn element_to_array_key(elem: ElementId) -> Option<ArrayKey> {
    let i = interner();
    match elem.kind() {
        ElementKind::Int => match i.get_int(elem) {
            IntInfo::Literal(n) => Some(ArrayKey::Int(*n)),
            _ => None,
        },
        ElementKind::String => {
            use crate::element::payload::scalar::StringLiteral;
            match i.get_string(elem).literal {
                StringLiteral::Value(atom) => Some(ArrayKey::String(atom)),
                _ => None,
            }
        }
        _ => None,
    }
}

fn literal_int(elem: ElementId) -> Option<i64> {
    if elem.kind() != ElementKind::Int {
        return None;
    }
    match interner().get_int(elem) {
        IntInfo::Literal(n) => Some(*n),
        _ => None,
    }
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
