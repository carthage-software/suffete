#![allow(dead_code)]

//! Test helpers translated to suffete's lattice + world APIs. Test files
//! in `tests/comparator_*.rs` consume these helpers.

use std::collections::HashMap;
use std::collections::HashSet;

use mago_atom::Atom;
use mago_atom::atom;

use suffete::ElementId;
use suffete::FlowFlags;
use suffete::TypeId;
use suffete::element::payload::ArrayKey;
use suffete::element::payload::KnownItemEntry;
use suffete::interner::interner;
use suffete::lattice;
use suffete::lattice::LatticeOptions;
use suffete::lattice::LatticeReport;
use suffete::prelude;
use suffete::world::NullWorld;
use suffete::world::TemplateParameter;
use suffete::world::Variance;
use suffete::world::World;

/// A [`World`] backed by hand-built tables: hierarchy edges, trait
/// usage, per-class type parameters, and per-extension type arguments.
///
/// Builder API:
///
/// - [`add_edge`](Self::add_edge): nominal `child extends/implements/uses
///   parent` edge.
/// - [`add_trait_use`](Self::add_trait_use): explicit trait usage (also
///   counts as an edge).
/// - [`declare`](Self::declare): register a class-like with no ancestors
///   so reflexive queries work for it.
/// - [`with_templates`](Self::with_templates): declare a class's type
///   parameters in order.
/// - [`with_extended`](Self::with_extended): declare what type arguments
///   `child` passes to `ancestor`'s type parameters (in `ancestor`'s
///   declaration order).
pub struct MockWorld {
    /// `child -> {ancestors including child itself}`.
    ancestors: HashMap<Atom, HashSet<Atom>>,
    /// `class -> {trait, trait, ...}` for direct trait usage.
    traits_used: HashMap<Atom, HashSet<Atom>>,
    /// `class -> [type params in declaration order]`.
    templates: HashMap<Atom, Vec<TemplateParameter>>,
    /// `(child, ancestor) -> [type_argument]` indexed by `ancestor`'s
    /// declaration order, expressed in `child`'s template namespace.
    extended: HashMap<(Atom, Atom), Vec<TypeId>>,
}

impl MockWorld {
    pub fn new() -> Self {
        Self {
            ancestors: HashMap::new(),
            traits_used: HashMap::new(),
            templates: HashMap::new(),
            extended: HashMap::new(),
        }
    }

    /// Add a single `child extends/implements parent` edge and recompute
    /// the transitive closure.
    pub fn add_edge(&mut self, child: &str, parent: &str) -> &mut Self {
        let child = atom(child);
        let parent = atom(parent);
        self.ancestors.entry(child).or_default().insert(child);
        self.ancestors.entry(parent).or_default().insert(parent);
        self.ancestors.get_mut(&child).unwrap().insert(parent);
        self.recompute_closure();
        self
    }

    /// Build from a list of `(child, parent)` pairs in one shot.
    pub fn from_edges(edges: &[(&str, &str)]) -> Self {
        let mut w = Self::new();
        for (c, p) in edges {
            w.add_edge(c, p);
        }
        w
    }

    /// Register a `class uses TraitName;` relation. Also records the
    /// edge for the ancestor closure (so [`World::descends_from`] answers
    /// yes) and the direct usage (so [`World::uses_trait`] also answers
    /// yes).
    pub fn add_trait_use(&mut self, class: &str, trait_: &str) -> &mut Self {
        self.add_edge(class, trait_);
        self.traits_used.entry(atom(class)).or_default().insert(atom(trait_));
        self
    }

    /// Register a class-like with no ancestors (so reflexive queries
    /// like `descends_from(C, C)` still answer yes).
    pub fn declare(&mut self, name: &str) -> &mut Self {
        let n = atom(name);
        self.ancestors.entry(n).or_default().insert(n);
        self
    }

    /// Declare `class_like`'s type parameters in declaration order. Each
    /// is a `(name, variance)` pair; bounds default to `None`.
    pub fn with_templates(&mut self, class_like: &str, params: &[(&str, Variance)]) -> &mut Self {
        let n = atom(class_like);
        self.ancestors.entry(n).or_default().insert(n);
        self.templates.insert(
            n,
            params
                .iter()
                .map(|(name, variance)| TemplateParameter { name: atom(name), variance: *variance, upper_bound: None })
                .collect(),
        );
        self
    }

