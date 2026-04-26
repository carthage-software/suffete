# Combination

> Union of types: the least upper bound $\tau \lor \sigma$. How it is built, the absorption rules that keep it finite, the generalisation thresholds that keep it tractable.

A *combination* is the least upper bound of two or more types under the subtyping relation defined in **[comparison.md](./comparison.md)**. Combination is the operation by which control flow re-converges: when an expression yields $\tau$ on one branch and $\sigma$ on another, its type at the join is $\tau \lor \sigma$. When a function returns either of two types, its return type is the union of those types. When a parameter accepts several alternatives, its type is their union.

This chapter describes the operation: what it is, what it must satisfy, what absorptions it performs, and how it remains tractable in the face of refinements that can grow without bound.

## 1. Specification

For any types $\tau$, $\sigma$:

$$
\tau \mathrel{<:} \tau \lor \sigma \qquad \sigma \mathrel{<:} \tau \lor \sigma \qquad \text{(upper bound)}
$$

$$
\forall \rho.\; (\tau \mathrel{<:} \rho \land \sigma \mathrel{<:} \rho) \implies (\tau \lor \sigma) \mathrel{<:} \rho \qquad \text{(least upper bound)}
$$

The union of $\tau$ and $\sigma$ is the smallest type that contains both as subtypes. It is unique up to equivalence.

By convention:

$$
\tau \lor \bot \equiv \tau
\qquad
\tau \lor \tau \equiv \tau
\qquad
\tau \lor \sigma \equiv \sigma \lor \tau
\qquad
(\tau \lor \sigma) \lor \rho \equiv \tau \lor (\sigma \lor \rho)
$$

Union is the join of the subtyping lattice.

## 2. Atom-set construction

Every type is the union of its atoms (a type is a finite multiset of atoms). The combination of two types is therefore the canonical form of the multiset union of their atoms.

$$
\frac{\tau = \alpha_1 \lor \dots \lor \alpha_n \qquad \sigma = \beta_1 \lor \dots \lor \beta_m}{\tau \lor \sigma = \mathrm{canonical}(\alpha_1, \dots, \alpha_n, \beta_1, \dots, \beta_m)} \;\text{(Atom-Union)}
$$

The canonical form is what gives the union meaning: without it, $\tau \lor \sigma$ would be syntactically distinct for every permutation of inputs. Canonicalisation is the apparatus that recovers a unique representative.

## 3. Canonicalisation

Canonicalisation is a deterministic, idempotent transformation that takes a multiset of atoms and produces a canonical type. It applies the following passes in sequence; later passes assume earlier ones have run.

### 3.1 Sort

Atoms are sorted by a fixed total order. This makes equality on canonical multisets trivially decidable as multiset equality, and makes hashing of types deterministic.

### 3.2 Deduplicate

Structurally equal atoms collapse: $\alpha \lor \alpha \equiv \alpha$.

### 3.3 Drop `never`

If the multiset contains atoms other than `never`, every `never` atom is dropped. The empty multiset becomes $\{\text{never}\}$, since a union always has at least one atom.

### 3.4 Apply absorptions

If two atoms $\alpha$, $\beta$ satisfy $\alpha \mathrel{<:} \beta$, then $\alpha$ is dropped from the multiset: the more specific atom is absorbed by the more general one.

$$
\alpha \mathrel{<:} \beta \implies \alpha \lor \beta \equiv \beta
$$

This is the principal absorption. Most concrete absorptions are instances of it:

- $\text{int} \lor \text{Literal}(42) \equiv \text{int}$.
- $\text{string} \lor \text{"hello"} \equiv \text{string}$.
- $\text{bool} \lor \text{true} \equiv \text{bool}$.
- $\text{Range}(0, 10) \lor \text{Literal}(5) \equiv \text{Range}(0, 10)$.
- $\text{mixed} \lor \tau \equiv \text{mixed}$ for every $\tau$.

Absorption is not symmetric: $\text{mixed}(\text{non\_null}) \lor \text{null} \equiv \text{mixed}$, *not* $\text{mixed}(\text{non\_null})$, because $\text{null} \mathrel{\not<:} \text{mixed}(\text{non\_null})$.

### 3.5 Merge family-specific structures

Some atoms admit *merging*: two distinct atoms in the same family combine into a single, broader atom. These are not absorptions (neither input is a subtype of the other) but are sound because the merged atom is the least upper bound.

