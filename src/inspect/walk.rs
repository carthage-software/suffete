//! Internal: short-circuiting deep walker. Mirrors the structural
//! descent of [`crate::transform::walk`] but plumbs a `bool`-returning
//! predicate through and stops the moment the answer is known.

use crate::ElementId;
use crate::ElementKind;
use crate::TypeId;
use crate::element::payload::ClassLikeStringSpecifier;
use crate::interner::interner;

pub(super) fn any<F: FnMut(ElementId) -> bool>(ty: TypeId, predicate: &mut F) -> bool {
    let view = ty.as_ref();
    for &elem in view.elements {
        if visit(elem, predicate) {
            return true;
        }
    }

    false
}

fn visit<F: FnMut(ElementId) -> bool>(elem: ElementId, predicate: &mut F) -> bool {
    if predicate(elem) {
        return true;
    }

    descend(elem, predicate)
}

fn descend<F: FnMut(ElementId) -> bool>(elem: ElementId, predicate: &mut F) -> bool {
    match elem.kind() {
        ElementKind::Object => descend_object(elem, predicate),
        ElementKind::List => descend_list(elem, predicate),
        ElementKind::Array => descend_keyed_array(elem, predicate),
        ElementKind::Iterable => descend_iterable(elem, predicate),
        ElementKind::ObjectShape => descend_object_shape(elem, predicate),
        ElementKind::ClassLikeString => descend_class_like_string(elem, predicate),
        ElementKind::GenericParameter => descend_generic_parameter(elem, predicate),
        ElementKind::Reference => descend_reference(elem, predicate),
        ElementKind::Conditional => descend_conditional(elem, predicate),
        ElementKind::Derived => descend_derived(elem, predicate),
        ElementKind::Callable => descend_callable(elem, predicate),
        _ => false,
    }
}

fn descend_object<F: FnMut(ElementId) -> bool>(elem: ElementId, predicate: &mut F) -> bool {
    let i = interner();
    let info = *i.get_object(elem);
    if let Some(args_id) = info.type_args {
        for &arg in i.get_type_list(args_id) {
            if any(arg, predicate) {
                return true;
            }
        }
    }

    if let Some(intersections_id) = info.intersections {
        for &conjunct in i.get_element_list(intersections_id) {
            if visit(conjunct, predicate) {
                return true;
            }
        }
    }

    false
}

fn descend_list<F: FnMut(ElementId) -> bool>(elem: ElementId, predicate: &mut F) -> bool {
    let i = interner();
    let info = *i.get_list(elem);
    if any(info.element_type, predicate) {
        return true;
    }

    if let Some(known_id) = info.known_elements {
        for entry in i.get_known_elements(known_id) {
            if any(entry.value, predicate) {
                return true;
            }
        }
    }

    false
}

fn descend_keyed_array<F: FnMut(ElementId) -> bool>(elem: ElementId, predicate: &mut F) -> bool {
    let i = interner();
    let info = *i.get_array(elem);
    if let Some(k) = info.key_param
        && any(k, predicate)
    {
        return true;
    }

    if let Some(v) = info.value_param
        && any(v, predicate)
    {
        return true;
    }

    if let Some(known_id) = info.known_items {
        for entry in i.get_known_items(known_id) {
            if any(entry.value, predicate) {
                return true;
            }
        }
    }

    false
}

fn descend_iterable<F: FnMut(ElementId) -> bool>(elem: ElementId, predicate: &mut F) -> bool {
    let i = interner();
    let info = *i.get_iterable(elem);
    any(info.key_type, predicate) || any(info.value_type, predicate)
}

fn descend_object_shape<F: FnMut(ElementId) -> bool>(elem: ElementId, predicate: &mut F) -> bool {
    let i = interner();
    let info = *i.get_object_shape(elem);
    let Some(known_id) = info.known_properties else { return false };
    for entry in i.get_known_properties(known_id) {
        if any(entry.value, predicate) {
            return true;
        }
    }

    false
}

fn descend_class_like_string<F: FnMut(ElementId) -> bool>(elem: ElementId, predicate: &mut F) -> bool {
    let i = interner();
    let info = *i.get_class_like_string(elem);
    match info.specifier {
        ClassLikeStringSpecifier::OfType { constraint } | ClassLikeStringSpecifier::Generic { constraint } => {
            any(constraint, predicate)
        }
        _ => false,
    }
}

fn descend_generic_parameter<F: FnMut(ElementId) -> bool>(elem: ElementId, predicate: &mut F) -> bool {
    let info = interner().get_generic_parameter(elem);
    any(info.constraint, predicate)
}

fn descend_reference<F: FnMut(ElementId) -> bool>(elem: ElementId, predicate: &mut F) -> bool {
    let i = interner();
    let info = *i.get_reference(elem);
    if let Some(args_id) = info.type_args {
        for &arg in i.get_type_list(args_id) {
            if any(arg, predicate) {
                return true;
            }
        }
    }

    if let Some(intersections_id) = info.intersections {
        for &conjunct in i.get_element_list(intersections_id) {
            if visit(conjunct, predicate) {
                return true;
            }
        }
    }

    false
}

fn descend_conditional<F: FnMut(ElementId) -> bool>(elem: ElementId, predicate: &mut F) -> bool {
    let info = *interner().get_conditional(elem);
    any(info.subject, predicate)
        || any(info.target, predicate)
        || any(info.then, predicate)
        || any(info.otherwise, predicate)
}

fn descend_derived<F: FnMut(ElementId) -> bool>(elem: ElementId, predicate: &mut F) -> bool {
    use crate::element::payload::DerivedInfo;
    let i = interner();
    let info = *i.get_derived(elem);
    match info {
        DerivedInfo::KeyOf(t) | DerivedInfo::ValueOf(t) | DerivedInfo::IntMaskOf(t) | DerivedInfo::New(t) => {
            any(t, predicate)
        }
        DerivedInfo::IndexAccess { target, index } => any(target, predicate) || any(index, predicate),
        DerivedInfo::PropertiesOf { target, .. } => any(target, predicate),
        DerivedInfo::IntMask(list) => i.get_type_list(list).iter().any(|&t| any(t, predicate)),
        DerivedInfo::TemplateType { object, class_name, template_name } => {
            any(object, predicate) || any(class_name, predicate) || any(template_name, predicate)
        }
    }
}

fn descend_callable<F: FnMut(ElementId) -> bool>(elem: ElementId, predicate: &mut F) -> bool {
    use crate::element::payload::CallableInfo;
    let i = interner();
    let info = *i.get_callable(elem);
    let sig_id = match info {
        CallableInfo::Signature(s) | CallableInfo::Closure(s) => s,
        _ => return false,
    };

    let sig = *i.get_signature(sig_id);
    if any(sig.return_type, predicate) {
        return true;
    }

    if let Some(t) = sig.throws
        && any(t, predicate)
    {
        return true;
    }

    if let Some(pid) = sig.parameters {
        for p in i.get_param_list(pid) {
            if any(p.type_, predicate) {
                return true;
            }
        }
    }

    false
}
