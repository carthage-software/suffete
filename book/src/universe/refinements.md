# Refinement axes

A *refinement axis* is a boolean (or low-cardinality) property carried directly on a type, not as a separate type. The lattice treats axes as bits on the same form: setting an axis tightens the type without changing what it is.

Axes are how suffete avoids exploding the universe. There is no separate "non-null mixed" form distinct from `mixed` ; `mixed` carries a `non-null` axis. There is no separate "non-empty string" form distinct from `string` ; `string` carries a `non-empty` axis. The lattice rules work over axes by checking that every axis the container constrains is implied by the input.

## The general rule

Given a form $F$ that carries an axis $a$, and a container of form $F$ with $a$ set:

- An input of the same form $F$ refines the container iff its $a$ is also set.
- An input of a different form $F'$ refines the container iff $F'$ structurally guarantees $a$.

The "structurally guarantees" half does the work. `int` structurally guarantees non-null (every integer is not null), so `int` refines `non-null mixed` even though `int` is not `mixed`.

## Axes on `mixed`

Four axes:

| Axis | PHP-side | Refinement |
|---|---|---|
| non-null | `non-null mixed` | The value is not `null`. |
| truthy | `truthy mixed` | The value is truthy at runtime. |
| falsy | `falsy mixed` | The value is falsy at runtime. |
| empty | analyser-internal | The value is `falsy` (paired with the `falsy` truthiness). |
| isset-from-loop | analyser-internal | The value flowed through a loop body. |

Combinations are valid. A `truthy` value cannot be `null` (PHP), so `truthy mixed` already implies `non-null`.

### Structural implications

What other types satisfy each axis:

- **non-null** is implied by every form except `null`, `void`, and `mixed` without the non-null axis. So `int`, `string`, `Foo`, `array`, `list<int>`, `resource`, `callable` all refine `non-null mixed`.
- **truthy** is implied by:
  - `true`
  - any object (objects are truthy in PHP)
  - any resource
  - non-zero integer literals; integer ranges that exclude 0
  - non-zero float literals
  - string literals that are not `""` or `"0"`
  - `truthy-string` and `non-empty-string` (with the truthy axis explicitly set)
  - `non-empty-array`, `non-empty-list`
- **falsy** is implied by:
  - `false`, `null`, `void`
  - `0`, `0.0` (and `-0.0`)
  - `""` and `"0"`
  - the empty array, the empty list
- **empty** is the same as **falsy** for refinement purposes.
- **isset-from-loop** propagates only on `mixed` itself; no other form structurally implies it.

## Axes on `string`

Five axes:

| Axis | PHP-side | Refinement |
|---|---|---|
| non-empty | `non-empty-string` | Length ≥ 1. |
| truthy | `truthy-string` | Truthy at runtime (excludes `""` and `"0"`). |
| lowercase | `lowercase-string` | Every character is lowercase. |
| uppercase | `uppercase-string` | Every character is uppercase. |
| numeric | `numeric-string` | Parses as int or float. |

Combinations are valid: `non-empty-lowercase-string` carries both axes.

Refinement direction: more axes = stricter. `non-empty-lowercase-string` refines `non-empty-string`, `lowercase-string`, and `string`. The same rule as for `mixed`: every axis the container constrains must be carried by the input.

### Structural implications

A string literal carries every axis it satisfies:

- non-empty iff the literal is not `""`.
- truthy iff the literal is not `""` and not `"0"`.
- lowercase iff every character is lowercase.
- uppercase iff every character is uppercase.
- numeric iff the literal parses as an int or float.

## Axes on resources

`resource` carries two refinements:

- **state** — `Open`, `Closed`, or unspecified.
- **kind** — an optional named kind (`curl`, `gd image`, ...).

Subtyping is the conjunction: state must match (or be unspecified on the container) and kind must match (or be unspecified on the container). See [resources](./resources.md).

## Axes on objects

A named class carries three modality flags:

- **`$this`** — the object is exactly the receiver.
- **`static`** — the object is the late-static class.
- A class can be analyser-known **`final`**; the universe does not flag this on the type itself but the lattice consults the analyser at refinement time.

`$this` implies `static`. The modality check at refinement time:

- A container marked `$this` accepts only `$this` inputs.
- A container marked `static` accepts `static` or `$this` inputs.
- A plain `Foo` accepts any of the above.

A structural object shape carries `sealed` as its main axis (see [objects](./objects.md)).

## Axes on arrays and lists

Both kinds carry:

- **non-empty** — at least one entry.
- **sealed** — the shape commits to having no entries beyond those listed.

Both follow the standard "more axes = stricter" rule.

## Why axes, not wrappers

A natural alternative would be to wrap a type in a refining wrapper: an outer "non-null" wrapping a `mixed`. Suffete uses axes for two reasons:

1. **No rule duplication.** With axes attached to the type, the rule for "does this axis hold?" lives in one place. With wrappers, every other type would need to know how to interact with the wrapper.
2. **Compactness.** A `non-null truthy lowercase string` is one type carrying three bits, not a chain of three nested wrappings.

The trade-off is that the forms carrying axes have more elaborate rules. Those rules are tested against the algebraic-law battery ([laws](../lattice/laws.md)) so the elaboration does not introduce soundness drift.

## A worked example

The PHP type `truthy lowercase non-empty-string` carries three string axes: non-empty, truthy, and lowercase.

Refinement:

- The literal `"foo"` refines it ; `"foo"` is non-empty, truthy, and lowercase.
- `"FOO"` does not — fails lowercase.
- `""` does not — fails non-empty and truthy.
- `"0"` does not — fails truthy.
- `non-empty-string` (only one axis) does not — missing lowercase and truthy.
- `lowercase-string` (only one axis) does not — missing non-empty and truthy.

> **See also:** [scalars](./scalars.md) for the string family in full; [special elements](./special.md) for the mixed axes; [refines](../lattice/refines.md) for the per-axis lattice rules.