    /// Declare what type arguments `child` passes to `ancestor`. The
    /// list is positional, in `ancestor`'s declaration order. Argument
    /// type ids may reference `child`'s own templates (via
    /// [`GenericParameter`](suffete::ElementKind::GenericParameter)
    /// elements) or be concrete.
    ///
    /// Implicitly registers `child extends ancestor` so
    /// [`World::descends_from`] answers yes.
    pub fn with_extended(&mut self, child: &str, ancestor: &str, args: Vec<TypeId>) -> &mut Self {
        self.add_edge(child, ancestor);
        self.extended.insert((atom(child), atom(ancestor)), args);
        self
    }

    fn recompute_closure(&mut self) {
        loop {
            let mut changed = false;
            let names: Vec<Atom> = self.ancestors.keys().copied().collect();
            for name in &names {
                let direct: Vec<Atom> = self.ancestors[name].iter().copied().collect();
                for d in direct {
                    if let Some(ancestors_of_d) = self.ancestors.get(&d).cloned() {
                        let entry = self.ancestors.get_mut(name).unwrap();
                        for a in ancestors_of_d {
                            if entry.insert(a) {
                                changed = true;
                            }
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }
    }
}

impl Default for MockWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl World for MockWorld {
    fn descends_from(&self, child: Atom, ancestor: Atom) -> bool {
        if child == ancestor {
            return true;
        }
        self.ancestors.get(&child).is_some_and(|set| set.contains(&ancestor))
    }

    fn uses_trait(&self, class: Atom, trait_: Atom) -> bool {
        self.traits_used.get(&class).is_some_and(|set| set.contains(&trait_))
    }

    fn arity(&self, class: Atom) -> usize {
        self.templates.get(&class).map(Vec::len).unwrap_or(0)
    }

    fn parameter_at(&self, class: Atom, position: usize) -> Option<TemplateParameter> {
        self.templates.get(&class).and_then(|params| params.get(position).cloned())
    }

    fn parameter_position(&self, class: Atom, name: Atom) -> Option<usize> {
        self.templates.get(&class).and_then(|params| params.iter().position(|p| p.name == name))
    }

    fn inherited_argument(&self, child: Atom, ancestor: Atom, position: usize) -> Option<TypeId> {
        if !self.descends_from(child, ancestor) {
            return None;
        }
        self.extended.get(&(child, ancestor)).and_then(|args| args.get(position).copied())
    }
}

/// An empty world: nothing knows about anything.
pub fn empty_world() -> NullWorld {
    NullWorld
}

// ---------------------------------------------------------------------------
// Refinement queries.
// ---------------------------------------------------------------------------

/// `lattice::refines(input, container, codebase, default options, _)`.
/// Mirrors mago's `union_comparator::is_contained_by`.
pub fn is_contained<W: World>(input: TypeId, container: TypeId, codebase: &W) -> bool {
    let mut report = LatticeReport::new();
    lattice::refines(input, container, codebase, LatticeOptions::default(), &mut report)
}

/// As [`is_contained`], but returns the [`LatticeReport`] alongside the
/// boolean answer so tests can inspect coercion flags.
pub fn is_contained_capturing<W: World>(input: TypeId, container: TypeId, codebase: &W) -> (bool, LatticeReport) {
    let mut report = LatticeReport::new();
    let v = lattice::refines(input, container, codebase, LatticeOptions::default(), &mut report);
    (v, report)
}

/// `is_contained` with the `ignore_null` / `ignore_false` / `inside_assertion`
/// option flags. Mirrors mago's
/// `is_contained_by(..., ignore_null, ignore_false, inside_assertion, _)`.
pub fn is_contained_with<W: World>(
    input: TypeId,
    container: TypeId,
    codebase: &W,
    ignore_null: bool,
    ignore_false: bool,
    inside_assertion: bool,
) -> bool {
    let options = LatticeOptions { ignore_null, ignore_false, inside_assertion };
    let mut report = LatticeReport::new();
    lattice::refines(input, container, codebase, options, &mut report)
}

/// Element-vs-element refinement query: wraps both in singleton unions and
/// calls [`is_contained`].
pub fn atomic_is_contained<W: World>(input: ElementId, container: ElementId, codebase: &W) -> bool {
    let i = interner();
    let it = i.intern_type(&[input], FlowFlags::EMPTY);
    let ct = i.intern_type(&[container], FlowFlags::EMPTY);
    is_contained(it, ct, codebase)
}

pub fn atomic_is_contained_capturing<W: World>(
    input: ElementId,
    container: ElementId,
    codebase: &W,
) -> (bool, LatticeReport) {
    let i = interner();
    let it = i.intern_type(&[input], FlowFlags::EMPTY);
    let ct = i.intern_type(&[container], FlowFlags::EMPTY);
    is_contained_capturing(it, ct, codebase)
}

// ---------------------------------------------------------------------------
// Assertion helpers.
// ---------------------------------------------------------------------------

#[track_caller]
pub fn assert_subtype(input: &TypeId, container: &TypeId) {
    let cb = empty_world();
    assert!(is_contained(*input, *container, &cb), "expected {input:?} <: {container:?} but it is not");
}

#[track_caller]
pub fn assert_not_subtype(input: &TypeId, container: &TypeId) {
    let cb = empty_world();
    assert!(!is_contained(*input, *container, &cb), "expected NOT ({input:?} <: {container:?}) but it is");
}

#[track_caller]
pub fn assert_atomic_subtype(input: &ElementId, container: &ElementId) {
    let cb = empty_world();
    assert!(atomic_is_contained(*input, *container, &cb), "expected atomic {input:?} <: {container:?}");
}

#[track_caller]
pub fn assert_atomic_not_subtype(input: &ElementId, container: &ElementId) {
    let cb = empty_world();
    assert!(!atomic_is_contained(*input, *container, &cb), "expected NOT (atomic {input:?} <: {container:?})");
}

// ---------------------------------------------------------------------------
// Element constructors. Mirrors mago's `t_*` helpers.
// ---------------------------------------------------------------------------

pub fn never() -> ElementId {
    prelude::NEVER
}
pub fn null() -> ElementId {
    prelude::NULL
}
pub fn void() -> ElementId {
    prelude::VOID
}
pub fn placeholder() -> ElementId {
    prelude::PLACEHOLDER
}
pub fn mixed() -> ElementId {
    prelude::MIXED
}
pub fn mixed_truthy() -> ElementId {
    prelude::TRUTHY_MIXED
}
pub fn mixed_falsy() -> ElementId {
    prelude::FALSY_MIXED
}
pub fn mixed_nonnull() -> ElementId {
    prelude::NON_NULL_MIXED
}

pub fn t_true() -> ElementId {
    prelude::TRUE
}
pub fn t_false() -> ElementId {
    prelude::FALSE
}
pub fn t_bool() -> ElementId {
    prelude::BOOL
}

pub fn t_int() -> ElementId {
    prelude::INT
}
pub fn t_lit_int(v: i64) -> ElementId {
    ElementId::int_literal(v)
}
pub fn t_int_from(from: i64) -> ElementId {
    ElementId::int_range(Some(from), None)
}
pub fn t_int_to(to: i64) -> ElementId {
    ElementId::int_range(None, Some(to))
}
pub fn t_int_range(lo: i64, hi: i64) -> ElementId {
    ElementId::int_range(Some(lo), Some(hi))
}
pub fn t_int_unspec_lit() -> ElementId {
    prelude::LITERAL_INT
}
pub fn t_positive_int() -> ElementId {
    prelude::POSITIVE_INT
}
pub fn t_negative_int() -> ElementId {
    prelude::NEGATIVE_INT
}
pub fn t_non_negative_int() -> ElementId {
    prelude::NON_NEGATIVE_INT
}
pub fn t_non_positive_int() -> ElementId {
    prelude::NON_POSITIVE_INT
}

pub fn t_float() -> ElementId {
    prelude::FLOAT
}
pub fn t_lit_float(v: f64) -> ElementId {
    ElementId::float_literal(v)
}
pub fn t_unspec_lit_float() -> ElementId {
    prelude::LITERAL_FLOAT
}

pub fn t_string() -> ElementId {
    prelude::STRING
}
pub fn t_lit_string(s: &str) -> ElementId {
    ElementId::string_literal(s)
}
pub fn t_non_empty_string() -> ElementId {
    prelude::NON_EMPTY_STRING
}
pub fn t_numeric_string() -> ElementId {
    prelude::NUMERIC_STRING
}
pub fn t_lower_string() -> ElementId {
    prelude::LOWERCASE_STRING
}
pub fn t_upper_string() -> ElementId {
    prelude::UPPERCASE_STRING
}
pub fn t_truthy_string() -> ElementId {
    prelude::TRUTHY_STRING
}
pub fn t_unspec_lit_string(non_empty: bool) -> ElementId {
    if non_empty { prelude::NON_EMPTY_LITERAL_STRING } else { prelude::LITERAL_STRING }
}
pub fn t_callable_string() -> ElementId {
    prelude::CALLABLE_STRING
}

pub fn t_array_key() -> ElementId {
    prelude::ARRAY_KEY
}
pub fn t_numeric() -> ElementId {
    prelude::NUMERIC
}
pub fn t_scalar() -> ElementId {
    prelude::SCALAR
}

pub fn t_class_string() -> ElementId {
    prelude::CLASS_STRING
}
pub fn t_interface_string() -> ElementId {
    prelude::INTERFACE_STRING
}
pub fn t_enum_string() -> ElementId {
    prelude::ENUM_STRING
}
pub fn t_trait_string() -> ElementId {
    prelude::TRAIT_STRING
}
pub fn t_lit_class_string(name: &str) -> ElementId {
    ElementId::class_string_literal(name)
}

pub fn t_resource() -> ElementId {
    prelude::RESOURCE
}
pub fn t_open_resource() -> ElementId {
    prelude::OPEN_RESOURCE
}
pub fn t_closed_resource() -> ElementId {
    prelude::CLOSED_RESOURCE
}

pub fn t_object_any() -> ElementId {
    prelude::OBJECT
}
pub fn t_named(name: &str) -> ElementId {
    ElementId::object_named(name)
}
pub fn t_enum(name: &str) -> ElementId {
    ElementId::enum_any(name)
}
pub fn t_enum_case(name: &str, case: &str) -> ElementId {
    ElementId::enum_case(name, case)
}

pub fn t_empty_array() -> ElementId {
    prelude::EMPTY_ARRAY
}

pub fn t_list(element: TypeId, non_empty: bool) -> ElementId {
    ElementId::list(element, non_empty)
}

pub fn t_keyed_unsealed(key: TypeId, value: TypeId, non_empty: bool) -> ElementId {
    ElementId::keyed_unsealed(key, value, non_empty)
}

pub fn t_keyed_sealed(items: std::collections::BTreeMap<ArrayKey, (bool, TypeId)>, non_empty: bool) -> ElementId {
    let entries: Vec<KnownItemEntry> =
        items.into_iter().map(|(key, (optional, value))| KnownItemEntry { key, value, optional }).collect();
    ElementId::keyed_sealed(&entries, non_empty)
}

pub fn t_iterable(key: TypeId, value: TypeId) -> ElementId {
    ElementId::iterable(key, value)
}

pub fn t_callable_mixed() -> ElementId {
    ElementId::callable_mixed()
}

pub fn t_closure_mixed() -> ElementId {
    ElementId::closure_mixed()
}

pub fn ak_int(n: i64) -> ArrayKey {
    ArrayKey::Int(n)
}

pub fn ak_str(s: &str) -> ArrayKey {
    ArrayKey::String(atom(s))
}

// ---------------------------------------------------------------------------
// Union builders. Mirrors mago's `u`, `u_many`, `ui`, `us`.
// ---------------------------------------------------------------------------

pub fn u(a: ElementId) -> TypeId {
    interner().intern_type(&[a], FlowFlags::EMPTY)
}

pub fn u_many(atoms: Vec<ElementId>) -> TypeId {
    interner().intern_type(&atoms, FlowFlags::EMPTY)
}

pub fn ui(v: i64) -> TypeId {
    u(t_lit_int(v))
}

pub fn us(s: &str) -> TypeId {
    u(t_lit_string(s))
}

pub fn name(s: &str) -> Atom {
    atom(s)
}
