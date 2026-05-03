#![allow(clippy::pub_use, clippy::arithmetic_side_effects)]

//! Type expansion: resolve non-structural type forms (`Alias`,
//! `Reference`, `Derived`, `Conditional`, contextual keywords) into
//! their structural definitions.
//!
//! [`expand`] is the no-context entry point for callers that just want
//! `Alias` / `Reference` / `Derived` resolution. [`expand_with`] takes
//! an explicit [`ExpansionContext`] and additionally substitutes
//! contextual keywords (`self`, `static`, `parent`) and evaluates
//! `Conditional` atoms when `eval_conditional` is on.
//!
//! # Stages
//!
//! - **Stage 1:** `Alias` resolution.
//! - **Stage 2:** `Reference` resolution (`SymbolReference`,
//!   `MemberReference`, `GlobalReference`).
//! - **Stage 3:** `Derived` evaluation (`KeyOf`, `ValueOf`,
//!   `IndexAccess`, `IntMask`, `IntMaskOf`, `TemplateType`,
//!   `PropertiesOf`, `New`).
//! - **Stage 4:** Contextual keyword substitution (`self`, `static`,
//!   `parent`, `$this`) and `Conditional` evaluation.
//!
//! # Structural descent
//!
//! Expansion descends into every nested type: `Object`
//! type arguments, list / keyed-array / iterable element-types, sealed
//! known items, class-like-string constraints, generic-parameter
//! constraints, conditional / derived / callable operands. The walk
//! is delegated to [`crate::transform::flat_map`]; this module owns
//! only the per-element resolution logic.

mod context;

pub use self::context::ExpansionContext;

use mago_atom::Atom;

use crate::ElementId;
use crate::ElementKind;
use crate::TypeId;
use crate::element::payload::ArrayKey;
use crate::element::payload::ClassLikeStringSpecifier;
use crate::element::payload::DerivedInfo;
use crate::element::payload::KeyedArrayFlags;
use crate::element::payload::KeyedArrayInfo;
use crate::element::payload::KnownItemEntry;
use crate::element::payload::NameSelector;
use crate::element::payload::ObjectFlags;
use crate::element::payload::ObjectInfo;
use crate::element::payload::scalar::IntInfo;
use crate::interner::interner;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::lattice::overlaps;
use crate::lattice::refines;
use crate::prelude::NON_NEGATIVE_INT;
use crate::prelude::TYPE_INT;
use crate::prelude::TYPE_MIXED;
use crate::prelude::TYPE_NEVER;
use crate::transform;
use crate::world::World;

/// Resolve every expandable atom inside `ty` against `world`, with the
/// default expansion context (no contextual class names, conditionals
/// preserved).
#[inline]
pub fn expand<W: World>(ty: TypeId, world: &W) -> TypeId {
    expand_with(ty, world, &ExpansionContext::default())
}

/// Like [`expand`] but with a caller-supplied [`ExpansionContext`].
/// Returns the same `TypeId` handle when nothing changed.
#[inline]
pub fn expand_with<W: World>(ty: TypeId, world: &W, ctx: &ExpansionContext) -> TypeId {
    transform::flat_map(ty, |elem| resolve_element(elem, world, ctx))
}

/// Per-element resolution. By the time this fires, [`crate::transform`]
/// has already recursively walked every nested `TypeId` carried in
/// `elem`'s payload ; the closure receives an element whose children
/// are fully expanded.
#[inline]
fn resolve_element<W: World>(elem: ElementId, world: &W, ctx: &ExpansionContext) -> Vec<ElementId> {
    match elem.kind() {
        ElementKind::Alias => resolve_alias(elem, world, ctx),
        ElementKind::Reference => resolve_reference(elem, world, ctx),
        ElementKind::MemberReference => resolve_member_reference(elem, world, ctx),
        ElementKind::GlobalReference => resolve_global_reference(elem, world, ctx),
        ElementKind::Derived => resolve_derived(elem, world, ctx),
        ElementKind::Conditional => resolve_conditional(elem, world, ctx),
        ElementKind::Object => resolve_object(elem, world, ctx),
        ElementKind::GenericParameter => resolve_generic_parameter(elem, ctx),
        _ => vec![elem],
    }
}

