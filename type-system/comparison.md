# Comparison

> The subtyping relation $\tau \mathrel{<:} \sigma$. Disjointness, overlap, and the coercion edges admitted in non-strict positions.

The vocabulary is fixed in **[types.md](./types.md)**. This chapter gives meaning to the relations *between* types: when one type is a subtype of another, when two types share inhabitants without one subsuming the other, when two types are categorically separate, and when a non-subtype edge is nonetheless admitted as a coercion.

## 1. The subtyping relation

$\tau \mathrel{<:} \sigma$ ("$\tau$ is a subtype of $\sigma$") means: every value of type $\tau$ is a value of type $\sigma$. Subtyping is reflexive, transitive, and stable under canonicalisation.

The relation is defined inductively. The rules below are presented in the order they apply: each rule is tried only when the earlier ones do not fire.

### 1.1 Universal axioms

$$
\frac{}{\bot \mathrel{<:} \tau} \\;\text{(Bot)}
\qquad
\frac{}{\tau \mathrel{<:} \top} \\;\text{(Top)}
\qquad
\frac{}{\tau \mathrel{<:} \tau} \\;\text{(Refl)}
$$

$$
\frac{\tau \mathrel{<:} \sigma \quad \sigma \mathrel{<:} \rho}{\tau \mathrel{<:} \rho} \\;\text{(Trans)}
$$

$\top$ denotes vanilla `mixed`. A constrained `mixed(c)` is *not* a universal supertype: $\text{mixed}(\text{non\\_null})$ does not admit `null`, so $\text{null} \mathrel{\not<:} \text{mixed}(\text{non\\_null})$.

### 1.2 Unions

A union is a least upper bound; subtyping distributes:

$$
\frac{\forall i.\\; \alpha_i \mathrel{<:} \sigma}{\alpha_1 \lor \dots \lor \alpha_n \mathrel{<:} \sigma} \\;\text{(Union-L)}
\qquad
\frac{\exists i.\\; \tau \mathrel{<:} \alpha_i}{\tau \mathrel{<:} \alpha_1 \lor \dots \lor \alpha_n} \\;\text{(Union-R)}
$$

(Union-L) reads "every input atom must be admissible by the container". (Union-R) reads "the input must fit some container atom". When both sides are non-atomic, both rules apply: every input atom must fit some container atom.

### 1.3 Scalar lattice

The scalar atoms form the lattice in §2 of **types.md**:

