# Casting and runtime compatibility

Two related but distinct operations live in their own modules:

- **`cast`** — produces a *new* type that the source type would coerce to in a given target context.
- **`compatibility`** — answers a *boolean*: does the source type, in any of its parts, share *runtime* compatibility with the target?

Both exist because PHP's runtime has rules that the static lattice doesn't fully capture. `int` does not refine `float` in the static sense (`int(0)` is not a float), but `int` *can be passed* to a `float` parameter (PHP coerces). The cast and compatibility operations let the analyser model those.

## `cast`

```rust,ignore
use suffete::cast;

let result: TypeId = cast::to(input, target_kind, &world, options);
```

The exact API has multiple cast variants depending on the target:

- `cast::to_int(ty)` — what is the type after `(int)$x`?
- `cast::to_float(ty)` — what is the type after `(float)$x`?
- `cast::to_string(ty)` — what is the type after `(string)$x`?
- `cast::to_bool(ty)` — what is the type after `(bool)$x`?
- `cast::to_array(ty)` — what is the type after `(array)$x`?

Each follows PHP's runtime cast semantics. The result is typically a single Element of the target kind, with as much refinement as can be preserved (e.g. casting `int(0)` to bool gives `false`; casting `int<-∞,-1>|int<1,∞>` to bool gives `true`).

```rust,ignore
use suffete::{TypeBuilder, prelude::{INT, INT_ZERO}};

let zero = TypeBuilder::new().push(INT_ZERO).build();
let one  = TypeBuilder::new().push(suffete::ElementId::int_literal(1)).build();
let any_int = TypeBuilder::new().push(INT).build();

let zero_bool = cast::to_bool(zero);    // false
let one_bool  = cast::to_bool(one);     // true
let any_bool  = cast::to_bool(any_int); // bool (could be either)
```

The cast operation is deterministic for refined inputs (a literal cast to a type produces a literal output) and conservative for unrefined inputs (an `int` cast to bool produces `bool`, since the analyser can't statically know which way it'll go).

## `compatibility`

```rust,ignore
use suffete::compatibility;

let compatible: bool = compatibility::runtime_compatible(a, b, &world, options);
```

Asks: *is there some pair of runtime values, one in `a` and one in `b`, that PHP would consider compatible?* The answer is a boolean.

This is **not the same as `overlaps`**. `overlaps` is the static type-set intersection: do `a` and `b` share a value in the kind sense? `runtime_compatible` is broader: does PHP's runtime allow a value of `a` to be used where a value of `b` is expected?

Examples where `runtime_compatible` differs from `overlaps`:

- `int` and `float`: `overlaps` returns `false` (no integer is a float); `runtime_compatible` returns `true` (PHP coerces).
- `numeric-string` and `int`: `overlaps` returns `false`; `runtime_compatible` returns `true` (PHP coerces).
- `int` and `bool`: `overlaps` returns `false`; `runtime_compatible` returns `true` in non-strict mode (PHP coerces 0 to false, non-zero to true).
- `Foo` and `class-string<Foo>`: `overlaps` returns `false` (one is an object, the other a string); `runtime_compatible` returns `true` if the analyser is checking parameter passing where a class-string can produce an instance.

Use cases:

- The analyser is checking a function call boundary in non-strict mode and wants to know whether to warn about the argument type.
- The analyser is checking a `switch` statement against `case` values and wants to know which cases are reachable under PHP's loose comparison.
- The analyser is checking an `==` (loose equality) operator and wants to know whether the comparison can possibly return `true`.

For *strict* questions ("could this value, statically, be both?"), use `overlaps` instead.

## How the two relate

The `runtime_compatible` operation is a superset of `overlaps`:

- `overlaps(a, b) → runtime_compatible(a, b)`. (If they share a value, they're compatible.)
- `runtime_compatible(a, b) does not imply overlaps(a, b)`. (Compatibility includes coercion edges.)

`refines(a, b) → runtime_compatible(a, b)` (assuming `a` is inhabited).

## Lattice options that affect cast / compatibility

The [`LatticeOptions`](../reference/options-reports.md) `php_runtime_coerce` toggle controls whether the lattice itself admits coercion edges in `refines`. Cast and compatibility are *always* coercion-aware ; they exist to model the runtime. The `php_runtime_coerce` option toggles whether `refines` *also* models the runtime (in non-strict mode) or stays strict.

In strict-types mode (`declare(strict_types=1)`), `refines` should be called with `php_runtime_coerce = false`. The cast and compatibility operations are unchanged ; they always reflect runtime behaviour.

## A worked example

```rust,ignore
use suffete::{TypeBuilder, prelude::{INT, FLOAT}, lattice::{self, LatticeOptions, LatticeReport}, world::NullWorld, compatibility, cast};

let world = NullWorld;
let opts = LatticeOptions::default();
let mut rep = LatticeReport::new();

let int_t = TypeBuilder::new().push(INT).build();
let float_t = TypeBuilder::new().push(FLOAT).build();

// Static refines: int does not refine float.
assert!(!lattice::refines(int_t, float_t, &world, opts, &mut rep));

// But runtime allows the coercion.
assert!(compatibility::runtime_compatible(int_t, float_t, &world, opts));

// Casting int to float gives float.
let casted = cast::to_float(int_t);
assert_eq!(casted, float_t);
```

## When to use which

| Question | Operation |
|---|---|
| Is `a` a strict subtype of `b`? | `lattice::refines` |
| Do `a` and `b` share a value statically? | `lattice::overlaps` |
| Is `a` runtime-compatible with `b` (including coercions)? | `compatibility::runtime_compatible` |
| What does `a` coerce to when forced to type `b`? | `cast::to_X` |
| What is the smallest type containing both `a` and `b`? | `lattice::join` |
| What is the type of `a`'s and `b`'s shared values? | `lattice::meet` |

## A subtle case: refines, overlaps, and runtime_compatible disagree

For `int` and `float`:

- `refines(int, float)` = `false` (strictly: an int is not a float).
- `overlaps(int, float)` = `false` (no integer is a float as a value).
- `runtime_compatible(int, float)` = `true` (PHP coerces).
- `meet(int, float)` = `never` (no shared values).
- `join(int, float)` = `int|float` (no canonical merge).

The strict static analysis (`refines`, `overlaps`, `meet`) treats them as disjoint. The runtime model (`runtime_compatible`, `cast`) acknowledges the coercion. The analyser chooses which to consult based on the diagnostic it's producing.

> **See also:** [refines](../lattice/refines.md), [overlaps](../lattice/overlaps.md) for the strict static answers; [Lattice options and reports](../reference/options-reports.md) for the `php_runtime_coerce` toggle.
