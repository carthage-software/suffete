//! Internal: the structural walker that backs every public function
//! in [`crate::transform`]. One implementation, four entry-point
//! shapes [`map`], [`flat_map`], [`filter_map`], [`filter`].
//!
//! The walker is post-order. For each element:
//!
//! 1. Recurse into every nested `TypeId` carried in the element's
//!    payload, transforming each via [`walk`] with the same closure.
//! 2. If any nested `TypeId` changed, re-intern the element with the
//!    rebuilt payload.
//! 3. Run the user closure on the (possibly rebuilt) element. The
//!    closure decides whether to drop, replace with one element, or
//!    expand to many.
//!
//! Each level commits with a single `intern_type` call. Nothing is
//! interned redundantly between levels.

use crate::ElementId;
use crate::TypeId;
use crate::element::payload::ClassLikeStringInfo;
use crate::element::payload::ClassLikeStringSpecifier;
use crate::element::payload::ConditionalInfo;
use crate::element::payload::DerivedInfo;
use crate::element::payload::GenericParameterInfo;
use crate::element::payload::IterableInfo;
use crate::element::payload::KeyedArrayInfo;
use crate::element::payload::KnownItemEntry;
use crate::element::payload::KnownPropertyEntry;
use crate::element::payload::ListInfo;
use crate::element::payload::ObjectInfo;
use crate::element::payload::ObjectShapeInfo;
use crate::element::payload::ParamInfo;
use crate::element::payload::Signature;
use crate::element::payload::SymbolReference;
use crate::interner::interner;

/// What the per-element closure returns. The walker translates this
/// into either a no-op, an in-place replacement, a 1→N expansion, or
/// an outright drop.
pub(super) enum Outcome {
    Unchanged,
    Single(ElementId),
    Many(Vec<ElementId>),
    Drop,
}

/// Walk `ty` post-order, applying `f` at every element position
/// (deep through every nested-type carrier). Returns the original
/// `TypeId` when nothing changed; otherwise interns the rebuilt
/// element list once, with the original flow flags preserved.
pub(super) fn walk<F: FnMut(ElementId) -> Outcome>(ty: TypeId, f: &mut F) -> TypeId {
    let view = ty.as_ref();
    let mut new_elements: Vec<ElementId> = Vec::with_capacity(view.elements.len());
    let mut changed = false;

    for &elem in view.elements {
        let rebuilt = walk_nested(elem, f);
        let target = match rebuilt {
            Some(e) => {
                changed = true;
                e
            }
            None => elem,
        };
        match f(target) {
            Outcome::Unchanged => new_elements.push(target),
            Outcome::Single(replaced) => {
                changed = true;
                new_elements.push(replaced);
            }
            Outcome::Many(replaced) => {
                changed = true;
                new_elements.extend(replaced);
            }
            Outcome::Drop => {
                changed = true;
            }
        }
    }

    if !changed {
        return ty;
    }
    interner().intern_type(&new_elements, ty.flags())
}

/// Recurse into every nested `TypeId` carried by `elem`'s payload.
/// Returns `Some(rebuilt_element)` when at least one nested type
/// changed, `None` otherwise.
fn walk_nested<F: FnMut(ElementId) -> Outcome>(elem: ElementId, f: &mut F) -> Option<ElementId> {
    use crate::ElementKind;
    match elem.kind() {
        ElementKind::Object => walk_object(elem, f),
        ElementKind::List => walk_list(elem, f),
        ElementKind::Array => walk_keyed_array(elem, f),
        ElementKind::Iterable => walk_iterable(elem, f),
        ElementKind::ObjectShape => walk_object_shape(elem, f),
        ElementKind::ClassLikeString => walk_class_like_string(elem, f),
        ElementKind::GenericParameter => walk_generic_parameter(elem, f),
        ElementKind::Reference => walk_reference(elem, f),
        ElementKind::Conditional => walk_conditional(elem, f),
        ElementKind::Derived => walk_derived(elem, f),
        ElementKind::Callable => walk_callable(elem, f),
        _ => None,
    }
}

fn walk_object<F: FnMut(ElementId) -> Outcome>(elem: ElementId, f: &mut F) -> Option<ElementId> {
    let i = interner();
    let info = *i.get_object(elem);

    let new_args = info.type_args.and_then(|id| {
        let args = i.get_type_list(id);
        let walked: Vec<TypeId> = args.iter().map(|&a| walk(a, f)).collect();
        if walked.iter().zip(args.iter()).all(|(w, o)| w == o) { None } else { Some(i.intern_type_list(&walked)) }
    });

    let new_intersections = info.intersections.and_then(|id| {
        let conjuncts = i.get_element_list(id);
        let walked: Vec<ElementId> = conjuncts.iter().map(|&c| walk_nested(c, f).unwrap_or(c)).collect();
        if walked.iter().zip(conjuncts.iter()).all(|(w, o)| w == o) {
            None
        } else {
            Some(i.intern_element_list(&walked))
        }
    });

    if new_args.is_none() && new_intersections.is_none() {
        return None;
    }

    Some(i.intern_object(ObjectInfo {
        type_args: new_args.or(info.type_args),
        intersections: new_intersections.or(info.intersections),
        ..info
    }))
}

