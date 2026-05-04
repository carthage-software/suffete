# Predicates: is_X, contains_X, and friends

The `predicates` module is the place for *single-call* structural questions about a `TypeId`. Each predicate answers one question and returns a `bool`. They are pure — no `World`, no options, no report.

```rust,ignore
use suffete::predicates::{
    is_int, is_string, is_truthy, contains_null, is_singleton, is_constant_foldable,
    contains_template_anywhere, is_fully_resolved,
};
```

This chapter walks through the naming conventions, the families, and the cost model.

## Naming conventions

The predicate names follow a strict scheme:

- **`is_X(ty)`** — *guaranteed*: every Element of `ty` is in family `X`. Conservative: `false` when any Element is outside `X`, including for `never` (the all-bottom type).
- **`contains_X(ty)`** — *possible at the top level*: at least one top-level Element of `ty` is in family `X`.
- **`is_truthy(ty)` / `is_falsy(ty)`** — every Element guaranteed truthy / falsy at runtime.
- **`could_be_truthy(ty)` / `could_be_falsy(ty)`** — at least one Element could be truthy / falsy.
- **`*_anywhere(ty)`** — recurses into every nested-type carrier (object args, list element types, callable signatures, etc.). Use these for "does this tree contain any unresolved Element?" or "is there a free template anywhere?".

`is_X` is *false on `never`*: `is_int(TYPE_NEVER) = false`, because the empty type contains no `Int` Elements (vacuously, also no non-`Int` Elements). The conservative reading is what most analyser callers want; if you want vacuous-true, check `is_X(ty) || ty == TYPE_NEVER`.

## Kind-family predicates

For each PHP type family, an `is_X` and (where useful) a `contains_X`:

| `is_X` | `contains_X` | Family |
|---|---|---|
| `is_int` | `contains_int` | `Int` |
| `is_float` | `contains_float` | `Float` |
| `is_string` | `contains_string` | `String` |
| `is_bool` | `contains_bool` | `Bool`, `True`, `False` |
| `is_null` | `contains_null` | `Null` |
| `is_void` | `contains_void` | `Void` |
| `is_list` | — | `List` |
| `is_keyed_array` | — | `Array` |
| `is_array` | `contains_array` | `Array`, `List` |
| `is_iterable` | `contains_iterable` | `Iterable` (the `Iterable` kind only ; not arrays/lists) |
| `is_object` | `contains_object` | `Object`, `Enum`, `ObjectShape`, `HasMethod`, `HasProperty`, `ObjectAny` |
| `is_resource` | `contains_resource` | `Resource` |
| `is_callable` | `contains_callable` | `Callable` |
| `is_array_key` | — | `ArrayKey` |
| `is_scalar` | — | `Scalar`, `Int`, `Float`, `String`, `Bool`, `True`, `False`, `ClassLikeString`, `Numeric`, `ArrayKey` |
| `is_numeric` | — | `Numeric`, `Int`, `Float` |
| — | `contains_mixed` | `Mixed` |

`is_X` is `slice.iter().all(|e| e.kind() in family)`. `contains_X` is `slice.iter().any(|e| e.kind() in family)`. Single-kind versions of these (`is_int`, `contains_int`, etc.) route through SIMD-accelerated [scans](../internals/simd.md); multi-kind versions use the scalar matches.

## Truthiness predicates

```rust,ignore
is_truthy(ty)         // every Element guaranteed truthy
is_falsy(ty)          // every Element guaranteed falsy
could_be_truthy(ty)   // at least one Element could be truthy
could_be_falsy(ty)    // at least one Element could be falsy
```

The `is_*` variants are vacuously `false` for `never`. The `could_be_*` variants are also `false` for `never` (no Elements, so no possibility).

The truthiness implications are the same as the [refinement axes](../universe/refinements.md) chapter: an Object is always truthy, an empty array is always falsy, `int(0)` is falsy, `int(7)` is truthy, `int<-∞,-1> | int<1,∞>` is truthy, an unconstrained `int` is `could_be` both, etc.

## Literal predicates

```rust,ignore
is_literal(ty)            // every Element is a literal-shaped value
                          // (specific int/float/string literal, true, false, null, void)
is_constant_foldable(ty)  // is_literal && is_singleton
                          // — most useful "can I constant-fold this?" check
```

