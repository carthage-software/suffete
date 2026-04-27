//! Type expansion: resolve non-structural type forms (`Alias`,
//! `Reference`, `Derived`, `Conditional`, contextual keywords) into
//! their structural definitions, per `type-system/generics.md` §7.
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
//!   `IndexAccess`, `IntMask`, `IntMaskOf`, `TemplateType`).
//!   `PropertiesOf` and `New` pass through until they gain dedicated
//!   `World` queries.
//! - **Stage 4 (current):** Contextual keyword substitution (`self`,
//!   `static`, `parent`, `$this`) and `Conditional` evaluation.
//!
//! # Structural descent
//!
//! Per `§7.4`, expansion descends into every nested type — `Object`
//! type arguments, list / keyed-array / iterable element-types, sealed
//! known items, and class-like-string constraints. Element kinds whose
//! payloads do not (yet) carry nested types pass through unchanged.

mod context;

pub use self::context::ExpansionContext;

use mago_atom::Atom;

use crate::ElementId;
use crate::ElementKind;
use crate::TypeId;
use crate::element::payload::ArrayKey;
use crate::element::payload::ClassLikeStringInfo;
use crate::element::payload::ClassLikeStringSpecifier;
use crate::element::payload::ConditionalInfo;
use crate::element::payload::DerivedInfo;
use crate::element::payload::IterableInfo;
use crate::element::payload::KeyedArrayFlags;
use crate::element::payload::KeyedArrayInfo;
use crate::element::payload::KnownItemEntry;
use crate::element::payload::ListInfo;
use crate::element::payload::NameSelector;
use crate::element::payload::ObjectFlags;
use crate::element::payload::ObjectInfo;
use crate::element::payload::scalar::IntInfo;
use crate::interner::interner;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::lattice::overlaps;
use crate::lattice::refines;
use crate::prelude::NEVER;
use crate::prelude::NON_NEGATIVE_INT;
use crate::prelude::TYPE_INT;
use crate::prelude::TYPE_MIXED;
use crate::prelude::TYPE_NEVER;
use crate::world::World;

/// Resolve every expandable atom inside `ty` against `world`, with the
/// default expansion context (no contextual class names, conditionals
/// preserved).
pub fn expand<W: World>(ty: TypeId, world: &W) -> TypeId {
    expand_with(ty, world, &ExpansionContext::default())
}

