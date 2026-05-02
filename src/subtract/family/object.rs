//! `Object \ B` precision via `Negated` conjuncts on the surviving
//! object's intersection list.

use crate::ElementId;
use crate::ElementKind;
use crate::FlowFlags;
use crate::element::payload::ObjectInfo;
use crate::interner::interner;
use crate::world::World;

/// `Object \ B` records `b` as a `Negated` conjunct on `a`'s
/// intersection list. `b` may be another `Object` (descendant,
/// sibling, or same-class with different args) or a structural
/// conjunct (`HasMethod` / `HasProperty` / `ObjectShape`). For
/// the strict bare descendant case the negation binds to the
/// bare ancestor class so the whole nominal subtree is excluded.
pub(in crate::subtract) fn object_descendant_minus<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
) -> Option<Vec<ElementId>> {
    if a.kind() != ElementKind::Object {
        return None;
    }
    let b_is_object = b.kind() == ElementKind::Object;
    let b_is_structural =
        matches!(b.kind(), ElementKind::HasMethod | ElementKind::HasProperty | ElementKind::ObjectShape);
    if !b_is_object && !b_is_structural {
        return None;
    }
    let i = interner();
    let a_info = *i.get_object(a);

    let strict_bare_descendant = if b_is_object {
        let b_info = *i.get_object(b);
        let descends = a_info.name != b_info.name && world.descends_from(b_info.name, a_info.name);
        descends && b_info.type_args.is_none() && b_info.intersections.is_none()
    } else {
        false
    };

    let exclude_atom = if strict_bare_descendant {
        let b_info = *i.get_object(b);
        i.intern_object(ObjectInfo { intersections: None, ..b_info })
    } else {
        b
    };

    let exclude_ty = i.intern_type(&[exclude_atom], FlowFlags::EMPTY);
    let new_negated = ElementId::negated(exclude_ty);

    let mut conjuncts: Vec<ElementId> = Vec::new();
    if let Some(id) = a_info.intersections {
        for &existing in i.get_element_list(id) {
            if strict_bare_descendant && existing.kind() == ElementKind::Negated {
                let neg_info = *i.get_negated(existing);
                let inner_elements = neg_info.inner.as_ref().elements;
                if inner_elements.len() == 1 && inner_elements[0].kind() == ElementKind::Object {
                    let existing_info = *i.get_object(inner_elements[0]);
                    let b_info = *i.get_object(b);
                    if world.descends_from(b_info.name, existing_info.name) {
                        return Some(vec![a]);
                    }
                }
            }

            conjuncts.push(existing);
        }
    }

    if !conjuncts.contains(&new_negated) {
        conjuncts.push(new_negated);
    }

    conjuncts.sort();

    let new_info = ObjectInfo { intersections: Some(i.intern_element_list(&conjuncts)), ..a_info };
    Some(vec![i.intern_object(new_info)])
}
