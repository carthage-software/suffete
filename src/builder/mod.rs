//! Build-then-finalise scratch buffer for [`TypeId`] mutation.
//!
//! Every mutation through [`TypeId`]'s constructors round-trips the
//! result through the interner: sort, dedup, canonicalise, hash,
//! dashmap lookup. For consumers that perform many mutations on the
//! same type before observing the result (assertion-handling loops,
//! per-statement type evolution, switch-arm merging) the per-step
//! interner cost dominates.
//!
//! [`TypeBuilder`] solves this by holding the elements and flow flags
//! in an owned scratch buffer. Mutations are direct `Vec` operations;
//! [`build`](TypeBuilder::build) runs the canonicalisation and intern
//! exactly once at the end. The same `TypeBuilder` instance can be
//! reused across many mutations: only the final `build` pays the
//! intern cost.
//!
//! # Origin short-circuit
//!
//! When constructed with [`from_type`](TypeBuilder::from_type), the
//! builder remembers the originating handle. If `build` is reached
//! with the buffer in the same shape (same element sequence, same
//! flags), it returns the original handle directly — no canonicalise,
//! no intern lookup. A buffer that diverges from the origin (any
//! mutation, any flag flip) and then returns to the origin shape is
//! still considered "changed" and rebuilt; tracking the actual diff
//! would defeat the point.
//!
//! # Querying mid-sequence
//!
//! The builder does **not** expose `refines` / `overlaps` / `meet`
//! against the in-progress buffer. Those operations need a `TypeId`.
//! Call [`build`](TypeBuilder::build) to finalise, query, then open a
//! fresh builder if more mutations follow.

use crate::ElementId;
use crate::FlowFlags;
use crate::TypeId;

/// Mutable scratch buffer for accumulating changes to a type before
/// committing the result through the interner.
///
/// See the [module documentation](self) for the rationale and
/// short-circuit semantics.
#[derive(Debug, Clone)]
pub struct TypeBuilder {
    elements: Vec<ElementId>,
    flags: FlowFlags,
    origin: Option<TypeId>,
    dirty: bool,
}

impl TypeBuilder {
    /// Construct an empty builder. [`build`](Self::build) will collapse
    /// to [`prelude::TYPE_NEVER`](crate::prelude::TYPE_NEVER) (matching
    /// the existing `TypeId::union(&[])` convention).
    #[inline]
    pub fn new() -> Self {
        Self { elements: Vec::new(), flags: FlowFlags::EMPTY, origin: None, dirty: false }
    }

    /// Open a builder backed by `ty`'s elements and flags. The
    /// origin handle is remembered so an unmodified `build()` returns
    /// the same `TypeId` without re-interning.
    pub fn from_type(ty: TypeId) -> Self {
        let view = ty.as_ref();
        Self { elements: view.elements.to_vec(), flags: ty.flags(), origin: Some(ty), dirty: false }
    }

    /// Current element buffer, in mutation order (not yet sorted /
    /// deduplicated / canonicalised). Cheap.
    #[inline]
    pub fn elements(&self) -> &[ElementId] {
        &self.elements
    }

    /// Current flow flags.
    #[inline]
    pub fn flags(&self) -> FlowFlags {
        self.flags
    }

    /// `true` iff the buffer contains no elements yet.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// `true` iff the buffer contains at least one occurrence of
    /// `element`. O(n) on the buffer length; intended for predicate
    /// dispatch in the same loop that mutates.
    #[inline]
    pub fn contains(&self, element: ElementId) -> bool {
        self.elements.contains(&element)
    }

    /// Number of elements currently in the buffer.
    #[inline]
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Append `element` to the buffer. Order is preserved during
    /// mutation; `build()` sorts before interning.
    pub fn push(&mut self, element: ElementId) -> &mut Self {
        self.elements.push(element);
        self.dirty = true;
        self
    }

    /// Append every element from `iter`.
    pub fn extend<I: IntoIterator<Item = ElementId>>(&mut self, iter: I) -> &mut Self {
        let before = self.elements.len();
        self.elements.extend(iter);
        if self.elements.len() != before {
            self.dirty = true;
        }

        self
    }

    /// Remove the first occurrence of `element`. No-op when absent.
    pub fn remove(&mut self, element: ElementId) -> &mut Self {
        if let Some(idx) = self.elements.iter().position(|e| *e == element) {
            self.elements.remove(idx);
            self.dirty = true;
        }
        self
    }

    /// Remove every occurrence of `element`.
    pub fn remove_all(&mut self, element: ElementId) -> &mut Self {
        let before = self.elements.len();
        self.elements.retain(|e| *e != element);
        if self.elements.len() != before {
            self.dirty = true;
        }

        self
    }

