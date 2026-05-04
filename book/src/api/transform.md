# Transformation: map, flat_map, filter

The `transform` module is the place for *building a new type from an existing one* by applying a closure at every Element position. Where [`inspect`](./inspect.md) recurses with a `bool`-returning predicate, `transform` recurses with an outcome-returning closure that decides whether to keep, replace, expand, or drop each Element.

```rust,ignore
use suffete::transform;

let result = transform::map(ty, |elem| /* new ElementId */);
let result = transform::flat_map(ty, |elem| /* iterator of ElementId */);
let result = transform::filter_map(ty, |elem| /* Option<ElementId> */);
let result = transform::filter(ty, |elem| /* bool */);
```

Each entry-point shape is a thin wrapper over the same underlying walker, which is post-order and recurses through every nested-type carrier (the same set as [inspect](./inspect.md)).

## The four entry points

### `map`

```rust,ignore
pub fn map<F>(ty: TypeId, f: F) -> TypeId
where F: FnMut(ElementId) -> ElementId;
```

Apply `f` at every Element position; replace each in place. The returned type has the same shape as the input but with each Element substituted.

```rust,ignore
// Replace every `int` with `int<0, ∞>` (positive ints).
let result = transform::map(ty, |elem| {
    if elem == suffete::prelude::INT {
        ElementId::int_range(IntRange::new(Some(0), None))
    } else {
        elem
    }
});
```

### `flat_map`

```rust,ignore
pub fn flat_map<I, F>(ty: TypeId, f: F) -> TypeId
where I: IntoIterator<Item = ElementId>, F: FnMut(ElementId) -> I;
```

Apply `f` at every Element position; expand each into zero or more new Elements. Useful when one Element decomposes into a union (e.g. an integer range split).

```rust,ignore
// Decompose every `int` into `int<-∞,-1> | int(0) | int<1,∞>`.
let result = transform::flat_map(ty, |elem| {
    if elem == suffete::prelude::INT {
        vec![negative_int, zero, positive_int]
    } else {
        vec![elem]
    }
});
```

### `filter_map`

```rust,ignore
pub fn filter_map<F>(ty: TypeId, f: F) -> TypeId
where F: FnMut(ElementId) -> Option<ElementId>;
```

The combination of map and filter: `Some(elem)` keeps with replacement, `None` drops.

```rust,ignore
// Drop every Mixed Element; keep the rest unchanged.
let result = transform::filter_map(ty, |elem| {
    if elem.kind() == ElementKind::Mixed { None } else { Some(elem) }
});
```

### `filter`

```rust,ignore
pub fn filter<F>(ty: TypeId, f: F) -> TypeId
where F: FnMut(ElementId) -> bool;
```

Keep an Element iff `f` returns `true`. Equivalent to `filter_map` with `if f(elem) { Some(elem) } else { None }`, but reads cleaner for the common case.

```rust,ignore
let result = transform::filter(ty, |elem| elem.kind() != ElementKind::Null);
// result is `ty` with all Null Elements removed
```

## How the walker works

The walker is post-order:

1. For each Element at the current level, recurse into every nested `TypeId` carrier (per the kind's payload), transforming each via the same closure.
2. If any nested `TypeId` changed, re-intern the Element with the rebuilt payload.
3. Run the closure on the (possibly rebuilt) Element. The closure decides drop / replace / expand / leave.

Each level commits with a single `intern_type` call. Nothing is interned redundantly between levels.

## Identity short-circuit

If the closure returns the input Element unchanged (or `Some(input)` for `filter_map`, or `true` for `filter`) at every level, the walker returns the *original* `TypeId` without re-interning. This is the common case when the transform happened to leave the type alone.

The closure can express this naturally:

```rust,ignore
transform::map(ty, |elem| {
    if elem == suffete::prelude::INT { suffete::prelude::INT } else { elem }
});
// Returns ty unchanged; walker detects no leaf changed.
```

## Recursion stops at non-payload types

The walker descends into every payload-bearing kind, but the closure is *not* called on the inner `TypeId`s themselves ; the walker substitutes those *recursively* using the same closure. The closure sees only `ElementId`s.

For example, `array<int, string>` is one Element of kind `Array` with payload `KeyedArrayInfo { key_param: TypeId(int), value_param: TypeId(string) }`. The walker:

1. Recurses into `TypeId(int)`, calling the closure on the `Int` Element.
2. Recurses into `TypeId(string)`, calling the closure on the `String` Element.
3. If either changed, rebuilds the `Array` Element with the new key/value.
4. Calls the closure on the `Array` Element.
5. The closure decides what to do with the `Array` Element.

The closure can replace the entire `Array` Element with something else, or replace just the key parameter (by examining `e.kind()` and the kind's payload), or leave it alone.

## A worked example: substitute a class name

Substitute every reference to class `OldName` with class `NewName` throughout a type:

```rust,ignore
use suffete::{ElementId, ElementKind, transform};
use suffete::interner::interner;
use mago_atom::{atom, Atom};

fn rename_class(ty: TypeId, from: Atom, to: Atom) -> TypeId {
    transform::map(ty, |elem| {
        match elem.kind() {
            ElementKind::Object => {
                let info = interner().get_object(elem);
                if info.name == from {
                    let mut new_info = *info;
                    new_info.name = to;
                    interner().intern_object(new_info)
                } else {
                    elem
                }
            }
            _ => elem,
        }
    })
}
```

The walker handles the recursion into `Object`'s `type_args`, and into nested arrays/lists/iterables/etc. The closure only handles the leaf decision.

## A worked example: drop nullability

```rust,ignore
use suffete::{ElementKind, transform};

fn drop_null(ty: TypeId) -> TypeId {
    transform::filter(ty, |elem| elem.kind() != ElementKind::Null)
}
```

The walker descends into every nested type and applies the filter. A nested `?int` (i.e. `int|null`) becomes `int`. A nested `array<int, ?Foo>` becomes `array<int, Foo>`.

## When to use `transform` vs `lattice` vs `template::substitute`

- **`transform`** when the change is *structural* and Element-by-Element. Useful for renames, drops, kind-substitutions.
- **`lattice::*`** when the question is set-theoretic (subtype, intersection, union, difference). The lattice handles the canonicalization rules; `transform` does not.
- **`template::substitute`** when the change is *parameter-driven*: replacing a free `GenericParameter` with a concrete type. `substitute` is `transform::map` specialised to the parameter case.

## Cost

Same as [`inspect`](./inspect.md): bounded by the tree size. The interner cost dominates when the transform actually changes things ; one intern call per level that has at least one changed Element. The identity short-circuit makes no-op transforms free.

> **See also:** [Inspection](./inspect.md) for the dual (boolean queries); [Substitute](../generics/substitute.md) for the parameter-substitution variant; [TypeBuilder](./construction.md) for an alternative when you want a *flat*, non-recursive build.