- Every scalar atom is $\mathrel{<:} \text{scalar}$.
- $\text{int}, \text{float}, \text{numeric-string} \mathrel{<:} \text{numeric}$.
- $\text{int}, \text{string}, \text{class-like-string} \mathrel{<:} \text{array-key}$.
- $\text{class-like-string} \mathrel{<:} \text{string}$ whenever the container `string` has no refinement requirement that the input does not satisfy.
- $\text{true} \mathrel{<:} \text{bool}$, $\text{false} \mathrel{<:} \text{bool}$. $\text{true} \mathrel{\\#} \text{false}$.

Inside *strict* positions (assertions, identity, type-guard contexts), $\text{int} \mathrel{\not<:} \text{float}$. Inside *flow* positions, $\text{int} \Rightarrow \text{float}$ is admitted as a coercion (see §3).

#### 1.3.1 Integer

$$
\frac{n \in [lo, hi]}{\text{Literal}(n) \mathrel{<:} \text{Range}(lo, hi)}
\qquad
\frac{[a, b] \subseteq [a', b']}{\text{Range}(a, b) \mathrel{<:} \text{Range}(a', b')}
$$

$$
\frac{n = m}{\text{Literal}(n) \mathrel{<:} \text{Literal}(m)}
\qquad
\frac{}{\text{Literal}(\\_) \mathrel{<:} \text{UnspecifiedLiteral}}
\qquad
\frac{}{\text{UnspecifiedLiteral} \mathrel{<:} \text{Unspecified}}
$$

Every integer atom is $\mathrel{<:} \text{Unspecified}$.

#### 1.3.2 Float

Analogous to integer over equality of literal values.

#### 1.3.3 String

Subtyping on strings is the conjunction of refinement-axis admission. Let $r(\tau)$ denote the set of refinement properties guaranteed by $\tau$. Then $\tau \mathrel{<:} \sigma$ iff every required property of $\sigma$ is in $r(\tau)$, plus:

- *Casing*: `Unspecified` admits any; otherwise the input's casing must match the container's.
- *Literal slot*: a literal-required container admits literal-origin inputs; a value-required container admits only the same value.

Combining the axes gives the named refinements as points in a 5-dimensional space: `non-empty-string`, `truthy-string`, `lowercase-string`, `non-empty-lowercase-string`, `truthy-numeric-string`, `callable-string`, `lowercase-callable-string`, and so on. Subtyping is monotone in adding refinements: a more-refined string is a subtype of a less-refined one.

#### 1.3.4 Class-like-string

Kinds (`Class`, `Interface`, `Enum`, `Trait`) are pairwise disjoint. Within a kind:

- $\text{Any}\\{k\\} \supseteq \text{Generic}\\{k, \dots\\} \supseteq \text{OfType}\\{k, T\\} \supseteq \text{Literal}\\{value\\}$.

$$
\frac{T \mathrel{<:} U}{\text{class-like-string}\langle k\rangle\\{T\\} \mathrel{<:} \text{class-like-string}\langle k\rangle\\{U\\}} \\;\text{(CLS-OfType)}
$$

$\text{Literal}\\{C\\}$ is $\mathrel{<:} \text{OfType}\\{k, U\\}$ iff $\Gamma$ confirms the class denoted by $C$ is itself $\mathrel{<:} U$.

$\text{class-like-string} \mathrel{<:} \text{string}$ (modulo container refinements), $\text{class-like-string} \mathrel{<:} \text{array-key}$, $\text{class-like-string} \mathrel{<:} \text{scalar}$.

### 1.4 Object atoms

Object subtyping combines four orthogonal axes: nominal hierarchy, generic parameters with variance, intersections, and shape compatibility.

#### 1.4.1 Nominal

$$
\frac{\Gamma \vdash C \preceq D \quad \mathrm{arity}(C) = \mathrm{arity}(D) = 0}{\text{Named}(C) \mathrel{<:} \text{Named}(D)} \\;\text{(Nom)}
$$

$\preceq$ is the reflexive transitive closure of `extends`, `implements`, and `use trait`.

#### 1.4.2 Parametric

$$
\frac{\Gamma \vdash C \preceq D \quad \forall i.\\; \mathrm{variance}(D, i) \models A_i \\;\mathrm{vs}\\; B_i}{\text{Named}(C, [\bar{A}]) \mathrel{<:} \text{Named}(D, [\bar{B}])} \\;\text{(Gen)}
$$

where $\mathrm{variance}(D, i) \models A \\;\mathrm{vs}\\; B$ is $A \equiv B$ for invariant, $A \mathrel{<:} B$ for covariant, $B \mathrel{<:} A$ for contravariant. A *readonly* parameter is sound to be covariant regardless of its placement.

When `class C extends D<X̄>`, instantiating $C\langle\bar{A}\rangle$ substitutes $\bar{A}$ for $C$'s own templates wherever they appear in $\bar{X}$, producing a binding to $D$ that subtyping consults.

#### 1.4.3 Intersection

$$
\frac{\text{input} \mathrel{<:} \mathrm{head}(\text{out}) \quad \forall J \in \text{out.intersections}.\\; \text{input} \mathrel{<:} J}{\text{input} \mathrel{<:} \text{out.head} \mathrel{\\&} \text{out.intersections}} \\;\text{(Int-R)}
$$

$$
\frac{\exists h' \in \\{\text{input.head}\\} \cup \text{input.intersections}.\\; h' \mathrel{<:} \text{out}}{\text{input.head} \mathrel{\\&} \text{input.intersections} \mathrel{<:} \text{out}} \\;\text{(Int-L)}
$$

#### 1.4.4 `static` and `$this`

`static<C>` and `$this<C>` admit $\mathrel{<:} \text{Named}(C)$ only inside strict positions when both sides agree on the modality flag. In flow positions, $\text{static}\langle C\rangle \Rightarrow \text{Named}(C)$ and $\textdollar\text{this}\langle C\rangle \Rightarrow \text{Named}(C)$ as coercions.

#### 1.4.5 Enums

$$
\frac{\Gamma \vdash E \preceq E' \quad \text{case}_C \subseteq \text{case}_D \\;\\;(\text{case}_D = \text{None admits any case})}{\text{Enum}(E, \text{case}_C) \mathrel{<:} \text{Enum}(E', \text{case}_D)} \\;\text{(Enum)}
$$