fn walk_list<F: FnMut(ElementId) -> Outcome>(elem: ElementId, f: &mut F) -> Option<ElementId> {
    let i = interner();
    let info = *i.get_list(elem);
    let new_elem_t = walk(info.element_type, f);

    let new_known = info.known_elements.and_then(|id| {
        let entries = i.get_known_elements(id);
        let walked: Vec<_> = entries
            .iter()
            .map(|entry| crate::element::payload::KnownElementEntry { value: walk(entry.value, f), ..*entry })
            .collect();
        if walked.iter().zip(entries.iter()).all(|(w, o)| w.value == o.value) {
            None
        } else {
            Some(i.intern_known_elements(&walked))
        }
    });

    if new_elem_t == info.element_type && new_known.is_none() {
        return None;
    }
    Some(i.intern_list(ListInfo {
        element_type: new_elem_t,
        known_elements: new_known.or(info.known_elements),
        ..info
    }))
}

fn walk_keyed_array<F: FnMut(ElementId) -> Outcome>(elem: ElementId, f: &mut F) -> Option<ElementId> {
    let i = interner();
    let info = *i.get_array(elem);
    let new_key = info.key_param.map(|t| walk(t, f));
    let new_value = info.value_param.map(|t| walk(t, f));

    let new_known = info.known_items.and_then(|id| {
        let entries = i.get_known_items(id);
        let walked: Vec<KnownItemEntry> =
            entries.iter().map(|entry| KnownItemEntry { value: walk(entry.value, f), ..*entry }).collect();
        if walked.iter().zip(entries.iter()).all(|(w, o)| w.value == o.value) {
            None
        } else {
            Some(i.intern_known_items(&walked))
        }
    });

    let key_changed = new_key != info.key_param;
    let value_changed = new_value != info.value_param;
    if !key_changed && !value_changed && new_known.is_none() {
        return None;
    }
    Some(i.intern_array(KeyedArrayInfo {
        key_param: new_key,
        value_param: new_value,
        known_items: new_known.or(info.known_items),
        ..info
    }))
}

fn walk_iterable<F: FnMut(ElementId) -> Outcome>(elem: ElementId, f: &mut F) -> Option<ElementId> {
    let i = interner();
    let info = *i.get_iterable(elem);
    let new_key = walk(info.key_type, f);
    let new_value = walk(info.value_type, f);
    if new_key == info.key_type && new_value == info.value_type {
        return None;
    }
    Some(i.intern_iterable(IterableInfo { key_type: new_key, value_type: new_value, ..info }))
}

fn walk_object_shape<F: FnMut(ElementId) -> Outcome>(elem: ElementId, f: &mut F) -> Option<ElementId> {
    let i = interner();
    let info = *i.get_object_shape(elem);
    let id = info.known_properties?;
    let entries = i.get_known_properties(id);
    let walked: Vec<KnownPropertyEntry> =
        entries.iter().map(|entry| KnownPropertyEntry { value: walk(entry.value, f), ..*entry }).collect();
    if walked.iter().zip(entries.iter()).all(|(w, o)| w.value == o.value) {
        return None;
    }
    Some(i.intern_object_shape(ObjectShapeInfo { known_properties: Some(i.intern_known_properties(&walked)), ..info }))
}

fn walk_class_like_string<F: FnMut(ElementId) -> Outcome>(elem: ElementId, f: &mut F) -> Option<ElementId> {
    let i = interner();
    let info = *i.get_class_like_string(elem);
    let new_specifier = match info.specifier {
        ClassLikeStringSpecifier::OfType { constraint } => {
            let walked = walk(constraint, f);
            if walked == constraint {
                return None;
            }
            ClassLikeStringSpecifier::OfType { constraint: walked }
        }
        ClassLikeStringSpecifier::Generic { constraint } => {
            let walked = walk(constraint, f);
            if walked == constraint {
                return None;
            }
            ClassLikeStringSpecifier::Generic { constraint: walked }
        }
        _ => return None,
    };
    Some(i.intern_class_like_string(ClassLikeStringInfo { specifier: new_specifier, ..info }))
}

fn walk_generic_parameter<F: FnMut(ElementId) -> Outcome>(elem: ElementId, f: &mut F) -> Option<ElementId> {
    let i = interner();
    let info = *i.get_generic_parameter(elem);
    let walked = walk(info.constraint, f);
    if walked == info.constraint {
        return None;
    }
    Some(i.intern_generic_parameter(GenericParameterInfo { constraint: walked, ..info }))
}

