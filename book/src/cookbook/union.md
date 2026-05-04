# Building a union from scratch

Three ways to construct a union type, depending on what the analyser is doing.

## Method 1: literal construction with TypeBuilder

For when the analyser knows the elements ahead of time (a parsed PHP type expression, a hand-coded fixture):

```rust,ignore
use suffete::{TypeBuilder, prelude::{INT, STRING, NULL}};

let int_or_string_or_null = TypeBuilder::new()
    .push(INT)
    .push(STRING)
    .push(NULL)
    .build();
```

`build` is *structural*: it sorts and dedups but does not apply join's canonicalisation rules (no range merging, no literal collapse, no subtype absorption).

If the analyser wants those rules applied:

```rust,ignore
let canonical = TypeBuilder::new()
    .push(INT)
    .push(STRING)
    .push(NULL)
    .build_canonical();
```

`build_canonical` runs the join's canonicalisation pass: subsumption, range merging, literal collapse, etc. Use it when the analyser wants the smallest representation.

## Method 2: incremental, via lattice::join

For when the analyser is producing a union from a set of computed values (e.g. the union of a switch's case values, or the join of a control-flow merge):

```rust,ignore
use suffete::{TypeId, lattice};

fn join_all<W: suffete::world::World>(types: &[TypeId], world: &W) -> TypeId {
    let mut result = suffete::prelude::TYPE_NEVER;
    let opts = lattice::LatticeOptions::default();
    let mut report = lattice::LatticeReport::new();
    for &t in types {
        result = lattice::join(result, t, world, opts, &mut report);
    }
    result
}
```

The fold starts at `TYPE_NEVER` (the join identity) and applies join pairwise. The result is the canonical union of every input.

Each `join` call applies the canonicalisation rules: range merging, literal collapse, etc.

## Method 3: from_type for incremental modification

For when the analyser already has a union and wants to push one or two more elements:

```rust,ignore
use suffete::{TypeBuilder, prelude::NULL};

let nullable = TypeBuilder::from_type(existing_type)
    .push(NULL)
    .build();
```

`from_type` enables the origin short-circuit: if the buffer is unchanged after the mutations, `build()` returns the original `TypeId` without re-interning. Useful when the mutations might be no-ops.

## Comparison of the three methods

| Method | Use when | Canonicalisation | Cost |
|---|---|---|---|
| `TypeBuilder::push * / build()` | Constructing from known elements | Structural only (sort, dedup) | Sort + intern lookup |
| `TypeBuilder::push * / build_canonical()` | Constructing from known elements, want canonical form | Full canonicalisation | Sort + canonicalise + intern |
| Repeated `lattice::join` | Folding a sequence | Per-step canonicalisation | N intern + N canonicalise |
| `TypeBuilder::from_type / push / build()` | Adding to an existing type | Structural only | Origin check + sort + intern |

For most analyser code, `build` (or `build_canonical` if you need the canonical form) is the right choice. The repeated-join pattern is for pipelines where each input is computed lazily.

## Worked example: union of switch case types

```php
function f(int $kind): string {
    return match ($kind) {
        1, 2 => 'a',
        3 => 'b',
        default => 'c',
    };
}
```

The analyser computes the type of each match-arm body and joins them to produce the function's return type:

```rust,ignore
let arm1 = ...; // type of 'a' = literal "a"
let arm2 = ...; // type of 'b' = literal "b"
let arm3 = ...; // type of 'c' = literal "c"

let return_t = join_all(&[arm1, arm2, arm3], &world);
// return_t == "a"|"b"|"c"  (or, after literal collapse threshold, just `string`)
```

If the analyser wants to preserve the literals (no collapse), use `TypeBuilder::push * / build`. If the analyser wants the canonical form (let the lattice decide whether to collapse), use `lattice::join` (which uses the lattice's literal-collapse threshold) or `build_canonical`.

## Worked example: making a type nullable

```rust,ignore
use suffete::{TypeBuilder, prelude::NULL, predicates::contains_null};

fn make_nullable(t: TypeId) -> TypeId {
    if contains_null(t) {
        return t;  // already nullable
    }
    TypeBuilder::from_type(t).push(NULL).build()
}
```

The `contains_null` check avoids the unnecessary intern lookup for already-nullable inputs. The `from_type` ensures the origin short-circuit fires when (paradoxically) the type was already nullable but we hit this code anyway.

## Worked example: union of class names

The analyser has a list of classes the user declared in a docblock and wants to construct the type:

```rust,ignore
use suffete::{TypeBuilder, ElementId};
use mago_atom::Atom;

fn classes_to_union(names: &[Atom]) -> TypeId {
    let mut b = TypeBuilder::new();
    for &name in names {
        b.push(ElementId::named_object(name));
    }
    b.build()
}

let t = classes_to_union(&[
    mago_atom::atom("Foo"),
    mago_atom::atom("Bar"),
    mago_atom::atom("Baz"),
]);
// t == Foo | Bar | Baz
```

If two of the classes are related by inheritance, `build_canonical` (or a separate `lattice::join` fold) would absorb the descendant ; `build` keeps them distinct.

## Performance notes

- `TypeBuilder::push` is O(1).
- `build` is O(n log n) for the sort plus the interner cost. The interner has a fast path that detects already-sorted-and-unique input via [SIMD](../internals/simd.md).
- `build_canonical` is O(n²) worst case (subsumption is pairwise), but typical inputs are small.
- `lattice::join` is the canonicalisation cost per pair, O(N) over the fold.

For analyser hot loops constructing many unions, consider:

- Reusing one `TypeBuilder` instance across iterations (clear and re-push).
- Pre-sorting if you know the input is canonical, so the interner's fast path fires.
- Choosing `build` over `build_canonical` if you don't need the canonical form ; the latter is strictly more expensive.

> **See also:** [TypeBuilder](../api/construction.md) for the construction API; [join](../lattice/join.md) for the canonicalisation rules; [Predicates](../api/predicates.md) for the `contains_null` and friends used as fast pre-checks.