A $\text{Named}(C)$ is $\mathrel{<:} \text{Enum}(E, \text{None})$ iff $\Gamma$ confirms $C$ extends an enum interface for $E$.

#### 1.4.6 Object shapes

$$
\frac{
\substack{
\text{sealed}_{\text{in}} \Rightarrow \text{sealed}_{\text{out}} \\
\forall (k, v_o)\\;\text{required in out:}\\; \exists (k, v_i)\\;\text{in in with}\\; v_i \mathrel{<:} v_o \\
\text{required-out} \Rightarrow \text{required-in}
}
}{\text{WithProperties}\\{\text{in}\\} \mathrel{<:} \text{WithProperties}\\{\text{out}\\}} \\;\text{(Shape)}
$$

A $\text{Named}(C)$ is $\mathrel{<:} \text{WithProperties}\\{\text{props}_{\text{out}}\\}$ iff $\Gamma$ records each required property on $C$ with a compatible declared type.

$\text{HasMethod}(m) \mathrel{<:} \text{Named}(C)$ iff $\Gamma$ records that $C$ has method $m$. Analogously for $\text{HasProperty}$.

### 1.5 Array atoms

$$
\frac{T_i \mathrel{<:} T_o}{\text{List}(T_i) \mathrel{<:} \text{List}(T_o)} \\;\text{(List)}
\qquad
\frac{T_i \mathrel{<:} T_o}{\text{List}(T_i) \mathrel{<:} \text{Keyed}(\text{int} \to T_o, \dots)} \\;\text{(List-To-Keyed)}
$$

For keyed arrays:

$$
\frac{
\substack{
\text{in.params} \mathrel{<:} \text{out.params} \\;\\;\text{(covariant on key, on value)} \\
\forall k\\;\text{required in out:}\\; \text{present in in (required, with covariant value)} \\
\text{non\\_empty}_{\text{in}} \Rightarrow \text{non\\_empty}_{\text{out}} \\
\text{sealed}_{\text{in}} \Rightarrow \text{sealed}_{\text{out}}
}
}{\text{Keyed}(\text{in}) \mathrel{<:} \text{Keyed}(\text{out})} \\;\text{(Keyed)}
$$

The empty sealed `array{}` is a subtype of every keyed array that does not require fields.

### 1.6 Iterable atoms

$$
\frac{K_i \mathrel{<:} K_o \quad V_i \mathrel{<:} V_o}{\text{Iterable}(K_i, V_i) \mathrel{<:} \text{Iterable}(K_o, V_o)} \\;\text{(Iter)}
$$

$$
\frac{}{\text{Array} \mathrel{<:} \text{Iterable}} \\;\text{(Array-Iter)}
\qquad
\frac{\Gamma \vdash C\\;\text{implements}\\;\text{Traversable}\langle K, V\rangle}{\text{Named}(C) \mathrel{<:} \text{Iterable}(K, V)} \\;\text{(Object-Iter)}
$$

When the container is $\text{iterable}\langle V\rangle$ (key elided), it desugars to $\text{iterable}\langle\text{mixed}, V\rangle$; on narrowing to an array branch, the key auto-coerces to `array-key`.

### 1.7 Callable atoms

Function-type subtyping is contravariant in parameters and covariant in return:

$$
\frac{
\substack{
\forall i.\\; P_{\text{out},i} \mathrel{<:} P_{\text{in},i} \\
R_{\text{in}} \mathrel{<:} R_{\text{out}} \\
\text{input.is\\_pure} \geq \text{output.is\\_pure} \\
\text{input.throws} \subseteq \text{output.throws}
}
}{\text{Signature}(\bar{P}_{\text{in}}, R_{\text{in}}, \dots) \mathrel{<:} \text{Signature}(\bar{P}_{\text{out}}, R_{\text{out}}, \dots)} \\;\text{(Sig)}
$$

Several non-callable atoms admit $\mathrel{<:} \text{Callable}(\text{Any})$ and, with signature compatibility, $\mathrel{<:} \text{Callable}(\text{Signature})$:

- a literal class-string with `::` separator,
- a callable-string,
- an object with `__invoke`,
- a 2-tuple array `[class-or-instance, method]`.

A `Closure` is $\mathrel{<:} \text{Callable}(\text{Signature})$ and $\mathrel{<:} \text{Named}(\backslash\text{Closure})$.

### 1.8 Resource atoms

Resources are isolated:

