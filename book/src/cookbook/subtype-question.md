# Answering "is A a subtype of B?"

The most common analyser question. Recipe with the full setup, expansion, and result-handling.

## The pattern

```rust,ignore
use suffete::{TypeId, lattice::{self, LatticeOptions, LatticeReport}, predicates::is_fully_resolved, expand};
use suffete::world::World;

fn is_subtype<W: World>(
    sub:    TypeId,
    sup:    TypeId,
    world:  &W,
    template_env: &TemplateEnv,
    strict: bool,
) -> (bool, LatticeReport) {
    // 1. Expand any unresolved Elements (Alias, Conditional, Derived, ...).
    let sub = if is_fully_resolved(sub) { sub } else { expand::expand(sub, world, template_env) };
    let sup = if is_fully_resolved(sup) { sup } else { expand::expand(sup, world, template_env) };

    // 2. Configure lattice options.
    let opts = LatticeOptions::default()
        .with_php_runtime_coerce(!strict);

    // 3. Run the lattice query, collecting report side-info.
    let mut report = LatticeReport::new();
    let result = lattice::refines(sub, sup, world, opts, &mut report);

    (result, report)
}
```

## Reading the result

The `(bool, LatticeReport)` pair is structured for diagnostic emission:

- `bool == true` ; the subtype check holds (under the chosen strictness).
- `bool == false` ; the check failed; the analyser surfaces a type error.
- `report.causes` ; bitset of which coercion-tolerant rules fired. Surface in the diagnostic if you want to warn (e.g. "this passes, but uses PHP's int-to-float coercion").

## Strictness

The `strict` parameter maps to PHP's `declare(strict_types=1)`:

- **strict mode** (`strict = true`): no coercion edges. `int <: float` is `false`. `numeric-string <: int` is `false`.
- **non-strict mode** (`strict = false`, the default): coercion edges admitted with `CoercionCauses::PHP_RUNTIME_COERCE` recorded.

The analyser typically chooses based on the file's `declare(strict_types)` directive. Files without a directive default to non-strict.

## Worked examples

### Direct subtype

```rust,ignore
use suffete::{TypeBuilder, prelude::{INT, STRING}};

let int_t  = TypeBuilder::new().push(INT).build();
let int_or_str = TypeBuilder::new().push(INT).push(STRING).build();

let (ok, _report) = is_subtype(int_t, int_or_str, &world, &template_env, true);
assert!(ok);
```

### Coercion-only subtype (non-strict)

```rust,ignore
use suffete::{TypeBuilder, prelude::{INT, FLOAT}};

let int_t = TypeBuilder::new().push(INT).build();
let float_t = TypeBuilder::new().push(FLOAT).build();

let (ok_strict, _) = is_subtype(int_t, float_t, &world, &template_env, true);
assert!(!ok_strict);  // strict: int does not refine float.

let (ok_loose, report) = is_subtype(int_t, float_t, &world, &template_env, false);
assert!(ok_loose);    // non-strict: int coerces.
assert!(report.causes.contains(suffete::lattice::CoercionCauses::PHP_RUNTIME_COERCE));
```

### Class-hierarchy subtype

```rust,ignore
use suffete::{TypeBuilder, ElementId};
use mago_atom::atom;

// Setup (in your World): class B extends A.

let b_t = TypeBuilder::new().push(ElementId::named_object(atom("B"))).build();
let a_t = TypeBuilder::new().push(ElementId::named_object(atom("A"))).build();

let (ok, _) = is_subtype(b_t, a_t, &world, &template_env, true);
assert!(ok);
```

### Generic subtype with variance

```rust,ignore
// World registers Iterator<T> as covariant on T.

let int_iter = TypeBuilder::new().push(
    ElementId::named_object_with_args(atom("Iterator"), &[suffete::prelude::TYPE_INT])
).build();
let mixed_iter = TypeBuilder::new().push(
    ElementId::named_object_with_args(atom("Iterator"), &[suffete::prelude::TYPE_MIXED])
).build();

let (ok, _) = is_subtype(int_iter, mixed_iter, &world, &template_env, true);
assert!(ok);  // covariance: int <: mixed
```

### Failure with a useful report

```rust,ignore
let foo = TypeBuilder::new().push(ElementId::named_object(atom("Foo"))).build();
let bar = TypeBuilder::new().push(ElementId::named_object(atom("Bar"))).build();

let (ok, report) = is_subtype(foo, bar, &world, &template_env, true);
assert!(!ok);

// report contains structured information you can use to construct
// a diagnostic message, like which family of rules failed.
```

## Performance

`refines` is the hot path. Suffete optimises for the common cases:

- **Reflexivity**: `refines(t, t) = true` in one comparison.
- **Singleton-vs-singleton**: most analyser queries are between one-Element types; the cartesian is degenerate.
- **Subsumption shortcut in unions**: if any Element on the left refines any Element on the right by the universal axioms (top, bot, placeholder), short-circuit.

Expected cost for a typical analyser query: tens of nanoseconds. Worst-case (deep generics, multi-conjunct intersections, full fan-out coverage): microseconds.

## When to expand

Expand *exactly once* per type, before the first lattice query. The expansion result is itself a `TypeId` and can be cached per `(input, world-version, template_env)` if the analyser sees it many times.

The `is_fully_resolved` check is cheap (a tree walk with a short-circuiting predicate), so the recipe above always-checks-then-expands. For analyser code that knows expansion has already happened (e.g. inside a single statement's analysis), skip the check.

## When to skip the world

If the question doesn't require the world (e.g. `is_subtype(int_or_string, int)`), you can pass `NullWorld`:

```rust,ignore
use suffete::world::NullWorld;
let (ok, _) = is_subtype(int_or_string, int_t, &NullWorld, &TemplateEnv::default(), true);
```

The lattice queries the world only when needed. For fully-structural inputs, the world is never asked.

> **See also:** [refines](../lattice/refines.md) for the rules; [overlaps](../lattice/overlaps.md) for the symmetric "share a value?" question; [LatticeOptions](../reference/options-reports.md) for the strictness toggles.
