# Glossary and notation

The book uses a small but consistent set of mathematical and PL-theory terms. They are defined here once. Subsequent chapters use them without re-defining.

## Notation

| Symbol | Meaning |
|--------|---------|
| $\tau, \sigma, \rho$ | Range over types. |
| $T, U, V$ | Range over template parameters. |
| $\Gamma$ | The **program environment**: the function from class, interface, trait, enum, function, and constant names to their declarations. In the API, this is the [`World`](../api/world.md) trait. |
| $\Theta$ | A **template environment**: a partial function from template parameters (qualified by their defining entity) to types. |
| $\Delta$ | A **defining entity**: a class-like, function, method, or closure that introduces template parameters. |
| $\tau \mathrel{<:} \sigma$ | $\tau$ **refines** $\sigma$: every value of type $\tau$ is also a value of type $\sigma$. |
| $\tau \equiv \sigma$ | $\tau \mathrel{<:} \sigma$ and $\sigma \mathrel{<:} \tau$. |
| $\tau \sqcup \sigma$ | **Join**: the least upper bound (union, with absorption rules applied). |
| $\tau \sqcap \sigma$ | **Meet**: the greatest lower bound (intersection, with literal/range collapse). |
| $\tau \setminus \sigma$ | **Set difference**: values in $\tau$ that are not in $\sigma$. |
| $\tau \mathrel{\\#} \sigma$ | $\tau$ and $\sigma$ are **disjoint**: $\tau \sqcap \sigma \equiv \bot$. |
| $\tau \Rightarrow \sigma$ | A **coercion edge**: an admissible non-subtype edge in non-strict positions. PHP's runtime, on the boundary, allows e.g. `int` to flow into a `float` parameter. The coercion edge is recorded on the [`LatticeReport`](../api/predicates.md). |
| $\bot$ | **Bottom**: the empty type, no values. PHP's `never`. |
| $\top$ | **Top**: the universal type, all values. PHP's `mixed`. |
| $\Gamma \vdash D \prec C$ | $\Gamma$ records that class-like $D$ extends, implements, or uses-as-trait $C$ (transitively). |
| $\sigma[T \mapsto \rho]$ | Capture-free substitution of $T$ by $\rho$ in $\sigma$. |
| $\sigma\Theta$ | Simultaneous substitution of $\sigma$ under the environment $\Theta$. |
| $\Theta_1 \circ \Theta_2$ | Composition of template environments. |
| $\mathit{tparam}_C$ | The indexed list of template parameters of class $C$, in declaration order. |
| $\mathit{ext}_{D \to C}$ | The **extension binding**: the type arguments $D$ supplies to $C$ along the inheritance chain. |
| $\mathit{specialise}(C, T, D\langle\bar\rho\rangle)$ | The type of $C$'s template parameter $T$ in the context of an instantiated descendant $D\langle\bar\rho\rangle$. |
| $\mathit{expand}(\tau)$ | Resolution of non-structural forms in $\tau$ (aliases, references, derived types, conditionals). |
| $\mathit{standin}(\sigma, \rho, \Theta)$ | An **inference round**: walk parameter $\sigma$ against argument $\rho$, accumulating bounds into $\Theta$. |
| $\mathit{infer}(\sigma, \Theta)$ | **Inferred-replacement** of templates in $\sigma$ using a fully-determined $\Theta$. |
| $\mathit{narrow}(\tau, \pi)$ | Apply the assertion $\pi$ to refine the type $\tau$. |

## Vocabulary

The terms below are used throughout the book in their PL-theory sense. Where a term has a more colloquial meaning that might confuse, the technical sense is noted.

**Atom.** The PL-theory name for the indivisible piece a type is built from. Suffete calls these **Elements**. The two terms are interchangeable; this book prefers "Element" everywhere except in references to literature.

**Element.** The indivisible unit a type is built from: a single integer literal, the unconstrained string, the named class `Foo`, the negation of `null`. See [Elements](../universe/elements.md).

**Type.** A *set* of values, expressed as a finite union of Elements. A singleton Element is a one-element type.

**Element kind.** The family or shape of an Element — `int`, `string`, named-class object, list, negation, intersection, conditional, and so on.

**Refines / refinement / subtype.** "Refines" is the verb form of "is a subtype of". $\tau$ **refines** $\sigma$ ($\tau \mathrel{<:} \sigma$) iff every value of $\tau$ is also a value of $\sigma$. "Refinement" is also used for the **refinement axes** that can be carried on certain element kinds (the truthiness, non-empty, non-null, isset-from-loop axes on `mixed`; the casing/numeric/non-empty axes on `string`); context disambiguates.

**Overlap / disjoint.** Two types **overlap** if their meet is inhabited (contains at least one value). They are **disjoint** if their meet is empty (uninhabited).

**Inhabited / uninhabited.** A type is inhabited if it contains at least one value. $\bot$ is the canonical uninhabited type, but some sealed shapes (e.g. an empty sealed object shape with required properties) and some Intersected wrappers (e.g. `Foo & !Foo`) are also uninhabited despite not being syntactically $\bot$.

**Meet ($\sqcap$).** The greatest lower bound of two types. Operationally: the largest type that refines both. PHP's intersection.

**Join ($\sqcup$).** The least upper bound of two types. Operationally: the smallest type that both refine into. PHP's union, with literal collapse and range merging applied.

**Subtract ($\setminus$).** Set-theoretic difference: $\tau \setminus \sigma$ is the values in $\tau$ that are not in $\sigma$. Used by the analyzer to eliminate a positive case (e.g. after `!is_int($x)`).

**Narrow.** The composite operation that combines an input type with an *assertion type*. Often equivalent to meet (positive assertions) or subtract (negative assertions) but with extra rules for cross-cutting concerns like the truthiness axes. See [narrow](../lattice/narrow.md).

**Lattice.** The set of types ordered by $\mathrel{<:}$, equipped with $\sqcap$ and $\sqcup$ as meet and join, with $\bot$ at the bottom and $\top$ at the top. The PHP type lattice is not a complete lattice in the strict mathematical sense — there are infinite ascending chains in literal-int ranges, for example — but the operations behave as a lattice does on the cases that matter.

**Variance.** The relationship between a generic parameter and the substitution direction. Covariant: a subtype of `T` in a subtype of `Box<T>`. Contravariant: a supertype of `T`. Invariant: only `T` itself. See [variance](../generics/variance.md).

**Substitution.** Replacing a template parameter with a concrete type, capture-free. Written $\sigma[T \mapsto \rho]$. See [substitute](../generics/substitute.md).

**Inference (standin).** A round of bound-collection: walking a parameter type against an argument type and recording, for each free template, the bounds the argument imposes. Repeated inference rounds against multiple call-site arguments accumulate into a template environment $\Theta$. See [standin](../generics/standin.md).

**Inferred-replacement.** Applying a fully-determined template environment to a type to produce the inferred concrete result. Sometimes called "infer". See [standin](../generics/standin.md).

**Inheritance specialisation.** Computing what a descendant supplies to an ancestor's template parameter, given the descendant's instantiation. See [specialise](../generics/specialise.md).

**Expansion.** Resolving the non-structural forms — aliases, references, member references, global references, conditionals, derived types — into structural ones. The lattice operations work on already-expanded inputs in most cases; expansion is the analyzer's responsibility. See [expand](../api/expand.md).

**Wrapper.** A composite Element that contains other Elements: a negation (set complement) and an intersection (head plus a list of conjuncts). Wrappers exist in the Element universe; the union itself is in the Type universe. See [wrappers](../universe/wrappers.md).

**Refinement axis.** A boolean or low-cardinality property carried on top of an Element. The narrowed-mixed kind carries non-null, truthiness, empty, and isset-from-loop axes. The string kind carries casing, numeric-shape, non-empty axes. Axes never multiply the element count — they are bits on the same Element. See [refinement axes](../universe/refinements.md).

**Coercion edge.** A non-subtype, non-overlap relationship that PHP's runtime allows on the parameter boundary. The classic example is `int` flowing into a `float` parameter. The lattice records the use of a coercion edge so the analyzer can warn about it.

**Sealed.** A keyed-array, list, or object shape is **sealed** when it asserts no entries beyond those it lists. Unsealed shapes admit additional entries.

**Defining entity.** The class-like, function, method, or closure that introduces a template parameter. Every template parameter is keyed by `(name, defining_entity)` so substitutions know which parameters they apply to.

**World.** The analyzer's codebase model, exposed to suffete as the source of class hierarchies, declared methods, declared properties, template parameter bounds. Suffete itself stores none of this.

> **See also:** [Quick tour](./quick-tour.md) for the notation in action, and [Element kinds](../reference/element-kinds.md) for the exhaustive list of element-kind names referenced throughout the book.
