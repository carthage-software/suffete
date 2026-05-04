# Narrowing a parameter type from instanceof

A canonical analyser pattern: the parameter has type `T`, and the body has an `instanceof` check. Inside the `if`, the type is narrowed to the intersection; in the `else`, the type is narrowed to the difference.

## The PHP

```php
function f(Foo|Bar|null $x): void {
    if ($x instanceof Foo) {
        // $x is Foo here
    } else {
        // $x is Bar|null here
    }
}
```

## The recipe

```rust,ignore
use suffete::{TypeId, lattice};
use suffete::world::World;

struct Branches { in_branch: TypeId, else_branch: TypeId }

fn narrow_instanceof<W: World>(
    input: TypeId,         // the parameter's type
    target: TypeId,        // the type from the instanceof's RHS, e.g. Foo
    world: &W,
) -> Branches {
    let opts = lattice::LatticeOptions::default();
    let mut report = lattice::LatticeReport::new();

    let in_branch = lattice::narrow(input, target, world, opts, &mut report);
    let else_branch = lattice::subtract(input, target, world, opts, &mut report);

    Branches { in_branch, else_branch }
}
```

The `narrow` for the positive case typically reduces to meet (`input ⊓ target`). The `subtract` for the negative case gives back the input minus the target.

## Worked example

```rust,ignore
use suffete::{TypeBuilder, ElementId, prelude::NULL};
use suffete::world::NullWorld;
use mago_atom::atom;

let foo = ElementId::named_object(atom("Foo"));
let bar = ElementId::named_object(atom("Bar"));

let input = TypeBuilder::new().push(foo).push(bar).push(NULL).build();
let target = TypeBuilder::new().push(foo).build();

let world = NullWorld;
let branches = narrow_instanceof(input, target, &world);

// branches.in_branch == Foo
// branches.else_branch == Bar | null
```

## When `target` is itself a union

```php
if ($x instanceof Foo || $x instanceof Bar) {
    // $x is Foo|Bar here
}
```

The analyser computes the union of the two `instanceof` targets and passes it as the target. The recipe is unchanged ; `narrow` and `subtract` handle multi-element targets.

## When `instanceof` is on a generic class

```php
function f(mixed $x): void {
    if ($x instanceof Iterator) {
        // $x is Iterator<mixed, mixed> here (the parameter's upper bounds)
    }
}
```

The analyser constructs the target as `Iterator<mixed, mixed>` (or with whatever bounds the codebase declares). The narrowing handles this through the world's parameter declaration: each parameter is filled with its upper bound, and the `from_template_default` flag is set so the variance check is permissive.

```rust,ignore
let target = TypeBuilder::new().push(
    ElementId::named_object_with_args(
        atom("Iterator"),
        &[suffete::prelude::TYPE_MIXED, suffete::prelude::TYPE_MIXED],
    )
).build();

let branches = narrow_instanceof(input, target, &world);
// branches.in_branch == Iterator<mixed, mixed>
// (with the from_template_default flag set on the args)
```

## When the input has no overlap with the target

```php
function f(string $x): void {
    if ($x instanceof Foo) {
        // unreachable; $x is `never` here
    }
}
```

```rust,ignore
let input = TypeBuilder::new().push(suffete::prelude::STRING).build();
let target = TypeBuilder::new().push(ElementId::named_object(atom("Foo"))).build();
let branches = narrow_instanceof(input, target, &world);
// branches.in_branch == TYPE_NEVER
// branches.else_branch == string
```

The analyser sees `in_branch == TYPE_NEVER` and can surface a diagnostic ("this branch is unreachable").

## When the input fully refines the target

```php
function f(Foo $x): void {
    if ($x instanceof Foo) {
        // always true; no narrowing needed
    } else {
        // unreachable; $x is `never` here
    }
}
```

```rust,ignore
let input = TypeBuilder::new().push(foo).build();
let target = TypeBuilder::new().push(foo).build();
let branches = narrow_instanceof(input, target, &world);
// branches.in_branch == Foo (unchanged)
// branches.else_branch == TYPE_NEVER
```

The analyser can warn ("this `instanceof` is always true") or use the result silently.

## Negation pattern

```php
function f(Foo|Bar|null $x): void {
    if (!($x instanceof Foo)) {
        // $x is Bar|null here
    } else {
        // $x is Foo here
    }
}
```

The analyser computes the same `Branches` and swaps which branch corresponds to which:

```rust,ignore
let branches = narrow_instanceof(input, target, &world);
// In the !instanceof branch (the if-body): branches.else_branch
// In the else branch: branches.in_branch
```

## Combining with other narrowings

The analyser can chain narrowings as control flow proceeds:

```php
function f(Foo|Bar|null $x): void {
    if ($x === null) { return; }     // narrow: subtract null
    if ($x instanceof Foo) { ... }   // narrow: meet with Foo
}
```

After the `null` check, the analyser narrows `$x` to `Foo|Bar`. Inside the `instanceof Foo` branch, narrows again to `Foo`. Each step is a standalone `narrow` or `subtract` call.

## Performance

`narrow` and `subtract` are both lattice operations. Their cost is bounded by the input sizes; for typical analyser inputs (small unions, no fan-out), each call is sub-microsecond.

The analyser typically runs many narrowings during a single function body's analysis. Reuse the `LatticeOptions` and `LatticeReport` if possible; both are cheap to construct, but reusing avoids the `Default` cost in tight loops.

> **See also:** [narrow](../lattice/narrow.md), [subtract](../lattice/subtract.md), [meet](../lattice/meet.md) for the underlying operations; [predicates::is_never](../api/predicates.md) for detecting unreachable branches in the result.
