# The PHP Type System

A type-theoretic description of the types that arise in static analysis of PHP, and the operations defined over them. Five chapters:

| Chapter | What it covers |
|---------|----------------|
| **[types.md](./types.md)** | The universe: every atom, every refinement axis, what each type denotes. |
| **[comparison.md](./comparison.md)** | The subtyping relation $\tau \mathrel{<:} \sigma$, plus disjointness, overlap, and the coercion edges admitted in non-strict positions. |
| **[combination.md](./combination.md)** | Union (least upper bound): how $\tau \lor \sigma$ is built, the absorption rules that keep it finite, and the generalisation thresholds that keep it tractable. |
| **[intersection.md](./intersection.md)** | Intersection (greatest lower bound), type-theoretic difference, and the narrowing operation that applies them in the presence of assertions. |
| **[generics.md](./generics.md)** | Generic abstractions, substitution, inference (standin), inferred-replacement, class-like specialisation, bound reconciliation, and the expansion operation that resolves non-structural type forms. |

## Notation

Used uniformly across the chapters:

| Symbol | Meaning |
|--------|---------|
| $\tau, \sigma, \rho$ | range over types |
| $T, U, V$ | range over template parameters |
| $\Gamma$ | the *program environment*: the function from class, interface, trait, enum, function, and constant names to their declarations |
| $\Theta$ | a *template environment*: a partial function from template parameters (qualified by their defining entity) to types |
| $\Delta$ | a *defining entity*: a class-like, function, method, or closure that introduces template parameters |
| $\tau \mathrel{<:} \sigma$ | every value of type $\tau$ is a value of type $\sigma$ |
| $\tau \equiv \sigma$ | $\tau \mathrel{<:} \sigma$ and $\sigma \mathrel{<:} \tau$ |
| $\tau \lor \sigma$ | least upper bound (union) |
| $\tau \land \sigma$ | greatest lower bound (intersection) |
| $\tau \setminus \sigma$ | difference: values in $\tau$ that are not in $\sigma$ |
| $\tau \mathrel{\\#} \sigma$ | disjoint: $\tau \land \sigma \equiv \bot$ |
| $\tau \Rightarrow \sigma$ | coercion: an admissible non-subtype edge in non-strict positions |
| $\bot$ | the empty type (`never`) |
| $\top$ | the universal type (vanilla `mixed`) |
| $\Gamma \vdash D \prec C$ | $\Gamma$ records that class-like $D$ extends/implements/uses-as-trait $C$ (transitively) |
| $\sigma[T \mapsto \rho]$ | capture-free substitution of $T$ by $\rho$ in $\sigma$ |
| $\sigma\Theta$ | the simultaneous substitution of $\sigma$ under $\Theta$ |
| $\Theta_1 \circ \Theta_2$ | composition of template environments |
| $\mathit{tparam}_C$ | the indexed list of template parameters of $C$, in declaration order |
| $\mathit{ext}_{D \to C}$ | the *extension binding*: the type arguments $D$ supplies to $C$ along the inheritance chain |
| $\mathit{specialise}(C, T, D\langle\bar\rho\rangle)$ | the type of $C$'s parameter $T$ in the context of an instantiated descendant $D\langle\bar\rho\rangle$ |
| $\mathit{expand}(\tau)$ | resolution of non-structural forms in $\tau$ (aliases, references, derived types, conditionals) |
| $\mathit{standin}(\sigma, \rho, \Theta)$ | inference round: walk parameter $\sigma$ against argument $\rho$, accumulating bounds into $\Theta$ |
| $\mathit{infer}(\sigma, \Theta)$ | inferred-replacement of templates in $\sigma$ using a fully-determined $\Theta$ |

## Reading order

1. Read **types.md** first to fix the vocabulary.
2. **comparison.md** assumes the vocabulary and gives meaning to the relations between types.
3. **combination.md** and **intersection.md** are duals; either can be read after **comparison.md** in either order.
4. **generics.md** layers on top: it uses subtyping (comparison.md) for variance and constraint checking, union (combination.md) for bound reconciliation and substitution-on-unions, and intersection (intersection.md) for upper-bound enforcement.

## What these chapters do not cover

- *Control-flow semantics*: how a path through a program produces assertions. The type system supplies the operations of intersection and difference; the analyser supplies the assertions and the flow.
- *Diagnostics policy*: which subtype failures produce which messages. The type system supplies the boolean answer and structured side information; what to do with it is the analyser's concern.
- *Runtime PHP coercion* in the language itself (e.g. `"0" == 0`). These chapters describe static types under static analysis.