$$
\frac{\text{closed}_{\text{in}}\\;\text{matches}\\;\text{closed}_{\text{out}}\\;\\;(\text{or out is None})}{\text{Resource}(\text{closed}_{\text{in}}) \mathrel{<:} \text{Resource}(\text{closed}_{\text{out}})} \\;\text{(Resource)}
$$

Subtyping between `Resource` and any non-resource atom holds only via `mixed`, generic parameters with constraint admitting resources, or `placeholder`.

### 1.9 Generic parameters

$$
\frac{\text{input} \equiv \text{output (same name, same defining scope)}}{T \mathrel{<:} T} \\;\text{(Same-T)}
$$

$$
\frac{C\\;\text{extends}\\;D\\;\text{with the same parameter transferred}}{T_C \mathrel{<:} T_D} \\;\text{(Inherited-T)}
$$

$$
\frac{\text{input.constraint} \mathrel{<:} \text{output} \quad \text{(output is not itself a generic parameter)}}{T \mathrel{<:} \text{output}} \\;\text{(Constraint)}
$$

When the output is a different generic parameter, subtyping fails outside template inference. Inside inference, a comparison between two parameters emits a *bound*: $T \mathrel{<:} U$ is recorded as an upper bound on $T$ for resolution by the inference algorithm.

### 1.10 Conditional atoms

$$
\frac{
\substack{
\Gamma; \Delta \vdash \text{subject is concrete} \\
\text{subject} \mathrel{<:} \text{target} \Rightarrow \text{result} = \text{then} \\
\text{subject} \mathrel{\not<:} \text{target} \Rightarrow \text{result} = \text{otherwise} \\
\text{result} \mathrel{<:} \sigma
}
}{(\text{subject is target ? then : otherwise}) \mathrel{<:} \sigma} \\;\text{(Cond-Eval)}
$$

When `subject` contains free templates, the conditional remains an atom and admits $\mathrel{<:}$ only by structural equivalence on each component.

### 1.11 References, aliases, derived atoms

A `Reference` is replaced by its resolution before subtyping is consulted. An `Alias` is, by default, transparent: its body substitutes for it. A `Derived` atom is evaluated when its inputs are concrete; while unresolved, two derived atoms are $\mathrel{<:}$ only by structural identity.

## 2. Disjointness and overlap

Subtyping does not exhaust the relations between types. Two types may fail to be in a subtype relation in either direction yet still share inhabitants. Three cases must be distinguished:

