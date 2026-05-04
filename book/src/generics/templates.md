# Template parameters in depth

The [universe chapter on templates](../universe/templates.md) covered the template-parameter Element kind. This chapter covers what *happens* with template parameters: how a class's parameters are declared, what the analyser must register about them, and how the lattice consults that information.

## Declaration

A class with template parameters is declared in PHP via PHPDoc:

```php
/**
 * @template T
 * @template-covariant V
 * @template K of array-key
 * @template U of Iterator = ArrayIterator
 */
class Box {
    // ...
}
```

The analyser parses each `@template` line and registers, for each parameter:

- A **name** (`T`, `V`, `K`, `U`).
- A **defining entity** (the class `Box`).
- An **upper bound** (the `of X` clause; defaults to `mixed`).
- A **variance** (covariant, contravariant, invariant; defaults to invariant unless declared otherwise).
- An optional **default** (the `= X` clause; used when the user supplies fewer arguments than declared).

Suffete itself does not store this information. The analyser registers it with its world implementation, and the lattice queries the world when it needs a parameter's variance, upper bound, or default.

## What the world supplies

The world tells the lattice three things about a class's template parameters:

- The **arity** (how many `@template` lines the class declares).
- For each position, the parameter's variance, upper bound, and default.
- For each (descendant, ancestor, position), the type the descendant supplies to the ancestor's parameter at that position.

The third one is what makes inheritance work: see [specialise](./specialise.md).

## Instantiation

A use-site instantiation `Box<int, string>` is a named-object Element that carries its type arguments in declaration order. The first argument fills `T`, the second fills `V`. If the class declares more parameters than supplied, the missing ones are filled from the upper bound (or the declared default) and the type is flagged as having received a template default ; the lattice tolerates the default at variance check time.

## Inheritance and parameter mapping

When `class Bag<X> extends Box<X, int>`:

- `Bag`'s `X` corresponds to `Box`'s `T`.
- `Box`'s `V` is bound to `int` from `Bag`'s perspective.

The lattice uses this when checking `Bag<string>` refines `Box<string, int>`: it asks the world what `Bag` supplies to `Box`'s parameters, substitutes `Bag`'s actual arguments through, and compares positionally with `Box`'s declared variance.

The full algorithm is in [specialise](./specialise.md).

## Defining entities

Every template parameter is keyed by `(name, defining_entity)`. Two parameters with the same name on the same class are the same parameter ; two with the same name on different classes are different parameters. The defining entity can be:

- A class-like (the parameter is declared on a class, interface, trait, or enum).
- A function or method.
- A closure (analyser-assigned identity).

Capture-free [substitution](./substitute.md) uses the defining entity to know which parameters a substitution applies to.

## Free vs bound vs partially-applied

A template-parameter Element is *free* until the analyser substitutes it. Three states:

- **Free.** The parameter appears in the type with no commitment to a value. `Box<T>::value` is `T` (free). The lattice can answer questions about `T` using its constraint as an upper bound.
- **Bound.** The parameter has been substituted. After `Box<int>::value`, `T := int`, the field type is `int`, no `T` Element remains.
- **Partially applied / default-filled.** The user wrote `Box` instead of `Box<int>`. The analyser fills `T` with the upper bound (`mixed` by default), flags the type as carrying a template default, and the lattice tolerates the default at variance check time (recording a coercion cause).

## A worked example

```php
/**
 * @template T
 * @template-covariant V
 */
class Map {
    /** @var array<T, V> */
    public array $entries = [];

    /**
     * @param T $key
     * @return V
     */
    public function get($key) { return $this->entries[$key]; }
}

/**
 * @extends Map<string, mixed>
 */
class StringMap extends Map {
    /** @var array<string, mixed> */
    public array $entries = [];
}
```

Inside `Map`'s body, the field `$entries` has type `array<T, V>`, where `T` and `V` are template-parameter Elements with defining entity `Map`.

Inside `StringMap`'s body, the field `$entries` has type `array<string, mixed>` ; fully concrete because `StringMap` extends `Map<string, mixed>`.

When the analyser checks `StringMap` refines `Map<string, mixed>`:

1. The lattice gets the container's parameters: `Map`'s `T` (invariant by default) and `V` (covariant, declared with `@template-covariant`).
2. It asks the world what `StringMap` supplies to `Map`'s position 0 ; the answer is `string`.
3. It asks the same for position 1 ; the answer is `mixed`.
4. Compare position 0 with invariant: `string` is equivalent to `string`. ✓
5. Compare position 1 with covariant: `mixed` refines `mixed`. ✓
6. Result: `StringMap` refines `Map<string, mixed>`. ✓

The variance, the inheritance binding, and the per-position check are all driven by the world. Suffete itself orchestrates the dispatch.

> **See also:** [variance](./variance.md) for the per-variance refinement rules; [substitute](./substitute.md) for how `T` is replaced with a concrete type; [standin](./standin.md) for inferring `T` from call-site arguments; [specialise](./specialise.md) for the inheritance-binding resolution.