#[inline]
fn resolve_alias<W: World>(elem: ElementId, world: &W, ctx: &ExpansionContext) -> Vec<ElementId> {
    if !ctx.eval_aliases {
        return vec![elem];
    }
    let info = interner().get_alias(elem);
    let Some(body) = world.alias_body(info.class_name, info.alias_name) else {
        return vec![elem];
    };
    expand_with(body, world, ctx).as_ref().elements.to_vec()
}

/// `SymbolReference("Foo", type_args, intersections)` is, semantically,
/// the same value-set as `Object("Foo", ...)`. Convert it; type args
/// and intersection conjuncts have already been walked by the
/// surrounding [`crate::transform`] call, so we just reuse them.
/// Contextual keyword substitution applies (a `self` / `static` /
/// `parent` reference picks up the corresponding context entry).
#[inline]
fn resolve_reference<W: World>(elem: ElementId, world: &W, ctx: &ExpansionContext) -> Vec<ElementId> {
    let i = interner();
    let info = *i.get_reference(elem);
    let resolved_name = resolve_keyword_name(info.name, ObjectFlags::default(), ctx).unwrap_or(info.name);
    let mut object = ObjectInfo { name: resolved_name, type_args: info.type_args, flags: ObjectFlags::default() };

    if ctx.fill_template_defaults && object.type_args.is_none() {
        let arity = world.template_parameter_arity(object.name);
        if arity > 0 {
            let filled: Vec<TypeId> = (0..arity)
                .map(|pos| {
                    world.template_parameter_at(object.name, pos).and_then(|p| p.upper_bound).unwrap_or(TYPE_MIXED)
                })
                .collect();
            object.type_args = Some(i.intern_type_list(&filled));
        }
    }

    vec![i.intern_object(object)]
}

/// Replace a free `GenericParameter` atom with its constraint. Gated
/// on [`ExpansionContext::substitute_template_constraints`]; when off,
/// the atom passes through (the common case ; comparing two template
/// parameters for identity must see them as opaque).
#[inline]
fn resolve_generic_parameter(elem: ElementId, ctx: &ExpansionContext) -> Vec<ElementId> {
    if !ctx.substitute_template_constraints {
        return vec![elem];
    }
    let info = interner().get_generic_parameter(elem);
    info.constraint.as_ref().elements.to_vec()
}

/// `Foo::CONST` (an `Identifier` selector on a `MemberReference`)
/// resolves to the constant's declared type via
/// [`World::class_constant_type`], recursively expanded. Other
/// selectors (wildcard / prefix / suffix) need a constant-enumeration
/// query and pass through unchanged for now.
#[inline]
fn resolve_member_reference<W: World>(elem: ElementId, world: &W, ctx: &ExpansionContext) -> Vec<ElementId> {
    if !ctx.eval_class_constants {
        return vec![elem];
    }
    let info = interner().get_member_reference(elem);
    let NameSelector::Identifier(constant) = info.selector else {
        return vec![elem];
    };
    let Some(body) = world.class_constant_type(info.class_like_name, constant) else {
        return vec![elem];
    };
    expand_with(body, world, ctx).as_ref().elements.to_vec()
}

/// A global constant reference resolves through
/// [`World::global_constant_type`]. Wildcard selectors pass through.
#[inline]
fn resolve_global_reference<W: World>(elem: ElementId, world: &W, ctx: &ExpansionContext) -> Vec<ElementId> {
    if !ctx.eval_global_constants {
        return vec![elem];
    }
    let info = interner().get_global_reference(elem);
    let NameSelector::Identifier(name) = info.selector else {
        return vec![elem];
    };
    let Some(body) = world.global_constant_type(name) else {
        return vec![elem];
    };
    expand_with(body, world, ctx).as_ref().elements.to_vec()
}