`is_literal` is true for types like `int(7)`, `"foo" | "bar"`, `true | false`, `int(0) | int(1)`. It is false for `int`, `non-empty-string`, `Foo`.

`is_constant_foldable` adds the singleton requirement: exactly one Element. Use this when the analyser wants to know whether the type can be replaced by a concrete value at this program point.

## Structural predicates

```rust,ignore
is_never(ty)         // ty == TYPE_NEVER
is_mixed(ty)         // ty == TYPE_MIXED (vanilla, no narrowing)
is_singleton(ty)     // exactly one Element
is_union(ty)         // more than one Element
```

`is_mixed` is the *vanilla* mixed test ; it returns `false` for narrowed mixed variants (`non-null mixed`, `truthy mixed`, etc.). To detect any mixed variant, use `contains_mixed`.

## Tree-walking predicates

Three predicates recurse into every nested-type carrier (using the [inspect](./inspect.md) walker):

```rust,ignore
contains_mixed_anywhere(ty)         // any Mixed (vanilla or narrowed) anywhere in the tree
contains_template_anywhere(ty)      // any free GenericParameter anywhere
contains_placeholder_anywhere(ty)   // any Placeholder anywhere
contains_unresolved_anywhere(ty)    // Alias, Reference, MemberReference, GlobalReference, Conditional, Derived
is_fully_resolved(ty)               // negation of contains_unresolved_anywhere
```

These are the predicates to call before invoking the lattice on a possibly-unresolved type. The lattice's contract is that it works on resolved inputs; the analyser checks `is_fully_resolved` and calls [`expand`](./expand.md) if not.

## Cost model

Predicates are cheap. The dispatch:

- The kind-family `is_X` / `contains_X` (single kind) variants use [SIMD scans](../internals/simd.md) with thresholds — sub-nanosecond for short slices, `O(n / lane_width)` for long ones.
- The multi-kind variants (`is_bool`, `is_object`, etc.) use scalar `matches!` with early exit.
- The truthiness predicates iterate Elements once and dispatch per-kind. Per-kind cost is constant (one to three comparisons). Total cost is `O(n)`.
- The literal predicates iterate Elements once with a per-kind kind-only check. `O(n)`.
- The tree-walking `*_anywhere` predicates use the [inspect](./inspect.md) walker with a short-circuiting predicate. Cost is `O(tree size)` worst case, but typical short-circuit on the first hit gives `O(small)` in practice.

None of the predicates allocate. None take a `&mut` argument. They are safe to call from any context.

## A worked example

```rust,ignore
use suffete::{TypeBuilder, prelude::{INT, STRING, NULL, TRUE}, ElementId};
use suffete::predicates::{
    is_int, is_string, is_singleton, contains_null, is_constant_foldable,
    is_truthy, could_be_falsy,
};

let int_or_string = TypeBuilder::new().push(INT).push(STRING).build();
assert!(!is_int(int_or_string));     // mixed kinds, not all int
assert!(!is_string(int_or_string));  // ditto
assert!(!is_singleton(int_or_string));
assert!(!contains_null(int_or_string));

let nullable_int = TypeBuilder::new().push(INT).push(NULL).build();
assert!(contains_null(nullable_int));

let lit = TypeBuilder::new().push(ElementId::int_literal(7)).build();
assert!(is_singleton(lit));
assert!(is_constant_foldable(lit));
assert!(is_truthy(lit));         // 7 is truthy
assert!(!could_be_falsy(lit));   // 7 is never falsy

let true_t = TypeBuilder::new().push(TRUE).build();
assert!(is_truthy(true_t));
```

## When you need more than a predicate

Predicates answer *single* yes/no questions. For:

- "Walk the type and collect data" → use [inspect](./inspect.md).
- "Walk the type and produce a new type" → use [transform](./transform.md).
- "Compare two types" → use the [lattice](../lattice/refines.md) operations.

> **See also:** [Inspection: walking the tree](./inspect.md) for the underlying walker the `*_anywhere` predicates use; [Refinement axes](../universe/refinements.md) for the truthiness rules; [Special elements](../universe/special.md) for the landmark Elements (`is_never`, `is_mixed`).
