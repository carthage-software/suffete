# TypeId, ElementId, and identity

Suffete is a handle-based crate. Every interesting value — a Type, an Element, an interned string of element IDs, a parameter list, a known-items list, a defining entity — is referenced by a small `NonZero` handle. The actual data lives in process-global arenas. Handles are cheap to compare, hash, copy, and pass around. The arenas are append-only and never freed.

This chapter covers the four handles a user of the public API will see most.

## `ElementId`

The interned handle to a single [`Element`](../universe/elements.md). Layout: `NonZeroU32`, packed as

```text
[kind tag: 6 bits] [arena slot: 26 bits]
```

The high 6 bits encode the [`ElementKind`](../reference/element-kinds.md) (1..=63 ; 64 values reserved, 30+ used). The low 26 bits encode the per-kind arena slot (0..=2^26-1 ≈ 67 million, more than enough).

Two `ElementId`s compare equal iff they refer to the same canonical Element ; this is the interner's contract. Equality is one `u32` compare; hashing is trivial.

```rust,ignore
use suffete::{ElementId, ElementKind};

// ElementId is Copy, Eq, Hash, Ord
let id: ElementId = suffete::prelude::INT;
assert_eq!(id.kind(), ElementKind::Int);
```

The `kind()` method is `(id.raw() >> 26) as u8` cast through `ElementKind`. Constant-time, branch-free.

The `view()` method resolves the handle to the borrowed [`Element`](../universe/elements.md) view, which carries a `&'static SomeInfo` for payload-bearing kinds:

```rust,ignore
use suffete::Element;
let elem: Element = id.view();
match elem {
    Element::Int(info) => { /* info: &'static IntInfo */ }
    Element::Object(info) => { /* info: &'static ObjectInfo */ }
    // ...
    _ => {}
}
```

For trivial kinds (no payload) the view returns a unit-like variant: `Element::Null`, `Element::Never`, `Element::Mixed(...)`, etc.

## `TypeId`

The interned handle to a [Type](../universe/elements.md): a union of Elements plus a small bag of [`FlowFlags`](../reference/options-reports.md) plus 8 bits of caller-defined `meta`. Layout: `NonZeroU64`, packed as

```text
[slot: 32 bits] [flags: 16 bits] [meta: 8 bits] [reserved: 8 bits]
```

- **`slot`** — the index into the type-content arena. Two `TypeId`s with the same slot share the same interned `Type` (the same element-set).
- **`flags`** — the [`FlowFlags`](../reference/options-reports.md) bitset. Riding on the handle keeps the arena content-keyed; toggling a flag is bit-twiddling, not a re-intern.
- **`meta`** — 8 bits of consumer-defined storage. Suffete never inspects it. Use it for tag-style metadata (provenance enum, severity, boolean markers); for anything that needs more bits or indexes a side table, the consumer should keep their own `HashMap<TypeId, T>`.
- **`reserved`** — reserved for future suffete use; always zero. Not exposed publicly.

Equality and hashing compare *all 64 bits*: `t1 == t2` means same content AND same flags AND same meta.

```rust,ignore
use suffete::{TypeId, FlowFlags, prelude::TYPE_INT};

// TypeId is Copy, Eq, Hash, Ord
let t: TypeId = TYPE_INT;
assert_eq!(t.flags(), FlowFlags::EMPTY);
assert_eq!(t.meta(), 0);
```

For comparison ignoring flags / meta:

```rust,ignore
let t1 = ...;
let t2 = ...;
assert!(t1.content_eq(&t2));   // same elements, ignore flags/meta
```

For deriving related handles in O(1) without touching the arena:

```rust,ignore
let with_flag = t.with_flags(t.flags().with_from_template_default(true));
let with_meta = t.with_meta(7);
```

The `with_*` methods return new handles in O(1) ; the underlying arena entry is unchanged.

### Resolving a `TypeId` to its content

```rust,ignore
let view = t.as_ref();
let elements: &'static [ElementId] = view.elements;
```

The returned `&'static` reference is a real `'static` slice into the per-type element-list arena ; safe to hold for the lifetime of the process.

## `ElementListId`

A handle to an interned slice `&'static [ElementId]`. Used as the type of intersection-conjunct lists, of any-kind element lists stored on a payload.

```rust,ignore
use suffete::ElementListId;
use suffete::interner::interner;

let elements: &[ElementId] = &[suffete::prelude::INT, suffete::prelude::STRING];
let id: ElementListId = interner().intern_element_list(elements);
let resolved: &'static [ElementId] = interner().get_element_list(id);
assert_eq!(resolved, elements);
```

`ElementListId` is also `NonZeroU32`. Two lists with the same content have the same `ElementListId`.

## `TypeListId`

A handle to an interned slice `&'static [TypeId]`. Used as the type of object type-args, of derived-info type lists.

Same shape as `ElementListId`. Same interning guarantee.

## Construction discipline

The intended pattern for building a type:

1. Use [`TypeBuilder`](./construction.md) to push elements (and set flags) over a sequence of mutations.
2. Call `build()` to intern once.

The [`TypeBuilder`](./construction.md) chapter covers the API in detail. Direct interner calls are also possible:

```rust,ignore
let t = suffete::interner::interner().intern_type(
    &[suffete::prelude::INT, suffete::prelude::STRING],
    suffete::FlowFlags::EMPTY,
);
```

The interner handles canonicalisation (sort + dedup) and dedup against existing entries.

## What guarantees handle equality

For `TypeId`:

- Same `(slot, flags, meta)` triple → same handle.
- Same content (sorted+deduped element list) + same flags + same meta → same handle (because the slot dedups by content).

For `ElementId`:

- Same kind + same payload → same handle.

The interner enforces both. Two `TypeId`s constructed at different points in time, on different threads, with the same logical inputs, will compare `==`. Likewise for `ElementId`.

This is what makes the lattice fast: two-element comparisons are one `u32` apiece. Hashing a `TypeId` is hashing one `u64`.

## Lifetime guarantees

- `&'static [ElementId]` returned from interner methods is real `'static` ; the arena is in a `OnceLock` for the process lifetime.
- `&'static SomeInfo` returned via `Element::Int(info)` (etc.) is real `'static`.
- `Atom` (interned strings via `mago_atom`) is also real `'static`.

The arenas grow over the lifetime of the process; they never shrink. Re-interning the same value is idempotent.

## A worked example

```rust,ignore
use suffete::{TypeBuilder, ElementId, prelude::{INT, STRING}};

let t1 = TypeBuilder::new().push(INT).push(STRING).build();
let t2 = TypeBuilder::new().push(STRING).push(INT).build();
let t3 = TypeBuilder::new().push(INT).push(STRING).push(INT).build();

assert_eq!(t1, t2);  // sort makes order irrelevant
assert_eq!(t1, t3);  // dedup makes duplicates irrelevant

assert_eq!(t1.as_ref().elements.len(), 2);
```

> **See also:** [Constructing types: TypeBuilder and prelude](./construction.md) for the user-facing build API; [The ElementId tag layout](../internals/element-id-layout.md) for the bit-level details; [Interning and the arenas](../internals/interner.md) for how the storage works.
