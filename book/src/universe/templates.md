# Templates and generic parameters

A *template parameter* is a placeholder in a generic declaration: `T` on a class `Box`, `K`/`V` on a class `Map`, `T` on a function `id`. PHP has no native generics syntax; they are introduced by `@template T` (or `@template-covariant`, `@template-contravariant`) in a PHPDoc block.

This chapter covers what a free template parameter *is* in the type universe. The operations that act on it — substitution, inference, specialisation — are in [Part IV](../generics/templates.md).

## What a template parameter carries

A template parameter has four pieces of information:

- A **name** (`T`, `K`, `V`, ...) — the user-visible identifier.
- A **defining entity** — the class-like, function, method, or closure that declared it. Two parameters with the same name on different entities are *different* parameters: `T` on `class Box` is distinct from `T` on `class Foo`.
- An **upper bound** (the constraint) — PHP-side, the `T extends Foo` clause. Defaults to `mixed`.
- An **optional qualifier** — used at the use site for specific forms like `T::class`.

The name plus the defining entity together identify the parameter. Substitution, inference, and specialisation all key on this pair.

## Subtyping

A free template parameter behaves like its constraint for subtype questions:

- $T \mathrel{<:} \tau$ iff $\mathit{constraint}(T) \mathrel{<:} \tau$.
- $\tau \mathrel{<:} T$ iff $\tau \mathrel{<:} \mathit{constraint}(T)$ ; with caveats (the variance-aware rules in [refines](../lattice/refines.md)).

Two parameters that share the same `(name, defining_entity)` are the same thing. Two with different defining entities are distinct even when their names match.

## How free parameters appear in types

Inside the body of a generic class, a free `T` shows up wherever it is used:

```php
/**
 * @template T
 */
class Box {
    /** @var T */
    public mixed $value;

    /** @return T */
    public function get() { return $this->value; }

    /** @param T $value */
    public function set($value): void { $this->value = $value; }
}
```

The field `$value`, the return of `get`, and the parameter of `set` all reference the same `T` (same name, same defining entity).

When the user instantiates `Box<int>` (in a PHPDoc context such as `@var Box<int>`), the analyser substitutes every `T` for `(name="T", defining_entity=Box)` with `int`, producing the concrete versions of the field, getter, and setter types.

## Free vs bound

A template parameter is *free* until it is substituted. After substitution, the parameter is gone — replaced with the concrete type.

A type that contains a free template is not "wrong" — it just has not been instantiated yet. The lattice still answers questions about it (using the constraint as the upper bound), but the answers are conservative: $T \mathrel{<:} \tau$ holds iff $\tau$ accepts every value the constraint admits.

## Variance

The variance of a parameter is *declared* at the class level, not on the parameter itself. The analyser registers each class's parameter list with the codebase model; the lattice asks for the declared variance when it needs it during a refines query on instantiated classes.

Variance is one of:

- **Covariant** ; subtype of `T` produces subtype of `Box<T>`.
- **Contravariant** ; supertype of `T` produces subtype of `Box<T>`.
- **Invariant** ; only the same `T` produces a refinement.

See the [variance](../generics/variance.md) chapter for the full rules.

## Default-filled parameters

When a class declares a `T` parameter and the user references the class as plain `Box` (rather than `Box<int>`), the analyser fills `T` with the constraint (`mixed` by default) and marks the resulting type as **default-filled**. The marker rides along with the type wherever it is later nested.

The variance check at refinement time consults the marker: a default-filled type-argument is allowed to flow either direction (subject to variance) without producing a strict refinement failure ; the lattice records the use of the default on its report so the analyser can warn about the unpinned position.

## A worked example

```php
/**
 * @template T
 */
class Box {
    /** @var T */
    public mixed $value;
}

/** @var Box<int> $ints */
$ints = new Box();
```

Inside the class body, the field `$value` has type `T` (free).

After the analyser sees the `@var Box<int>` annotation, it builds the instantiated `Box`. It walks the class's stored field types and substitutes each `T` with `int`, producing the concrete field type for *this instance*.

The `T` parameter itself is not modified ; substitution is a pure function returning new types.

> **See also:** [Substitute](../generics/substitute.md) for the substitution operation; [Variance](../generics/variance.md) for how variance is declared and used; [Standin](../generics/standin.md) for the inference round that binds free parameters.
