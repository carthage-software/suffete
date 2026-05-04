//! `Object \ B` precision via a `Negated` conjunct on the surviving
//! object, expressed through the
//! [`Intersected`](crate::ElementKind::Intersected) wrapper.

use crate::ElementId;
use crate::ElementKind;
use crate::FlowFlags;
use crate::interner::interner;
use crate::world::World;

/// `Object \ B` records `b` as a `Negated` conjunct of `a`. `b` may be
/// another `Object` (descendant, sibling, or same-class with different
/// args) or a structural conjunct (`HasMethod` / `HasProperty` /
/// `ObjectShape`). For the strict bare-descendant case the exclusion
/// binds to the bare descendant class so the whole nominal subtree is
/// excluded.
pub(in crate::subtract) fn object_descendant_minus<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
) -> Option<Vec<ElementId>> {
    if a.kind() != ElementKind::Object {
        return None;
    }

    let b_is_object = b.kind() == ElementKind::Object;
    let b_is_intersected = b.kind() == ElementKind::Intersected;
    let b_is_structural =
        matches!(b.kind(), ElementKind::HasMethod | ElementKind::HasProperty | ElementKind::ObjectShape);

    if !b_is_object && !b_is_structural && !b_is_intersected {
        return None;
    }

    let i = interner();
    let a_info = *i.get_object(a);

    let (head, exclude_atom) = if b_is_intersected {
        let info = *i.get_intersected(b);
        let head_is_object = info.head.kind() == ElementKind::Object;
        if head_is_object {
            let b_info = *i.get_object(info.head);
            let descends = a_info.name != b_info.name
                && world.descends_from(b_info.name, a_info.name)
                && b_info.type_args.is_none();

            let atom = if descends { i.intern_object(b_info) } else { b };
            (Some(info.head), atom)
        } else if matches!(
            info.head.kind(),
            ElementKind::HasMethod | ElementKind::HasProperty | ElementKind::ObjectShape
        ) {
            (Some(info.head), b)
        } else {
            (None, b)
        }
    } else if b_is_object {
        let b_info = *i.get_object(b);
        let descends = a_info.name != b_info.name && world.descends_from(b_info.name, a_info.name);

        let atom = if descends && b_info.type_args.is_none() { i.intern_object(b_info) } else { b };
        (None, atom)
    } else {
        (None, b)
    };

    if head.is_none() && b_is_intersected {
        return None;
    }

    let exclude_ty = i.intern_type(&[exclude_atom], FlowFlags::EMPTY);
    let new_negated = ElementId::negated(exclude_ty);
    Some(vec![ElementId::intersected(a, &[new_negated])])
}
