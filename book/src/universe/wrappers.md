# Wrappers: negation and intersection

Two forms *wrap* other types: they exist to compose the universe with set complement and set intersection. They are the universe's two combinators, and they account for most of the lattice's complexity ; whenever the rules need to do something nontrivial, it's usually because a wrapper is involved.

| Form | PHP-side | Denotes |
|---|---|---|
| Negation | `!T`, `!int`, `!Foo` | Set complement: every value *not* in $T$. |
| Intersection | `Foo & Bar & Baz` | Set intersection of two or more types. |

## Negation

PHP-side: `!T`. Denotes the set complement of $T$ — every value in the PHP universe *except* those in $T$.

### Subtyping

The standard rules:

- $\mathit{never} \mathrel{<:} \neg \tau$ for every $\tau$ ; never is in every set's complement.
- $\tau \mathrel{<:} \neg \sigma$ iff $\tau \sqcap \sigma \equiv \bot$ (i.e. $\tau$ and $\sigma$ are disjoint).
- $\neg \sigma \mathrel{<:} \tau$ iff $\neg \tau \mathrel{<:} \sigma$ (Boolean duality, when both inner types are well-formed for the rule).
- $\neg \neg \tau \equiv \tau$ ; double negation collapses.
- $\neg \mathit{never} \equiv \top$ (vanilla mixed); $\neg \top \equiv \mathit{never}$.

### Negation in narrowing

The most common use of negation is in [narrowing](../lattice/narrow.md). After `!is_int($x)`, the analyser narrows `$x` by the type `!int`. The lattice's `subtract` operation handles this directly.

## Intersection

PHP-side: `Foo & Bar & Baz`. Denotes the set intersection of all parts.

An intersection is represented as a **head** and a list of **conjuncts**. `Foo & Bar & Baz` has `Foo` as head and `[Bar, Baz]` as conjuncts. The head is canonicalised so that `Foo & Bar` and `Bar & Foo` are the same intersection.

### One representation, always

Every PHP intersection is expressed as the same wrapper, regardless of the head's type. `Foo & Bar`, `int & literal-int`, `array<int, V> & Countable`, `T & Foo` (a generic parameter conjoined with a class) — they all use the wrapper. There is no in-payload conjunct list on individual types.

### Subtyping

The standard rules from PL theory:

- $\tau \mathrel{<:} (H \sqcap C_1 \sqcap \dots \sqcap C_n)$ iff $\tau \mathrel{<:} H$ and $\tau \mathrel{<:} C_i$ for every $i$ (intersection on the right is a conjunction of refinements).
- $(H \sqcap C_1 \sqcap \dots \sqcap C_n) \mathrel{<:} \sigma$ iff $H \mathrel{<:} \sigma$ or some $C_i \mathrel{<:} \sigma$ (intersection on the left is a disjunction; some side must do the work).

The asymmetry — intersection on the left is a disjunction, intersection on the right is a conjunction — is the standard "Int-L / Int-R" rule.

### Uninhabited intersections

An intersection can be uninhabited — `Foo & !Foo` is empty. The construction phase detects the trivial cases:

- `H & !H` (a conjunct that negates the head) collapses to $\bot$.
- A conjunct that is lattice-disjoint with the head also collapses.

The full uninhabited check is part of [overlaps](../lattice/overlaps.md).

## A worked example

The PHP type:

```php
Stringable & Countable & !Iterator
```

is an intersection with `Stringable` as head, and `[Countable, !Iterator]` as conjuncts.

A class `Foo` refines this iff:

- $\mathit{Foo} \mathrel{<:} \mathit{Stringable}$ (the analyser's codebase confirms `Foo` implements `Stringable`).
- $\mathit{Foo} \mathrel{<:} \mathit{Countable}$ (similarly for `Countable`).
- $\mathit{Foo} \mathrel{\\#} \mathit{Iterator}$ (`Foo` does not implement `Iterator`).

The lattice asks the codebase for the first two and uses [overlaps](../lattice/overlaps.md) for the third.

> **See also:** [refines](../lattice/refines.md) for the Int-L / Int-R rules; [overlaps](../lattice/overlaps.md) for the uninhabited-detection logic; [meet](../lattice/meet.md) for how intersections are constructed during the meet operation; [subtract](../lattice/subtract.md) for how negations are introduced.
