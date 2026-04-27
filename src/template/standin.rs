//! Standin replacement: template inference at call sites, per
//! `type-system/generics.md` §4.2.
//!
//! The operation walks a *parameter type* alongside an *argument
//! type* in lockstep. Wherever the parameter mentions a template
//! parameter `T`, a bound on `T` is recorded against the corresponding
//! sub-type of the argument; the parameter slot itself is replaced by
//! `T`'s constraint (the loosest type a value at that position can
//! still inhabit). The caller threads the same [`StandinState`] across
//! every parameter of a call site, then runs bound reconciliation
//! (§6) to materialise each `T`'s witness.
//!
//! # Public API
//!
//! ```ignore
//! let mut state = StandinState::new();
//! let opts = StandinOptions::default();
//! let refined = standin(parameter, argument, &world, &mut state, &opts);
//! // ... repeat for each call-site argument ...
//! // Then run reconciliation on `state.bounds_for(...)`.
//! ```
//!
//! # First-cut scope
//!
//! - `GenericParameter T` (anywhere in the parameter tree): record a
//!   bound and emit `T`'s constraint.
//! - Same-class generic objects: walk type arguments by position, with
//!   the world-declared variance for each parameter (covariant ⇒
//!   lower bound, contravariant ⇒ upper, invariant ⇒ equality).
//! - `List(τ)` against `List(σ)` or `Iterable(_, σ)`: covariant walk.
//! - `Iterable(τ_K, τ_V)` against another iterable or a list: covariant
//!   walks on key and value.
//! - Distribution over union: each parameter atom inspects every
//!   argument atom that could contribute (literal-equal atoms are
//!   filtered per spec §4.2.4 once that case is observed).
//!
//! Other parameter shapes (keyed arrays, callables, descendant generic
//! objects, conditional/derived parameters) pass through unchanged for
//! now; precision can only grow as more co-traversal cases land.

use std::collections::BTreeMap;

use mago_atom::Atom;
use mago_span::Span;

use crate::ElementId;
use crate::ElementKind;
use crate::FlowFlags;
use crate::TypeId;
use crate::element::payload::DefiningEntityId;
use crate::interner::interner;
use crate::world::Variance;
use crate::world::World;

/// What kind of bound was recorded on a template parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BoundKind {
    /// `T ≽ τ` — `T` must be a supertype of `τ`. Collected at
    /// covariant positions.
    Lower,
    /// `T ≼ τ` — `T` must be a subtype of `τ`. Collected at
    /// contravariant positions.
    Upper,
    /// `T = τ` — collected at invariant positions; equivalent to a
    /// Lower and Upper bound at the same time.
    Equality,
}

/// One bound entry recorded for a template parameter during the
/// standin walk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bound {
    pub kind: BoundKind,
    pub ty: TypeId,
    /// Call-site argument index (per spec §4.2.2).
    pub argument_offset: u32,
    /// Structural depth at which the bound was collected (per spec
    /// §6.2). The top of the parameter type is depth `0`; each descent
    /// into a generic-parameter application increments by one.
    pub depth: u32,
    /// For [`BoundKind::Equality`] bounds, the class whose template
    /// parameter declaration introduced the equality (the class whose
    /// type-arg position is invariant). `None` for non-equality bounds
    /// and for equality bounds collected outside any class context (e.g.
    /// at the top level of a free-function call).
    pub equality_bound_classlike: Option<Atom>,
    /// Source location of the binding site (the call argument expression
    /// that produced this bound). `None` when the caller did not supply
    /// one — span propagation is opt-in via [`StandinOptions::span`].
    pub span: Option<Span>,
}

/// Identity of a template parameter inside the inference environment.
/// Two parameters with the same surface name in different defining
/// entities (e.g. two unrelated `T`s in two classes) are distinct keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TemplateKey {
    pub defining_entity: DefiningEntityId,
    pub name: Atom,
}

/// Caller-controlled options for [`standin`].
#[derive(Debug, Clone, Copy)]
pub struct StandinOptions {
    /// The call-site argument index this walk corresponds to. Used to
    /// tag bounds so reconciliation (§6) can group them per-position.
    pub argument_offset: u32,
    /// Variance assumed for the top-level walk. Defaults to `Invariant`
    /// — the soundest choice when no surrounding container declares a
    /// position-specific variance.
    pub default_variance: Variance,
    /// Maximum structural descent depth. Walks past this depth replace
    /// the parameter slot with its constraint (no further bound is
    /// recorded). Defaults to `8`, which is enough for realistic PHP
    /// generics while bounding cost on cycles in template constraints.
    pub max_depth: u32,
    /// Source location of the call-site argument the walk is operating
    /// on. Stamped onto every recorded [`Bound`]; consumers use it to
    /// point template-inference diagnostics at the user's code.
    pub span: Option<Span>,
}

