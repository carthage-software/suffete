//! Capture-free substitution of template parameters in a [`Type`](crate::Type).
//!
//! Implements the substitution operator $\sigma\Theta$ from
//! `type-system/generics.md` §3 — replacing every free occurrence of a
//! template parameter inside `ty` with the type the caller's closure
//! supplies. Substitution is structural, recurses into the nested type
//! arguments of every payload that carries them, and re-canonicalises the
//! result through the interner so equal substitutions share handles.
//!
//! The closure is parameterised on [`GenericParameterInfo`] rather than on
//! `(name, defining_entity)` pairs so the caller can inspect the
//! constraint or the qualifier when deciding what to substitute. Returning
//! `None` from the closure leaves the parameter in place (per §3.1
//! `SubstMiss`); returning `Some(replacement)` performs the rewrite.

use crate::ElementId;
use crate::ElementKind;
use crate::TypeId;
use crate::element::payload::ClassLikeStringInfo;
use crate::element::payload::ClassLikeStringSpecifier;
use crate::element::payload::GenericParameterInfo;
use crate::element::payload::IterableInfo;
use crate::element::payload::KeyedArrayInfo;
use crate::element::payload::KnownItemEntry;
use crate::element::payload::ListInfo;
use crate::element::payload::ObjectInfo;
use crate::interner::interner;

/// Apply a substitution closure to every free template parameter in `ty`.
///
/// `sub` is consulted for each [`GenericParameter`](ElementKind::GenericParameter)
/// element encountered during a structural walk:
///
/// - `Some(replacement)` — the parameter is replaced; the replacement's
///   elements flow into the surrounding union.
/// - `None` — the parameter is left in place. (The closure may still
///   substitute inside its constraint by recursing if it wants to;
///   suffete does not do this automatically.)
///
/// The walk recurses into `Object` type arguments, list and keyed-array
/// element / key / value parameters, sealed-shape known items, iterable
/// key / value, and class-like-string `OfType` / `Generic` constraints.
/// Other element families pass through unchanged for now (callable
/// signatures, conditionals, derived types, intersections, references —
/// each will gain its own recursion when its substitution semantics
/// land).
///
/// Returns the same [`TypeId`] handle when nothing changed (interner
/// dedup makes repeated calls free).
pub fn substitute<F>(ty: TypeId, sub: &F) -> TypeId
where
    F: Fn(&GenericParameterInfo) -> Option<TypeId>,
{
    let i = interner();
    let original = ty.as_ref();
    let mut new_elements: Vec<ElementId> = Vec::with_capacity(original.elements.len());
    let mut changed = false;

    for &elem in original.elements {
        match substitute_in_element(elem, sub) {
            ElementSub::Unchanged => new_elements.push(elem),
            ElementSub::Single(new_elem) => {
                changed = true;
                new_elements.push(new_elem);
            }
            ElementSub::Many(elems) => {
                changed = true;
                new_elements.extend(elems);
            }
        }
    }

    if !changed {
        return ty;
    }

    i.intern_type(&new_elements, original.flags)
}

/// What `substitute_in_element` returns. `Unchanged` is the common case
/// (no template hit); `Single` covers in-place rewrites; `Many` covers
/// the case where a `GenericParameter` is replaced by a multi-element
/// union that flat-merges into the enclosing type.
enum ElementSub {
    Unchanged,
    Single(ElementId),
    Many(Vec<ElementId>),
}

fn substitute_in_element<F>(elem: ElementId, sub: &F) -> ElementSub
where
    F: Fn(&GenericParameterInfo) -> Option<TypeId>,
{
    let i = interner();
    match elem.kind() {
        ElementKind::GenericParameter => substitute_generic_parameter(elem, sub),
        ElementKind::Object => substitute_object(elem, sub),
        ElementKind::List => substitute_list(elem, sub),
        ElementKind::Array => substitute_keyed_array(elem, sub),
        ElementKind::Iterable => substitute_iterable(elem, sub),
        ElementKind::ClassLikeString => substitute_class_like_string(elem, sub),
        _ => {
            let _ = i;
            ElementSub::Unchanged
        }
    }
}