#### 3.5.1 Bool

$$
\text{true} \lor \text{false} \equiv \text{bool}
$$

#### 3.5.2 Integer ranges

Adjacent ranges merge:

$$
\text{Range}(a, b) \lor \text{Range}(b+1, c) \equiv \text{Range}(a, c)
$$

$$
\text{Range}(a, b) \lor \text{Literal}(b+1) \equiv \text{Range}(a, b+1) \quad \text{when}\; b+1 \leq +\infty
$$

$$
\text{Literal}(a) \lor \text{Literal}(a+1) \equiv \text{Range}(a, a+1)
$$

Disjoint ranges are kept disjoint:

$$
\text{Range}(1, 3) \lor \text{Range}(5, 7) \quad \text{keeps both atoms; values 4 are excluded}
$$

#### 3.5.3 Mixed constraints

When a $\text{mixed}(c_1)$ and a $\text{mixed}(c_2)$ both appear in the multiset, they merge into a single $\text{mixed}(c_1 \sqcup c_2)$ whose constraint is the join of the two, i.e. the relaxation that admits both.

When $\text{mixed}(c)$ appears alongside an atom $\tau$ not admitted by $c$, the constraint relaxes:

$$
\text{mixed}(\text{non\_null}) \lor \text{null} \equiv \text{mixed}
\qquad
\text{mixed}(\text{truthy}) \lor \text{false} \equiv \text{mixed}
$$

#### 3.5.4 Object hierarchy collapse

If the multiset contains $\text{Named}(C_1), \text{Named}(C_2), \dots, \text{Named}(C_n)$ and $C_i \preceq C_j$ for some $i \neq j$, then $\text{Named}(C_i)$ is absorbed by $\text{Named}(C_j)$. This is an instance of the general subsumption rule of §3.4 but is mentioned here because it is by far the most common collapse on object-heavy unions.

#### 3.5.5 Array / list

$\text{List}(T)$ and $\text{Keyed}(\text{parameters}=(\text{int}, T), \text{known\_items}=\text{None})$ are equivalent and collapse into one atom (the list form, which carries additional structural information). Two lists $\text{List}(T)$ and $\text{List}(U)$ combine into $\text{List}(T \lor U)$. Two keyed arrays with the same shape but different value types combine pointwise on their values.

When a sealed array shape is unioned with an unsealed one for the same set of keys, the result is unsealed.

### 3.6 Generalise unbounded growth

Some refinements can grow without bound: a series of literal-int values, a series of literal-string values, a series of distinct sealed shapes. Without intervention, every distinct literal a program produces accumulates as its own atom, and operations on the resulting type become quadratic.

The type system therefore admits *generalisation thresholds*. When the count of literal atoms in a single shape exceeds an analyser-chosen threshold, those literals collapse to their unrefined supertype:

- many $\text{Literal}(n)$ atoms past the threshold $\to \text{int}$.
- many $\text{Literal}(x)$ atoms past the threshold $\to \text{float}$.
- many $\text{Literal}(\text{"…"})$ atoms past the threshold $\to \text{string}$.
- many sealed $\text{array}\{\dots\}$ shapes past the threshold $\to \text{array}\langle K, V\rangle$ with $K$ the join of all keys' types and $V$ the join of all values' types.

Generalisation is a soundness-preserving approximation: the generalised type is a true supertype of every input atom. It is *not* invertible; once an atom is generalised, its origin is forgotten. This is the design choice that keeps unions tractable.

The thresholds themselves are configuration, not part of the type theory. The principle (that combination admits supertype approximations past a point) is the part that matters.

### 3.7 Idempotence

After all the passes above, applying canonicalisation again is a no-op. Canonicalisation is a fixed point.

## 4. Properties of combination

For any types $\tau$, $\sigma$, $\rho$:

