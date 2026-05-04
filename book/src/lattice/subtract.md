# Set difference: subtract

The subtract $\tau \setminus \sigma$ removes from $\tau$ the values that are in $\sigma$. PHP-side: the type after a negative narrowing — `$x` of type `int|string` after `is_int($x)` returns `false` becomes `string`.

## What "set difference" means

Subtract is the type whose value-set is $\tau \setminus \sigma$, expressed as concretely as the kind system allows.

Examples:

| $\tau$ | $\sigma$ | $\tau \setminus \sigma$ |
|---|---|---|
| `int\|string` | `int` | `string` |
| `int\|string\|null` | `null` | `int\|string` |
| `int<0,10>` | `int<5,15>` | `int<0,4>` |
| `bool` | `true` | `false` |
| `mixed` | `null` | `non-null mixed` |
| `Foo\|Bar` | `Foo` | `Bar` |
| `non-empty-list<int>` | `list{}` | `non-empty-list<int>` (no overlap to remove) |
| `int` | `int` | `never` |

## How subtract is computed

For each Element on the left, attempt to subtract every Element on the right; collect the results.

Per-pair subtract may *split* a single Element into multiple pieces:

- `int<0,10> \ int(5)` produces `int<0,4> | int<6,10>` (range punching).
- `bool \ true` produces `false`.
- `mixed \ null` produces `non-null mixed`.

## Element-level dispatch

For a per-pair subtract $a \setminus b$:

1. **Reflexivity** — $a \setminus a = \emptyset$ (the input is fully removed).
2. **`never` on the right** — $a \setminus \bot = a$ (nothing to remove).
3. **`never` on the left** — $\bot \setminus b = \bot$.
4. **No overlap** — if $a$ and $b$ are disjoint, $a \setminus b = a$.
5. **Subsumption** — if $a \mathrel{<:} b$, then $a \setminus b = \emptyset$ (everything in $a$ is removed).
6. **Family-specific subtract rules** — last resort.

The early exit on disjoint pairs is essential for performance: most subtract calls in an analyser are negative narrowings on small types where most pairs are disjoint.

## True-union dominator subtract

The `scalar`, `numeric`, and `array-key` Elements are *true unions* (disjoint covers). When the right-hand side lands inside one of their members, the left-hand side decomposes:

```
scalar      = bool | int | float | string
numeric     = int | float | numeric-string
array-key   = int | string

scalar \ int       → bool | float | string
numeric \ float    → int | numeric-string
array-key \ string → int
```

## Family-specific subtract rules

### Int

- `int<0,10> \ int(5) = int<0,4> | int<6,10>` ; range punching.
- `int<0,10> \ int<5,15> = int<0,4>` ; the overlap is removed.
- `int \ literal-int = int` (the overlap is one literal among infinitely many; the unspecified set does not measurably shrink).
- `int \ int = never`.

### Float

- `float \ float-literal = float` (no special punching for floats; one literal does not measurably change the unspecified set).
- `float-literal-x \ float-literal-x = never`.

### String

- `string \ literal "foo" = string` (one literal does not change the unspecified set).
- `non-empty-string \ "" = non-empty-string` (no overlap; no change).
- `string \ non-empty-string = literal ""` (only the empty string remains).
- `string \ string = never`.

### Bool, true, false

- `bool \ true = false`.
- `bool \ false = true`.
- `bool \ bool = never`.
- `true \ false = true` (no overlap).

### Object

- `Foo \ Foo = never`.
- `Foo \ Bar = Foo` (no overlap if neither extends the other; world-aware).
- `Foo \ Bar = never` if `Foo` extends `Bar` (every Foo is a Bar; remove all Foo).
- `Foo \ Bar = Foo` if `Bar` extends `Foo` (some Foo is a Bar, but not all; we cannot easily express the difference, so conservatively keep all of Foo).

### List, Array

- Sealed list / array subtract is shape-by-shape; rarely splits, mostly all-or-nothing.
- Generic list / array subtract is per-parameter.
- A sealed list minus the empty list returns the original (no overlap; sealed non-empty has no element to remove).

### Mixed

- `mixed \ null = non-null mixed` ; the canonical case for nullable narrowing.
- `mixed \ false = non-falsy mixed` (truthy mixed | null).
- `mixed \ truthy = falsy mixed`.

## Why subtract may produce many Elements

A single Element minus another may produce zero, one, or many Elements:

- **Zero**: full removal (e.g. `int(5) \ int(5)`).
- **One**: most cases (the input is unchanged or replaced with a slightly narrower form).
- **Many**: range punching (e.g. `int<0,10> \ int(5)`), or other splitting.

## A worked example

`int|string` minus `int` is `string` ; the Element-by-Element dispatch removes the `int` Element entirely and leaves `string` untouched (disjoint from `int`).

`int<0, 10>` minus `int(5)` is `int<0, 4> | int<6, 10>` ; range punching produces a two-Element result.

`bool` minus `true` is `false` ; the bool family has an exact rule.

## When subtract is conservative

PHP types are infinite in some axes (literal strings, literal ints) and finite in others (booleans, enum cases). Subtract is *exact* on the finite cases (because the kind system can express the complement) and *conservative* on the infinite cases:

- `string \ "foo" = string` (we can't express "every string except 'foo'" precisely, so we keep `string` and lose nothing soundness-wise).
- `int<0,10> \ "foo" = int<0,10>` (no overlap; nothing to remove).

## Properties

The [laws](./laws.md) chapter checks:

- **Bound**: $(\tau \setminus \sigma) \mathrel{<:} \tau$ for every $\tau, \sigma$.
- **Disjoint after**: $(\tau \setminus \sigma) \sqcap \sigma \equiv \bot$ when the subtract is exact (and is "best-effort disjoint" when it is conservative).
- **Identity**: $\tau \setminus \bot \equiv \tau$.
- **Annihilator**: $\bot \setminus \sigma \equiv \bot$.
- **Self**: $\tau \setminus \tau \equiv \bot$ (when the subtract can express the full removal).

The Bound property is the most important: subtract must never produce a *larger* type than the input. If subtract returns $T$ such that $T$ is not a subtype of $\tau$, that is a soundness violation.

> **See also:** [meet](./meet.md) for the operation that uses subtract internally for negation; [narrow](./narrow.md) for the assertion-aware operation that often boils down to subtract; [laws](./laws.md) for the algebraic checks.