#[inline]
fn resolve_derived<W: World>(elem: ElementId, world: &W, ctx: &ExpansionContext) -> Vec<ElementId> {
    let info = *interner().get_derived(elem);
    let evaluated = match info {
        DerivedInfo::KeyOf(target) => Some(eval_key_of(target)),
        DerivedInfo::ValueOf(target) => Some(eval_value_of(target)),
        DerivedInfo::IndexAccess { target, index } => Some(eval_index_access(target, index)),
        DerivedInfo::IntMask(operands) => Some(eval_int_mask(operands)),
        DerivedInfo::IntMaskOf(target) => Some(eval_int_mask_of(target)),
        DerivedInfo::TemplateType { object: _, class_name, template_name } => {
            eval_template_type(class_name, template_name, world, ctx)
        }
        DerivedInfo::PropertiesOf { target, visibility } => eval_properties_of(target, visibility, world),
        DerivedInfo::New(target) => eval_new(target, world),
    };
    match evaluated {
        Some(t) => t.as_ref().elements.to_vec(),
        None => vec![elem],
    }
}

/// Conditional `T is U ? A : B` (or its negated form).
///
/// When `ctx.eval_conditional` is on, the test `subject <: target` is
/// decided via the lattice. A subtype hit picks the then-branch (or
/// the otherwise-branch when negated); a disjoint pair picks the
/// other side; an undecidable test widens to the union of both
/// branches ().
///
/// When `ctx.eval_conditional` is off, the atom is preserved
/// unchanged ; its operands have already been walked by the enclosing
/// transform call.
#[inline]
fn resolve_conditional<W: World>(elem: ElementId, world: &W, ctx: &ExpansionContext) -> Vec<ElementId> {
    if !ctx.eval_conditional {
        return vec![elem];
    }

    let info = *interner().get_conditional(elem);
    let mut report = LatticeReport::new();
    let opts = LatticeOptions::default();
    let test_passes = refines(info.subject, info.target, world, opts, &mut report);
    let test_disjoint = !overlaps(info.subject, info.target, world, opts, &mut report);

    let (chosen_then, chosen_otherwise) =
        if info.negated { (info.otherwise, info.then) } else { (info.then, info.otherwise) };

    let result = if test_passes {
        chosen_then
    } else if test_disjoint {
        chosen_otherwise
    } else {
        let mut elems: Vec<ElementId> = Vec::new();
        elems.extend_from_slice(chosen_then.as_ref().elements);
        elems.extend_from_slice(chosen_otherwise.as_ref().elements);
        TypeId::union(&elems)
    };
    result.as_ref().elements.to_vec()
}

/// Resolve a named-object atom. Combines three independent stages:
///
/// - Contextual keyword substitution (`self` / `static` / `parent` /
///   `$this`) when the corresponding [`ExpansionContext`] entry is set.
/// - Final-function collapse: with [`ExpansionContext::function_is_final`],
///   drop the `is_static` / `is_this` modality flags on a named class
///   that already has a concrete name (no `static_class` binding
///   needed).
/// - Default-fill of unfilled generic positions when
///   [`ExpansionContext::fill_template_defaults`] is on.
#[inline]
fn resolve_object<W: World>(elem: ElementId, world: &W, ctx: &ExpansionContext) -> Vec<ElementId> {
    let i = interner();
    let mut info = *i.get_object(elem);
    let mut changed = false;

    #[allow(clippy::else_if_without_else)]
    if let Some(class) = resolve_keyword_name(info.name, info.flags, ctx) {
        info = ObjectInfo { name: class, flags: info.flags.with_is_static(false).with_is_this(false), ..info };
        changed = true;
    } else if ctx.function_is_final && (info.flags.is_static() || info.flags.is_this()) {
        info = ObjectInfo { flags: info.flags.with_is_static(false).with_is_this(false), ..info };
        changed = true;
    }

    if ctx.fill_template_defaults && info.type_args.is_none() {
        let arity = world.template_parameter_arity(info.name);
        if arity > 0 {
            let filled: Vec<TypeId> = (0..arity)
                .map(|pos| {
                    world.template_parameter_at(info.name, pos).and_then(|p| p.upper_bound).unwrap_or(TYPE_MIXED)
                })
                .collect();
            info = ObjectInfo { type_args: Some(i.intern_type_list(&filled)), ..info };
            changed = true;
        }
    }

    if changed { vec![i.intern_object(info)] } else { vec![elem] }
}

