# Constructing types: TypeBuilder and prelude

There are two routes to a `TypeId`:

1. **`TypeBuilder`** — a mutable scratch buffer. Push elements, change flags, then `build()` to intern once. The intended path for most analyser-side type construction.
2. **`prelude`** — well-known constants for the common cases (`INT`, `STRING`, `TYPE_NEVER`, `TYPE_MIXED`, `EMPTY_ARRAY`, etc.). Use these directly when applicable.

For per-Element construction (named objects, sealed shapes, callable signatures, etc.), the [`ElementId`](./handles.md) constructors and the interner are the underlying tools; `TypeBuilder` is a layer on top.

## `TypeBuilder`

```rust,ignore
use suffete::{TypeBuilder, TypeId, ElementId, FlowFlags};

let mut b = TypeBuilder::new();
b.push(suffete::prelude::INT);
b.push(suffete::prelude::STRING);
let t: TypeId = b.build();
```

`TypeBuilder` is a `Vec<ElementId> + FlowFlags` plus an optional *origin* `TypeId`. Mutations are direct vec operations; `build()` calls the interner once at the end.

### Construction

```rust,ignore
TypeBuilder::new();                 // empty buffer, EMPTY flags
TypeBuilder::from_type(some_t);     // start from an existing TypeId; remembers it as origin
```

`from_type` enables the **origin short-circuit**: if `build()` is reached with the buffer in the same shape and flags as the origin, it returns the original `TypeId` without re-interning. Useful for transforms that mostly leave the type alone.

### Element mutations

```rust,ignore
b.push(elem);                         // append
b.extend(iter_of_elems);              // extend
b.remove(elem);                       // remove first occurrence
b.remove_all(elem);                   // remove every occurrence
b.retain(|e| /* keep this? */);       // arbitrary filter
b.replace(old, new);                  // replace first occurrence
b.map(|e| /* transform */);           // in-place per-element map
b.flat_map(|e| /* expand */);         // 1-to-N expansion
b.contains(elem);                     // O(n) lookup (SIMD-accelerated)
```

Mutations preserve the order they happened in. The interner sorts and dedups on `build`.

### Flag mutations

```rust,ignore
b.set_flags(FlowFlags::EMPTY.with_from_template_default(true));
b.modify_flags(|f| f.with_from_template_default(true));
```

Flags are `FlowFlags` ; a 16-bit bitset. The most relevant in user code:

- `from_template_default` — set by suffete when a type-arg was filled with the parameter's upper bound rather than the user's value. Used by the variance check at refinement time.

The full list is in the [reference](../reference/options-reports.md).

### Build modes

```rust,ignore
let t1 = b.build();             // sort + dedup, intern.
let t2 = b.build_canonical();   // sort + dedup + apply canonicalisation rules from `join`, intern.
```

- **`build()`** runs the interner's structural canonicalisation: sort the element list by `ElementId`, dedup, intern. No subsumption, no range merging, no literal collapse. The result is the input verbatim, just canonicalised.
- **`build_canonical()`** runs the join's canonicalisation: subsumption, range merging, literal collapse, true-union dominators, etc. Equivalent to `lattice::join(t1, TYPE_NEVER, ...)` for any `t1`. Use this when you want the analyser's "official" canonical form.

The `build()` path is used by the analyser when it knows the input is already canonical or when the caller doesn't want collapses (e.g. preserving `int|literal-int` as two distinct elements). `build_canonical()` is used when the analyser wants the smallest possible expression.

### Origin short-circuit

```rust,ignore
let original = ...; // some existing TypeId

let t = TypeBuilder::from_type(original)
    .map(|e| if e == suffete::prelude::INT { suffete::prelude::STRING } else { e })
    .build();

// If the map happened to return every element unchanged, t == original.
// If anything changed, t is a new interned TypeId.
```

The dirty-tracking is conservative: any mutation that *could* have changed the buffer flips the dirty bit, even if the mutation was a no-op (e.g. `remove(elem)` for an `elem` not in the buffer). The dirty bit only affects the short-circuit; build always produces a correct result.

## `prelude`

The prelude exposes well-known constants. Every Element kind that has a singleton trivial form has a constant; many of the common payload-bearing forms have one too.

### Element constants (`ElementId`)

