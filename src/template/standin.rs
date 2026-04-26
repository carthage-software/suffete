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
}

impl Default for StandinOptions {
    fn default() -> Self {
        Self { argument_offset: 0, default_variance: Variance::Invariant }
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
}

/// Bounds collected across one or more standin walks. The same
/// [`StandinState`] is threaded through every parameter of a call so
/// reconciliation sees the full bound set per template parameter.
#[derive(Debug, Default, Clone)]
pub struct StandinState {
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
    walk_type(parameter, argument, options.default_variance, 0, world, state, options)
}

fn walk_type<W: World>(
    parameter: TypeId,
    argument: TypeId,
    variance: Variance,
    depth: u32,
    world: &W,
    state: &mut StandinState,
    options: &StandinOptions,
) -> TypeId {
    if parameter == argument {
        return parameter;
    }

    let p_type = parameter.as_ref();
    let mut new_elements: Vec<ElementId> = Vec::with_capacity(p_type.elements.len());
    let mut changed = false;

    for &p_elem in p_type.elements {
        let projected = project_argument(p_elem, argument);
        match walk_element(p_elem, projected, variance, depth, world, state, options) {
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

    let joined = crate::join::compute(&new_elements);
    interner().intern_type(&joined, p_type.flags)
}

enum Walk {
    Unchanged,
    Single(ElementId),
    Many(Vec<ElementId>),
}

fn walk_element<W: World>(
    parameter: ElementId,
    argument: TypeId,
    variance: Variance,
    depth: u32,
    world: &W,
    state: &mut StandinState,
    options: &StandinOptions,
) -> Walk {
    match parameter.kind() {
        ElementKind::GenericParameter => walk_generic_parameter(parameter, argument, variance, depth, state, options),
        ElementKind::Object => walk_object(parameter, argument, depth, world, state, options),
        ElementKind::List => walk_list(parameter, argument, depth, world, state, options),
        ElementKind::Iterable => walk_iterable(parameter, argument, depth, world, state, options),
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
    state: &mut StandinState,
    options: &StandinOptions,
) -> Walk {
    let info = interner().get_generic_parameter(parameter);
    let key = TemplateKey { defining_entity: info.defining_entity, name: info.name };
    let kind = match variance {
        Variance::Covariant => BoundKind::Lower,
        Variance::Contravariant => BoundKind::Upper,
        Variance::Invariant => BoundKind::Equality,
    };
    state.record(key, Bound { kind, ty: argument, argument_offset: options.argument_offset, depth });

    let constraint = info.constraint;
    let elements = constraint.as_ref().elements;
    if elements.len() == 1 { Walk::Single(elements[0]) } else { Walk::Many(elements.to_vec()) }
}

/// `Object(C, [τ_i])` against an argument that resolves to `Object(C,
/// [σ_i])`: walk every type argument by position, with the world-
/// declared variance for that parameter slot. Descendant arguments
/// (where the argument's class is a subclass of `C`) need extension
/// resolution and pass through unchanged in the first cut.
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

    let arg_elems = argument.as_ref().elements;
    let matching_arg = arg_elems
        .iter()
        .find(|&&e| e.kind() == ElementKind::Object && i.get_object(e).name == p_info.name)
        .copied();
    let Some(arg_elem) = matching_arg else {
        return Walk::Unchanged;
    };
    let a_info = *i.get_object(arg_elem);
    let Some(a_args_id) = a_info.type_args else {
        return Walk::Unchanged;
    };
    let a_args = i.get_type_list(a_args_id);

    let mut new_args: Vec<TypeId> = Vec::with_capacity(p_args.len());
    let mut changed = false;
    for (idx, &p_arg) in p_args.iter().enumerate() {
        let Some(&a_arg) = a_args.get(idx) else {
            new_args.push(p_arg);
            continue;
        };
        let variance =
            world.template_parameter_at(p_info.name, idx).map(|p| p.variance).unwrap_or(Variance::Invariant);
        let refined = walk_type(p_arg, a_arg, variance, depth + 1, world, state, options);
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

/// `List(τ)` against `List(σ)` or `Iterable(_, σ)`: walk τ vs σ
/// covariantly. The element type's variance is treated as covariant
/// for inference (per spec §4.2: "covariant positions accumulate
/// lower bounds").
fn walk_list<W: World>(
    parameter: ElementId,
    argument: TypeId,
    depth: u32,
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

    let refined = walk_type(p_info.element_type, a_elem_t, Variance::Covariant, depth + 1, world, state, options);
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

    let new_key = walk_type(p_info.key_type, a_key, Variance::Covariant, depth + 1, world, state, options);
    let new_value = walk_type(p_info.value_type, a_value, Variance::Covariant, depth + 1, world, state, options);
    if new_key == p_info.key_type && new_value == p_info.value_type {
        return Walk::Unchanged;
    }
    Walk::Single(
        i.intern_iterable(crate::element::payload::IterableInfo {
            key_type: new_key,
            value_type: new_value,
            ..p_info
        }),
    )
}

/// Pass the entire argument through to the next walk. Refinements like
/// "pick the Object atom whose name matches" happen inside each
/// per-element handler, not here.
fn project_argument(_parameter: ElementId, argument: TypeId) -> TypeId {
    argument
}
