#![allow(clippy::pub_use)]

//! Process-global, append-only, lock-free-read interning primitives.
//!
//! Two building blocks are exposed by this module:
//!
//! - [`Arena<T>`]: dedupes whole values (one slot per unique `T`), used for
//!   per-kind element payloads and the side-tables (`IntRange`,
//!   `DefiningEntity`, `Signature`, `CallableAlias`).
//! - [`SliceArena<T>`]: dedupes slices of values, used for the secondary
//!   list handles (`TypeListId`, `ElementListId`, `KnownItemsId`,
//!   `KnownElementsId`, `KnownPropertiesId`, `ParamListId`).
//!
//! Both arenas:
//!
//! - Live for the entire process lifetime (they are placed inside a global
//!   `OnceLock<Interner>`), so the references they hand out are `'static`.
//! - Are append-only: a slot, once assigned, never moves. Stored values are
//!   never freed, never reallocated. This is the load-bearing invariant
//!   that lets the public API hand out plain `&'static T` references.
//! - Have lock-free reads: looking up a slot is one atomic load through
//!   chunked storage (delegated to [`boxcar`]), no mutex acquired.
//! - Have serialised writes via [`dashmap`]'s entry API, but only for the
//!   "is this value already interned?" check; once that decides to insert,
//!   the actual append into the chunked storage is itself lock-free.
//!
//! Higher layers (per-kind arenas, the type arena, the boot routine, the
//! public construction API on `TypeId` / `ElementId`) build on top of
//! these primitives and do not introduce any additional concurrency
//! mechanism.

mod arena;
mod boot;
mod store;

pub use self::arena::Arena;
pub use self::arena::SliceArena;
pub use self::store::Interner;
pub use self::store::interner;