fn substitute_generic_parameter<F>(elem: ElementId, sub: &F) -> ElementSub
where
    F: Fn(&GenericParameterInfo) -> Option<TypeId>,
{
    let info = interner().get_generic_parameter(elem);
    let Some(replacement) = sub(info) else {
        return ElementSub::Unchanged;
    };

    let elements = replacement.as_ref().elements;
    if elements.len() == 1 { ElementSub::Single(elements[0]) } else { ElementSub::Many(elements.to_vec()) }
}

fn substitute_object<F>(elem: ElementId, sub: &F) -> ElementSub
where
    F: Fn(&GenericParameterInfo) -> Option<TypeId>,
{
    let i = interner();
    let info = *i.get_object(elem);
    let Some(args_id) = info.type_args else {
        return ElementSub::Unchanged;
    };

    let args = i.get_type_list(args_id);
    let new_args: Vec<TypeId> = args.iter().map(|&a| substitute(a, sub)).collect();
    if new_args.iter().zip(args.iter()).all(|(n, o)| n == o) {
        return ElementSub::Unchanged;
    }

    let new_args_id = i.intern_type_list(&new_args);
    let new_info = ObjectInfo { type_args: Some(new_args_id), ..info };
    ElementSub::Single(i.intern_object(new_info))
}

fn substitute_list<F>(elem: ElementId, sub: &F) -> ElementSub
where
    F: Fn(&GenericParameterInfo) -> Option<TypeId>,
{
    let i = interner();
    let info = *i.get_list(elem);
    let new_element_type = substitute(info.element_type, sub);

    let new_known = info.known_elements.map(|id| {
        let entries = i.get_known_elements(id);
        let new_entries: Vec<_> = entries
            .iter()
            .map(|entry| crate::element::payload::KnownElementEntry { value: substitute(entry.value, sub), ..*entry })
            .collect();
        let unchanged = new_entries.iter().zip(entries.iter()).all(|(n, o)| n.value == o.value);
        if unchanged { (id, false) } else { (i.intern_known_elements(&new_entries), true) }
    });

    let known_changed = new_known.is_some_and(|(_, ch)| ch);
    if new_element_type == info.element_type && !known_changed {
        return ElementSub::Unchanged;
    }

    let new_info = ListInfo { element_type: new_element_type, known_elements: new_known.map(|(id, _)| id), ..info };
    ElementSub::Single(i.intern_list(new_info))
}

fn substitute_keyed_array<F>(elem: ElementId, sub: &F) -> ElementSub
where
    F: Fn(&GenericParameterInfo) -> Option<TypeId>,
{
    let i = interner();
    let info = *i.get_array(elem);
    let new_key = info.key_param.map(|t| substitute(t, sub));
    let new_value = info.value_param.map(|t| substitute(t, sub));

    let new_known = info.known_items.map(|id| {
        let entries = i.get_known_items(id);
        let new_entries: Vec<KnownItemEntry> =
            entries.iter().map(|entry| KnownItemEntry { value: substitute(entry.value, sub), ..*entry }).collect();
        let unchanged = new_entries.iter().zip(entries.iter()).all(|(n, o)| n.value == o.value);
        if unchanged { (id, false) } else { (i.intern_known_items(&new_entries), true) }
    });

    let key_unchanged = new_key == info.key_param;
    let value_unchanged = new_value == info.value_param;
    let known_unchanged = new_known.is_none_or(|(_, ch)| !ch);
    if key_unchanged && value_unchanged && known_unchanged {
        return ElementSub::Unchanged;
    }

    let new_info =
        KeyedArrayInfo { key_param: new_key, value_param: new_value, known_items: new_known.map(|(id, _)| id), ..info };
    ElementSub::Single(i.intern_array(new_info))
}

fn substitute_iterable<F>(elem: ElementId, sub: &F) -> ElementSub
where
    F: Fn(&GenericParameterInfo) -> Option<TypeId>,
{
    let i = interner();
    let info = *i.get_iterable(elem);
    let new_key = substitute(info.key_type, sub);
    let new_value = substitute(info.value_type, sub);
    if new_key == info.key_type && new_value == info.value_type {
        return ElementSub::Unchanged;
    }

    let new_info = IterableInfo { key_type: new_key, value_type: new_value, ..info };
    ElementSub::Single(i.intern_iterable(new_info))
}

