# Element kinds

Every Element is one of the following kinds. The kind is the high 6 bits of the `ElementId` ; see [the layout chapter](../internals/element-id-layout.md). Trivial-kind Elements have no payload; payload-bearing kinds carry a `&'static SomeInfo` reference into the matching arena.

This is reference material, organised by family.

## Trivial kinds (no payload)

| Kind | Prelude constant | Denotes |
|---|---|---|
| `Null` | `NULL` | The value `null`. |
| `Never` | `NEVER` | The empty type ($\bot$). |
| `Void` | `VOID` | The PHP `void` (absence of return value). |
| `Placeholder` | `PLACEHOLDER` | Inference-time hole. |
| `Bool` | `BOOL` | `true` or `false`. |
| `True` | `TRUE` | The value `true`. |
| `False` | `FALSE` | The value `false`. |
| `Scalar` | `SCALAR` | `bool \| int \| float \| string`. |
| `Numeric` | `NUMERIC` | `int \| float \| numeric-string`. |
| `ArrayKey` | `ARRAY_KEY` | `int \| string`. |
| `ObjectAny` | `OBJECT_ANY` | Any object, no class commitment. |
| `Mixed` | `MIXED` (vanilla) | Universe top ($\top$), with optional axes via `MixedInfo`. |

`Mixed` is technically payload-bearing (the `MixedInfo` carries the four axes), but the vanilla form (all axes default) is exposed as a constant and behaves trivially.

## Scalar payload kinds

| Kind | Payload | Denotes |
|---|---|---|
| `Int` | `IntInfo` | Integer (Unspecified, UnspecifiedLiteral, Literal(n), Range(IntRange)). |
| `Float` | `FloatInfo` | Float (Unspecified, Literal, NonZero). |
| `String` | `StringInfo` | String with literal slot + refinement flags. |
| `ClassLikeString` | `ClassLikeStringInfo` | A string naming a class-like (kind + specifier). |

See [scalars](../universe/scalars.md) and [class-like strings](../universe/class-like-string.md).

## Object family

| Kind | Payload | Denotes |
|---|---|---|
| `Object` | `ObjectInfo` | Named class (with optional type args, intersections, modality flags). |
| `Enum` | `EnumInfo` | Enum (with optional case). |
| `ObjectShape` | `ObjectShapeInfo` | Structural object type (with known properties, sealed/unsealed). |
| `HasMethod` | `HasMethodInfo` | Anything declaring a method named `m`. |
| `HasProperty` | `HasPropertyInfo` | Anything declaring a property named `p`. |

See [objects](../universe/objects.md).

## Collection family

| Kind | Payload | Denotes |
|---|---|---|
| `Array` | `KeyedArrayInfo` | Keyed array (generic, sealed shape, or empty). |
| `List` | `ListInfo` | Int-keyed list (generic or sealed). |
| `Iterable` | `IterableInfo` | `iterable<K, V>`. |

See [arrays](../universe/arrays.md) and [iterables and callables](../universe/iterables-callables.md).

## Resource

| Kind | Payload | Denotes |
|---|---|---|
| `Resource` | `ResourceInfo` | PHP resource (open / closed, named kind). |

See [resources](../universe/resources.md).

## Callable

| Kind | Payload | Denotes |
|---|---|---|
| `Callable` | `CallableInfo` | Bare, signature, or closure form. |

See [iterables and callables](../universe/iterables-callables.md).

## Wrappers

| Kind | Payload | Denotes |
|---|---|---|
| `Negated` | `NegatedInfo` | Set complement: `!T`. |
| `Intersected` | `IntersectedInfo` | Head + conjunct list: `H & C1 & ... & Cn`. |

See [wrappers](../universe/wrappers.md).

## Generics

| Kind | Payload | Denotes |
|---|---|---|
| `GenericParameter` | `GenericParameterInfo` | Free template parameter (`T`). |

See [templates](../universe/templates.md).

## Unresolved

| Kind | Payload | Denotes |
|---|---|---|
| `Alias` | `AliasInfo` | Type alias (resolved via `World::resolve_alias`). |
| `Reference` | `SymbolReference` | Name reference (class, alias, parameter, etc.). |
| `MemberReference` | `MemberReference` | Class-member type (`Foo::T`). |
| `GlobalReference` | `GlobalReference` | Global type variable. |
| `Conditional` | `ConditionalInfo` | `T extends U ? X : Y`. |
| `Derived` | `DerivedInfo` | `KeyOf<T>`, `ValueOf<T>`, `IndexAccess<T, K>`, etc. |
| `Variable` | `VariableInfo` | Analyser-introduced inference placeholder. |

See [unresolved elements](../universe/unresolved.md) and [expand](../api/expand.md).

## How to enumerate

```rust,ignore
use suffete::ElementKind;

// Every variant in declaration order.
for k in ElementKind::iter() {
    println!("{:?}", k);
}
```

The total count of variants is the size of `core::mem::variant_count::<ElementKind>()`. The maximum tag value is one less than the count.

## Constructing each kind

For trivial kinds, use the prelude constants. For payload-bearing kinds, use the `ElementId` constructors when available, or `interner().intern_*` directly:

```rust,ignore
use suffete::{ElementId, prelude::INT};
use suffete::interner::interner;
use suffete::element::payload::*;
use mago_atom::atom;

// Trivial.
let _ = suffete::prelude::NULL;
let _ = INT;

// Convenience constructors.
let _ = ElementId::int_literal(7);
let _ = ElementId::string_literal("hello");
let _ = ElementId::named_object(atom("Foo"));
let _ = ElementId::named_object_with_args(atom("Box"), &[suffete::prelude::TYPE_INT]);
let _ = ElementId::enum_case(atom("Status"), atom("Active"));

// Direct interner.
let info = ResourceInfo { state: ResourceState::Open, kind: Some(atom("curl")) };
let _ = interner().intern_resource(info);
```

The full list of constructors is in `src/element/id.rs`; the per-kind interner methods are generated by the `element_arena_methods!` macro in `src/interner/store.rs`.

> **See also:** [Elements: the indivisible types](../universe/elements.md) for the conceptual overview; [The ElementId tag layout](../internals/element-id-layout.md) for the bit layout; [Prelude constants](./prelude.md) for the full prelude.