- **Idempotence**: $\tau \lor \tau \equiv \tau$.
- **Commutativity**: $\tau \lor \sigma \equiv \sigma \lor \tau$.
- **Associativity**: $(\tau \lor \sigma) \lor \rho \equiv \tau \lor (\sigma \lor \rho)$.
- **Identity**: $\tau \lor \bot \equiv \tau$.
- **Annihilation**: $\tau \lor \top \equiv \top$.
- **Monotonicity**: if $\tau \mathrel{<:} \tau'$ and $\sigma \mathrel{<:} \sigma'$, then $\tau \lor \sigma \mathrel{<:} \tau' \lor \sigma'$.
- **Distribution over intersection**: $\tau \lor (\sigma \land \rho) \equiv (\tau \lor \sigma) \land (\tau \lor \rho)$ (only when $\tau$, $\sigma$, $\rho$ are concrete enough that the intersection commutes with the union; not all derived/generic atoms permit this rewrite).
- **Stability under canonicalisation**: $\mathrm{canonical}(\tau) \lor \mathrm{canonical}(\sigma) \equiv \mathrm{canonical}(\tau \lor \sigma)$.

## 5. Where combination occurs

Combination is the ambient operation of static analysis. It is invoked at every point where two or more values flow together:

- **Control-flow joins**: when two branches converge, each variable's type at the join is the union of its types on each branch.
- **Loop fixpoints**: a variable's type at the head of a loop is the union of its type before the loop and its type at the back-edge from the loop body.
- **Switch and match**: the result type is the union of the result types of each arm.
- **Conditional expressions**: `cond ? a : b` yields $\mathrm{type}(a) \lor \mathrm{type}(b)$.
- **Phi nodes**: abstractly, every joining of definitions is a union.
- **Function return types**: when a function has multiple return statements, the inferred return type is the union of their types.
- **Property and parameter aggregation**: when a value is assigned to a property of declared type $\tau$ and the assigned value is $\sigma$, the property's narrowed type at that point reflects $\sigma$; subsequent reads in another branch see $\tau \lor \sigma$ if the original value is still flowable.
- **Generic inference (lower bounds)**: when a template parameter $T$ is constrained from below by several arguments, its inferred type is the union of those arguments.

Every one of these is the same operation, applied uniformly: the least upper bound on the subtype lattice, with canonicalisation, with absorptions, with generalisation past thresholds.

## 6. Worked examples

| Input | Result | Reason |
|-------|--------|--------|
| $\text{int} \lor \text{string}$ | $\text{int} \lor \text{string}$ | disjoint, kept |
| $\text{int} \lor \text{Literal}(42)$ | $\text{int}$ | absorption |
| $\text{true} \lor \text{false}$ | $\text{bool}$ | merging |
| $\text{Range}(1, 3) \lor \text{Literal}(4)$ | $\text{Range}(1, 4)$ | merging |
| $\text{Range}(1, 3) \lor \text{Literal}(7)$ | $\text{Range}(1, 3) \lor \text{Literal}(7)$ | kept (gap at 4–6) |
| $\text{Named}(C) \lor \text{Named}(D)$, $D \preceq C$ | $\text{Named}(C)$ | absorption |
| $\text{Named}(C) \lor \text{Named}(D)$, unrelated | $\text{Named}(C) \lor \text{Named}(D)$ | kept |
| $\text{list}\langle\text{int}\rangle \lor \text{list}\langle\text{string}\rangle$ | $\text{list}\langle\text{int} \lor \text{string}\rangle$ | covariant pointwise |
| $\text{array}\{a: \text{int}\} \lor \text{array}\{a: \text{string}\}$ | $\text{array}\{a: \text{int} \lor \text{string}\}$ | pointwise on shared keys |
| $\text{mixed}(\text{non\_null}) \lor \text{null}$ | $\text{mixed}$ | constraint relaxation |
| $\text{Literal}(1) \lor \dots \lor \text{Literal}(200)$ | $\text{int}$ | past threshold |

## 7. What combination does not do

- It does not *evaluate* derived atoms. $\text{KeyOf}(\tau) \lor \text{KeyOf}(\sigma)$ remains a union of two derived atoms unless their inputs are concrete enough to evaluate.
- It does not resolve references or aliases. Combination operates on types as given; if two unresolved references happen to denote the same target, the resulting union retains both atoms until resolution is applied.
- It does not introduce intersections. The union of object atoms is a union, not the intersection of their types. Intersection is the dual operation, described in **[intersection.md](./intersection.md)**.
- It does not apply *narrowing*. The reverse direction of combination (given a union, removing some atoms because an assertion has ruled them out) is the difference operation, also in **[intersection.md](./intersection.md)**.

Combination is purely additive. Subtractive reasoning lives elsewhere.
