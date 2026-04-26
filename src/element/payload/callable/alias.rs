use std::mem::size_of;

use mago_atom::Atom;
use mago_span::Span;

use crate::handle::define_handle;

define_handle! {
    /// Handle to an interned [`CallableAlias`]. Pulled out so
    /// [`CallableInfo`](super::CallableInfo) itself stays at 8 bytes.
    CallableAliasId
}

/// A reference to a known callable: a free function, a method on a class,
/// or a closure expression at a known source position.
///
/// Functions are referenceable by name (`Atom`), methods by `(class, method)`,
/// and closures need a source [`Span`] because they have no name. Each variant
/// gets its own canonical entry in the [`CallableAliasId`] interner; two
/// references to the same function/method/closure share one slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CallableAlias {
    Function(Atom),
    Method { class: Atom, method: Atom },
    Closure(Span),
}

// `Method` is the size driver among the name-based variants (16 bytes payload).
// `Closure(Span)` may match or exceed that depending on `mago_span::Span`'s
// layout. The whole alias is interned via `CallableAliasId`, so a slightly
// larger entry costs one allocation per *unique* callable, not per use site.
const _: () = assert!(size_of::<CallableAlias>() <= 40);