fn substitute_class_like_string<F>(elem: ElementId, sub: &F) -> ElementSub
where
    F: Fn(&GenericParameterInfo) -> Option<TypeId>,
{
    let i = interner();
    let info = *i.get_class_like_string(elem);
    let new_specifier = match info.specifier {
        ClassLikeStringSpecifier::OfType { constraint } => {
            let new_constraint = substitute(constraint, sub);
            if new_constraint == constraint {
                return ElementSub::Unchanged;
            }
            ClassLikeStringSpecifier::OfType { constraint: new_constraint }
        }
        ClassLikeStringSpecifier::Generic { constraint } => {
            let new_constraint = substitute(constraint, sub);
            if new_constraint == constraint {
                return ElementSub::Unchanged;
            }
            ClassLikeStringSpecifier::Generic { constraint: new_constraint }
        }
        _ => return ElementSub::Unchanged,
    };

    let new_info = ClassLikeStringInfo { specifier: new_specifier, ..info };
    ElementSub::Single(i.intern_class_like_string(new_info))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FlowFlags;
    use crate::element::payload::DefiningEntity;
    use crate::prelude::INT;
    use crate::prelude::STRING;
    use crate::prelude::TYPE_INT;
    use crate::prelude::TYPE_INT_OR_STRING;
    use crate::prelude::TYPE_MIXED;
    use crate::prelude::TYPE_STRING;

    fn class_def(name: &str) -> DefiningEntity {
        DefiningEntity::ClassLike(mago_atom::atom(name))
    }

    fn ty_of(elem: ElementId) -> TypeId {
        interner().intern_type(&[elem], FlowFlags::EMPTY)
    }

    #[test]
    fn no_template_no_change() {
        let result = substitute(TYPE_INT, &|_: &GenericParameterInfo| -> Option<TypeId> { None });
        assert_eq!(result, TYPE_INT);
    }

    #[test]
    fn unchanged_returns_same_handle() {
        // Even when the type contains a template, if the closure returns
        // None, the handle is identical (no re-interning).
        let t = ElementId::generic_parameter("T", class_def("Box"), TYPE_MIXED);
        let ty = ty_of(t);
        let result = substitute(ty, &|_| None);
        assert_eq!(result, ty);
    }

    #[test]
    fn template_replaced_with_singleton() {
        let t = ElementId::generic_parameter("T", class_def("Box"), TYPE_MIXED);
        let ty = ty_of(t);
        let result = substitute(ty, &|info| {
            assert_eq!(info.name, mago_atom::atom("T"));
            Some(TYPE_INT)
        });
        assert_eq!(result, TYPE_INT);
    }

    #[test]
    fn template_replaced_with_union_flattens() {
        let t = ElementId::generic_parameter("T", class_def("Box"), TYPE_MIXED);
        let ty = ty_of(t);
        let result = substitute(ty, &|_| Some(TYPE_INT_OR_STRING));
        assert_eq!(result, TYPE_INT_OR_STRING);
    }

    #[test]
    fn capture_freeness_qualifier_matters() {
        // Two `T`s in different defining entities are distinct.
        let outer_t = ElementId::generic_parameter("T", class_def("Outer"), TYPE_MIXED);
        let inner_t = ElementId::generic_parameter("T", class_def("Inner"), TYPE_MIXED);
        let outer_ty = ty_of(outer_t);
        let inner_ty = ty_of(inner_t);

        let outer_class = class_def("Outer");
        let result_outer = substitute(outer_ty, &|info| {
            if info.defining_entity == interner().intern_defining_entity(outer_class) { Some(TYPE_INT) } else { None }
        });
        assert_eq!(result_outer, TYPE_INT);

        // The same closure leaves Inner's T alone.
        let result_inner = substitute(inner_ty, &|info| {
            if info.defining_entity == interner().intern_defining_entity(outer_class) { Some(TYPE_INT) } else { None }
        });
        assert_eq!(result_inner, inner_ty);
    }

    #[test]
    fn distributes_over_union() {
        // Substitute T inside a union { T, string } where T -> int.
        let t = ElementId::generic_parameter("T", class_def("X"), TYPE_MIXED);
        let union = interner().intern_type(&[t, STRING], FlowFlags::EMPTY);
        let result = substitute(union, &|_| Some(TYPE_INT));
        // Result should be { int, string } regardless of input order.
        assert_eq!(result.as_ref().elements.len(), 2);
        assert!(result.as_ref().elements.contains(&INT));
        assert!(result.as_ref().elements.contains(&STRING));
    }

    #[test]
    fn substitutes_inside_object_type_args() {
        // Container<T> with T -> int becomes Container<int>.
        let t = ElementId::generic_parameter("T", class_def("Container"), TYPE_MIXED);
        let t_ty = ty_of(t);
        let container_t = {
            let info = ObjectInfo {
                name: mago_atom::atom("Container"),
                type_args: Some(interner().intern_type_list(&[t_ty])),
                intersections: None,
                flags: crate::element::payload::ObjectFlags::default(),
            };
            ty_of(interner().intern_object(info))
        };

        let container_int = {
            let info = ObjectInfo {
                name: mago_atom::atom("Container"),
                type_args: Some(interner().intern_type_list(&[TYPE_INT])),
                intersections: None,
                flags: crate::element::payload::ObjectFlags::default(),
            };
            ty_of(interner().intern_object(info))
        };

        let result = substitute(container_t, &|_| Some(TYPE_INT));
        assert_eq!(result, container_int);
    }

    #[test]
    fn substitutes_inside_list_element_type() {
        // list<T> with T -> string becomes list<string>.
        let t = ElementId::generic_parameter("T", class_def("X"), TYPE_MIXED);
        let list_t = ty_of(ElementId::list(ty_of(t), false));
        let list_string = ty_of(ElementId::list(TYPE_STRING, false));

        let result = substitute(list_t, &|_| Some(TYPE_STRING));
        assert_eq!(result, list_string);
    }

    #[test]
    fn substitutes_inside_iterable_key_and_value() {
        // iterable<K, V> with K -> int, V -> string.
        let k = ElementId::generic_parameter("K", class_def("Iter"), TYPE_MIXED);
        let v = ElementId::generic_parameter("V", class_def("Iter"), TYPE_MIXED);
        let iter_kv = ty_of(ElementId::iterable(ty_of(k), ty_of(v)));
        let iter_int_string = ty_of(ElementId::iterable(TYPE_INT, TYPE_STRING));

        let iter_def = interner().intern_defining_entity(class_def("Iter"));
        let result = substitute(iter_kv, &|info| {
            if info.defining_entity != iter_def {
                return None;
            }
            match info.name.as_str() {
                "K" => Some(TYPE_INT),
                "V" => Some(TYPE_STRING),
                _ => None,
            }
        });
        assert_eq!(result, iter_int_string);
    }

    #[test]
    fn substitutes_inside_keyed_array_value() {
        // array<string, T> with T -> int becomes array<string, int>.
        let t = ElementId::generic_parameter("T", class_def("X"), TYPE_MIXED);
        let arr_t = ty_of(ElementId::keyed_unsealed(TYPE_STRING, ty_of(t), false));
        let arr_int = ty_of(ElementId::keyed_unsealed(TYPE_STRING, TYPE_INT, false));

        let result = substitute(arr_t, &|_| Some(TYPE_INT));
        assert_eq!(result, arr_int);
    }

    #[test]
    fn nested_object_substitution() {
        // Container<List<T>> with T -> int becomes Container<List<int>>.
        let t = ElementId::generic_parameter("T", class_def("X"), TYPE_MIXED);
        let list_t_ty = ty_of(ElementId::list(ty_of(t), false));
        let container_list_t = {
            let info = ObjectInfo {
                name: mago_atom::atom("Container"),
                type_args: Some(interner().intern_type_list(&[list_t_ty])),
                intersections: None,
                flags: crate::element::payload::ObjectFlags::default(),
            };
            ty_of(interner().intern_object(info))
        };

        let list_int_ty = ty_of(ElementId::list(TYPE_INT, false));
        let container_list_int = {
            let info = ObjectInfo {
                name: mago_atom::atom("Container"),
                type_args: Some(interner().intern_type_list(&[list_int_ty])),
                intersections: None,
                flags: crate::element::payload::ObjectFlags::default(),
            };
            ty_of(interner().intern_object(info))
        };

        let result = substitute(container_list_t, &|_| Some(TYPE_INT));
        assert_eq!(result, container_list_int);
    }
}