impl Default for StandinOptions {
    fn default() -> Self {
        Self { argument_offset: 0, default_variance: Variance::Invariant, max_depth: 8, span: None }
    }
}

impl StandinOptions {
    #[must_use]
    pub const fn with_argument_offset(mut self, offset: u32) -> Self {
        self.argument_offset = offset;
        self
    }

    #[must_use]
    pub const fn with_default_variance(mut self, variance: Variance) -> Self {
        self.default_variance = variance;
        self
    }

    #[must_use]
    pub const fn with_max_depth(mut self, depth: u32) -> Self {
        self.max_depth = depth;
        self
    }

    #[must_use]
    pub const fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }
}

/// Definition of a template parameter as it exists in the inference
/// scope, distinct from any bound inferred for it. Mirrors mago's
/// `GenericTemplate`. See report §11 — the analyzer needs to ask "does
/// this template exist in scope" before it asks "what was inferred",
/// because a template never bound is indistinguishable from one that
/// doesn't exist if you only carry bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenericTemplate {
    pub defining_entity: DefiningEntityId,
    pub constraint: TypeId,
}

/// Definitions and bounds collected across one or more standin walks.
/// The same [`StandinState`] is threaded through every parameter of a
/// call so reconciliation sees the full set per template parameter.
///
/// Two parallel maps:
///
/// - `template_types` records *definitions* — the templates the inference
///   scope knows about, with their constraints.
/// - `bounds` records *inferred bounds* — what the standin walk
///   discovered for each template.
///
/// The walk auto-declares every template it encounters; consumers can
/// also call [`StandinState::declare`] explicitly for templates that
/// don't appear in any walked parameter type.
#[derive(Debug, Default, Clone)]
pub struct StandinState {
    template_types: BTreeMap<TemplateKey, GenericTemplate>,
    bounds: BTreeMap<TemplateKey, Vec<Bound>>,
}

impl StandinState {
    pub fn new() -> Self {
        Self::default()
    }

