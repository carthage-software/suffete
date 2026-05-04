# Soundness: the algebraic laws

The lattice operations form an algebra. There are identities every implementation of meet, join, subtract, narrow, refines, and overlaps must satisfy. Suffete checks them on every CI run, on a property-test battery generating thousands of distinct type triples per case.

This chapter lists the laws, explains what each one rules out, and shows the test infrastructure that runs them.

## Why laws matter

A lattice implementation that fails one of these is not just imprecise — it is *wrong*. Downstream analysers will report bugs that aren't there, miss bugs that are, or flip-flop on the same code depending on which order the analyser asks questions. Worse, the bugs are not local: a bug in meet shows up as wrong output in unrelated parts of the analyser that happened to call meet.

Suffete catches them by treating the laws as proofs that must hold *on every input*. The property tests generate random types and check the laws on them; a single failure stops the build.

## The laws

### Idempotence

| Operation | Law |
|---|---|
| meet | $\tau \sqcap \tau \equiv \tau$ |
| join | $\tau \sqcup \tau \equiv \tau$ |
| narrow | $\mathit{narrow}(\tau, \tau) \equiv \tau$ |

A type combined with itself is itself. Failure means: meet/join/narrow is producing a type that is structurally different from the input despite the inputs being equal.

### Commutativity

| Operation | Law |
|---|---|
| meet | $\tau \sqcap \sigma \equiv \sigma \sqcap \tau$ |
| join | $\tau \sqcup \sigma \equiv \sigma \sqcup \tau$ |
| overlaps | $\mathit{overlaps}(\tau, \sigma) = \mathit{overlaps}(\sigma, \tau)$ |

The order of arguments to a symmetric operation doesn't matter. Failure means: the operation is privileging the left or right argument.

### Associativity

| Operation | Law |
|---|---|
| meet | $(\tau \sqcap \sigma) \sqcap \rho \equiv \tau \sqcap (\sigma \sqcap \rho)$ |
| join | $(\tau \sqcup \sigma) \sqcup \rho \equiv \tau \sqcup (\sigma \sqcup \rho)$ |

Grouping doesn't matter. Failure means: the canonicalisation step is missing some pair.

### Identity

| Operation | Law |
|---|---|
| meet | $\tau \sqcap \top \equiv \tau$ |
| join | $\tau \sqcup \bot \equiv \tau$ |
| subtract | $\tau \setminus \bot \equiv \tau$ |
| narrow | $\mathit{narrow}(\tau, \top) \equiv \tau$ |

The top is the meet identity, the bottom is the join identity. Subtracting nothing leaves the input alone. Narrowing by no assertion leaves the input alone.

### Annihilator

| Operation | Law |
|---|---|
| meet | $\tau \sqcap \bot \equiv \bot$ |
| join | $\tau \sqcup \top \equiv \top$ |
| subtract | $\bot \setminus \sigma \equiv \bot$ |
| narrow | $\mathit{narrow}(\tau, \bot) \equiv \bot$ |
| narrow | $\mathit{narrow}(\bot, \pi) \equiv \bot$ |

Bottom annihilates meet; top annihilates join. Subtracting from bottom leaves bottom. Narrowing by an impossible assertion (or narrowing the empty type) yields bottom.

### Absorption

| Operation pair | Law |
|---|---|
| meet/join | $\tau \sqcap (\tau \sqcup \sigma) \equiv \tau$ |
| join/meet | $\tau \sqcup (\tau \sqcap \sigma) \equiv \tau$ |

The "lattice absorption" identities. These tie meet and join together: a value that is a meet of $\tau$ and a join with $\tau$ is just $\tau$. Failure means: the operations are defined inconsistently with each other.

### Bounds (the GLB and LUB properties)

| Operation | Law |
|---|---|
| meet (GLB) | For every $\rho$ such that $\rho \mathrel{<:} \tau$ and $\rho \mathrel{<:} \sigma$, $\rho \mathrel{<:} (\tau \sqcap \sigma)$. |
| join (LUB) | For every $\rho$ such that $\tau \mathrel{<:} \rho$ and $\sigma \mathrel{<:} \rho$, $(\tau \sqcup \sigma) \mathrel{<:} \rho$. |