/// Map `self` / `static` / `parent` / the `is_static` / `is_this`
/// modality flags to a concrete class name pulled from `ctx`. Returns
/// `None` when no keyword applies (the atom is a plain `Named(C)`) or
/// when the context lacks the required entry.
#[inline]
fn resolve_keyword_name(name: Atom, flags: ObjectFlags, ctx: &ExpansionContext) -> Option<Atom> {
    let name_str = name.as_str();
    if flags.is_this() || flags.is_static() || name_str == "static" {
        ctx.static_class
    } else if name_str == "self" {
        ctx.self_class
    } else if name_str == "parent" {
        ctx.parent_class
    } else {
        None
    }
}

/// `key-of<τ>`: keys admissible by `τ`. Operand has
/// already been expanded by the surrounding walk.
#[inline]
fn eval_key_of(target: TypeId) -> TypeId {
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
                Some(count_nz) => {
                    let count = count_nz.get() as i64;
                    keys.push(ElementId::int_range(Some(0), Some(count - 1)));
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
        ElementKind::ObjectShape => {
            let info = *i.get_object_shape(only);
            let Some(known_id) = info.known_properties else {
                return TYPE_NEVER;
            };
            let entries = i.get_known_properties(known_id);
            let keys: Vec<ElementId> =
                entries.iter().map(|entry| ElementId::string_literal(entry.name.as_str())).collect();
            if keys.is_empty() { TYPE_NEVER } else { TypeId::union(&keys) }
        }
        _ => TYPE_MIXED,
    }
}

/// `value-of<τ>`: values admissible by `τ`.
#[inline]
fn eval_value_of(target: TypeId) -> TypeId {
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
        ElementKind::ObjectShape => {
            let info = *i.get_object_shape(only);
            let Some(known_id) = info.known_properties else {
                return TYPE_NEVER;
            };
            let mut values: Vec<ElementId> = Vec::new();
            for entry in i.get_known_properties(known_id) {
                values.extend_from_slice(entry.value.as_ref().elements);
            }
            if values.is_empty() { TYPE_NEVER } else { TypeId::union(&values) }
        }
        _ => TYPE_MIXED,
    }
}

/// `τ[κ]`.
#[inline]
fn eval_index_access(target: TypeId, index: TypeId) -> TypeId {
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
        ElementKind::ObjectShape => {
            let info = *i.get_object_shape(only);
            let Some(known_id) = info.known_properties else {
                return TYPE_NEVER;
            };
            if let Some(literal) = single_string_literal_atom_from_element(key_elem) {
                for entry in i.get_known_properties(known_id) {
                    if entry.name == literal {
                        return entry.value;
                    }
                }
                return TYPE_NEVER;
            }
            let mut values: Vec<ElementId> = Vec::new();
            for entry in i.get_known_properties(known_id) {
                values.extend_from_slice(entry.value.as_ref().elements);
            }
            if values.is_empty() { TYPE_NEVER } else { TypeId::union(&values) }
        }
        _ => TYPE_MIXED,
    }
}

#[inline]
fn single_string_literal_atom_from_element(elem: ElementId) -> Option<Atom> {
    use crate::element::payload::scalar::StringLiteral;
    if elem.kind() != ElementKind::String {
        return None;
    }
    match interner().get_string(elem).literal {
        StringLiteral::Value(atom) => Some(atom),
        _ => None,
    }
}

