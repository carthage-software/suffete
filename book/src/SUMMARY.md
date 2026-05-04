# Summary

[Introduction](./introduction.md)

# Part I — Foundations

- [What suffete is](./foundations/what-is-suffete.md)
- [Why a separate type system](./foundations/why.md)
- [Quick tour](./foundations/quick-tour.md)
- [Glossary and notation](./foundations/glossary.md)

# Part II — The Type Universe

- [Elements: the indivisible types](./universe/elements.md)
- [Scalars](./universe/scalars.md)
- [Special elements](./universe/special.md)
- [Objects, enums, and structural object types](./universe/objects.md)
- [Arrays and lists](./universe/arrays.md)
- [Iterables and callables](./universe/iterables-callables.md)
- [Resources](./universe/resources.md)
- [Class-like strings](./universe/class-like-string.md)
- [Refinement axes](./universe/refinements.md)
- [Wrappers: Negated and Intersected](./universe/wrappers.md)
- [Unresolved elements](./universe/unresolved.md)
- [Templates and generic parameters](./universe/templates.md)

# Part III — The Lattice

- [Subtyping: refines](./lattice/refines.md)
- [Overlap and disjointness](./lattice/overlaps.md)
- [Greatest lower bound: meet](./lattice/meet.md)
- [Least upper bound: join](./lattice/join.md)
- [Set difference: subtract](./lattice/subtract.md)
- [Narrowing under assertions](./lattice/narrow.md)
- [Soundness: the algebraic laws](./lattice/laws.md)

# Part IV — Generics

- [Template parameters in depth](./generics/templates.md)
- [Variance](./generics/variance.md)
- [Substitution](./generics/substitute.md)
- [Inference: standin and infer](./generics/standin.md)
- [Inheritance specialisation](./generics/specialise.md)

# Part V — Public API

- [TypeId, ElementId, and identity](./api/handles.md)
- [Constructing types: TypeBuilder and prelude](./api/construction.md)
- [Predicates: is_X, contains_X, and friends](./api/predicates.md)
- [Inspection: walking the tree](./api/inspect.md)
- [Transformation: map, flat_map, filter](./api/transform.md)
- [Hierarchy and World](./api/world.md)
- [Casting and runtime compatibility](./api/cast-compatibility.md)
- [Expansion: resolving unresolved elements](./api/expand.md)
- [Serialization](./api/serialize.md)

# Part VI — Cookbook

- [Answering "is A a subtype of B?"](./cookbook/subtype-question.md)
- [Building a union from scratch](./cookbook/union.md)
- [Narrowing a parameter type from instanceof](./cookbook/instanceof-narrow.md)
- [Resolving a generic class against an instance](./cookbook/generic-instance.md)
- [Walking a type to find every class name](./cookbook/find-classes.md)

# Part VII — Internals (optional reading)

- [Interning and the arenas](./internals/interner.md)
- [The ElementId tag layout](./internals/element-id-layout.md)
- [SIMD scans](./internals/simd.md)
- [Performance philosophy](./internals/performance.md)

# Part VIII — Reference

- [Element kinds](./reference/element-kinds.md)
- [Lattice options and reports](./reference/options-reports.md)
- [Prelude constants](./reference/prelude.md)

---

[Contributing](./contributing.md)
