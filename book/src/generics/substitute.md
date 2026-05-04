# Substitution

Substitution replaces a template parameter with a concrete type. PHP-side: when the user writes `Box<int>` and the analyser instantiates `Box`'s body, every `T` becomes `int`.

## Capture-free

Substitution is **capture-free**: it walks the type tree and applies the substitution only to free parameters that match the substitution's keys, identified by `(name, defining_entity)`. A parameter declared on a different entity but sharing the same name is *not* substituted.

In practice:

- A template parameter with defining entity `Box` and a substitution targeting `Box::T` is substituted.
- A template parameter with defining entity `Foo` is not substituted by the same substitution, even if it is also called `T`.

This keeps substitution semantically clean even when types nest other generic uses.

## How substitution walks the tree

Substitution is a structural transform. It recurses into every nested type carrier:

- An object's type arguments.
- A list's element type and known elements.
- A keyed array's key parameter, value parameter, and known items.
- An iterable's key and value types.
- An object shape's known properties.
- A callable's parameter types, return type, and throws.
- A class-like-string's constraint.
- A template parameter's *constraint* (the parameter Element itself is the substitution target).
- An alias or reference's type arguments.
- A conditional's subject, target, then, and otherwise branches.
- A derived type's nested types.
- A negation's inner.
- An intersection's head and conjuncts.

At each leaf (every template-parameter Element), the substitution table is consulted. If the table has a binding, the type is substituted in place; otherwise the parameter is kept.

## Substitution and unions

A substitution may replace one parameter with a *union* type. The walker handles this correctly: substituting `T := int|string` into the type `T|null` produces `int|string|null`, with the substitution flat-merged into the parent union (the lattice's join is run to collapse).

## Substitution into nested generic uses

When the type contains other generic uses (e.g. `class-string<T>` or `Box<T>`), substitution walks into them and replaces the substituted parameter wherever it appears. So substituting `T := int` into `Box<T> | class-string<T> | T` produces `Box<int> | class-string<int> | int`.

## Identity short-circuit

If the substitution produces no change (no bound parameter is found in the input), substitution returns the *original* type ; no re-interning, no allocation. This is the common case in the analyser when a callsite happens not to bind any parameter the substitution targets.

The walker maintains this guarantee even through deep recursion: as long as no leaf changes, the parents propagate the original handles up.

## Multi-step substitution

A single substitution call can carry bindings for several parameters at once, keyed by `(name, defining_entity)`. The walker visits each parameter once and looks up its binding in the table.

## A worked example

```php
/**
 * @template T
 * @template V
 */
class Map {
    /** @var array<T, V> */
    public array $entries = [];

    /**
     * @param T $key
     * @return V
     */
    public function get($key) { /* ... */ }
}
```

When the analyser instantiates `Map<string, int>` and asks for the concrete type of `$entries`:

1. The declared field type is `array<T, V>`.
2. The analyser builds a substitution: `Map::T := string`, `Map::V := int`.
3. Substitution walks `array<T, V>`, finds the two parameter Elements, replaces them.
4. The result is `array<string, int>`.

A different class with its own `T` would *not* be touched by this substitution, because the defining entity differs.

> **See also:** [Templates in depth](./templates.md) for the parameter Element kind; [standin](./standin.md) for the inverse direction (collecting bounds from arguments); [specialise](./specialise.md) for inheritance binding.