fn walk_reference<F: FnMut(ElementId) -> Outcome>(elem: ElementId, f: &mut F) -> Option<ElementId> {
    let i = interner();
    let info = *i.get_reference(elem);

    let new_args = info.type_args.and_then(|id| {
        let args = i.get_type_list(id);
        let walked: Vec<TypeId> = args.iter().map(|&a| walk(a, f)).collect();
        if walked.iter().zip(args.iter()).all(|(w, o)| w == o) { None } else { Some(i.intern_type_list(&walked)) }
    });

    let new_intersections = info.intersections.and_then(|id| {
        let conjuncts = i.get_element_list(id);
        let walked: Vec<ElementId> = conjuncts.iter().map(|&c| walk_nested(c, f).unwrap_or(c)).collect();
        if walked.iter().zip(conjuncts.iter()).all(|(w, o)| w == o) {
            None
        } else {
            Some(i.intern_element_list(&walked))
        }
    });

    if new_args.is_none() && new_intersections.is_none() {
        return None;
    }
    Some(i.intern_reference(SymbolReference {
        type_args: new_args.or(info.type_args),
        intersections: new_intersections.or(info.intersections),
        ..info
    }))
}

fn walk_conditional<F: FnMut(ElementId) -> Outcome>(elem: ElementId, f: &mut F) -> Option<ElementId> {
    let i = interner();
    let info = *i.get_conditional(elem);
    let subject = walk(info.subject, f);
    let target = walk(info.target, f);
    let then = walk(info.then, f);
    let otherwise = walk(info.otherwise, f);
    if subject == info.subject && target == info.target && then == info.then && otherwise == info.otherwise {
        return None;
    }
    Some(i.intern_conditional(ConditionalInfo { subject, target, then, otherwise, negated: info.negated }))
}

fn walk_derived<F: FnMut(ElementId) -> Outcome>(elem: ElementId, f: &mut F) -> Option<ElementId> {
    let i = interner();
    let info = *i.get_derived(elem);
    let walked = match info {
        DerivedInfo::KeyOf(t) => {
            let w = walk(t, f);
            if w == t {
                return None;
            }
            DerivedInfo::KeyOf(w)
        }
        DerivedInfo::ValueOf(t) => {
            let w = walk(t, f);
            if w == t {
                return None;
            }
            DerivedInfo::ValueOf(w)
        }
        DerivedInfo::IndexAccess { target, index } => {
            let target_w = walk(target, f);
            let index_w = walk(index, f);
            if target_w == target && index_w == index {
                return None;
            }
            DerivedInfo::IndexAccess { target: target_w, index: index_w }
        }
        DerivedInfo::PropertiesOf { target, visibility } => {
            let target_w = walk(target, f);
            if target_w == target {
                return None;
            }
            DerivedInfo::PropertiesOf { target: target_w, visibility }
        }
        DerivedInfo::IntMask(list) => {
            let raw = i.get_type_list(list);
            let walked: Vec<TypeId> = raw.iter().map(|&t| walk(t, f)).collect();
            if walked.iter().zip(raw.iter()).all(|(w, o)| w == o) {
                return None;
            }
            DerivedInfo::IntMask(i.intern_type_list(&walked))
        }
        DerivedInfo::IntMaskOf(t) => {
            let w = walk(t, f);
            if w == t {
                return None;
            }
            DerivedInfo::IntMaskOf(w)
        }
        DerivedInfo::TemplateType { object, class_name, template_name } => {
            let o = walk(object, f);
            let c = walk(class_name, f);
            let t = walk(template_name, f);
            if o == object && c == class_name && t == template_name {
                return None;
            }
            DerivedInfo::TemplateType { object: o, class_name: c, template_name: t }
        }
        DerivedInfo::New(t) => {
            let w = walk(t, f);
            if w == t {
                return None;
            }
            DerivedInfo::New(w)
        }
    };
    Some(i.intern_derived(walked))
}

fn walk_callable<F: FnMut(ElementId) -> Outcome>(elem: ElementId, f: &mut F) -> Option<ElementId> {
    use crate::element::payload::CallableInfo;
    let i = interner();
    let info = *i.get_callable(elem);
    let sig_id = match info {
        CallableInfo::Signature(s) | CallableInfo::Closure(s) => s,
        _ => return None,
    };
    let sig = *i.get_signature(sig_id);
    let new_return = walk(sig.return_type, f);
    let new_throws = sig.throws.map(|t| walk(t, f));
    let new_params = sig.parameters.and_then(|pid| {
        let params = i.get_param_list(pid);
        let walked: Vec<ParamInfo> = params.iter().map(|p| ParamInfo { type_: walk(p.type_, f), ..*p }).collect();
        if walked.iter().zip(params.iter()).all(|(w, o)| w.type_ == o.type_) {
            None
        } else {
            Some(i.intern_param_list(&walked))
        }
    });

    let return_changed = new_return != sig.return_type;
    let throws_changed = new_throws != sig.throws;
    if !return_changed && !throws_changed && new_params.is_none() {
        return None;
    }
    let new_sig = i.intern_signature(Signature {
        return_type: new_return,
        throws: new_throws.or(sig.throws),
        parameters: new_params.or(sig.parameters),
        ..sig
    });
    let rebuilt = match info {
        CallableInfo::Signature(_) => CallableInfo::Signature(new_sig),
        CallableInfo::Closure(_) => CallableInfo::Closure(new_sig),
        _ => return None,
    };
    Some(i.intern_callable(rebuilt))
}
