use std::mem::size_of;

use crate::ElementId;

/// A union of one or more [`Element`](crate::Element)s.
///
/// `elements` is sorted, deduplicated, and lives in the slice arena, so two
/// types with the same element set share one slice.
///
/// Flow flags do **not** live here — they ride on the [`TypeId`](crate::TypeId)
/// itself, so the same content shares a single arena slot regardless of
/// the flag combinations the consumer wraps it in. Read flags via
/// [`TypeId::flags`](crate::TypeId::flags); the [`Type`] value behind the
/// handle is content-only.
///
/// Construct via [`Interner::intern_type`](crate::interner::Interner::intern_type)
/// (or the wrappers on [`TypeId`](crate::TypeId)). Read via
/// [`TypeId::as_ref`](crate::TypeId::as_ref).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Type {
    pub elements: &'static [ElementId],
}

const _: () = assert!(size_of::<Type>() <= 16);
