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
use suffete::element::payload::Visibility;
use suffete::interner::interner;
use suffete::lattice;
use suffete::lattice::LatticeOptions;
use suffete::lattice::LatticeReport;
use suffete::prelude;
use suffete::world::ClassProperty;
use suffete::world::EnumBacking;
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
#[derive(Debug)]
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
    /// `class -> {method, method, ...}` for methods declared directly on
    /// the class. Inheritance is walked via `ancestors` at query time.
    methods: HashMap<Atom, HashSet<Atom>>,
    /// Per-class ordered list of declared properties. Each entry
    /// carries the name, declared type, and visibility. Inheritance
    /// is walked at query time across `ancestors`.
    properties: HashMap<Atom, Vec<ClassProperty>>,
    /// `enum -> backing kind` for declared enums.
    enums: HashMap<Atom, EnumBacking>,
    /// `name -> declared class-like kind` (interface / trait when set
    /// explicitly; classes and enums are inferred). Names absent here
    /// are treated as plain classes when they appear in `ancestors`.
    class_like_kinds: HashMap<Atom, suffete::element::payload::ClassLikeKind>,
    /// `class -> ()` for classes declared `final` (or implicitly so,
    /// like enums). Used by [`World::is_final`].
    final_classes: HashSet<Atom>,
    /// `(class, alias_name) -> alias body type` for declared `@type`
    /// aliases. Body may itself contain other aliases, references, or
    /// derived types ; expansion is recursive.
    aliases: HashMap<(Atom, Atom), TypeId>,
    /// `(class, constant_name) -> constant type` for class-level constants.
    class_constants: HashMap<(Atom, Atom), TypeId>,
    /// `name -> constant type` for global constants.
    global_constants: HashMap<Atom, TypeId>,
}

impl MockWorld {
    pub fn new() -> Self {
        Self {
            ancestors: HashMap::new(),
            traits_used: HashMap::new(),
            templates: HashMap::new(),
            extended: HashMap::new(),
            methods: HashMap::new(),
            properties: HashMap::new(),
            enums: HashMap::new(),
            class_like_kinds: HashMap::new(),
            final_classes: HashSet::new(),
            aliases: HashMap::new(),
            class_constants: HashMap::new(),
            global_constants: HashMap::new(),
        }
    }