- **Subsumption**: $\tau \mathrel{<:} \sigma$. Every $\tau$-value is a $\sigma$-value.
- **Overlap**: $\tau \land \sigma \mathrel{\not\equiv} \bot$, but neither subsumes the other. Some values inhabit both.
- **Disjointness**: $\tau \mathrel{\\#} \sigma$. No value inhabits both.

### 2.1 Categorical disjointness

The principal disjointness axes in PHP are categorical:

|            | `bool` | `int` | `float` | `string` | `object` | `array` | `resource` |
|------------|:------:|:-----:|:-------:|:--------:|:--------:|:-------:|:----------:|
| `bool`     | ⊓ | # | # | # | # | # | # |
| `int`      | # | ⊓ | #\* | # | # | # | # |
| `float`    | # | #\* | ⊓ | # | # | # | # |
| `string`   | # | # | # | ⊓ | # | # | # |
| `object`   | # | # | # | # | ⊓ | # | # |
| `array`    | # | # | # | # | # | ⊓ | # |
| `resource` | # | # | # | # | # | # | ⊓ |

(⊓ denotes a non-empty overlap with the type itself; # denotes disjointness.)

\* $\text{int} \mathrel{\\#} \text{float}$ strictly; $\text{int} \Rightarrow \text{float}$ as a coercion in flow positions (see §3).

`null` is disjoint from every type other than itself, `mixed` (without `non_null`), `placeholder`, and any generic parameter whose constraint admits null. `void` is disjoint from every type other than itself.

### 2.2 Principal overlap edges

Some pairs of types are *not* in a subtype relation in either direction yet share inhabitants. Recognising these is essential for identity assertions: `$x === $y` is satisfiable iff the types of `$x` and `$y` overlap.

- `mixed` overlaps every type by construction.
- `array-key` overlaps `int`, `string`, and `class-like-string` (each is one of its branches).
- `numeric` overlaps `int`, `float`, and `string` (via numeric strings).
- `class-like-string` overlaps `string`. When both sides are literal-with-value, overlap holds iff the values are equal.
- `Iterable` overlaps `Array` and any object implementing `Traversable`.
- A generic parameter overlaps its constraint and any type that overlaps its constraint.
- A `Closure` overlaps $\text{Named}(\backslash\text{Closure})$, `Callable(Any)`, and any compatible signature.
- An object with `__invoke` overlaps `Callable(Any)`.

The complete construction of intersections, including the cases above, is in **[intersection.md](./intersection.md)**.

## 3. Coercion

Static analysis distinguishes *strict* positions (assertions, identity comparisons, type guards) from *flow* positions (parameter binding, return values). In flow positions, several non-subtype edges are admitted as *coercions*: not subtypes, but accepted with diagnostic side information rather than refused outright.

The principal coercion edges:

| From | To | Justification |
|------|-----|---------------|
| $\text{int}$ | $\text{float}$ | PHP arithmetic widens implicitly. |
| $\text{Object::Any}$ | $\text{Named}(C)$ | Downcast: only safe if confirmed at run time. |
| $\text{bool}$ | $\text{true}$ or $\text{false}$ | Downcast within the bool lattice. |
| $\text{string}\\;(\text{boring})$ | a string refinement | Downcast from a less-specified string. |
| $\text{non-empty-string}$ | $\text{class-like-string}$ | Permitted with verification that the value names a class. |
| $\text{array-key}$ | $\text{int}$ or $\text{string}$ | Downcast to a specific branch. |
| $\text{static}\langle C\rangle$ | $\text{Named}(C, \text{is\\_static}=\text{false})$ | The static reference flattens to its declaration class. |
| $\textdollar\text{this}\langle C\rangle$ | $\text{Named}(C, \text{is\\_this}=\text{false})$ | Analogously. |

Coercion is asymmetric: each edge above is $\text{From} \Rightarrow \text{To}$, not $\text{To} \Rightarrow \text{From}$. Strict positions reject every coercion edge; flow positions accept them. The diagnostic severity attached to a particular coercion is the analyser's concern, not the type system's.

## 4. Side information from comparison

A comparison $\tau \mathrel{<:} \sigma$ produces, in addition to a boolean, structured side information that downstream operations (notably *narrowing* in **[intersection.md](./intersection.md)**) consume:

- whether the answer required a coercion,
- the cause of any failure (e.g. nested `mixed`),
- any template bounds discovered during the recursion ($T \mathrel{<:} \dots$, $T \mathrel{:>} \dots$, $T = \dots$),
- a more specific replacement type when one can be inferred (e.g. comparing $\text{array}\\{a: 1, b: 2\\}$ against $\text{array}\langle\text{string}, \text{int}\rangle$ produces a replacement $\text{array}\\{a: \text{int}, b: \text{int}\\}$ for diagnostics).

The boolean answer alone is rarely sufficient; the side information is what allows downstream operations to refine, suggest, and explain.

## 5. Properties of $\mathrel{<:}$

For any types $\tau$, $\sigma$, $\rho$:

- **Reflexivity**: $\tau \mathrel{<:} \tau$.
- **Transitivity**: $\tau \mathrel{<:} \sigma$ and $\sigma \mathrel{<:} \rho$ imply $\tau \mathrel{<:} \rho$.
- **Antisymmetry up to equivalence**: $\tau \mathrel{<:} \sigma$ and $\sigma \mathrel{<:} \tau$ imply $\tau \equiv \sigma$.
- **Stability under canonicalisation**: $\tau \mathrel{<:} \sigma$ iff $\mathrm{canonical}(\tau) \mathrel{<:} \mathrm{canonical}(\sigma)$.
- **Compatibility with operations**:
  - $(\tau \lor \sigma) \mathrel{<:} \rho$ iff $\tau \mathrel{<:} \rho$ and $\sigma \mathrel{<:} \rho$.
  - $\tau \mathrel{<:} (\sigma \land \rho)$ iff $\tau \mathrel{<:} \sigma$ and $\tau \mathrel{<:} \rho$.
  - $(\tau \land \sigma) \mathrel{<:} \tau$ and $\tau \mathrel{<:} (\tau \lor \sigma)$ always.

Subtyping forms a preorder on types, and a partial order on equivalence classes: the principal lattice on which combination (**[combination.md](./combination.md)**) and intersection (**[intersection.md](./intersection.md)**) operate.
