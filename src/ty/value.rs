use std::mem::size_of;

use crate::ElementId;
use crate::FlowFlags;

/// A union of one or more [`Element`](crate::Element)s, plus flow flags.
///
/// `atoms` is sorted, deduplicated, and lives in the slice arena, so two
/// types with the same atom set share one slice. Equality and hashing are
/// trivial: a `Type` is two pointer-sized fields and one `u16`.
///
/// Construct via [`Interner::intern_type`](crate::interner::Interner::intern_type)
/// (or the wrappers on [`TypeId`](crate::TypeId)). Read via
/// [`TypeId::as_ref`](crate::TypeId::as_ref).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Type {
    pub atoms: &'static [ElementId],
    pub flags: FlowFlags,
}

const _: () = assert!(size_of::<Type>() <= 24);