    /// Add a single `child extends/implements parent` edge and recompute
    /// the transitive closure.
    pub fn add_edge(&mut self, child: &str, parent: &str) -> &mut Self {
        let child_atom = atom(child);
        let parent_atom = atom(parent);
        self.ancestors.entry(child_atom).or_default().insert(child_atom);
        self.ancestors.entry(parent_atom).or_default().insert(parent_atom);
        self.ancestors.get_mut(&child_atom).unwrap().insert(parent_atom);
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

    /// Tag `name` as an interface so [`World::class_like_kind`] returns
    /// `Interface`. Implicitly declares `name` for the ancestor closure.
    pub fn declare_interface(&mut self, name: &str) -> &mut Self {
        let n = atom(name);
        self.ancestors.entry(n).or_default().insert(n);
        self.class_like_kinds.insert(n, suffete::element::payload::ClassLikeKind::Interface);
        self
    }

    /// Tag `name` as a trait so [`World::class_like_kind`] returns
    /// `Trait`. Implicitly declares `name` for the ancestor closure.
    pub fn declare_trait(&mut self, name: &str) -> &mut Self {
        let n = atom(name);
        self.ancestors.entry(n).or_default().insert(n);
        self.class_like_kinds.insert(n, suffete::element::payload::ClassLikeKind::Trait);
        self
    }

    /// Mark `name` as `final` (no subclasses possible). Implicitly
    /// declares `name` for the ancestor closure.
    pub fn with_final(&mut self, name: &str) -> &mut Self {
        let n = atom(name);
        self.ancestors.entry(n).or_default().insert(n);
        self.final_classes.insert(n);
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

    /// Set the upper bound (`@template T of Foo`) on `class_like`'s
    /// `name`d template parameter.
    pub fn with_template_bound(&mut self, class_like: &str, name: &str, bound: TypeId) -> &mut Self {
        let class = atom(class_like);
        let template = atom(name);
        if let Some(params) = self.templates.get_mut(&class)
            && let Some(p) = params.iter_mut().find(|p| p.name == template)
        {
            p.upper_bound = Some(bound);
        }
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

    /// Declare that `class` has a method `name` (directly; inheritance
    /// is walked at query time).
    pub fn with_method(&mut self, class: &str, name: &str) -> &mut Self {
        let n = atom(class);
        self.ancestors.entry(n).or_default().insert(n);
        self.methods.entry(n).or_default().insert(atom(name));
        self
    }

    /// Declare a public property `name: type` on `class`. Use
    /// [`with_visible_property`](Self::with_visible_property) to set a
    /// non-default visibility.
    pub fn with_property(&mut self, class: &str, name: &str, type_: TypeId) -> &mut Self {
        self.with_visible_property(class, name, type_, Visibility::Public)
    }

    /// Declare a property with explicit visibility.
    pub fn with_visible_property(
        &mut self,
        class: &str,
        name: &str,
        type_: TypeId,
        visibility: Visibility,
    ) -> &mut Self {
        let n = atom(class);
        self.ancestors.entry(n).or_default().insert(n);
        self.properties.entry(n).or_default().push(ClassProperty { name: atom(name), type_, visibility });
        self
    }

    /// Declare a pure enum: cases expose only `name`.
    pub fn with_pure_enum(&mut self, name: &str) -> &mut Self {
        let n = atom(name);
        self.ancestors.entry(n).or_default().insert(n);
        self.enums.insert(n, EnumBacking::Pure);
        self
    }

    /// Declare a backed enum: cases expose `name` and `value`, where
    /// `value` is of `backing` (typically `int` or `string`).
    pub fn with_backed_enum(&mut self, name: &str, backing: TypeId) -> &mut Self {
        let n = atom(name);
        self.ancestors.entry(n).or_default().insert(n);
        self.enums.insert(n, EnumBacking::Backed(backing));
        self
    }

    /// Declare a `@type` alias on `class`: `Class::alias = body`.
    pub fn with_alias(&mut self, class: &str, alias: &str, body: TypeId) -> &mut Self {
        let n = atom(class);
        self.ancestors.entry(n).or_default().insert(n);
        self.aliases.insert((n, atom(alias)), body);
        self
    }

    /// Declare a class constant: `Class::CONST: type`.
    pub fn with_class_constant(&mut self, class: &str, name: &str, type_: TypeId) -> &mut Self {
        let n = atom(class);
        self.ancestors.entry(n).or_default().insert(n);
        self.class_constants.insert((n, atom(name)), type_);
        self
    }

    /// Declare a global constant: `define('NAME', value)`.
    pub fn with_global_constant(&mut self, name: &str, type_: TypeId) -> &mut Self {
        self.global_constants.insert(atom(name), type_);
        self
    }

    /// Collect every property declared on `class` or one of its
    /// ancestors, ordered by depth-first declaration. Subclass
    /// declarations precede ancestor declarations of the same name.
    fn collect_visible_properties(&self, class: Atom) -> Vec<ClassProperty> {
        let Some(ancestors) = self.ancestors.get(&class) else {
            return Vec::new();
        };

        let mut chain: Vec<Atom> = vec![class];
        for &a in ancestors {
            if a != class {
                chain.push(a);
            }
        }

        let mut seen: HashSet<Atom> = HashSet::new();
        let mut out: Vec<ClassProperty> = Vec::new();
        for c in chain {
            if let Some(props) = self.properties.get(&c) {
                for p in props {
                    if seen.insert(p.name) {
                        out.push(*p);
                    }
                }
            }
        }

        out
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

    fn template_parameter_arity(&self, class: Atom) -> usize {
        self.templates.get(&class).map_or(0, Vec::len)
    }

    fn template_parameter_at(&self, class: Atom, position: usize) -> Option<TemplateParameter> {
        let params = self.templates.get(&class)?;
        params.get(position).cloned()
    }

    fn template_parameter_index(&self, class: Atom, name: Atom) -> Option<usize> {
        let params = self.templates.get(&class)?;
        params.iter().position(|p| p.name == name)
    }

    fn inherited_template_argument(&self, child: Atom, ancestor: Atom, position: usize) -> Option<TypeId> {
        if !self.descends_from(child, ancestor) {
            return None;
        }
        if let Some(args) = self.extended.get(&(child, ancestor))
            && let Some(arg) = args.get(position).copied()
        {
            return Some(arg);
        }
        // Transitive case: walk the chain via any direct parent that
        // descends to `ancestor`. We pick the first such parent and
        // recurse, returning the parent's args at `position` (each
        // edge in `arb_world` already supplies `mixed` defaults, so
        // chain composition collapses to `mixed`).
        for (parent_child, parent_ancestor) in self.extended.keys() {
            if *parent_child != child {
                continue;
            }
            if self.descends_from(*parent_ancestor, ancestor)
                && let Some(arg) = self.inherited_template_argument(*parent_ancestor, ancestor, position)
            {
                return Some(arg);
            }
        }
        None
    }

    fn class_has_method(&self, class: Atom, method: Atom) -> bool {
        let Some(ancestors) = self.ancestors.get(&class) else {
            return false;
        };
        ancestors.iter().any(|a| self.methods.get(a).is_some_and(|m| m.contains(&method)))
    }

    fn class_property_type(&self, class: Atom, property: Atom) -> Option<TypeId> {
        self.collect_visible_properties(class).into_iter().find(|p| p.name == property).map(|p| p.type_)
    }

    fn class_has_property(&self, class: Atom, property: Atom) -> bool {
        let Some(ancestors) = self.ancestors.get(&class) else {
            return false;
        };

        ancestors.iter().any(|a| self.properties.get(a).is_some_and(|props| props.iter().any(|p| p.name == property)))
    }

    fn class_property_count(&self, class: Atom) -> usize {
        self.collect_visible_properties(class).len()
    }

    fn class_property_at(&self, class: Atom, position: usize) -> Option<ClassProperty> {
        self.collect_visible_properties(class).into_iter().nth(position)
    }

    fn enum_backing(&self, enum_name: Atom) -> Option<EnumBacking> {
        self.enums.get(&enum_name).copied()
    }

    fn class_like_kind(&self, name: Atom) -> Option<suffete::element::payload::ClassLikeKind> {
        if let Some(k) = self.class_like_kinds.get(&name) {
            return Some(*k);
        }
        if self.enums.contains_key(&name) {
            return Some(suffete::element::payload::ClassLikeKind::Enum);
        }
        if self.ancestors.contains_key(&name) {
            return Some(suffete::element::payload::ClassLikeKind::Class);
        }
        None
    }

    fn is_final(&self, name: Atom) -> bool {
        // Enums are implicitly final in PHP ; they cannot be
        // subclassed. Explicit `with_final` declarations are tracked
        // in `final_classes`.
        self.enums.contains_key(&name) || self.final_classes.contains(&name)
    }

    fn alias_body(&self, class: Atom, alias: Atom) -> Option<TypeId> {
        self.aliases.get(&(class, alias)).copied()
    }

    fn class_constant_type(&self, class: Atom, constant: Atom) -> Option<TypeId> {
        let ancestors = self.ancestors.get(&class)?;
        ancestors.iter().find_map(|a| self.class_constants.get(&(*a, constant)).copied())
    }

    fn global_constant_type(&self, name: Atom) -> Option<TypeId> {
        self.global_constants.get(&name).copied()
    }
}

/// An empty world: nothing knows about anything.
pub const fn empty_world() -> NullWorld {
    NullWorld
}

/// `lattice::refines(input, container, world, default options, _)`.
pub fn is_contained<W: World>(input: TypeId, container: TypeId, world: &W) -> bool {
    let mut report = LatticeReport::new();
    lattice::refines(input, container, world, LatticeOptions::default(), &mut report)
}

/// As [`is_contained`], but returns the [`LatticeReport`] alongside the
/// boolean answer so tests can inspect coercion flags.
pub fn is_contained_capturing<W: World>(input: TypeId, container: TypeId, world: &W) -> (bool, LatticeReport) {
    let mut report = LatticeReport::new();
    let v = lattice::refines(input, container, world, LatticeOptions::default(), &mut report);
    (v, report)
}

/// `is_contained` with the `ignore_null` / `ignore_false` / `inside_assertion`
/// option flags.
pub fn is_contained_with<W: World>(
    input: TypeId,
    container: TypeId,
    world: &W,
    ignore_null: bool,
    ignore_false: bool,
    inside_assertion: bool,
) -> bool {
    let options = LatticeOptions { ignore_null, ignore_false, inside_assertion };
    let mut report = LatticeReport::new();
    lattice::refines(input, container, world, options, &mut report)
}

/// Element-vs-element refinement query: wraps both in singleton unions and
/// calls [`is_contained`].
pub fn atomic_is_contained<W: World>(input: ElementId, container: ElementId, world: &W) -> bool {
    let i = interner();
    let it = i.intern_type(&[input], FlowFlags::EMPTY);
    let ct = i.intern_type(&[container], FlowFlags::EMPTY);
    is_contained(it, ct, world)
}

pub fn overlaps<W: World>(a: TypeId, b: TypeId, world: &W) -> bool {
    let mut report = LatticeReport::new();
    lattice::overlaps(a, b, world, LatticeOptions::default(), &mut report)
}

pub fn atomic_overlaps<W: World>(a: ElementId, b: ElementId, world: &W) -> bool {
    let i = interner();
    let at = i.intern_type(&[a], FlowFlags::EMPTY);
    let bt = i.intern_type(&[b], FlowFlags::EMPTY);
    overlaps(at, bt, world)
}

pub fn atomic_is_contained_capturing<W: World>(
    input: ElementId,
    container: ElementId,
    world: &W,
) -> (bool, LatticeReport) {
    let i = interner();
    let it = i.intern_type(&[input], FlowFlags::EMPTY);
    let ct = i.intern_type(&[container], FlowFlags::EMPTY);
    is_contained_capturing(it, ct, world)
}

#[track_caller]
pub fn assert_subtype(input: TypeId, container: TypeId) {
    let cb = empty_world();
    assert!(is_contained(input, container, &cb), "expected {input:?} <: {container:?} but it is not");
}

#[track_caller]
pub fn assert_not_subtype(input: TypeId, container: TypeId) {
    let cb = empty_world();
    assert!(!is_contained(input, container, &cb), "expected NOT ({input:?} <: {container:?}) but it is");
}

#[track_caller]
pub fn assert_atomic_subtype(input: ElementId, container: ElementId) {
    let cb = empty_world();
    assert!(atomic_is_contained(input, container, &cb), "expected atomic {input:?} <: {container:?}");
}

#[track_caller]
pub fn assert_atomic_not_subtype(input: ElementId, container: ElementId) {
    let cb = empty_world();
    assert!(!atomic_is_contained(input, container, &cb), "expected NOT (atomic {input:?} <: {container:?})");
}

pub const fn never() -> ElementId {
    prelude::NEVER
}

pub const fn null() -> ElementId {
    prelude::NULL
}

pub const fn void() -> ElementId {
    prelude::VOID
}

pub const fn placeholder() -> ElementId {
    prelude::PLACEHOLDER
}

pub const fn mixed() -> ElementId {
    prelude::MIXED
}

pub const fn mixed_truthy() -> ElementId {
    prelude::TRUTHY_MIXED
}

pub const fn mixed_falsy() -> ElementId {
    prelude::FALSY_MIXED
}

pub const fn mixed_nonnull() -> ElementId {
    prelude::NON_NULL_MIXED
}

pub const fn t_true() -> ElementId {
    prelude::TRUE
}

pub const fn t_false() -> ElementId {
    prelude::FALSE
}

pub const fn t_bool() -> ElementId {
    prelude::BOOL
}

pub const fn t_int() -> ElementId {
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

pub const fn t_int_unspec_lit() -> ElementId {
    prelude::LITERAL_INT
}

pub const fn t_positive_int() -> ElementId {
    prelude::POSITIVE_INT
}

pub const fn t_negative_int() -> ElementId {
    prelude::NEGATIVE_INT
}

pub const fn t_non_negative_int() -> ElementId {
    prelude::NON_NEGATIVE_INT
}

pub const fn t_non_positive_int() -> ElementId {
    prelude::NON_POSITIVE_INT
}

pub const fn t_float() -> ElementId {
    prelude::FLOAT
}

pub fn t_lit_float(v: f64) -> ElementId {
    ElementId::float_literal(v)
}

pub const fn t_unspec_lit_float() -> ElementId {
    prelude::LITERAL_FLOAT
}

pub const fn t_string() -> ElementId {
    prelude::STRING
}

pub fn t_lit_string(s: &str) -> ElementId {
    ElementId::string_literal(s)
}

pub const fn t_non_empty_string() -> ElementId {
    prelude::NON_EMPTY_STRING
}

pub const fn t_numeric_string() -> ElementId {
    prelude::NUMERIC_STRING
}

pub const fn t_lower_string() -> ElementId {
    prelude::LOWERCASE_STRING
}

pub const fn t_upper_string() -> ElementId {
    prelude::UPPERCASE_STRING
}

pub const fn t_truthy_string() -> ElementId {
    prelude::TRUTHY_STRING
}

pub const fn t_unspec_lit_string(non_empty: bool) -> ElementId {
    if non_empty { prelude::NON_EMPTY_LITERAL_STRING } else { prelude::LITERAL_STRING }
}

pub const fn t_callable_string() -> ElementId {
    prelude::CALLABLE_STRING
}

pub const fn t_array_key() -> ElementId {
    prelude::ARRAY_KEY
}

pub const fn t_numeric() -> ElementId {
    prelude::NUMERIC
}

pub const fn t_scalar() -> ElementId {
    prelude::SCALAR
}

pub const fn t_class_string() -> ElementId {
    prelude::CLASS_STRING
}

pub const fn t_interface_string() -> ElementId {
    prelude::INTERFACE_STRING
}

pub const fn t_enum_string() -> ElementId {
    prelude::ENUM_STRING
}

pub const fn t_trait_string() -> ElementId {
    prelude::TRAIT_STRING
}

pub fn t_lit_class_string(name: &str) -> ElementId {
    ElementId::class_string_literal(name)
}

pub fn t_class_string_of(constraint: TypeId) -> ElementId {
    use suffete::element::payload::ClassLikeKind;
    use suffete::element::payload::ClassLikeStringInfo;
    use suffete::element::payload::ClassLikeStringSpecifier;
    interner().intern_class_like_string(ClassLikeStringInfo {
        kind: ClassLikeKind::Class,
        specifier: ClassLikeStringSpecifier::OfType { constraint },
    })
}

pub fn t_interface_string_of(constraint: TypeId) -> ElementId {
    use suffete::element::payload::ClassLikeKind;
    use suffete::element::payload::ClassLikeStringInfo;
    use suffete::element::payload::ClassLikeStringSpecifier;
    interner().intern_class_like_string(ClassLikeStringInfo {
        kind: ClassLikeKind::Interface,
        specifier: ClassLikeStringSpecifier::OfType { constraint },
    })
}

pub const fn t_resource() -> ElementId {
    prelude::RESOURCE
}

pub const fn t_open_resource() -> ElementId {
    prelude::OPEN_RESOURCE
}

pub const fn t_closed_resource() -> ElementId {
    prelude::CLOSED_RESOURCE
}

pub const fn t_object_any() -> ElementId {
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

/// Construct a generic-named-object element: `Foo<arg1, arg2, ...>`.
pub fn t_generic_named(name: &str, args: Vec<TypeId>) -> ElementId {
    use suffete::element::payload::ObjectFlags;
    use suffete::element::payload::ObjectInfo;
    let i = interner();
    let info =
        ObjectInfo { name: atom(name), type_args: Some(i.intern_type_list(&args)), flags: ObjectFlags::default() };
    i.intern_object(info)
}

/// Construct a named-object element with `head & conjunct1 & conjunct2 …`.
pub fn t_named_intersected(head: &str, conjuncts: &[ElementId]) -> ElementId {
    use suffete::element::payload::ObjectFlags;
    use suffete::element::payload::ObjectInfo;
    let i = interner();
    let info = ObjectInfo { name: atom(head), type_args: None, flags: ObjectFlags::default() };
    let head_elem = i.intern_object(info);
    ElementId::intersected(head_elem, conjuncts)
}

/// Named object marked `static<C>` (late-static-bound modality).
pub fn t_named_static(name: &str) -> ElementId {
    use suffete::element::payload::ObjectFlags;
    use suffete::element::payload::ObjectInfo;
    let i = interner();
    let info = ObjectInfo { name: atom(name), type_args: None, flags: ObjectFlags::default().with_is_static(true) };
    i.intern_object(info)
}

pub fn t_has_method(name: &str) -> ElementId {
    use suffete::element::payload::HasMethodInfo;
    interner().intern_has_method(HasMethodInfo { method_name: atom(name) })
}

pub fn t_has_property(name: &str) -> ElementId {
    use suffete::element::payload::HasPropertyInfo;
    interner().intern_has_property(HasPropertyInfo { property_name: atom(name) })
}

/// `object{p1: T1, p2?: T2, ...}` element. Each entry is `(name, type, optional)`.
pub fn t_object_shape(props: &[(&str, TypeId, bool)], sealed: bool) -> ElementId {
    use suffete::element::payload::KnownPropertyEntry;
    use suffete::element::payload::ObjectShapeFlags;
    use suffete::element::payload::ObjectShapeInfo;
    let i = interner();
    let entries: Vec<KnownPropertyEntry> =
        props.iter().map(|(n, t, opt)| KnownPropertyEntry { name: atom(n), value: *t, optional: *opt }).collect();
    let known = if entries.is_empty() { None } else { Some(i.intern_known_properties(&entries)) };
    let info = ObjectShapeInfo { known_properties: known, flags: ObjectShapeFlags::default().with_sealed(sealed) };
    i.intern_object_shape(info)
}

/// Named object marked `$this<C>`.
pub fn t_named_this(name: &str) -> ElementId {
    use suffete::element::payload::ObjectFlags;
    use suffete::element::payload::ObjectInfo;
    let i = interner();
    let info = ObjectInfo {
        name: atom(name),
        type_args: None,
        flags: ObjectFlags::default().with_is_static(true).with_is_this(true),
    };
    i.intern_object(info)
}

/// Construct a template-parameter element referring to `class_name`'s
/// template parameter `template_name`. Constraint defaults to `mixed`.
pub fn t_template(class_name: &str, template_name: &str) -> ElementId {
    use suffete::element::payload::DefiningEntity;
    ElementId::generic_parameter(template_name, DefiningEntity::ClassLike(atom(class_name)), prelude::TYPE_MIXED)
}

/// Same as [`t_template`] but with an explicit constraint type.
pub fn t_template_of(class_name: &str, template_name: &str, constraint: TypeId) -> ElementId {
    use suffete::element::payload::DefiningEntity;
    ElementId::generic_parameter(template_name, DefiningEntity::ClassLike(atom(class_name)), constraint)
}

pub const fn t_empty_array() -> ElementId {
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

pub fn t_callable_any() -> ElementId {
    use suffete::element::payload::CallableInfo;
    interner().intern_callable(CallableInfo::Any)
}

/// Build a `callable(params): return_type` element. Each parameter is given as
/// its type plus a `(has_default, by_reference, variadic)` flag triple.
pub fn t_callable_sig(params: &[(TypeId, bool, bool, bool)], return_type: TypeId, pure: bool) -> ElementId {
    use suffete::element::payload::CallableInfo;
    use suffete::element::payload::ParamFlags;
    use suffete::element::payload::ParamInfo;
    use suffete::element::payload::Signature;
    use suffete::element::payload::SignatureFlags;
    let i = interner();
    let info_params: Vec<ParamInfo> = params
        .iter()
        .enumerate()
        .map(|(idx, (ty, has_default, by_ref, variadic))| ParamInfo {
            name: atom(&format!("p{idx}")),
            type_: *ty,
            flags: ParamFlags::EMPTY.with_has_default(*has_default).with_by_reference(*by_ref).with_variadic(*variadic),
        })
        .collect();
    let trailing_variadic = info_params.last().is_some_and(|p| p.flags.variadic());
    let param_list = if info_params.is_empty() { None } else { Some(i.intern_param_list(&info_params)) };
    let sig = i.intern_signature(Signature {
        parameters: param_list,
        return_type,
        throws: None,
        flags: SignatureFlags::EMPTY.with_is_variadic(trailing_variadic).with_is_pure(pure),
    });
    i.intern_callable(CallableInfo::Signature(sig))
}

/// Convenience: `callable(p1, p2, ...): return` with no defaults / variadics
/// / by-ref / purity.
pub fn t_callable(params: &[TypeId], return_type: TypeId) -> ElementId {
    let p: Vec<(TypeId, bool, bool, bool)> = params.iter().map(|t| (*t, false, false, false)).collect();
    t_callable_sig(&p, return_type, false)
}

pub const fn ak_int(n: i64) -> ArrayKey {
    ArrayKey::Int(n)
}

pub fn ak_str(s: &str) -> ArrayKey {
    ArrayKey::String(atom(s))
}

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
