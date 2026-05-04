# Lattice options and reports

The lattice operations take two extra parameters beyond the types and the world: `LatticeOptions` and a `&mut LatticeReport`. This chapter documents both.

## `LatticeOptions`

The configuration knobs for a single lattice query. All fields default to "strict reading"; the analyser sets them based on the file's strictness mode and any per-call overrides.

```rust,ignore
use suffete::lattice::LatticeOptions;

let opts = LatticeOptions::default();
let opts = opts.with_php_runtime_coerce(false);  // strict mode
let opts = opts.with_ignore_null(true);          // ignore null in unions
let opts = opts.with_ignore_false(true);         // ignore false in unions
```

The full set of options:

| Field | Default | Effect |
|---|---|---|
| `php_runtime_coerce` | `true` | Admit PHP runtime coercion edges (`int → float`, `numeric-string → int/float`). Records `CoercionCauses::PHP_RUNTIME_COERCE` on the report when fired. Set to `false` for strict-types mode. |
| `ignore_null` | `false` | When checking `refines(τ, σ)`, drop `null` from `τ` before the per-Element check. Used by analyser code that has separately verified non-nullability. |
| `ignore_false` | `false` | Same for `false`. Used in nullable-via-false patterns. |
| `merge_list_element_types` | `true` | In join, merge sealed-list element types where possible. |
| `merge_keyed_array_params` | `true` | In join, merge keyed-array key/value parameters where possible. |
| `int_literal_collapse_threshold` | (small) | The number of distinct int literals at which join collapses to `int` or a range. |
| `string_literal_collapse_threshold` | (small) | Same for string literals. |

The `with_*` methods return a new `LatticeOptions` ; the type is `Copy`, so chaining is cheap.

## `LatticeReport`

A buffer the lattice writes into during a query. It carries:

- **Coercion causes** ; a bitset of which special rules fired, suitable for the analyser to surface in a diagnostic.

```rust,ignore
use suffete::lattice::{LatticeOptions, LatticeReport, CoercionCauses};

let mut report = LatticeReport::new();
// ... call lattice operations, passing &mut report ...

if report.causes.contains(CoercionCauses::PHP_RUNTIME_COERCE) {
    // The check passed but used a runtime-coercion edge.
    // Surface a warning if the analyser's policy requires.
}

if report.causes.contains(CoercionCauses::TEMPLATE_DEFAULT) {
    // The check passed but used a default-filled template parameter.
}
```

### `CoercionCauses`

A bitflags type:

| Cause | Meaning |
|---|---|
| `PHP_RUNTIME_COERCE` | A PHP-runtime coercion edge was used (e.g. `int → float`). |
| `TEMPLATE_DEFAULT` | A default-filled template parameter was tolerated. |
| `IGNORE_NULL` | The query ran with `null` ignored. |
| `IGNORE_FALSE` | The query ran with `false` ignored. |

The bits are non-zero when the corresponding rule contributed to the answer. The analyser reads the bitset after the query and decides what to do.

### Reusing reports

`LatticeReport::new()` is cheap (just a zeroed bitset). For analyser hot loops, reuse one instance:

```rust,ignore
let mut report = LatticeReport::new();
for query in queries {
    report.reset();   // clear the bitset
    let _ = lattice::refines(query.a, query.b, &world, opts, &mut report);
    // ... handle the result and the report ...
}
```

`reset` clears the bitset in O(1).

## `FlowFlags`

A 16-bit bitset that rides on every `TypeId`. Stored in the `flags` field of the handle's `u64` representation.

```rust,ignore
use suffete::FlowFlags;

let f = FlowFlags::EMPTY;
let f = f.with_from_template_default(true);
let f = f.with_isset_from_loop(true);
let f = f.with_explicitly_nullable(true);
```

The full set of flags:

| Flag | Meaning |
|---|---|
| `from_template_default` | This type-arg was filled with the parameter's upper bound rather than the user's value. The variance check tolerates it (recording `CoercionCauses::TEMPLATE_DEFAULT`). |
| `isset_from_loop` | This value flowed through a loop body. Used by the analyser's flow-typing to track variables introduced inside a loop. |
| `explicitly_nullable` | The type was constructed as nullable via `?T` syntax (rather than as a separate `T \| null` union). Carried for diagnostic precision. |

The flags do not affect lattice operations directly ; they are metadata the analyser reads out via `TypeId::flags()`. The exception is `from_template_default`, which the lattice's variance check consults.

## A worked example

```rust,ignore
use suffete::{TypeBuilder, prelude::{INT, FLOAT}};
use suffete::{lattice::{self, LatticeOptions, LatticeReport, CoercionCauses}, world::NullWorld};

let int_t = TypeBuilder::new().push(INT).build();
let float_t = TypeBuilder::new().push(FLOAT).build();

let world = NullWorld;
let mut report = LatticeReport::new();

// Strict mode: int does not refine float.
let opts_strict = LatticeOptions::default().with_php_runtime_coerce(false);
assert!(!lattice::refines(int_t, float_t, &world, opts_strict, &mut report));

// Non-strict mode: int coerces.
report.reset();
let opts_loose = LatticeOptions::default();  // php_runtime_coerce = true
assert!(lattice::refines(int_t, float_t, &world, opts_loose, &mut report));
assert!(report.causes.contains(CoercionCauses::PHP_RUNTIME_COERCE));
```

The same `int <: float` query, two different opts, two different answers. The report tells the analyser *why* the loose answer is loose.

> **See also:** [refines](../lattice/refines.md), [meet](../lattice/meet.md), [join](../lattice/join.md), [subtract](../lattice/subtract.md), [narrow](../lattice/narrow.md), [overlaps](../lattice/overlaps.md) for the operations that consume these.
