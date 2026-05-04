# Inspection: walking the tree

The `inspect` module is the place for *deep, short-circuiting* boolean queries on a `TypeId`. Where [`predicates`](./predicates.md) answer single questions about top-level structure, `inspect` recurses into every nested-type carrier in the tree and stops the moment the answer is known.

```rust,ignore
use suffete::inspect;

inspect::any(ty, |elem| /* predicate */);   // true iff at least one Element in the tree satisfies
inspect::all(ty, |elem| /* predicate */);   // true iff every Element in the tree satisfies
```

The closure is called at every level: top-level union elements, plus every Element nested inside any payload. It is **not** called twice on the same Element.

## What "every nested-type carrier" means

The walker descends through:

- `Object`'s `type_args` and `intersections`.
- `List`'s `element_type` and `known_elements`.
- `Array`'s `key_param`, `value_param`, `known_items`, `intersections`.
- `Iterable`'s `key_type`, `value_type`, `intersections`.
- `ObjectShape`'s `known_properties`, `intersections`.
- `HasMethod`'s and `HasProperty`'s `intersections`.
- `Callable`'s `Signature`'s `parameters`, `return_type`, `throws`.
- `ClassLikeString`'s `OfType` / `Generic` constraint.
- `GenericParameter`'s `constraint`.
- `Reference`'s `type_args` and `intersections`.
- `Conditional`'s 4 operands.
- `Derived`'s nested `TypeId`s (all 8 variants).
- `Negated`'s `inner`.
- `Intersected`'s `head` and `conjuncts`.

A trivial-kind Element (no payload to recurse into) is visited once and the walker moves on.

## `any`

```rust,ignore
pub fn any<F: FnMut(ElementId) -> bool>(ty: TypeId, mut predicate: F) -> bool;
```

Returns `true` iff some Element anywhere in `ty`'s tree satisfies `predicate`. Short-circuits on the first match.

```rust,ignore
use suffete::ElementKind;

let has_object = inspect::any(ty, |e| matches!(e.kind(), ElementKind::Object | ElementKind::Enum));
```

## `all`

```rust,ignore
pub fn all<F: FnMut(ElementId) -> bool>(ty: TypeId, mut predicate: F) -> bool;
```

Returns `true` iff every Element anywhere in `ty`'s tree satisfies `predicate`. Short-circuits on the first failure.

`all` is the negation of `any` with a flipped predicate:

```rust,ignore
inspect::all(ty, |e| /* P(e) */) == !inspect::any(ty, |e| !/* P(e) */)
```

Both signatures are available because `all` reads more naturally for the "for-every" case.

## How the recursion works

The walker is post-order. For each Element it:

1. Calls the predicate on the Element itself.
2. If the predicate did not short-circuit, descends into every nested `TypeId` carrier (per the kind's payload).
3. For each nested `TypeId`, recurses with the same predicate.

The descent is implemented as a per-kind dispatch: the walker has a `descend_object`, a `descend_list`, etc. The dispatch is exhaustive ; every payload-bearing kind has a descender, and trivial kinds skip directly to the next.

## Cost

The cost of `inspect::any(ty, p)` is bounded by the size of `ty`'s tree, where the tree size is the number of Element occurrences across all nested types. For a flat type like `int|string`, the tree size is 2. For a deeply nested generic like `Map<Foo, list<array{a: int, b: ?Bar}>>`, the tree size is in the tens.

The walker does not allocate. It uses the call stack for recursion and a `&mut F` for the closure.

The most common use of `inspect::any` is the `*_anywhere` predicates ([predicates](./predicates.md)) and the equivalent custom predicates analyser code writes. Performance is rarely a concern for this module ; the closure is the cost driver.

## A worked example: collecting class names

The walker is for boolean queries; collecting data requires using a `RefCell` or owning the closure's environment:

```rust,ignore
use std::cell::RefCell;
use suffete::{ElementKind, inspect, TypeId};
use suffete::interner::interner;
use mago_atom::Atom;

fn collect_class_names(ty: TypeId) -> Vec<Atom> {
    let names = RefCell::new(Vec::new());
    inspect::any(ty, |e| {
        match e.kind() {
            ElementKind::Object => {
                names.borrow_mut().push(interner().get_object(e).name);
            }
            ElementKind::Enum => {
                names.borrow_mut().push(interner().get_enum(e).name);
            }
            _ => {}
        }
        false  // don't short-circuit; visit every Element
    });
    names.into_inner()
}
```

Returning `false` from the predicate keeps the walker going; the caller gets the full enumeration.

For a *transformation* (build a new type), use [`transform`](./transform.md) instead.

## A worked example: finding a free template

```rust,ignore
use suffete::{ElementKind, inspect, TypeId};
use suffete::interner::interner;
use mago_atom::{Atom, atom};

fn contains_template_named(ty: TypeId, target: Atom, target_class: Atom) -> bool {
    inspect::any(ty, |e| {
        if e.kind() == ElementKind::GenericParameter {
            let info = interner().get_generic_parameter(e);
            info.name == target  /* and check the defining_entity matches target_class */
        } else {
            false
        }
    })
}
```

Short-circuits on the first hit. If the type doesn't contain the target template anywhere, the cost is the full tree walk.

## The relationship to `predicates`

Several predicates in the [predicates](./predicates.md) module are thin wrappers around `inspect::any`:

- `contains_mixed_anywhere(ty)` is `inspect::any(ty, |e| e.kind() == ElementKind::Mixed)`.
- `contains_template_anywhere(ty)` is `inspect::any(ty, |e| e.kind() == ElementKind::GenericParameter)`.
- `contains_placeholder_anywhere(ty)` is `inspect::any(ty, |e| e.kind() == ElementKind::Placeholder)`.

Use the predicate when you want the standard variant. Use `inspect::any` directly when you have a custom condition.

## A subtle case: visiting the same Element twice

The walker is *non-deduplicating*. If two distinct nested positions reach the same `ElementId` (because the same Element appears in multiple places in the tree), the predicate is called for each *occurrence*, not just once per `ElementId`.

```rust,ignore
let int_t = TypeBuilder::new().push(suffete::prelude::INT).build();
let nested = TypeBuilder::new()
    .push(ElementId::keyed_array(int_t, int_t))   // array<int, int>
    .build();

let mut count = 0;
inspect::any(nested, |_| { count += 1; false });
// count == 3: the array Element + the two int Elements (one for k, one for v)
```

This is usually what you want; if you need uniqueness, accumulate into a `HashSet<ElementId>` in the closure's environment.

> **See also:** [Predicates](./predicates.md) for the standard wrappers; [Transformation](./transform.md) for the dual operation that produces a new type; [Element kinds](../reference/element-kinds.md) for the exhaustive list of payload-bearing kinds the walker descends into.
