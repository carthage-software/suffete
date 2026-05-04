# What suffete is

Suffete is a Rust crate that implements the PHP type system as a self-contained, queryable data structure with a comprehensive set of operations.

It has three deliverables:

1. **A representation.** A handle-based, intern-deduplicated, content-addressed data model that can express every PHP type a real-world analyzer needs to express.
2. **Operations on that representation.** Subtyping, overlap, intersection, union, set difference, narrowing, generic substitution, generic inference, expansion of unresolved forms, structural transformations.
3. **A `World` trait.** The single abstraction by which the type system asks questions about the user's codebase: "does class `D` extend class `C`?", "what does class `C` declare for property `p`?", "what is the upper bound of template parameter `T` on class `C`?". An analyzer plugs in its codebase model behind this trait; suffete itself stays codebase-agnostic.

That is the entire surface. There is no parser. There is no AST. There is no notion of a file, a statement, a scope, a control-flow graph, a diagnostic, or a configuration.

## What suffete does

Concretely, given two `TypeId`s and a `World`, suffete answers:

- $\tau \mathrel{<:} \sigma$ — does every value of type $\tau$ also have type $\sigma$? ([refines](../lattice/refines.md))
- $\tau \mathrel{\\#} \sigma$ — are $\tau$ and $\sigma$ disjoint? ([overlaps](../lattice/overlaps.md))
- $\tau \sqcap \sigma$ — what is the greatest lower bound? ([meet](../lattice/meet.md))
- $\tau \sqcup \sigma$ — what is the least upper bound? ([join](../lattice/join.md))
- $\tau \setminus \sigma$ — what remains of $\tau$ after removing $\sigma$? ([subtract](../lattice/subtract.md))
- $\mathit{narrow}(\tau, \pi)$ — given an assertion $\pi$ that holds on top of $\tau$, what is the refined type? ([narrow](../lattice/narrow.md))

It also answers, given a type, simpler structural questions: is this guaranteed truthy? does this contain `null`? is this a single literal value the analyzer can constant-fold? does this contain a free template parameter anywhere in its tree? — and many more. These are the [predicates](../api/predicates.md).

And it offers the build-side primitives: making a new type from elements, walking a type and rebuilding it under a transformation, applying a generic substitution, expanding an alias.

## What suffete does not do

It does not parse PHP. There is no lexer, no parser, no docblock interpreter. The analyzer brings types in from somewhere — a parser, a serialised cache, a hand-built fixture — and constructs `TypeId`s through suffete's builders and prelude.

It does not run a control-flow analysis. The lattice operations are pure functions of their inputs; they do not know which line of code they came from, what branch they are on, or which assertions led to them being asked. The analyzer asks; suffete answers.

It does not produce diagnostics. When `refines` returns `false`, the analyzer chooses what message to display, where to point the user, and at what severity. Suffete returns a boolean and a [`LatticeReport`](../api/predicates.md) carrying structured side information; the message-writing is the analyzer's job.

It does not maintain a codebase model. Class hierarchies, declared properties, declared methods, template parameter bounds — all of those live in the analyzer's data structures and are queried through [`World`](../api/world.md) on demand.

## What this buys you

Three things, in roughly decreasing order of value to a downstream consumer:

**Single source of truth for hard semantics.** Subtyping in PHP is not simple. The interaction of generics and intersections, of nullability and the truthiness axes on `mixed`, of literal collapse with refined ranges, of object shape with `HasMethod` — these are conditions that every analyzer has to get right and few of them do. If suffete is correct, every analyzer that depends on suffete is correct on those conditions, automatically.

**A property-test battery, run for everyone.** Suffete is verified against an algebraic-law battery: idempotence, commutativity, associativity, identity, absorption, the GLB and LUB bounds, the soundness interlock between the operations. Every PR runs that battery. When suffete says $\tau \sqcap \sigma$ is some value, you can rely on it being a lower bound — that is checked on thousands of cases per CI run.

**Performance you don't have to think about.** The interner, the SIMD scans, the canonical-form fast paths, the singleton caches — they are tuned, and they run on every analyzer that depends on suffete, without the analyzer needing to know how. You write `refines(t1, t2, world, opts, &mut report)`; it returns in nanoseconds for typical inputs.

## How it relates to the rest of Carthage

Suffete is being designed in isolation so that the type-system contract can be specified, exercised, and benchmarked without an analyzer in the loop. Once it is stable, it is intended to replace the type-system core inside [Mago](https://github.com/carthage-software/mago), the Carthage Software PHP toolchain.

Mago is one consumer. Anyone else writing a PHP analyzer in Rust is invited to be another. Suffete will not change its API on Mago's behalf; if it works for Mago, it works for everyone.

> **See also:** [Why a separate type system](./why.md) — the rationale for not building this inside an analyzer in the first place.