```rust,ignore
use suffete::prelude::*;

NEVER, MIXED, NULL, VOID, PLACEHOLDER,           // landmarks
TRUE, FALSE, BOOL,                                // booleans
INT, FLOAT, STRING, NUMERIC, SCALAR, ARRAY_KEY,  // scalars (unrefined)
NUMERIC_STRING,                                   // numeric-string
NON_EMPTY_STRING, EMPTY_STRING, INT_ZERO,         // common refinements + literals
OBJECT_ANY,                                       // any object
ITERABLE_MIXED_MIXED,                             // iterable<mixed, mixed>
CALLABLE,                                         // bare callable
RESOURCE, OPEN_RESOURCE, CLOSED_RESOURCE,         // resources
EMPTY_ARRAY,                                      // array{}
```

The complete list is in the [prelude reference](../reference/prelude.md).

### Type constants (`TypeId`)

The `TYPE_*` constants are one-element types wrapping the corresponding Element:

```rust,ignore
use suffete::prelude::*;

TYPE_NEVER, TYPE_MIXED, TYPE_NULL, TYPE_VOID,
TYPE_TRUE, TYPE_FALSE, TYPE_BOOL,
TYPE_INT, TYPE_FLOAT, TYPE_STRING, TYPE_NUMERIC, TYPE_SCALAR,
// ...
```

`TYPE_INT` is `TypeBuilder::new().push(INT).build()`, but pre-computed at boot time and exposed as a `const`. Use the `TYPE_*` constants when you need a `TypeId` for a single element ; it saves a build call.

## Per-element construction

For Elements that need a payload, use the constructors on `ElementId`:

```rust,ignore
ElementId::int_literal(42);                     // int(42)
ElementId::string_literal("hello");             // literal "hello"
ElementId::int_range(IntRange::new(Some(0), Some(100)));  // int<0,100>
ElementId::named_object(atom("Foo"));           // Foo
ElementId::named_object_with_args(atom("Box"), &[TYPE_INT]);  // Box<int>
ElementId::enum_case(atom("Status"), atom("Active"));  // Status::Active
ElementId::generic_parameter(name, defining_entity, constraint);
ElementId::intersected(head, &[conjunct1, conjunct2]);  // head & conjunct1 & conjunct2
```

For more elaborate Elements (callable signatures, sealed shapes), call the interner methods directly:

```rust,ignore
use suffete::interner::interner;

let info = ObjectShapeInfo {
    known_properties: Some(...),
    flags: ObjectShapeFlags::default().with_sealed(true),
};
let shape = interner().intern_object_shape(info);
```

The full set of interner methods is in `src/interner/store.rs`, generated by the `element_arena_methods!` macro.

## A worked example

Build the PHP type `non-empty-list<int|string>|null`:

```rust,ignore
use suffete::{TypeBuilder, ElementId};
use suffete::prelude::{INT, STRING, NULL};
use suffete::element::payload::{ListInfo, ListFlags};
use suffete::interner::interner;

// First the inner element type: int|string
let int_or_string = TypeBuilder::new().push(INT).push(STRING).build();

// Then the list element
let list_elem: ElementId = ElementId::list(int_or_string, /* non_empty: */ true);

// Finally the union with null
let result = TypeBuilder::new()
    .push(list_elem)
    .push(NULL)
    .build();
```

Three intern calls (one for the inner union's element list, one for the list payload, one for the outer union). All idempotent ; calling this code twice with the same inputs produces the same `TypeId` both times.

## Performance notes

- `TypeBuilder::new()` is `Vec::new()` ; no allocation until pushing.
- `push` is `Vec::push`; amortised O(1).
- `build()` allocates a temporary sorted vec; the cost is `O(n log n)` for the sort plus the interner's hash lookup. The interner has a fast path that detects already-sorted-and-unique input (via [`simd::is_sorted_strict`](../internals/simd.md)) and skips the temporary allocation entirely.
- Singleton types (`build()` on a one-element buffer) hit a per-Element cache and skip both sort and the dashmap lookup.

For analyser code that builds many types in a loop, reuse a single `TypeBuilder` across iterations:

```rust,ignore
let mut b = TypeBuilder::new();
for elem in elements_to_process {
    b.push(elem);
    let _ = b.build();
    b = TypeBuilder::new();   // or b.set_flags(FlowFlags::EMPTY) ; both reset
}
```

> **See also:** [TypeId, ElementId, and identity](./handles.md) for the handle types; [Prelude constants](../reference/prelude.md) for the full list of `prelude::*`; [Lattice options and reports](../reference/options-reports.md) for `FlowFlags`.