#[inline]
fn eval_int_mask(operands: crate::TypeListId) -> TypeId {
    let i = interner();
    let raw = i.get_type_list(operands);
    let mut literals: Vec<i64> = Vec::with_capacity(raw.len());
    for &operand in raw {
        let elems = operand.as_ref().elements;
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

#[inline]
fn eval_int_mask_of(target: TypeId) -> TypeId {
    let mut literals: Vec<i64> = Vec::new();
    for &el in target.as_ref().elements {
        match literal_int(el) {
            Some(v) => literals.push(v),
            None => return TYPE_MIXED,
        }
    }
    int_mask_union(&literals)
}

#[inline]
fn int_mask_union(literals: &[i64]) -> TypeId {
    let n = literals.len();
    if n == 0 {
        return TypeId::union(&[ElementId::int_literal(0)]);
    }
    if n > 16 {
        return TYPE_INT;
    }
    let total = 1u32 << n;
    let mut values: alloc::collections::BTreeSet<i64> = alloc::collections::BTreeSet::new();
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

#[inline]
fn eval_template_type<W: World>(
    class_name: TypeId,
    template_name: TypeId,
    world: &W,
    ctx: &ExpansionContext,
) -> Option<TypeId> {
    let class = single_object_or_reference_name(class_name)?;
    let template = single_string_literal_atom(template_name)?;
    let position = world.template_parameter_index(class, template)?;
    let parameter = world.template_parameter_at(class, position)?;
    Some(expand_with(parameter.upper_bound.unwrap_or(TYPE_MIXED), world, ctx))
}

/// `properties-of<C>`: enumerate `C`'s declared
/// properties and produce a sealed `array{name: type, ...}` shape.
/// `visibility` filters the enumeration; `None` keeps every visible
/// property.
#[inline]
fn eval_properties_of<W: World>(
    target: TypeId,
    visibility: Option<crate::element::payload::Visibility>,
    world: &W,
) -> Option<TypeId> {
    let class = single_object_or_reference_name(target)?;

    let count = world.class_property_count(class);
    let mut entries: Vec<KnownItemEntry> = Vec::with_capacity(count);
    for position in 0..count {
        let Some(property) = world.class_property_at(class, position) else {
            continue;
        };

        if let Some(required) = visibility
            && property.visibility != required
        {
            continue;
        }

        entries.push(KnownItemEntry { key: ArrayKey::String(property.name), value: property.type_, optional: false });
    }

    entries.sort_by_key(|e| e.key);

    let i = interner();
    let info = KeyedArrayInfo {
        key_param: None,
        value_param: None,
        known_items: Some(i.intern_known_items(&entries)),
        flags: KeyedArrayFlags::default(),
    };

    Some(TypeId::union(&[i.intern_array(info)]))
}

/// `new<C>`: the type produced by `new C(...)`. The
/// constructor-driven template inference path is left for a later
/// stage; this first cut produces `Object(C)` (with `mixed` filled in
/// for any templates `C` declares) so the result at least has the
/// right nominal class.
#[inline]
fn eval_new<W: World>(target: TypeId, world: &W) -> Option<TypeId> {
    let class = extract_class_name_from_class_string_or_object(target)?;

    let arity = world.template_parameter_arity(class);
    let i = interner();
    let info = if arity == 0 {
        ObjectInfo { name: class, type_args: None, flags: ObjectFlags::default() }
    } else {
        let args: Vec<TypeId> = (0..arity)
            .map(|p| world.template_parameter_at(class, p).and_then(|t| t.upper_bound).unwrap_or(TYPE_MIXED))
            .collect();
        ObjectInfo { name: class, type_args: Some(i.intern_type_list(&args)), flags: ObjectFlags::default() }
    };
    Some(TypeId::union(&[i.intern_object(info)]))
}

/// Try to read a class-like name from `ty`, accepting either a single
/// `Object(C)` / `Reference(C)` atom or a single literal class-string
/// `class-string<Foo>`.
#[inline]
fn extract_class_name_from_class_string_or_object(ty: TypeId) -> Option<Atom> {
    let elems = ty.as_ref().elements;
    if elems.len() != 1 {
        return None;
    }
    let i = interner();
    match elems[0].kind() {
        ElementKind::Object => Some(i.get_object(elems[0]).name),
        ElementKind::Reference => Some(i.get_reference(elems[0]).name),
        ElementKind::ClassLikeString => match i.get_class_like_string(elems[0]).specifier {
            ClassLikeStringSpecifier::Literal { value } => Some(value),
            _ => None,
        },
        _ => None,
    }
}

#[inline]
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

#[inline]
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

#[inline]
fn array_key_to_element(key: ArrayKey) -> Option<ElementId> {
    match key {
        ArrayKey::Int(n) => Some(ElementId::int_literal(n)),
        ArrayKey::String(atom) => Some(ElementId::string_literal(atom.as_str())),
        ArrayKey::Const { .. } => None,
    }
}

#[inline]
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

#[inline]
fn literal_int(elem: ElementId) -> Option<i64> {
    if elem.kind() != ElementKind::Int {
        return None;
    }
    match interner().get_int(elem) {
        IntInfo::Literal(n) => Some(*n),
        _ => None,
    }
}
