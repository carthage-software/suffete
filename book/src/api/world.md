# Hierarchy and World

Suffete is codebase-agnostic. It does not know which classes the user has declared, what they extend, what methods and properties they declare, what template parameters they have, what their declared variances are, or what type aliases the user has defined. All of that knowledge lives in the analyser and is exposed to suffete through one trait: `World`.

```rust,ignore
use suffete::world::World;
```

This chapter covers the trait surface and the conventions analyser implementations should follow.

## The trait

`World` is a *query interface*. Every method takes a `&self` and returns a value (or `None`). Suffete never mutates the world; it only reads.

Sketch of the surface (the actual trait has more methods; this is the most-used subset):

```rust,ignore
pub trait World: Sized + Sync {
    // Inheritance
    fn descends_from(&self, descendant: Atom, ancestor: Atom) -> bool;
    fn inherited_template_argument(
        &self,
        descendant: Atom,
        ancestor: Atom,
        position: usize,
    ) -> Option<TypeId>;

    // Class properties and methods
    fn class_has_method(&self, class: Atom, method: Atom) -> bool;
    fn class_has_property(&self, class: Atom, property: Atom) -> bool;
    fn class_property_type(&self, class: Atom, property: Atom) -> Option<TypeId>;
    fn class_constant_type(&self, class: Atom, constant: Atom) -> Option<TypeId>;

    // Class-like classification
    fn is_final(&self, class_like: Atom) -> bool;
    fn is_interface(&self, class_like: Atom) -> bool;
    fn is_trait(&self, class_like: Atom) -> bool;
    fn is_enum(&self, class_like: Atom) -> bool;

    // Templates
    fn template_parameter_arity(&self, class_like: Atom) -> usize;
    fn template_parameter_at(&self, class_like: Atom, position: usize)
        -> Option<TemplateParameter>;
    fn template_parameter_index(&self, class_like: Atom, name: Atom)
        -> Option<usize>;

    // Enums
    fn enum_backing(&self, enum_name: Atom) -> Option<EnumBacking>;

    // Aliases
    fn resolve_alias(&self, name: Atom) -> Option<TypeId>;

    // ... and more
}
```

The exact method list lives in `src/world/mod.rs`.

## What suffete asks the world

The lattice's family rules consult the world wherever it cannot answer from the type alone:

- **Object family** (`refines`, `meet`, `overlaps`):
  - `descends_from(D, C)` for inheritance checks.
  - `inherited_template_argument(D, C, i)` for variance through inheritance.
  - `template_parameter_at(C, i)` for declared variance.
  - `class_has_method(C, m)` for `Foo <: has-method<m>`.
  - `class_has_property(C, p)` for `Foo <: has-property<p>`.
  - `class_property_type(C, p)` for shape-vs-named subtyping.
  - `is_final(C)` for finality-aware refines and meet.

- **Enum family**:
  - `enum_backing(E)` for the structural shape of `Status::Active`'s `value` property.

- **Class-like-string family**:
  - `descends_from(C, D)` for `class-string<C> <: class-string<D>`.
  - `is_interface(C)`, `is_enum(C)`, `is_trait(C)` for the kind-axis check.

- **Conditional and Derived expansion** (in [`expand`](./expand.md)):
  - `class_constant_type(C, K)` for `MemberReference`.
  - `resolve_alias(name)` for `Alias`.
  - The full set of derived-type queries.

## The `NullWorld`

Suffete ships a trivial implementation: `NullWorld`. It returns:

- `descends_from`: only `descendant == ancestor`.
- `class_has_method`, `class_has_property`: always `false`.
- All the others: `None` or zero or empty.

`NullWorld` is useful for:

- Examples and tests where the analyser's codebase model is irrelevant.
- The lattice's [join canonicalisation](../lattice/join.md), which uses `NullWorld` for the structural-only subsumption check (so that `int|float` is not collapsed to `float` via PHP runtime coercion).

```rust,ignore
use suffete::world::NullWorld;
let world = NullWorld;
```

## Implementing `World`

The analyser implements `World` by reading from its codebase model. A sketch:

```rust,ignore
use suffete::world::{World, TemplateParameter, EnumBacking, Variance};
use suffete::TypeId;
use mago_atom::Atom;

pub struct AnalyserWorld {
    classes: HashMap<Atom, ClassInfo>,
    aliases: HashMap<Atom, TypeId>,
    // ... other tables ...
}

impl World for AnalyserWorld {
    fn descends_from(&self, d: Atom, a: Atom) -> bool {
        if d == a { return true; }
        let mut current = self.classes.get(&d);
        while let Some(info) = current {
            if info.parents.contains(&a) { return true; }
            current = info.parents.first().and_then(|p| self.classes.get(p));
        }
        false
    }

    fn class_has_method(&self, c: Atom, m: Atom) -> bool {
        self.classes.get(&c)
            .map(|info| info.methods.contains_key(&m))
            .unwrap_or(false)
    }

    fn template_parameter_at(&self, c: Atom, i: usize) -> Option<TemplateParameter> {
        let info = self.classes.get(&c)?;
        info.template_parameters.get(i).copied()
    }

    // ... and so on ...
}
```

