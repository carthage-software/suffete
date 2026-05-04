# Prelude constants

The `suffete::prelude` module exposes well-known constants. Importing it gives you the common-case Elements and Types without going through the interner.

```rust,ignore
use suffete::prelude::*;
```

## Elements (`ElementId`)

### Landmarks

| Constant | Element | Notes |
|---|---|---|
| `NEVER` | `Never` | $\bot$. |
| `MIXED` | `Mixed` (vanilla) | $\top$. |
| `NULL` | `Null` | The value `null`. |
| `VOID` | `Void` | The PHP `void`. |
| `PLACEHOLDER` | `Placeholder` | Inference hole. |

### Booleans

| Constant | Element |
|---|---|
| `TRUE` | `True` |
| `FALSE` | `False` |
| `BOOL` | `Bool` |

### Scalars (unrefined)

| Constant | Element |
|---|---|
| `INT` | `Int` (unspecified) |
| `FLOAT` | `Float` (unspecified) |
| `STRING` | `String` (unspecified, no flags) |
| `NUMERIC` | `Numeric` (true union) |
| `SCALAR` | `Scalar` (true union) |
| `ARRAY_KEY` | `ArrayKey` (true union) |

### Common refinements

| Constant | Element |
|---|---|
| `NUMERIC_STRING` | `String` (unspecified, with `is_numeric=true`) |
| `NON_EMPTY_STRING` | `String` (unspecified, with `is_non_empty=true`) |
| `EMPTY_STRING` | `String` (literal `""`) |
| `INT_ZERO` | `Int` (literal `0`) |

### Object family

| Constant | Element |
|---|---|
| `OBJECT_ANY` | `ObjectAny` |

### Iterable / callable

| Constant | Element |
|---|---|
| `ITERABLE_MIXED_MIXED` | `Iterable<mixed, mixed>` |
| `CALLABLE` | `Callable` (bare form, no signature) |

### Resources

| Constant | Element |
|---|---|
| `RESOURCE` | `Resource` (unrefined) |
| `OPEN_RESOURCE` | `Resource` (state = Open, no kind) |
| `CLOSED_RESOURCE` | `Resource` (state = Closed, no kind) |

### Arrays

| Constant | Element |
|---|---|
| `EMPTY_ARRAY` | `array{}` (the empty sealed shape) |

## Types (`TypeId`)

Each `TYPE_*` constant is the singleton type wrapping the corresponding Element. Use these when you need a `TypeId` for a single Element ; it saves the construction call.

| Constant | Type | Equivalent |
|---|---|---|
| `TYPE_NEVER` | `never` | `TypeBuilder::new().push(NEVER).build()` |
| `TYPE_MIXED` | `mixed` | `TypeBuilder::new().push(MIXED).build()` |
| `TYPE_NULL` | `null` | ... |
| `TYPE_VOID` | `void` | ... |
| `TYPE_TRUE` | `true` | ... |
| `TYPE_FALSE` | `false` | ... |
| `TYPE_BOOL` | `bool` | ... |
| `TYPE_INT` | `int` | ... |
| `TYPE_FLOAT` | `float` | ... |
| `TYPE_STRING` | `string` | ... |
| `TYPE_NUMERIC` | `numeric` | ... |
| `TYPE_SCALAR` | `scalar` | ... |
| `TYPE_ARRAY_KEY` | `array-key` | ... |
| `TYPE_NUMERIC_STRING` | `numeric-string` | ... |
| `TYPE_NON_EMPTY_STRING` | `non-empty-string` | ... |
| `TYPE_OBJECT_ANY` | `object` | ... |
| `TYPE_RESOURCE` | `resource` | ... |
| `TYPE_OPEN_RESOURCE` | `open-resource` | ... |
| `TYPE_CLOSED_RESOURCE` | `closed-resource` | ... |
| `TYPE_EMPTY_ARRAY` | `array{}` | ... |

The `TYPE_*` constants are computed at boot time and exposed as `const`s. Comparing `t == TYPE_INT` is a single u64 compare ; faster than constructing a `TypeBuilder` and calling `build`.

## When to use the prelude

For the trivial cases:

```rust,ignore
use suffete::prelude::*;

// Use the Element constant when constructing a union:
let t = TypeBuilder::new().push(INT).push(STRING).build();

// Use the Type constant when you need a TypeId directly:
let int_only: TypeId = TYPE_INT;

// Use the Type constant in operations:
let result = lattice::refines(some_t, TYPE_MIXED, &world, opts, &mut report);
```

For non-trivial Elements (named objects, sealed shapes, callable signatures), use the `ElementId` constructors or the interner directly. The prelude only covers the well-known singletons.

## Naming conventions

- **`UPPER_SNAKE_CASE`** for `ElementId` constants.
- **`TYPE_UPPER_SNAKE_CASE`** for `TypeId` constants.
- The element name matches the PHP-side name, lowercased and snake-cased: `INT`, `STRING`, `OBJECT_ANY`, `EMPTY_ARRAY`, `NUMERIC_STRING`.

## Adding to the prelude

The prelude is the place for *frequently-needed* constants. New entries should:

- Have a clearly-defined PHP-side meaning (so the name is unambiguous).
- Be a singleton (one canonical Element / Type per name).
- Get used often enough that an analyser shouldn't have to construct it inline.

Refinement variants that can be expressed as flag combinations on the basic kinds (e.g. `truthy-string`) are typically *not* in the prelude ; they are constructed via the interner with a `StringInfo` carrying the flags.

> **See also:** [Constructing types: TypeBuilder and prelude](../api/construction.md) for the broader construction API; [Element kinds](./element-kinds.md) for the full kind list.
