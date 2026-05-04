# Quick tour

This chapter walks through suffete by example. It is not a tutorial — the assumed audience already knows what a type system does — but a 10-minute look at the API surface so the rest of the book has a concrete shape to point at.

We will build types, ask the lattice questions about them, narrow under an assertion, and substitute through a generic. Every snippet in this chapter compiles against the suffete crate at HEAD.

## Setting up

```rust,ignore
use suffete::{
    TypeBuilder, TypeId,
    prelude::{INT, STRING, NULL, FALSE, TYPE_MIXED, TYPE_NEVER},
    lattice::{self, LatticeOptions, LatticeReport},
    world::NullWorld,
};
```

`TypeBuilder` is the mutable builder for a type. `prelude::*` exposes the well-known constants — `INT` is the `ElementId` for the unconstrained integer; `STRING` is the unconstrained string; `TYPE_MIXED` is the [`TypeId`](../api/handles.md) of the universal top; `TYPE_NEVER` is the `TypeId` of the empty bottom. `NullWorld` is the trivial [`World`](../api/world.md) that knows nothing about classes ; useful for examples that don't touch the codebase.

## Constructing a union

The PHP type `int|string` is a two-element union:

```rust,ignore
let int_or_string: TypeId = TypeBuilder::new()
    .push(INT)
    .push(STRING)
    .build();
```

`TypeBuilder::build` interns the result, sorting and deduplicating the elements. Call it twice with the same elements (in any order) and you get the same `TypeId` back — handle equality is content equality.

```rust,ignore
let a = TypeBuilder::new().push(INT).push(STRING).build();
let b = TypeBuilder::new().push(STRING).push(INT).build();
assert_eq!(a, b);
```

## Asking the lattice

Is `int|string` a subtype of `int|string|null`? Yes:

```rust,ignore
let nullable = TypeBuilder::new().push(INT).push(STRING).push(NULL).build();

let world = NullWorld;
let opts = LatticeOptions::default();
let mut report = LatticeReport::new();

assert!(lattice::refines(int_or_string, nullable, &world, opts, &mut report));
```

The reverse direction does not hold — `null` is not in `int|string`:

```rust,ignore
assert!(!lattice::refines(nullable, int_or_string, &world, opts, &mut report));
```

`refines` returns a boolean; the `&mut LatticeReport` collects structured side information about *why* a particular answer was reached (coercion edges, template defaults, etc.). For most questions you will ignore the report.

## Overlap and disjointness

`refines` is one-directional. To ask "is there any value in both types?" use [`overlaps`](../lattice/overlaps.md):

```rust,ignore
let int_only = TypeBuilder::new().push(INT).build();
let string_only = TypeBuilder::new().push(STRING).build();

assert!(!lattice::overlaps(int_only, string_only, &world, opts, &mut report));
assert!(lattice::overlaps(int_or_string, int_only, &world, opts, &mut report));
```

## Meet, join, subtract

The three combinators return `TypeId`:

```rust,ignore
// meet (greatest lower bound, ⊓): the values both types share.
let common = lattice::meet(int_or_string, nullable, &world, opts, &mut report);
// common == int|string

// join (least upper bound, ⊔): the smallest type containing both.
let either = lattice::join(int_only, string_only, &world, opts, &mut report);
// either == int|string

// subtract: what remains after removing the second from the first.
let only_string = lattice::subtract(int_or_string, int_only, &world, opts, &mut report);
// only_string == string
```

## Narrowing under an assertion

Suppose you have a value of type `int|string|null` and the analyzer has just observed an assertion that excludes `null`. The result is `int|string`:

```rust,ignore
let after_null_check = lattice::narrow(
    nullable,         // input type
    int_or_string,    // assertion: the value is one of these
    &world, opts, &mut report,
);
// after_null_check == int|string
```

Narrowing is the operation that consumes the assertions an analyzer extracts from `if`, `instanceof`, comparisons against constants, and so on.

## Predicates

For top-level structural questions — "is every element in this type guaranteed truthy?", "does this type contain any object element?", "is this type a single literal that can be constant-folded?" — you call into the [`predicates`](../api/predicates.md) module:

```rust,ignore
use suffete::predicates::{is_truthy, contains_null, is_constant_foldable};

assert!(!contains_null(int_or_string));
assert!(contains_null(nullable));

let one = TypeBuilder::new().push(suffete::ElementId::int_literal(1)).build();
assert!(is_truthy(one));
assert!(is_constant_foldable(one));
```

These do not need a `World`; they ask only about the structure of the type as you've handed it to them.

## A generic, briefly

Generics get their own [Part IV](../generics/templates.md). One snippet to show the shape:

```rust,ignore
use suffete::{ElementId, template::substitute};
use suffete::element::payload::GenericParameterInfo;

// Suppose `T` is bound on class `Box<T>`. Substitute `int` for it everywhere
// in `T|null`.
let t = ElementId::generic_parameter("T", "Box", /* upper bound: */ TYPE_MIXED);
let t_or_null = TypeBuilder::new().push(t).push(NULL).build();

let int_t = TypeBuilder::new().push(INT).build();
let result = substitute(t_or_null, &|info: &GenericParameterInfo| {
    if info.name.as_str() == "T" { Some(int_t) } else { None }
});
// result == int|null
```

The substitution is capture-free. Recursion into nested types (object type-args, callable parameters, conditional then/else, etc.) is handled by the walker; you supply only the leaf decision.

## Where to look next

If you want the data model first: [Part II — The Type Universe](../universe/elements.md) starts with the element kinds and works through every payload variant.

If you want the operations first: [Part III — The Lattice](../lattice/refines.md) covers `refines`, `overlaps`, `meet`, `join`, `subtract`, `narrow` in order.

If you want to start writing code: [Part V — Public API](../api/handles.md) is the by-module API reference, and [Part VI — Cookbook](../cookbook/subtype-question.md) shows the API composed into common analyzer recipes.

> **See also:** [What suffete is](./what-is-suffete.md) and [Glossary and notation](./glossary.md).
