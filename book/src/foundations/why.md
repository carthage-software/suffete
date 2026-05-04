# Why a separate type system

The PHP type system is a serious piece of work. It is not as small as it looks from the docs. There are at least a dozen design decisions a real-world analyzer has to make about how types interact, and most of them are the difference between a useful tool and a frustrating one.

This chapter is the case for solving those problems exactly once.

## The problem with bundling

Most static analyzers grow a type system organically. The analyzer is built first, the type system is whatever the analyzer needed at the moment, and the seams between the two stop existing. Three things happen.

**Specification rots.** The type system never becomes a thing that can be written down; its rules live in the analyzer's source as comments-near-callsites and inferred-from-tests. New contributors patch existing rules without seeing the whole; the rules drift; nobody has a complete picture; and the only way to find out what the rules are is to read every callsite.

**Cross-tool divergence.** If you and your colleagues use a different analyzer for different reasons — one for IDE feedback, one for CI, one for refactoring — the answers you get to the same question differ. Each analyzer disagrees with the others on at least some of: literal-int range merging, the truthiness collapse on `mixed`, what happens to `HasMethod` under intersection with a class that lacks it, whether `int` refines `float`, whether a sealed shape with all-optional keys equals an empty shape, what variance `Traversable<K, V>` has on each parameter. In an ideal world all three tools agree. In practice they don't, and there is no shared spec to negotiate against.

**Performance is invisible.** The type system is sprinkled throughout the analyzer. There is no single hot loop to optimise. The cost of a `subtype_of` call is one HashMap allocation here, one `clone()` there, one quadratic loop somewhere else, none of which look bad locally. The only people who can move the needle are the ones with the entire architecture in their head, and they are usually busy.

## The bet

The bet suffete makes is that all three of those problems get easier if the type system is **outside** the analyzer.

**Specification becomes the artifact.** You can run property tests on it. You can write a book about it. You can argue about whether a particular law should hold without simultaneously arguing about which line of which analyzer file should change.

**Convergence becomes possible.** Two analyzers that share the same type-system crate cannot disagree on the type-system answers. They can still disagree on flow analysis, on diagnostic policy, on what the user actually meant — those are analyzer concerns. But "is `int<0,10>` a subtype of `int`?" has one answer in Rust, and every consumer gets it.

**Performance becomes a single target.** The hot loops are visible. The benchmarks are runnable. The optimisations live in one place and benefit everyone.

## What this costs

There are three real costs.

**An API surface.** Bundled type systems have no API; they have callsites. A standalone type system has a public Rust API that is permanent in the sense that breaking it breaks every consumer at once. Suffete acknowledges this: the API is currently unstable, broken often, and will stay that way until the design has settled. Once stable, breaking it requires a major version.

**A `World` indirection.** The type system needs to ask the analyzer about the codebase: class hierarchies, declared methods, template parameter bounds. Bundled type systems can reach directly into the analyzer's HashMaps. Suffete cannot; it goes through the [`World`](../api/world.md) trait. The analyzer pays for one virtual dispatch per query. Suffete amortises this with caching where it can and goes for free wherever the question can be answered without the world.

**Vocabulary alignment.** The analyzer and the type-system crate have to agree on what a type *is*. If the analyzer wants to express a type the crate cannot represent, the analyzer is stuck. The mitigation is the **completeness mandate**: any PHP type that a real-world analyzer expresses — Mago, PHPStan, Psalm, Hack — must be representable in suffete. If you find one that isn't, that is a bug.

## Why now, why Rust

Mago is a Rust toolchain. Doing the same work in two languages would be untenable; doing it in not-Rust would lose Mago's performance baseline. The same reasoning that pulled mago into Rust pulls suffete with it.

Beyond Mago, the long-term story is that other Rust-based PHP tools — IDE servers, refactoring tools, test runners that need to reason about types — should not have to re-implement this. There aren't many of them today. Suffete bets there will be more, and that they would all rather build on top of a settled spec than write their own.

> **See also:** [What suffete is](./what-is-suffete.md) — the surface area suffete deliberately covers (and what it deliberately doesn't).