    /// All bounds recorded for `(defining_entity, name)`, in insertion
    /// order. Empty when no bound has been collected.
    pub fn bounds_for(&self, key: TemplateKey) -> &[Bound] {
        self.bounds.get(&key).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Iterate over every recorded `(key, bounds)` pair.
    pub fn iter(&self) -> impl Iterator<Item = (&TemplateKey, &[Bound])> {
        self.bounds.iter().map(|(k, v)| (k, v.as_slice()))
    }

    /// Register a template parameter as existing in scope. Idempotent on
    /// the same `(key, constraint)` pair; subsequent calls with a
    /// different constraint overwrite (the latest declaration wins, as
    /// inner scopes shadow outer ones).
    pub fn declare(&mut self, key: TemplateKey, constraint: TypeId) {
        self.template_types.insert(key, GenericTemplate { defining_entity: key.defining_entity, constraint });
    }

    /// Declaration recorded for `key`, or `None` when the template has
    /// never been declared (or auto-declared by a walk).
    pub fn declaration(&self, key: TemplateKey) -> Option<&GenericTemplate> {
        self.template_types.get(&key)
    }

    /// `true` iff `key` has a declaration recorded. The analyzer uses
    /// this to distinguish "no bound was inferred for an in-scope
    /// template" from "this template doesn't exist in this scope".
    pub fn is_declared(&self, key: TemplateKey) -> bool {
        self.template_types.contains_key(&key)
    }

    /// Iterate over every recorded `(key, declaration)` pair.
    pub fn declarations(&self) -> impl Iterator<Item = (&TemplateKey, &GenericTemplate)> {
        self.template_types.iter()
    }

    fn record(&mut self, key: TemplateKey, bound: Bound) {
        self.bounds.entry(key).or_default().push(bound);
    }
}

/// Walk `parameter` and `argument` in lockstep; record bounds against
/// any template parameters mentioned in `parameter`. Returns the
/// refined parameter type — the standin — which mentions no template
/// parameter from `parameter`'s defining entity.
///
/// `state` accumulates bounds; reuse one across every parameter of a
/// call site so reconciliation sees the full set.
pub fn standin<W: World>(
    parameter: TypeId,
    argument: TypeId,
    world: &W,
    state: &mut StandinState,
    options: &StandinOptions,
) -> TypeId {
    walk_type(parameter, argument, options.default_variance, 0, None, world, state, options)
}

#[allow(clippy::too_many_arguments)]
fn walk_type<W: World>(
    parameter: TypeId,
    argument: TypeId,
    variance: Variance,
    depth: u32,
    introducing_class: Option<Atom>,
    world: &W,
    state: &mut StandinState,
    options: &StandinOptions,
) -> TypeId {
    if parameter == argument {
        return parameter;
    }
    if depth > options.max_depth {
        return collapse_to_constraints(parameter);
    }

    let p_type = parameter.as_ref();
    let mut new_elements: Vec<ElementId> = Vec::with_capacity(p_type.elements.len());
    let mut changed = false;

    for &p_elem in p_type.elements {
        let projected = project_argument(p_elem, argument);
        match walk_element(p_elem, projected, variance, depth, introducing_class, world, state, options) {
            Walk::Unchanged => new_elements.push(p_elem),
            Walk::Single(e) => {
                changed = true;
                new_elements.push(e);
            }
            Walk::Many(es) => {
                changed = true;
                new_elements.extend(es);
            }
        }
    }

    if !changed {
        return parameter;
    }

    interner().intern_type(&new_elements, p_type.flags)
}

/// Past the iteration-depth cutoff (§4.2.3): replace any template
/// parameter atom in `parameter` with its constraint, leaving other
/// atoms untouched. No bound is recorded — the walk terminated.
fn collapse_to_constraints(parameter: TypeId) -> TypeId {
    let i = interner();
    let p_type = parameter.as_ref();
    let mut new_elements: Vec<ElementId> = Vec::with_capacity(p_type.elements.len());
    let mut changed = false;
    for &p_elem in p_type.elements {
        if p_elem.kind() == ElementKind::GenericParameter {
            let info = i.get_generic_parameter(p_elem);
            new_elements.extend_from_slice(info.constraint.as_ref().elements);
            changed = true;
        } else {
            new_elements.push(p_elem);
        }
    }
    if !changed {
        return parameter;
    }
    i.intern_type(&new_elements, p_type.flags)
}

enum Walk {
    Unchanged,
    Single(ElementId),
    Many(Vec<ElementId>),
}

#[allow(clippy::too_many_arguments)]
fn walk_element<W: World>(
    parameter: ElementId,
    argument: TypeId,
    variance: Variance,
    depth: u32,
    introducing_class: Option<Atom>,
    world: &W,
    state: &mut StandinState,
    options: &StandinOptions,
) -> Walk {
    match parameter.kind() {
        ElementKind::GenericParameter => {
            walk_generic_parameter(parameter, argument, variance, depth, introducing_class, state, options)
        }
        ElementKind::Object => walk_object(parameter, argument, depth, world, state, options),
        ElementKind::List => walk_list(parameter, argument, depth, introducing_class, world, state, options),
        ElementKind::Array => walk_keyed_array(parameter, argument, depth, introducing_class, world, state, options),
        ElementKind::Iterable => walk_iterable(parameter, argument, depth, introducing_class, world, state, options),
        ElementKind::Callable => walk_callable(parameter, argument, depth, introducing_class, world, state, options),
        _ => Walk::Unchanged,
    }
}

/// `T` against argument `ρ`: record a bound on `T` decorated with the
/// current variance, then emit `T`'s constraint as the refined type.
/// When the constraint is unbounded (`mixed`), the standin keeps the
/// loosest possible witness.
fn walk_generic_parameter(
    parameter: ElementId,
    argument: TypeId,
    variance: Variance,
    depth: u32,
    introducing_class: Option<Atom>,
    state: &mut StandinState,
    options: &StandinOptions,
) -> Walk {
    let info = interner().get_generic_parameter(parameter);
    let key = TemplateKey { defining_entity: info.defining_entity, name: info.name };
    if !state.is_declared(key) {
        state.declare(key, info.constraint);
    }
    let kind = match variance {
        Variance::Covariant => BoundKind::Lower,
        Variance::Contravariant => BoundKind::Upper,
        Variance::Invariant => BoundKind::Equality,
    };
    let equality_bound_classlike = if matches!(kind, BoundKind::Equality) { introducing_class } else { None };
    state.record(
        key,
        Bound {
            kind,
            ty: argument,
            argument_offset: options.argument_offset,
            depth,
            equality_bound_classlike,
            span: options.span,
        },
    );

    let constraint = info.constraint;
    let elements = constraint.as_ref().elements;
    if elements.len() == 1 { Walk::Single(elements[0]) } else { Walk::Many(elements.to_vec()) }
}

/// `Object(C, [τ_i])` against an argument that resolves to a class in
/// `C`'s closure. Same-class args walk by position; descendant args
/// (`D <: C`) project through `World::inherited_template_argument` and
/// then substitute `D`'s actual type arguments to recover the type
/// `D` passes for `C`'s `i`-th slot. The variance comes from `C`'s
/// declaration, not `D`'s.
fn walk_object<W: World>(
    parameter: ElementId,
    argument: TypeId,
    depth: u32,
    world: &W,
    state: &mut StandinState,
    options: &StandinOptions,
) -> Walk {
    let i = interner();
    let p_info = *i.get_object(parameter);
    let Some(p_args_id) = p_info.type_args else {
        return Walk::Unchanged;
    };
    let p_args: Vec<TypeId> = i.get_type_list(p_args_id).to_vec();

    let matching_arg = argument.as_ref().elements.iter().copied().find(|&e| {
        e.kind() == ElementKind::Object && {
            let a_info = i.get_object(e);
            a_info.name == p_info.name || world.descends_from(a_info.name, p_info.name)
        }
    });
    let Some(arg_elem) = matching_arg else {
        return Walk::Unchanged;
    };
    let a_info = *i.get_object(arg_elem);

    let mut new_args: Vec<TypeId> = Vec::with_capacity(p_args.len());
    let mut changed = false;
    for (idx, &p_arg) in p_args.iter().enumerate() {
        let a_arg = projected_object_arg(p_info.name, &a_info, idx, world);
        let variance = world.template_parameter_at(p_info.name, idx).map(|p| p.variance).unwrap_or(Variance::Invariant);
        let refined = match a_arg {
            Some(t) => walk_type(p_arg, t, variance, depth + 1, Some(p_info.name), world, state, options),
            None => p_arg,
        };
        if refined != p_arg {
            changed = true;
        }
        new_args.push(refined);
    }

    if !changed {
        return Walk::Unchanged;
    }

    let new_args_id = i.intern_type_list(&new_args);
    Walk::Single(i.intern_object(crate::element::payload::ObjectInfo { type_args: Some(new_args_id), ..p_info }))
}

/// Pick the type the argument passes to `container_class`'s `position`-th
/// type parameter. For same-class arguments that's just
/// `argument.type_args[position]`. For descendant arguments it goes
/// through `World::inherited_template_argument` and substitutes the
/// argument's own template arguments into the inherited expression.
fn projected_object_arg<W: World>(
    container_class: Atom,
    argument_object: &crate::element::payload::ObjectInfo,
    position: usize,
    world: &W,
) -> Option<TypeId> {
    let i = interner();

    if argument_object.name == container_class {
        let id = argument_object.type_args?;
        return i.get_type_list(id).get(position).copied();
    }

    let inherited = world.inherited_template_argument(argument_object.name, container_class, position)?;

    let actual_args: Vec<TypeId> = argument_object.type_args.map(|id| i.get_type_list(id).to_vec()).unwrap_or_default();
    let argument_entity =
        i.intern_defining_entity(crate::element::payload::DefiningEntity::ClassLike(argument_object.name));

    Some(crate::template::substitute(
        inherited,
        &|info: &crate::element::payload::GenericParameterInfo| -> Option<TypeId> {
            if info.defining_entity != argument_entity {
                return None;
            }
            let pos = world.template_parameter_index(argument_object.name, info.name)?;
            actual_args.get(pos).copied()
        },
    ))
}

/// `List(τ)` against `List(σ)` or `Iterable(_, σ)`: walk τ vs σ
/// covariantly. The element type's variance is treated as covariant
/// for inference (per spec §4.2: "covariant positions accumulate
/// lower bounds").
fn walk_list<W: World>(
    parameter: ElementId,
    argument: TypeId,
    depth: u32,
    introducing_class: Option<Atom>,
    world: &W,
    state: &mut StandinState,
    options: &StandinOptions,
) -> Walk {
    let i = interner();
    let p_info = *i.get_list(parameter);

    let arg_element_type = argument.as_ref().elements.iter().find_map(|&e| match e.kind() {
        ElementKind::List => Some(i.get_list(e).element_type),
        ElementKind::Iterable => Some(i.get_iterable(e).value_type),
        _ => None,
    });

    let Some(a_elem_t) = arg_element_type else {
        return Walk::Unchanged;
    };

    let refined =
        walk_type(p_info.element_type, a_elem_t, Variance::Covariant, depth + 1, introducing_class, world, state, options);
    if refined == p_info.element_type {
        return Walk::Unchanged;
    }
    Walk::Single(i.intern_list(crate::element::payload::ListInfo { element_type: refined, ..p_info }))
}

/// `Iterable(τ_K, τ_V)` against `Iterable(σ_K, σ_V)` or `List(σ)`
/// (which exposes `int` keys and `σ` values).
fn walk_iterable<W: World>(
    parameter: ElementId,
    argument: TypeId,
    depth: u32,
    introducing_class: Option<Atom>,
    world: &W,
    state: &mut StandinState,
    options: &StandinOptions,
) -> Walk {
    let i = interner();
    let p_info = *i.get_iterable(parameter);

    let pair = argument.as_ref().elements.iter().find_map(|&e| match e.kind() {
        ElementKind::Iterable => {
            let info = i.get_iterable(e);
            Some((info.key_type, info.value_type))
        }
        ElementKind::List => {
            let info = i.get_list(e);
            Some((interner().intern_type(&[crate::prelude::INT], FlowFlags::EMPTY), info.element_type))
        }
        _ => None,
    });

    let Some((a_key, a_value)) = pair else {
        return Walk::Unchanged;
    };

    let new_key =
        walk_type(p_info.key_type, a_key, Variance::Covariant, depth + 1, introducing_class, world, state, options);
    let new_value =
        walk_type(p_info.value_type, a_value, Variance::Covariant, depth + 1, introducing_class, world, state, options);
    if new_key == p_info.key_type && new_value == p_info.value_type {
        return Walk::Unchanged;
    }
    Walk::Single(i.intern_iterable(crate::element::payload::IterableInfo {
        key_type: new_key,
        value_type: new_value,
        ..p_info
    }))
}

/// `Keyed(τ_K, τ_V, {k → τ})` against a keyed-array argument: walk
/// `τ_K` against the argument's key parameter (covariantly), `τ_V`
/// against the value parameter, and each known item against the
/// argument's matching known item. Iterable arguments contribute their
/// key/value to `τ_K` / `τ_V` only — known-item entries don't have a
/// corresponding projection.
fn walk_keyed_array<W: World>(
    parameter: ElementId,
    argument: TypeId,
    depth: u32,
    introducing_class: Option<Atom>,
    world: &W,
    state: &mut StandinState,
    options: &StandinOptions,
) -> Walk {
    let i = interner();
    let p_info = *i.get_array(parameter);

    let arg_pair = argument.as_ref().elements.iter().find_map(|&e| match e.kind() {
        ElementKind::Array => {
            let info = *i.get_array(e);
            Some(KeyedProjection { key: info.key_param, value: info.value_param, known_items: info.known_items })
        }
        ElementKind::Iterable => {
            let info = i.get_iterable(e);
            Some(KeyedProjection { key: Some(info.key_type), value: Some(info.value_type), known_items: None })
        }
        _ => None,
    });
    let Some(arg) = arg_pair else {
        return Walk::Unchanged;
    };

    let new_key = match (p_info.key_param, arg.key) {
        (Some(p_k), Some(a_k)) => {
            Some(walk_type(p_k, a_k, Variance::Covariant, depth + 1, introducing_class, world, state, options))
        }
        _ => p_info.key_param,
    };
    let new_value = match (p_info.value_param, arg.value) {
        (Some(p_v), Some(a_v)) => {
            Some(walk_type(p_v, a_v, Variance::Covariant, depth + 1, introducing_class, world, state, options))
        }
        _ => p_info.value_param,
    };

    let new_known = p_info.known_items.map(|id| {
        let p_entries = i.get_known_items(id);
        let a_entries: &[crate::element::payload::KnownItemEntry] =
            arg.known_items.map(|aid| i.get_known_items(aid)).unwrap_or(&[]);
        let mut new_entries: Vec<crate::element::payload::KnownItemEntry> = Vec::with_capacity(p_entries.len());
        let mut changed_inner = false;
        for entry in p_entries {
            let arg_value = a_entries.iter().find(|e| e.key == entry.key).map(|e| e.value);
            let refined_value = match arg_value {
                Some(av) => {
                    walk_type(entry.value, av, Variance::Covariant, depth + 1, introducing_class, world, state, options)
                }
                None => entry.value,
            };
            if refined_value != entry.value {
                changed_inner = true;
            }
            new_entries.push(crate::element::payload::KnownItemEntry { value: refined_value, ..*entry });
        }
        if changed_inner { (i.intern_known_items(&new_entries), true) } else { (id, false) }
    });

    let key_changed = new_key != p_info.key_param;
    let value_changed = new_value != p_info.value_param;
    let known_changed = new_known.is_some_and(|(_, ch)| ch);
    if !key_changed && !value_changed && !known_changed {
        return Walk::Unchanged;
    }

    Walk::Single(i.intern_array(crate::element::payload::KeyedArrayInfo {
        key_param: new_key,
        value_param: new_value,
        known_items: new_known.map(|(id, _)| id),
        ..p_info
    }))
}

struct KeyedProjection {
    key: Option<TypeId>,
    value: Option<TypeId>,
    known_items: Option<crate::element::payload::KnownItemsId>,
}

/// `Callable(Sig(s_p))` against a callable argument: walk parameter
/// types pointwise contravariantly and the return type covariantly.
/// Aliases and `Any` callables pass through.
fn walk_callable<W: World>(
    parameter: ElementId,
    argument: TypeId,
    depth: u32,
    introducing_class: Option<Atom>,
    world: &W,
    state: &mut StandinState,
    options: &StandinOptions,
) -> Walk {
    use crate::element::payload::CallableInfo;

    let i = interner();
    let p_info = *i.get_callable(parameter);
    let p_sig_id = match p_info {
        CallableInfo::Signature(s) | CallableInfo::Closure(s) => s,
        _ => return Walk::Unchanged,
    };

    let arg_sig_id = argument.as_ref().elements.iter().find_map(|&e| {
        if e.kind() != ElementKind::Callable {
            return None;
        }
        match *i.get_callable(e) {
            CallableInfo::Signature(s) | CallableInfo::Closure(s) => Some(s),
            _ => None,
        }
    });
    let Some(a_sig_id) = arg_sig_id else {
        return Walk::Unchanged;
    };

    let p_sig = *i.get_signature(p_sig_id);
    let a_sig = *i.get_signature(a_sig_id);

    let new_return = walk_type(
        p_sig.return_type,
        a_sig.return_type,
        Variance::Covariant,
        depth + 1,
        introducing_class,
        world,
        state,
        options,
    );

    let new_param_list = p_sig.parameters.map(|pid| {
        let p_params = i.get_param_list(pid);
        let a_params: &[crate::element::payload::ParamInfo] =
            a_sig.parameters.map(|aid| i.get_param_list(aid)).unwrap_or(&[]);
        let mut new_params: Vec<crate::element::payload::ParamInfo> = Vec::with_capacity(p_params.len());
        let mut changed_inner = false;
        for (idx, p_param) in p_params.iter().enumerate() {
            let arg_param_type = a_params.get(idx).map(|p| p.type_);
            let refined = match arg_param_type {
                Some(at) => walk_type(
                    p_param.type_,
                    at,
                    Variance::Contravariant,
                    depth + 1,
                    introducing_class,
                    world,
                    state,
                    options,
                ),
                None => p_param.type_,
            };
            if refined != p_param.type_ {
                changed_inner = true;
            }
            new_params.push(crate::element::payload::ParamInfo { type_: refined, ..*p_param });
        }
        if changed_inner { (i.intern_param_list(&new_params), true) } else { (pid, false) }
    });

    let return_changed = new_return != p_sig.return_type;
    let params_changed = new_param_list.is_some_and(|(_, ch)| ch);
    if !return_changed && !params_changed {
        return Walk::Unchanged;
    }

    let new_sig = i.intern_signature(crate::element::payload::Signature {
        return_type: new_return,
        parameters: new_param_list.map(|(id, _)| id).or(p_sig.parameters),
        ..p_sig
    });
    let new_callable = match p_info {
        CallableInfo::Signature(_) => CallableInfo::Signature(new_sig),
        CallableInfo::Closure(_) => CallableInfo::Closure(new_sig),
        _ => return Walk::Unchanged,
    };
    Walk::Single(i.intern_callable(new_callable))
}

/// Pass the entire argument through to the next walk. Refinements like
/// "pick the Object atom whose name matches" happen inside each
/// per-element handler, not here.
fn project_argument(_parameter: ElementId, argument: TypeId) -> TypeId {
    argument
}
