# Inheritance specialisation

When `StringList extends ArrayList<string>` is checked against `Iterator<string>`, the lattice has to compute *what `StringList` supplies to `Iterator`'s template parameter*. The chain — `StringList → ArrayList<string> → Iterator<T>` (where `ArrayList implements Iterator`) — has to be unwound, and `T` has to be resolved through every step.

This is **inheritance specialisation**, written formally as $\mathit{specialise}(C, T, D\langle\bar\rho\rangle)$: given that descendant `D` is instantiated with arguments $\bar\rho$, what is the type of ancestor `C`'s template parameter `T`?

## What the world supplies

The lattice cannot resolve specialisation alone ; it requires *codebase knowledge* of which classes extend which, with what type arguments. The world answers, for any (descendant, ancestor, position), the type the descendant supplies to that parameter, expressed in the descendant's own template namespace.

When the lattice walks `D <: C<\bar\rho>`, it asks the world for each position of `C`'s parameters and substitutes through.

## What "in the descendant's namespace" means

The world's answer may itself contain template-parameter Elements that refer to the descendant's parameters. For example:

```php
/**
 * @template X
 * @implements Iterator<X>
 */
class Bag implements Iterator { /* ... */ }
```

`Bag` says: "I implement `Iterator<X>`, where `X` is *my* parameter." The world's answer for `Bag`'s contribution to `Iterator`'s position 0 is a template-parameter Element for `Bag::X`.

When the lattice is checking `Bag<int> <: Iterator<int>`:

1. Ask the world for `Bag`'s contribution to `Iterator`'s position 0 ; the answer is `Bag::X`.
2. Substitute `Bag`'s actual arguments through the answer: `Bag::X := int` → result is `int`.
3. Compare with the container's `int`, with `Iterator`'s declared variance.

The substitution in step 2 is a [substitute](./substitute.md) call with the binding `Bag::X := int`.

## A multi-step chain

```php
/**
 * @template T
 */
interface Iterator { /* ... */ }

/**
 * @template T
 * @implements Iterator<T>
 */
class ArrayIterator implements Iterator { /* ... */ }

/**
 * @template U
 * @extends ArrayIterator<U>
 */
class TypedList extends ArrayIterator { /* ... */ }
```

For `TypedList<string> <: Iterator<string>`:

1. Walk: `TypedList → Iterator` is not direct.
2. The world records: `TypedList extends ArrayIterator<U>`. So `TypedList`'s contribution to `ArrayIterator`'s position 0 is `TypedList::U`.
3. The world records: `ArrayIterator implements Iterator<T>`. So `ArrayIterator`'s contribution to `Iterator`'s position 0 is `ArrayIterator::T`.
4. Compose: `TypedList`'s contribution to `Iterator`'s position 0 is the composed result, `TypedList::U`. (The world walks the chain on the analyser's behalf.)
5. The lattice substitutes `TypedList`'s actual arguments: `TypedList::U := string`.
6. Result: `string`.
7. Compare with the container's `Iterator<string>` ; `string $\equiv$ string` (or covariant), passes.

## Variance through the chain

Variance is per-parameter on each ancestor. The lattice consults the variance of `Iterator`'s `T` (covariant), not `TypedList`'s `U` (which has its own variance, declared independently).

This is correct: when checking `TypedList<X> <: Iterator<Y>`, the question is whether `Iterator` accepts the supplied parameter at the supplied variance, not whether `TypedList`'s parameter is somehow compatible with `Iterator`'s.

## The same-class case

When the input and container are the *same class*, no chain walk is needed:

```
Box<int> <: Box<numeric>
```

The lattice asks `Box`'s parameter variance and compares positionally. With `T` covariant: `int <: numeric` ✓. With `T` invariant: `int $\equiv$ numeric` is false (numeric admits float, int does not), the refines fails.

## Resolving arity differences

If an ancestor declares more parameters than the descendant, the descendant must supply values for all of them. If an ancestor declares fewer, the descendant cannot supply more.

The world handles this in its inheritance-mapping implementation; the lattice does not enforce arity here.

## When the world has no mapping

If the world has no mapping for a given (descendant, ancestor, position), the lattice falls back to using the parameter's upper bound (or `mixed`) for that position.

This is the conservative answer: the analyser couldn't prove the inheritance, so don't enforce a tight refinement.

## A worked example

```php
/**
 * @template T
 * @template-covariant V
 */
interface Map {
    /**
     * @param T $key
     * @return V
     */
    public function get($key);
}

/**
 * @template W
 * @implements Map<string, W>
 */
class StringMap implements Map { /* ... */ }
```

For `StringMap<int> <: Map<string, int>`:

1. Input class `StringMap` is not the same as container class `Map`.
2. The world says `StringMap` descends from `Map`.
3. The world supplies `StringMap`'s contribution to `Map`'s position 0: `string` (from `@implements Map<string, W>`, position 0).
4. The world supplies `StringMap`'s contribution to `Map`'s position 1: `StringMap::W` (the `W` from the `@implements` clause).
5. The lattice substitutes `StringMap`'s actual args: `StringMap::W := int`. The position-1 result becomes `int`.
6. Compare positions:
   - Position 0: `string $\equiv$ string` (Map's T is invariant; both pass).
   - Position 1: `int <: int` (Map's V is covariant; passes).
7. Result: `StringMap<int> <: Map<string, int>`. ✓

## Why specialisation lives outside the lattice

Specialisation requires codebase knowledge: which classes extend which, with what type arguments. The lattice itself is codebase-agnostic; it asks the world, which is the analyser's responsibility. Specialisation is the *protocol* between the lattice and the world for the inheritance case.

This separation is what lets the lattice's correctness be checkable in isolation: the inheritance mapping is a black-box function the world supplies, and the lattice's behaviour is correct *given* the world's answers. The world's answers themselves are the analyser's correctness concern.

> **See also:** [Templates in depth](./templates.md) for the parameter Element kind; [variance](./variance.md) for the rules each position is checked under; [substitute](./substitute.md) for the substitution applied during specialisation.
