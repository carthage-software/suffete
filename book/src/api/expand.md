# Expansion: resolving unresolved elements

The `expand` module resolves the [unresolved Element kinds](../universe/unresolved.md) — `Alias`, `Reference`, `MemberReference`, `GlobalReference`, `Conditional`, `Derived`, `Variable` — into structural types the lattice can reason about.

```rust,ignore
use suffete::expand;

let resolved: TypeId = expand::expand(input, &world, &template_env);
```

The contract is one direction: **the analyser must call `expand` on a type before passing it to the lattice** if the type might contain any unresolved Element. The lattice does not invoke expansion itself; the recursion would loop, and the analyser knows when expansion is safe in a way the lattice cannot.

## Why expansion is the analyser's job

Resolving an `Alias` requires looking the alias name up in the analyser's alias table. Resolving a `Conditional` requires evaluating the subject vs target subtype check, which requires the world. Resolving a `Derived` requires walking into the target type and applying the transformation.

All of these are traversals the analyser may want to control: cache the result, abort on a cycle, expand only some kinds (e.g. expand aliases but leave conditionals lazy), produce diagnostics on unresolved names. The lattice would have one fixed strategy; the analyser benefits from having the choice.

## What `expand` does

`expand(ty, world, template_env)` walks the type tree and replaces every unresolved Element with its structural form, recursing into nested types. The high-level rules:

- **`Alias { name }`** — resolve via `world.resolve_alias(name)`. If the alias is itself an alias chain, follow until structural.
- **`Reference { name, type_args, intersections }`** — resolve `name` to a class-like or a template parameter via the world's symbol table. Substitute `type_args` if applicable.
- **`MemberReference { class, name }`** — resolve the member name on the class via `world.class_constant_type` (or similar member-type query).
- **`GlobalReference { name }`** — resolve via the analyser's global type variable table.
- **`Conditional { subject, target, then, otherwise }`** — under `template_env`, check `subject <: target`. Return `then` if yes, `otherwise` if no.
- **`Derived(...)`** — apply the per-variant transformation.
- **`Variable { id }`** — look up `id` in the analyser's inference state.

The `template_env` argument is the current substitution environment — bindings from template parameters to types. Used by `Conditional` (to substitute `subject` and `target` before the check) and by `Derived` variants that reference templates.

## Worked example: `Alias`

```php
/** @type UserId = positive-int */

function find(UserId $id): User { ... }
```

The parameter type is parsed as `Alias { name: "UserId" }`. Before the analyser checks `find(7)`, it expands:

```rust,ignore
let alias_t = ...; // contains Alias { name: "UserId" }
let resolved = expand::expand(alias_t, &world, &template_env);
// resolved == int<1, ∞>  (the underlying type of UserId)
```

The world's `resolve_alias("UserId")` returns the underlying type; expansion substitutes it.

## Worked example: `Conditional`

```php
/**
 * @template T
 * @return ($T extends int ? string : bool)
 */
function classify(): mixed { ... }
```

The return type is `Conditional { subject: T, target: int, then: string, otherwise: bool }`. After the call site has bound `T := int(7)`:

```rust,ignore
let template_env = ...; // T → int(7)
let cond_t = ...; // Conditional element wrapped in a TypeId
let resolved = expand::expand(cond_t, &world, &template_env);
// Step 1: substitute template_env into the conditional.
// subject (T) becomes int(7); target stays int.
// Step 2: check int(7) <: int → true.
// Step 3: take the `then` branch.
// resolved == string
```

If `T` were bound to `bool` instead, the `target` check would fail and the `otherwise` branch (`bool`) would be taken.

## Worked example: `Derived` (KeyOf)

```php
/** @type Shape = array{a: int, b: string} */
/** @type Keys = key-of<Shape> */
```

The type `Keys` is `Derived(KeyOf(Alias("Shape")))`. Expansion:

1. Resolve the inner `Alias("Shape")` → `array{a: int, b: string}`.
2. Apply `KeyOf` to the result: extract the keys → `'a' | 'b'`.

The `Derived(KeyOf)` variant has a per-kind transformation. For keyed arrays, it returns the union of literal-string keys (or `array-key` if the array is unsealed). For lists, it returns `int` (or a range of valid indices). For objects, it returns the property names.

## Recursion and termination

Aliases can be chains: `A = B`, `B = C`, `C = int`. Expansion follows the chain. Suffete does not detect cycles directly ; the world's `resolve_alias` is expected to return `None` for unknown names, which terminates the chain naturally.

If the analyser needs cycle detection, it should detect cycles in its alias table during ingestion and report a diagnostic before suffete sees the type.

## Partial expansion

Sometimes the analyser wants to expand *some* kinds but not others ; expand aliases but defer conditionals until enough template bindings exist, for example. The crate offers per-kind expansion helpers in addition to the catch-all `expand::expand`:

```rust,ignore
let alias_only = expand::expand_aliases(input, &world);
let conditionals_only = expand::expand_conditionals(input, &world, &template_env);
```

The exact list of partial expanders is in `src/expand/`. The catch-all is the most common entry point.

## Cost

Expansion is O(tree size) for the type, plus the per-kind costs:

- `Alias` and `Reference` resolution: one world query per kind.
- `Conditional` resolution: one `refines` call (lattice cost) plus the substitution.
- `Derived` resolution: variant-specific, but bounded by the input's tree size.

The most expensive cases are `Conditional` chains where each branch is itself a `Conditional` with a non-trivial subject ; the lattice is invoked recursively. Most analyser-level types resolve in microseconds.

## Idempotence

`expand` is idempotent: `expand(expand(t)) == expand(t)` (assuming no world changes between calls). The analyser can call expand without worrying about double expansion.

## Detecting whether expansion is needed

The [predicates](./predicates.md) chapter exposes:

```rust,ignore
use suffete::predicates::is_fully_resolved;

if !is_fully_resolved(ty) {
    ty = expand::expand(ty, &world, &template_env);
}
```

This is the recommended pattern for analyser code that consumes types from the type-source layer (the parser, the codebase model, the docblock interpreter) and feeds them to the lattice.

## A worked example: full pipeline

```rust,ignore
use suffete::{TypeId, expand, lattice::{self, LatticeOptions, LatticeReport}};
use suffete::predicates::is_fully_resolved;

fn analyser_check<W: suffete::world::World>(
    input: TypeId,
    expected: TypeId,
    world: &W,
    template_env: &TemplateEnv,
) -> bool {
    let input    = if is_fully_resolved(input)    { input    } else { expand::expand(input, world, template_env) };
    let expected = if is_fully_resolved(expected) { expected } else { expand::expand(expected, world, template_env) };

    let opts = LatticeOptions::default();
    let mut report = LatticeReport::new();

    lattice::refines(input, expected, world, opts, &mut report)
}
```

The pattern: expand each side if needed, then call the lattice. Trivial wrapper, but the right interface to enforce on every analyser-side query.

> **See also:** [Unresolved elements](../universe/unresolved.md) for the kinds expansion handles; [Predicates](./predicates.md) for `is_fully_resolved`; [World](./world.md) for the resolution methods used during expansion; [Conditional and Derived](../universe/unresolved.md) for the per-variant rules.