    /// Keep only elements for which `predicate` returns `true`.
    pub fn retain<F: FnMut(&ElementId) -> bool>(&mut self, mut predicate: F) -> &mut Self {
        let before = self.elements.len();
        self.elements.retain(|e| predicate(e));
        if self.elements.len() != before {
            self.dirty = true;
        }

        self
    }

    /// Replace the first occurrence of `old` with `new`. No-op when
    /// `old` is absent.
    pub fn replace(&mut self, old: ElementId, new: ElementId) -> &mut Self {
        if let Some(idx) = self.elements.iter().position(|e| *e == old)
            && self.elements[idx] != new
        {
            self.elements[idx] = new;
            self.dirty = true;
        }

        self
    }

    /// Apply `f` to every element, replacing each in place.
    pub fn map<F: FnMut(ElementId) -> ElementId>(&mut self, mut f: F) -> &mut Self {
        for slot in self.elements.iter_mut() {
            let new = f(*slot);
            if new != *slot {
                *slot = new;
                self.dirty = true;
            }
        }

        self
    }

    /// Apply `f` to every element, expanding each to zero or more
    /// elements. Useful for narrowing patterns where one atom
    /// decomposes into a union (e.g. an integer range split).
    pub fn flat_map<I, F>(&mut self, mut f: F) -> &mut Self
    where
        I: IntoIterator<Item = ElementId>,
        F: FnMut(ElementId) -> I,
    {
        let original = std::mem::take(&mut self.elements);
        let mut rebuilt = Vec::with_capacity(original.len());
        let mut changed = false;
        for elem in original {
            let mut iter = f(elem).into_iter();
            match (iter.next(), iter.next()) {
                (Some(only), None) => {
                    if only != elem {
                        changed = true;
                    }
                    rebuilt.push(only);
                }
                (Some(first), Some(second)) => {
                    changed = true;
                    rebuilt.push(first);
                    rebuilt.push(second);
                    rebuilt.extend(iter);
                }
                (None, _) => {
                    changed = true;
                }
            }
        }

        self.elements = rebuilt;
        if changed {
            self.dirty = true;
        }

        self
    }

    /// Replace the entire flow-flag set.
    pub fn set_flags(&mut self, flags: FlowFlags) -> &mut Self {
        if flags != self.flags {
            self.flags = flags;
            self.dirty = true;
        }

        self
    }

    /// Apply `f` to the current flow flags, replacing them with the
    /// returned value.
    pub fn modify_flags<F: FnOnce(FlowFlags) -> FlowFlags>(&mut self, f: F) -> &mut Self {
        let new = f(self.flags);
        if new != self.flags {
            self.flags = new;
            self.dirty = true;
        }

        self
    }

    /// Finalise the buffer through the type interner. Returns the
    /// original `TypeId` directly when the buffer is unchanged from a
    /// [`from_type`](Self::from_type) origin (no intern roundtrip in
    /// that case).
    ///
    /// The interner sorts and deduplicates the element slice for
    /// canonical handle identity, but applies **no merge rules**:
    /// `[TRUE, FALSE]` does not collapse to `BOOL`, range unions are
    /// not merged, refinements are not absorbed. Callers that want
    /// the full lattice-canonical form route through the combiner
    /// (the `join` module, once it grows the payload-driven rules).
    ///
    /// Empty buffers collapse to
    /// [`prelude::TYPE_NEVER`](crate::prelude::TYPE_NEVER), matching the
    /// interner's empty-input convention.
    pub fn build(self) -> TypeId {
        if !self.dirty
            && let Some(origin) = self.origin
        {
            return origin;
        }

        crate::interner::interner().intern_type(&self.elements, self.flags)
    }

    /// Finalise the buffer through the join's payload-driven merge
    /// rules: refined-int range merging, string-axis collapse,
    /// scalar synthesis, list/keyed-array element-type union, and
    /// subtype-driven absorption. Use [`build`](Self::build) when the
    /// caller does not want these collapses applied.
    pub fn build_canonical(self) -> TypeId {
        let opts = crate::join::JoinOptions::default()
            .with_absorb_refinements(true)
            .with_merge_int_ranges(true)
            .with_merge_string_axes(true)
            .with_merge_array_shapes(true)
            .with_merge_list_element_types(true)
            .with_merge_keyed_array_params(true)
            .with_synthesise_scalar(true);
        let canon = crate::join::compute_with(&self.elements, &opts);
        crate::interner::interner().intern_type(&canon, self.flags)
    }
}

impl Default for TypeBuilder {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl From<TypeId> for TypeBuilder {
    #[inline]
    fn from(ty: TypeId) -> Self {
        Self::from_type(ty)
    }
}
