# Serialization

The `serialize` module provides serde implementations for `TypeId`, `ElementId`, and the surrounding handle types. The intended use case: persisting types across analyser runs (caching), exchanging types over a wire (LSP, RPC), or producing a human-readable rendering for diagnostics.

```rust,ignore
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Diagnostic {
    pub message: String,
    pub expected: suffete::TypeId,
    pub actual:   suffete::TypeId,
}
```

`TypeId` and `ElementId` implement `Serialize` and `Deserialize` ; the same is true of the support handles (`ElementListId`, `TypeListId`, etc.).

## Wire format

The wire format serialises the *content* of the handle, not the bit pattern. Two reasons:

1. **Cross-process stability.** The bit pattern of a `TypeId` is a process-local arena slot ; it has no meaning in another process. The content (the element kinds, the payloads, the nested types) is portable.
2. **Compactness for human-readable output.** Producing a JSON like `{"kind": "Object", "name": "Foo", "args": [{"kind": "Int"}]}` is more useful in a diagnostic than `{"slot": 1234, "flags": 0, "meta": 0}`.

The exact format is suffete-defined; consumers should not rely on the specific shape, only on round-trip safety: serialise + deserialise produces an equivalent handle (same `==` after re-interning).

## Round-trip safety

```rust,ignore
let original: TypeId = ...;
let json = serde_json::to_string(&original).unwrap();
let recovered: TypeId = serde_json::from_str(&json).unwrap();
assert_eq!(original, recovered);
```

Deserialising the JSON re-interns the type into the *current* process's arenas. The recovered `TypeId` may have a different bit pattern from the original (different process, different arena state) but compares equal via the content-based `Eq`.

## Serialising types referencing the world

Some types reference *names* the world owns: class names, enum names, alias names, template parameter names. Serialisation includes the names. Deserialisation interns the names through the standard `mago_atom::atom!` machinery; the recovered type carries the same names.

This means: a deserialised type can be passed to a `World` of a different analyser run, and the world's queries on those names will work as expected (assuming the codebase still has the same names).

## What is not serialised

- The `meta` byte of `TypeId`. The `meta` is consumer-defined; serialisation includes it, but suffete does not know what it means.
- Internal arena slot indices. They are process-local.
- Any caches or precomputed state.

## Performance

Serialisation walks the type tree and emits a structured representation. The cost is O(tree size). For an analyser caching thousands of types, the serialise-side cost is the dominant one (deserialise re-interns into existing arenas, which is fast).

For very large collections, `bincode` or `postcard` are faster than `serde_json` ; both are supported via serde's standard mechanisms.

## A worked example

```rust,ignore
use suffete::{TypeBuilder, prelude::{INT, STRING}};

let original = TypeBuilder::new().push(INT).push(STRING).build();

// Serialise to JSON.
let json = serde_json::to_string_pretty(&original).unwrap();
println!("{}", json);

// Deserialise back.
let recovered: suffete::TypeId = serde_json::from_str(&json).unwrap();
assert_eq!(original, recovered);
```

## Using serialize for diagnostics

Diagnostics typically want a *human-readable* rendering of the type, not the wire format. For that, use the `Display` implementation:

```rust,ignore
let pretty: String = format!("{}", original.as_ref());   // "int|string"
```

Or the [`Typed`](https://github.com/carthage-software/suffete/tree/main/src/typed.rs) trait's `pretty_with_indent` method for multi-line, indented output suitable for hover-style displays.

The serialize module is for storage and transport. The Display / `Typed::pretty` methods are for diagnostics.

> **See also:** [Handles](./handles.md) for the underlying types being serialised; the `Typed` trait for diagnostic-quality rendering.