/// Like [`expand`] but with a caller-supplied [`ExpansionContext`].
/// Returns the same `TypeId` handle when nothing changed.
pub fn expand_with<W: World>(ty: TypeId, world: &W, ctx: &ExpansionContext) -> TypeId {
    let i = interner();
    let original = ty.as_ref();
    let mut new_elements: Vec<ElementId> = Vec::with_capacity(original.elements.len());
    let mut changed = false;

    for &elem in original.elements {
        match expand_element(elem, world, ctx) {
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

    i.intern_type(&new_elements, original.flags)
}

/// What `expand_element` returns. `Unchanged` is the common case;
/// `Single` covers in-place rewrites; `Many` covers a body whose
/// expansion is a multi-element union and flat-merges into the
/// surrounding type.
enum Expansion {
    Unchanged,
    Single(ElementId),
    Many(Vec<ElementId>),
}

fn expand_element<W: World>(elem: ElementId, world: &W, ctx: &ExpansionContext) -> Expansion {
    match elem.kind() {
        ElementKind::Alias => expand_alias(elem, world, ctx),
        ElementKind::Reference => expand_reference(elem, world, ctx),
        ElementKind::MemberReference => expand_member_reference(elem, world, ctx),
        ElementKind::GlobalReference => expand_global_reference(elem, world, ctx),
        ElementKind::Derived => expand_derived(elem, world, ctx),
        ElementKind::Conditional => expand_conditional(elem, world, ctx),
        ElementKind::Object => expand_object(elem, world, ctx),
        ElementKind::List => expand_list(elem, world, ctx),
        ElementKind::Array => expand_keyed_array(elem, world, ctx),
        ElementKind::Iterable => expand_iterable(elem, world, ctx),
        ElementKind::ClassLikeString => expand_class_like_string(elem, world, ctx),
        _ => Expansion::Unchanged,
    }
}

fn expand_alias<W: World>(elem: ElementId, world: &W, ctx: &ExpansionContext) -> Expansion {
    let info = interner().get_alias(elem);
    let Some(body) = world.alias_body(info.class_name, info.alias_name) else {
        return Expansion::Unchanged;
    };
    type_to_expansion(expand_with(body, world, ctx))
}

/// `SymbolReference("Foo", type_args, intersections)` is, semantically,
/// the same value-set as `Object("Foo", ...)`. Convert it; type args
/// and intersection conjuncts are recursively expanded so a reference
/// like `Foo<MyAlias>` resolves to `Object("Foo", [<expanded alias>])`.
fn expand_reference<W: World>(elem: ElementId, world: &W, ctx: &ExpansionContext) -> Expansion {
    let i = interner();
    let info = *i.get_reference(elem);

    let new_args = info.type_args.map(|id| {
        let args = i.get_type_list(id);
        let expanded: Vec<TypeId> = args.iter().map(|&a| expand_with(a, world, ctx)).collect();
        i.intern_type_list(&expanded)
    });

    let new_intersections = info.intersections.map(|id| {
        let conjuncts = i.get_element_list(id);
        let expanded: Vec<ElementId> = conjuncts
            .iter()
            .flat_map(|&c| match expand_element(c, world, ctx) {
                Expansion::Unchanged => vec![c],
                Expansion::Single(e) => vec![e],
                Expansion::Many(es) => es,
            })
            .collect();
        i.intern_element_list(&expanded)
    });

    let resolved_name = resolve_keyword_name(info.name, ObjectFlags::default(), ctx);
    Expansion::Single(i.intern_object(ObjectInfo {
        name: resolved_name.unwrap_or(info.name),
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
fn expand_member_reference<W: World>(elem: ElementId, world: &W, ctx: &ExpansionContext) -> Expansion {
    let info = interner().get_member_reference(elem);
    let NameSelector::Identifier(constant) = info.selector else {
        return Expansion::Unchanged;
    };
    let Some(body) = world.class_constant_type(info.class_like_name, constant) else {
        return Expansion::Unchanged;
    };
    type_to_expansion(expand_with(body, world, ctx))
}

/// A global constant reference resolves through
/// [`World::global_constant_type`]. Wildcard selectors pass through.
fn expand_global_reference<W: World>(elem: ElementId, world: &W, ctx: &ExpansionContext) -> Expansion {
    let info = interner().get_global_reference(elem);
    let NameSelector::Identifier(name) = info.selector else {
        return Expansion::Unchanged;
    };
    let Some(body) = world.global_constant_type(name) else {
        return Expansion::Unchanged;
    };
    type_to_expansion(expand_with(body, world, ctx))
}

fn expand_derived<W: World>(elem: ElementId, world: &W, ctx: &ExpansionContext) -> Expansion {
    let info = *interner().get_derived(elem);
    match info {
        DerivedInfo::KeyOf(target) => type_to_expansion(eval_key_of(target, world, ctx)),
        DerivedInfo::ValueOf(target) => type_to_expansion(eval_value_of(target, world, ctx)),
        DerivedInfo::IndexAccess { target, index } => type_to_expansion(eval_index_access(target, index, world, ctx)),
        DerivedInfo::IntMask(operands) => type_to_expansion(eval_int_mask(operands, world, ctx)),
        DerivedInfo::IntMaskOf(target) => type_to_expansion(eval_int_mask_of(target, world, ctx)),
        DerivedInfo::TemplateType { object: _, class_name, template_name } => {
            match eval_template_type(class_name, template_name, world, ctx) {
                Some(t) => type_to_expansion(t),
                None => Expansion::Unchanged,
            }
        }
        DerivedInfo::PropertiesOf { target, visibility } => match eval_properties_of(target, visibility, world, ctx) {
            Some(t) => type_to_expansion(t),
            None => Expansion::Unchanged,
        },
        DerivedInfo::New(target) => match eval_new(target, world, ctx) {
            Some(t) => type_to_expansion(t),
            None => Expansion::Unchanged,
        },
    }
}

/// Conditional `T is U ? A : B` (or its negated form).
///
/// When `ctx.eval_conditional` is on, the test `subject <: target` is
/// decided via the lattice. A subtype hit picks the then-branch (or
/// the otherwise-branch when negated); a disjoint pair picks the
/// other side; an undecidable test widens to the union of both
/// branches (per spec §7.3).
///
/// When `ctx.eval_conditional` is off, the atom is preserved but its
/// four operand types are still expanded recursively.
fn expand_conditional<W: World>(elem: ElementId, world: &W, ctx: &ExpansionContext) -> Expansion {
    let i = interner();
    let info = *i.get_conditional(elem);
    let subject = expand_with(info.subject, world, ctx);
    let target = expand_with(info.target, world, ctx);
    let then_t = expand_with(info.then, world, ctx);
    let otherwise_t = expand_with(info.otherwise, world, ctx);

    if !ctx.eval_conditional {
        let unchanged =
            subject == info.subject && target == info.target && then_t == info.then && otherwise_t == info.otherwise;
        if unchanged {
            return Expansion::Unchanged;
        }
        return Expansion::Single(i.intern_conditional(ConditionalInfo {
            subject,
            target,
            then: then_t,
            otherwise: otherwise_t,
            negated: info.negated,
        }));
    }

    let mut report = LatticeReport::new();
    let opts = LatticeOptions::default();
    let test_passes = refines(subject, target, world, opts, &mut report);
    let test_disjoint = !overlaps(subject, target, world, opts, &mut report);

    let (chosen_then, chosen_otherwise) = if info.negated { (otherwise_t, then_t) } else { (then_t, otherwise_t) };

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
    type_to_expansion(result)
}

fn type_to_expansion(ty: TypeId) -> Expansion {
    let elements = ty.as_ref().elements;
    if elements.is_empty() {
        Expansion::Single(NEVER)
    } else if elements.len() == 1 {
        Expansion::Single(elements[0])
    } else {
        Expansion::Many(elements.to_vec())
    }
}

/// `key-of<τ>` per spec §7.3: keys admissible by `τ`.
fn eval_key_of<W: World>(target: TypeId, world: &W, ctx: &ExpansionContext) -> TypeId {
    let target = expand_with(target, world, ctx);
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
fn eval_value_of<W: World>(target: TypeId, world: &W, ctx: &ExpansionContext) -> TypeId {
    let target = expand_with(target, world, ctx);
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
fn eval_index_access<W: World>(target: TypeId, index: TypeId, world: &W, ctx: &ExpansionContext) -> TypeId {
    let target = expand_with(target, world, ctx);
    let index = expand_with(index, world, ctx);
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

fn eval_int_mask<W: World>(operands: crate::TypeListId, world: &W, ctx: &ExpansionContext) -> TypeId {
    let i = interner();
    let raw = i.get_type_list(operands);
    let mut literals: Vec<i64> = Vec::with_capacity(raw.len());
    for &operand in raw {
        let expanded = expand_with(operand, world, ctx);
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

fn eval_int_mask_of<W: World>(target: TypeId, world: &W, ctx: &ExpansionContext) -> TypeId {
    let expanded = expand_with(target, world, ctx);
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

/// `properties-of<C>` per spec §7.3: enumerate `C`'s declared
/// properties and produce a sealed `array{name: type, ...}` shape.
/// `visibility` filters the enumeration; `None` keeps every visible
/// property.
fn eval_properties_of<W: World>(
    target: TypeId,
    visibility: Option<crate::element::payload::Visibility>,
    world: &W,
    ctx: &ExpansionContext,
) -> Option<TypeId> {
    let target = expand_with(target, world, ctx);
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

/// `new<C>` per spec §7.3: the type produced by `new C(...)`. The
/// constructor-driven template inference path is left for a later
/// stage; this first cut produces `Object(C)` (with `mixed` filled in
/// for any templates `C` declares) so the result at least has the
/// right nominal class. Returns `None` when the operand can't be
/// reduced to a single class-like name (e.g. a class-string union),
/// leaving the atom unexpanded.
fn eval_new<W: World>(target: TypeId, world: &W, ctx: &ExpansionContext) -> Option<TypeId> {
    let target = expand_with(target, world, ctx);
    let class = extract_class_name_from_class_string_or_object(target)?;

    let arity = world.template_parameter_arity(class);
    let i = interner();
    let info = if arity == 0 {
        ObjectInfo { name: class, type_args: None, intersections: None, flags: ObjectFlags::default() }
    } else {
        let args: Vec<TypeId> = (0..arity)
            .map(|p| world.template_parameter_at(class, p).and_then(|t| t.upper_bound).unwrap_or(TYPE_MIXED))
            .collect();
        ObjectInfo {
            name: class,
            type_args: Some(i.intern_type_list(&args)),
            intersections: None,
            flags: ObjectFlags::default(),
        }
    };
    Some(TypeId::union(&[i.intern_object(info)]))
}

/// Try to read a class-like name from `ty`, accepting either a single
/// `Object(C)` / `Reference(C)` atom or a single literal class-string
/// `class-string<Foo>`.
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

fn expand_object<W: World>(elem: ElementId, world: &W, ctx: &ExpansionContext) -> Expansion {
    let i = interner();
    let info = *i.get_object(elem);

    let (new_args_id, args_changed) = match info.type_args {
        None => (None, false),
        Some(id) => {
            let args = i.get_type_list(id);
            let expanded: Vec<TypeId> = args.iter().map(|&a| expand_with(a, world, ctx)).collect();
            if expanded.iter().zip(args.iter()).all(|(n, o)| n == o) {
                (Some(id), false)
            } else {
                (Some(i.intern_type_list(&expanded)), true)
            }
        }
    };

    let resolved_name = resolve_keyword_name(info.name, info.flags, ctx);
    let name_changed = resolved_name.is_some();

    if !args_changed && !name_changed {
        return Expansion::Unchanged;
    }

    let final_flags = if name_changed { info.flags.with_is_static(false).with_is_this(false) } else { info.flags };
    let new_info = ObjectInfo {
        name: resolved_name.unwrap_or(info.name),
        type_args: new_args_id,
        intersections: info.intersections,
        flags: final_flags,
    };
    Expansion::Single(i.intern_object(new_info))
}

/// Map `self` / `static` / `parent` / the `is_static` / `is_this`
/// modality flags to a concrete class name pulled from `ctx`. Returns
/// `None` when no keyword applies (the atom is a plain `Named(C)`) or
/// when the context lacks the required entry — in either case the
/// caller leaves the atom's name unchanged.
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

fn expand_list<W: World>(elem: ElementId, world: &W, ctx: &ExpansionContext) -> Expansion {
    let i = interner();
    let info = *i.get_list(elem);
    let new_element_type = expand_with(info.element_type, world, ctx);

    let new_known = info.known_elements.map(|id| {
        let entries = i.get_known_elements(id);
        let new_entries: Vec<_> = entries
            .iter()
            .map(|entry| crate::element::payload::KnownElementEntry {
                value: expand_with(entry.value, world, ctx),
                ..*entry
            })
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

fn expand_keyed_array<W: World>(elem: ElementId, world: &W, ctx: &ExpansionContext) -> Expansion {
    let i = interner();
    let info = *i.get_array(elem);
    let new_key = info.key_param.map(|t| expand_with(t, world, ctx));
    let new_value = info.value_param.map(|t| expand_with(t, world, ctx));

    let new_known = info.known_items.map(|id| {
        let entries = i.get_known_items(id);
        let new_entries: Vec<KnownItemEntry> = entries
            .iter()
            .map(|entry| KnownItemEntry { value: expand_with(entry.value, world, ctx), ..*entry })
            .collect();
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

fn expand_iterable<W: World>(elem: ElementId, world: &W, ctx: &ExpansionContext) -> Expansion {
    let i = interner();
    let info = *i.get_iterable(elem);
    let new_key = expand_with(info.key_type, world, ctx);
    let new_value = expand_with(info.value_type, world, ctx);
    if new_key == info.key_type && new_value == info.value_type {
        return Expansion::Unchanged;
    }
    Expansion::Single(i.intern_iterable(IterableInfo { key_type: new_key, value_type: new_value, ..info }))
}

fn expand_class_like_string<W: World>(elem: ElementId, world: &W, ctx: &ExpansionContext) -> Expansion {
    let i = interner();
    let info = *i.get_class_like_string(elem);
    let new_specifier = match info.specifier {
        ClassLikeStringSpecifier::OfType { constraint } => {
            let new_constraint = expand_with(constraint, world, ctx);
            if new_constraint == constraint {
                return Expansion::Unchanged;
            }
            ClassLikeStringSpecifier::OfType { constraint: new_constraint }
        }
        ClassLikeStringSpecifier::Generic { constraint } => {
            let new_constraint = expand_with(constraint, world, ctx);
            if new_constraint == constraint {
                return Expansion::Unchanged;
            }
            ClassLikeStringSpecifier::Generic { constraint: new_constraint }
        }
        _ => return Expansion::Unchanged,
    };
    Expansion::Single(i.intern_class_like_string(ClassLikeStringInfo { specifier: new_specifier, ..info }))
}
