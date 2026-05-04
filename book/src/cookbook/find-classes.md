# Walking a type to find every class name

The analyser wants to extract every class-like name a type references — for refactoring, for symbol tracking, for diagnostics. The recipe uses [`inspect`](../api/inspect.md) with a closure that collects into a `RefCell`.

## The recipe

```rust,ignore
use std::cell::RefCell;
use suffete::{TypeId, ElementKind, inspect};
use suffete::interner::interner;
use mago_atom::Atom;

fn collect_class_names(ty: TypeId) -> Vec<Atom> {
    let names = RefCell::new(Vec::new());
    inspect::any(ty, |elem| {
        let i = interner();
        match elem.kind() {
            ElementKind::Object => {
                names.borrow_mut().push(i.get_object(elem).name);
            }
            ElementKind::Enum => {
                names.borrow_mut().push(i.get_enum(elem).name);
            }
            ElementKind::ClassLikeString => {
                use suffete::element::payload::ClassLikeStringSpecifier;
                let info = i.get_class_like_string(elem);
                if let ClassLikeStringSpecifier::Literal(name) = info.specifier {
                    names.borrow_mut().push(name);
                }
            }
            _ => {}
        }
        false  // don't short-circuit; visit every Element
    });
    names.into_inner()
}
```

Returning `false` from the predicate keeps the walker going. If you want to short-circuit on the first hit (e.g. "does this type reference any class?"), return `true` after a match.

## How `inspect::any` enumerates

The walker is post-order and recurses into every nested-type carrier ([inspect](../api/inspect.md) chapter has the full list). For each Element:

1. Recurse into nested types (object args, list element, callable params, etc.).
2. Call the closure on this Element.
3. If the closure returns `true`, short-circuit; otherwise continue.

A type like `Box<Map<string, Foo|Bar>> | null` produces this enumeration order (post-order):

```
string, Foo, Bar, Map, Box, null
```

The recipe above pushes only the class-like Elements (`Foo`, `Bar`, `Map`, `Box`), giving `[Foo, Bar, Map, Box]`.

## Worked example

```rust,ignore
use suffete::{TypeBuilder, ElementId};
use mago_atom::atom;

let inner = TypeBuilder::new()
    .push(ElementId::named_object(atom("Foo")))
    .push(ElementId::named_object(atom("Bar")))
    .build();

let map_t = TypeBuilder::new().push(
    ElementId::named_object_with_args(atom("Map"), &[suffete::prelude::TYPE_STRING, inner])
).build();

let outer = TypeBuilder::new().push(
    ElementId::named_object_with_args(atom("Box"), &[map_t])
).push(suffete::prelude::NULL).build();

let names = collect_class_names(outer);
// names == [Foo, Bar, Map, Box]  (in some order)
```

## Deduplicating

The walker visits the same `ElementId` once per *occurrence*, not per ID. If a class appears multiple times in the tree, the closure runs multiple times. To dedupe:

```rust,ignore
use std::collections::HashSet;

fn collect_unique_class_names(ty: TypeId) -> HashSet<Atom> {
    let names = RefCell::new(HashSet::new());
    inspect::any(ty, |elem| {
        let i = interner();
        match elem.kind() {
            ElementKind::Object => { names.borrow_mut().insert(i.get_object(elem).name); }
            ElementKind::Enum   => { names.borrow_mut().insert(i.get_enum(elem).name); }
            _ => {}
        }
        false
    });
    names.into_inner()
}
```

`HashSet::insert` does the dedup; the closure runs as many times as there are occurrences, but only unique names are stored.

## Reach into intersection conjuncts

The walker recurses into `Object`'s `intersections` automatically. For `Foo & Bar`, the recipe collects both `Foo` and `Bar`. For deeply nested intersections (e.g. `Foo & Bar & Baz<Qux>`), the recipe collects all four.

## Filtering by `class_like_kind`

If you want only *class names* (not interfaces, traits, enums), the recipe can filter:

```rust,ignore
fn collect_class_only<W: World>(ty: TypeId, world: &W) -> Vec<Atom> {
    let names = RefCell::new(Vec::new());
    inspect::any(ty, |elem| {
        if elem.kind() == ElementKind::Object {
            let name = interner().get_object(elem).name;
            // Filter: only keep classes, exclude interfaces/traits.
            if !world.is_interface(name) && !world.is_trait(name) {
                names.borrow_mut().push(name);
            }
        }
        false
    });
    names.into_inner()
}
```

The world query keeps the recipe codebase-aware.

## Performance

`inspect::any` is O(tree size) when the closure never short-circuits. For typical analyser types (tree size of a few dozen Elements), the walk is sub-microsecond.

The closure cost is the dominant factor. The recipes above do one `interner().get_object(elem)` per `Object` Element, which is one `&'static` arena read per element ; sub-nanosecond.

Allocating the result vector or hashset has its own cost; for hot paths, reuse a pre-allocated buffer.

## A subtle case: ClassLikeString specifier

`ClassLikeString` Elements may carry their class name in the `Literal(name)` specifier, in the `OfType { constraint }` (a generic constraint), or in the `Generic { constraint }` (a `::class` lookup on a generic). The recipe above handles only the `Literal` case.

For `OfType` and `Generic`, the constraint is a `TypeId` ; the walker descends into it automatically and the closure sees any `Object` Elements inside the constraint.

## Variant: collect class names with positions

If the analyser wants to know *where* in the type each class name appeared (e.g. for diagnostic spans), the closure can track context using a stack in its environment:

```rust,ignore
let names_with_positions = RefCell::new(Vec::new());
let position = RefCell::new(0u32);
inspect::any(ty, |elem| {
    *position.borrow_mut() += 1;
    if elem.kind() == ElementKind::Object {
        let name = interner().get_object(elem).name;
        names_with_positions.borrow_mut().push((name, *position.borrow()));
    }
    false
});
```

The position counter increments on every visit, giving each class a stable position in the walk.

> **See also:** [Inspection](../api/inspect.md) for the walker; [Element kinds](../reference/element-kinds.md) for the full list of payload-bearing kinds and their nested-type carriers; [World](../api/world.md) for codebase-aware filtering.