The implementation can use any storage strategy ; HashMap, indexed slices, B-trees, lazy-loading from disk. Suffete only cares about the answers.

## Performance contract

Suffete calls `World` methods *frequently* during lattice operations on object-family types. A `refines(IntList, Iterable<int>)` involves at least:

- One `descends_from` (to confirm IntList implements Iterable).
- One `inherited_template_argument` (to get IntList's contribution to Iterable's parameter).
- One `template_parameter_at` (for the declared variance).

For an analyser checking thousands of refines calls per file, a slow `World` is a bottleneck. The conventions:

- **`descends_from` should be O(1) amortised.** Pre-compute the transitive closure of inheritance; cache the answer.
- **`class_has_method` and `class_has_property` should be O(1).** Use a HashMap.
- **`inherited_template_argument` should be O(1) amortised.** Pre-compute the inheritance bindings.
- **`template_parameter_at` should be O(1).** Index by position into a small vector.

The analyser pays the up-front cost when it ingests the codebase; suffete pays the per-query cost on every lattice call.

## Threading

The trait requires `Sync`. Lattice operations are pure functions and can be called from multiple threads simultaneously, all sharing one `&World`. The analyser's implementation must allow concurrent reads (typical: read-only after ingestion).

## A worked example: descendant lookup

```rust,ignore
use suffete::world::World;
use suffete::{TypeBuilder, ElementId};
use mago_atom::atom;

struct DemoWorld;

impl World for DemoWorld {
    fn descends_from(&self, d: mago_atom::Atom, a: mago_atom::Atom) -> bool {
        // For demo: class B descends from class A; everything else is itself only.
        if d == a { return true; }
        if d.as_str() == "B" && a.as_str() == "A" { return true; }
        false
    }

    // ... other methods stub out to None / false / 0 ...

    fn template_parameter_arity(&self, _: mago_atom::Atom) -> usize { 0 }
    fn template_parameter_at(&self, _: mago_atom::Atom, _: usize)
        -> Option<suffete::world::TemplateParameter> { None }
    fn template_parameter_index(&self, _: mago_atom::Atom, _: mago_atom::Atom)
        -> Option<usize> { None }
    fn inherited_template_argument(&self, _: mago_atom::Atom, _: mago_atom::Atom, _: usize)
        -> Option<suffete::TypeId> { None }
    fn class_has_method(&self, _: mago_atom::Atom, _: mago_atom::Atom) -> bool { false }
    fn class_has_property(&self, _: mago_atom::Atom, _: mago_atom::Atom) -> bool { false }
    fn class_property_type(&self, _: mago_atom::Atom, _: mago_atom::Atom) -> Option<suffete::TypeId> { None }
    fn class_constant_type(&self, _: mago_atom::Atom, _: mago_atom::Atom) -> Option<suffete::TypeId> { None }
    fn is_final(&self, _: mago_atom::Atom) -> bool { false }
    fn is_interface(&self, _: mago_atom::Atom) -> bool { false }
    fn is_trait(&self, _: mago_atom::Atom) -> bool { false }
    fn is_enum(&self, _: mago_atom::Atom) -> bool { false }
    fn enum_backing(&self, _: mago_atom::Atom) -> Option<suffete::world::EnumBacking> { None }
    fn resolve_alias(&self, _: mago_atom::Atom) -> Option<suffete::TypeId> { None }
}

let world = DemoWorld;
let class_a = TypeBuilder::new().push(ElementId::named_object(atom("A"))).build();
let class_b = TypeBuilder::new().push(ElementId::named_object(atom("B"))).build();

let mut report = suffete::lattice::LatticeReport::new();
let opts = suffete::lattice::LatticeOptions::default();

assert!(suffete::lattice::refines(class_b, class_a, &world, opts, &mut report));
```

## Hierarchy module

Suffete ships a `hierarchy` module with helpers for managing class-hierarchy data structures. It's a convenience for analyser implementations that don't already have one. See `src/hierarchy/mod.rs` for the API.

The `hierarchy` module is *not* a `World` implementation by itself ; it's a kit you can compose into your `World`. Most analysers will have their own.

> **See also:** [Templates in depth](../generics/templates.md) for the template-related world methods; [Specialise](../generics/specialise.md) for the inheritance-binding protocol; [Expand](./expand.md) for the resolution methods on the world.
