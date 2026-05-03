//! Capture-free substitution of template parameters in a [`Type`](crate::Type).
//!
//! Replaces every free occurrence of a template parameter inside `ty`
//! with the type the caller's closure supplies. The structural walk is
//! delegated to [`crate::transform::flat_map`]; this module only owns
//! the per-element decision.
//!
//! The closure is parameterised on [`GenericParameterInfo`] rather
//! than on `(name, defining_entity)` pairs so the caller can inspect
//! the constraint or the qualifier when deciding what to substitute.
//! Returning `None` from the closure leaves the parameter in place;
//! returning `Some(replacement)` performs the rewrite.

use crate::ElementId;
use crate::ElementKind;
use crate::TypeId;
use crate::element::payload::GenericParameterInfo;
use crate::interner::interner;
use crate::transform;

/// Apply a substitution closure to every free template parameter in
/// `ty`.
///
/// `sub` is consulted for each
/// [`GenericParameter`](ElementKind::GenericParameter) element
/// encountered during the structural walk:
///
/// - `Some(replacement)`: the parameter is replaced; the
///   replacement's elements flow into the surrounding union.
/// - `None`: the parameter is left in place. The closure may still
///   substitute inside the parameter's constraint by recursing if it
///   wants to; suffete does not do this automatically.
///
/// Returns the same [`TypeId`] handle when nothing changed.
#[inline]
pub fn substitute<F>(ty: TypeId, sub: &F) -> TypeId
where
    F: Fn(&GenericParameterInfo) -> Option<TypeId>,
{
    transform::flat_map(ty, |elem| substitute_element(elem, sub))
}