Meet is *the greatest lower bound* — any other lower bound refines it. Join is *the least upper bound* — it refines any other upper bound. These are the soundness interlocks between meet/join and refines.

Failure of the GLB property means: the lattice is computing a meet that is *not actually* the greatest lower bound — it's leaving a smaller type that still bounds both inputs from below. Imprecise but sound.

Failure in the *other direction* — meet returning something that is *not* a lower bound — would be a soundness violation. The bound checks both ways.

### Subsumption interlock

| Law |
|---|
| $\tau \mathrel{<:} \sigma \iff \tau \sqcap \sigma \equiv \tau$ |
| $\tau \mathrel{<:} \sigma \iff \tau \sqcup \sigma \equiv \sigma$ |

Subtype is equivalent to "meet is unchanged" or "join is unchanged". A failure means: refines is disagreeing with what meet/join compute, which is a global inconsistency.

### Subtract bound

| Law |
|---|
| $(\tau \setminus \sigma) \mathrel{<:} \tau$ |

Subtract never produces a *larger* type than the input. Failure is a soundness violation — the analyser would think a value has more refinement than it does.

### Subtract disjoint after

| Law (when subtract is exact) |
|---|
| $(\tau \setminus \sigma) \sqcap \sigma \equiv \bot$ |

After removing $\sigma$ from $\tau$, what remains is disjoint from $\sigma$. This holds when subtract is exact (the kind system can express the complement); when subtract is conservative, the law holds in the weaker form "approximately disjoint" and is checked with a `is_uninhabited` or `overlaps == false` proxy.

### Refines/overlaps interlock

| Law |
|---|
| $\tau \mathrel{<:} \sigma$ and $\tau$ inhabited implies $\mathit{overlaps}(\tau, \sigma)$ |

Subtype implies overlap (assuming the subtype is non-empty). Failure means: refines is saying yes while overlaps is saying no, which is a contradiction.

### Narrow vs meet

| Law (positive assertions) |
|---|
| $\mathit{narrow}(\tau, \pi) \mathrel{<:} (\tau \sqcap \pi)$ |

Narrow's positive form is *at least as tight* as meet. Often equal; sometimes strictly tighter (the axis-propagation rules of narrow do work meet doesn't).

## How they're checked

A property-test battery generates random worlds (with random class hierarchies and template parameters) and random types exercising every Element kind family, then asserts every law on every triple. Failures shrink to a minimal counter-example and are recorded so future CI runs re-check the same input.

Two batteries cover the law set:

- The pair battery runs every pair-shaped law on $(\tau, \sigma)$. Idempotence, commutativity, identity, annihilator, GLB/LUB, subsumption interlock, subtract bound, refines/overlaps interlock.
- The triple battery runs the triple-shaped laws on $(\tau, \sigma, \rho)$. Associativity (meet, join), absorption.

Each property-test case calls both batteries on the generated triples.

## Why laws beat tests

Hand-written tests check what the author thought to test. The author writes "meet of int and string is never" because that's the obvious case. They miss the edge cases — the nested intersection with a negated head, the sealed shape with an optional uninhabited entry, the narrowed-mixed with the truthy axis interacting with a literal float.

Property tests check what the *laws* require, and the random generator finds the cases the author didn't think of. A single CI run on a thousand random triples covers more ground than a year of manually-written test cases.

The trade-off is that property tests find counter-examples but don't tell you *why*. The investigation work is on the human. The shrunk inputs and an instrumented path through the lattice (showing which rule fired on which Element pair) keep that investigation short.

> **See also:** [refines](./refines.md), [overlaps](./overlaps.md), [meet](./meet.md), [join](./join.md), [subtract](./subtract.md), [narrow](./narrow.md) — the operations the laws are about.