#[inline]
fn substitute_element<F>(elem: ElementId, sub: &F) -> Vec<ElementId>
where
    F: Fn(&GenericParameterInfo) -> Option<TypeId>,
{
    if elem.kind() != ElementKind::GenericParameter {
        return vec![elem];
    }
    let info = interner().get_generic_parameter(elem);
    match sub(info) {
        Some(replacement) => replacement.as_ref().elements.to_vec(),
        None => vec![elem],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FlowFlags;
    use crate::element::payload::DefiningEntity;
    use crate::element::payload::ObjectInfo;
    use crate::prelude::INT;
    use crate::prelude::STRING;
    use crate::prelude::TYPE_INT;
    use crate::prelude::TYPE_INT_OR_STRING;
    use crate::prelude::TYPE_MIXED;
    use crate::prelude::TYPE_STRING;

    #[inline]
    fn class_def(name: &str) -> DefiningEntity {
        DefiningEntity::ClassLike(mago_atom::atom(name))
    }

    #[inline]
    fn ty_of(elem: ElementId) -> TypeId {
        interner().intern_type(&[elem], FlowFlags::EMPTY)
    }

    #[test]
    #[inline]
    fn no_template_no_change() {
        let result = substitute(TYPE_INT, &|_: &GenericParameterInfo| -> Option<TypeId> { None });
        assert_eq!(result, TYPE_INT);
    }

    #[test]
    #[inline]
    fn substitutes_top_level_template() {
        let t = ElementId::generic_parameter("T", class_def("X"), TYPE_MIXED);
        let t_ty = ty_of(t);
        let result = substitute(t_ty, &|_| Some(TYPE_INT));
        assert_eq!(result, TYPE_INT);
    }

    #[test]
    #[inline]
    fn miss_leaves_template_in_place() {
        let t = ElementId::generic_parameter("T", class_def("X"), TYPE_MIXED);
        let t_ty = ty_of(t);
        let result = substitute(t_ty, &|_| None);
        assert_eq!(result, t_ty);
    }

    #[test]
    #[inline]
    fn replacement_with_union_flat_merges_into_parent() {
        let t = ElementId::generic_parameter("T", class_def("X"), TYPE_MIXED);
        let union_t = interner().intern_type(&[t, INT], FlowFlags::EMPTY);
        let int_or_string = interner().intern_type(&[INT, STRING], FlowFlags::EMPTY);
        let result = substitute(union_t, &|_| Some(int_or_string));
        assert_eq!(result, TYPE_INT_OR_STRING);
    }

    #[test]
    #[inline]
    fn unchanged_returns_same_handle() {
        let t = ElementId::generic_parameter("T", class_def("Box"), TYPE_MIXED);
        let ty = ty_of(t);
        let result = substitute(ty, &|_| None);
        assert_eq!(result, ty);
    }

    #[test]
    #[inline]
    fn capture_freeness_qualifier_matters() {
        let outer_t = ElementId::generic_parameter("T", class_def("Outer"), TYPE_MIXED);
        let inner_t = ElementId::generic_parameter("T", class_def("Inner"), TYPE_MIXED);
        let outer_ty = ty_of(outer_t);
        let inner_ty = ty_of(inner_t);

        let outer_class = class_def("Outer");
        let result_outer = substitute(outer_ty, &|info| {
            if info.defining_entity == interner().intern_defining_entity(outer_class) { Some(TYPE_INT) } else { None }
        });
        assert_eq!(result_outer, TYPE_INT);

        let result_inner = substitute(inner_ty, &|info| {
            if info.defining_entity == interner().intern_defining_entity(outer_class) { Some(TYPE_INT) } else { None }
        });
        assert_eq!(result_inner, inner_ty);
    }

    #[test]
    #[inline]
    fn distributes_over_union() {
        let t = ElementId::generic_parameter("T", class_def("X"), TYPE_MIXED);
        let union = interner().intern_type(&[t, STRING], FlowFlags::EMPTY);
        let result = substitute(union, &|_| Some(TYPE_INT));
        assert_eq!(result.as_ref().elements.len(), 2);
        assert!(result.as_ref().elements.contains(&INT));
        assert!(result.as_ref().elements.contains(&STRING));
    }

    #[test]
    #[inline]
    fn substitutes_inside_object_type_args() {
        let t = ElementId::generic_parameter("T", class_def("Container"), TYPE_MIXED);
        let t_ty = ty_of(t);
        let container_t = {
            let info = ObjectInfo {
                name: mago_atom::atom("Container"),
                type_args: Some(interner().intern_type_list(&[t_ty])),
                flags: crate::element::payload::ObjectFlags::default(),
            };
            ty_of(interner().intern_object(info))
        };

        let container_int = {
            let info = ObjectInfo {
                name: mago_atom::atom("Container"),
                type_args: Some(interner().intern_type_list(&[TYPE_INT])),
                flags: crate::element::payload::ObjectFlags::default(),
            };
            ty_of(interner().intern_object(info))
        };

        let result = substitute(container_t, &|_| Some(TYPE_INT));
        assert_eq!(result, container_int);
    }

    #[test]
    #[inline]
    fn substitutes_inside_list_element_type() {
        let t = ElementId::generic_parameter("T", class_def("X"), TYPE_MIXED);
        let list_t = ty_of(ElementId::list(ty_of(t), false));
        let list_string = ty_of(ElementId::list(TYPE_STRING, false));

        let result = substitute(list_t, &|_| Some(TYPE_STRING));
        assert_eq!(result, list_string);
    }

    #[test]
    #[inline]
    fn substitutes_inside_iterable_key_and_value() {
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
    #[inline]
    fn substitutes_inside_keyed_array_value() {
        let t = ElementId::generic_parameter("T", class_def("X"), TYPE_MIXED);
        let arr_t = ty_of(ElementId::keyed_unsealed(TYPE_STRING, ty_of(t), false));
        let arr_int = ty_of(ElementId::keyed_unsealed(TYPE_STRING, TYPE_INT, false));

        let result = substitute(arr_t, &|_| Some(TYPE_INT));
        assert_eq!(result, arr_int);
    }

    #[test]
    #[inline]
    fn nested_object_substitution() {
        let t = ElementId::generic_parameter("T", class_def("X"), TYPE_MIXED);
        let list_t_ty = ty_of(ElementId::list(ty_of(t), false));
        let container_list_t = {
            let info = ObjectInfo {
                name: mago_atom::atom("Container"),
                type_args: Some(interner().intern_type_list(&[list_t_ty])),
                flags: crate::element::payload::ObjectFlags::default(),
            };
            ty_of(interner().intern_object(info))
        };

        let list_int_ty = ty_of(ElementId::list(TYPE_INT, false));
        let container_list_int = {
            let info = ObjectInfo {
                name: mago_atom::atom("Container"),
                type_args: Some(interner().intern_type_list(&[list_int_ty])),
                flags: crate::element::payload::ObjectFlags::default(),
            };
            ty_of(interner().intern_object(info))
        };

        let result = substitute(container_list_t, &|_| Some(TYPE_INT));
        assert_eq!(result, container_list_int);
    }
}
