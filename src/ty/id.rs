use std::num::NonZeroU64;

use crate::ElementId;
use crate::FlowFlags;
use crate::Type;
use crate::handle::define_handle;

/// An interned handle to a [`Type`](crate::Type).
///
/// Layout: `NonZeroU64`, packed as
///
/// ```text
/// [slot: 32 bits] [flags: 16 bits] [meta: 8 bits] [reserved: 8 bits]
/// ```
///
/// - **`slot`** is the index into the type-content arena. Two `TypeId`s
///   with the same slot share the same interned [`Type`] (i.e. the same
///   element-set). The arena keys on content only, so adding or
///   removing flow flags does not allocate a new slot.
/// - **`flags`** is the [`FlowFlags`] bitset. Riding on the handle keeps
///   the arena content-keyed; toggling a flag is bit-twiddling, not a
///   re-intern.
/// - **`meta`** is 8 bits of consumer-defined storage. Suffete never
///   inspects it. Use it for tag-style metadata (provenance enum,
///   severity, boolean markers); for anything that needs more bits or
///   indexes a side table, the consumer should keep their own
///   `HashMap<TypeId, T>`.
/// - **`reserved`** is reserved for future suffete use; always zero.
///   Not exposed publicly.
///
/// Equality and hashing compare all 64 bits, so `t1 == t2` means
/// "same content, same flags, same meta". Use [`TypeId::content_eq`]
/// for content-only comparison, and [`TypeId::with_flags`] /
/// [`TypeId::with_meta`] to derive related handles in O(1) without
/// touching the arena.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TypeId(NonZeroU64);

const SLOT_BITS: u32 = 32;
const FLAGS_BITS: u32 = 16;
const META_BITS: u32 = 8;
const RESERVED_BITS: u32 = 8;

const _: () = assert!(SLOT_BITS + FLAGS_BITS + META_BITS + RESERVED_BITS == 64);

const SLOT_SHIFT: u32 = FLAGS_BITS + META_BITS + RESERVED_BITS;
const FLAGS_SHIFT: u32 = META_BITS + RESERVED_BITS;
const META_SHIFT: u32 = RESERVED_BITS;

const SLOT_MASK: u64 = ((1u64 << SLOT_BITS) - 1) << SLOT_SHIFT;
const FLAGS_MASK: u64 = ((1u64 << FLAGS_BITS) - 1) << FLAGS_SHIFT;
const META_MASK: u64 = ((1u64 << META_BITS) - 1) << META_SHIFT;

impl TypeId {
    /// Construct from a raw slot index (1-based; slot `0` is reserved
    /// for the `NonZero` niche). Flags and meta are zero. Reserved for
    /// the interner.
    #[inline]
    pub(crate) const fn from_slot(slot: u32) -> Self {
        // SAFETY: caller is responsible for `slot != 0`. Used only by the
        // interner and the prelude well-known constants.
        unsafe { Self(NonZeroU64::new_unchecked((slot as u64) << SLOT_SHIFT)) }
    }

    /// Construct from the encoded `(slot, flags, meta)` triple. Reserved
    /// for the interner.
    #[inline]
    pub(crate) fn from_parts(slot: u32, flags: FlowFlags, meta: u8) -> Self {
        let bits =
            ((slot as u64) << SLOT_SHIFT) | ((flags.bits() as u64) << FLAGS_SHIFT) | ((meta as u64) << META_SHIFT);
        // SAFETY: slot is 1-based; the high SLOT_BITS are non-zero, so the
        // whole word is non-zero.
        unsafe { Self(NonZeroU64::new_unchecked(bits)) }
    }

    /// Arena slot — content identity, ignoring flags and meta.
    #[inline]
    pub(crate) const fn slot(self) -> u32 {
        ((self.0.get() & SLOT_MASK) >> SLOT_SHIFT) as u32
    }

    /// Flow flags carried on the handle.
    #[inline]
    pub const fn flags(self) -> FlowFlags {
        FlowFlags::from_bits(((self.0.get() & FLAGS_MASK) >> FLAGS_SHIFT) as u16)
    }

    /// 8 bits of consumer-defined metadata. Suffete never inspects this
    /// field; the value is whatever the consumer last wrote with
    /// [`TypeId::with_meta`] (default `0`).
    #[inline]
    pub const fn meta(self) -> u8 {
        ((self.0.get() & META_MASK) >> META_SHIFT) as u8
    }

    /// Same content slot, with `flags` substituted. O(1), no arena hit.
    #[inline]
    #[must_use]
    pub fn with_flags(self, flags: FlowFlags) -> Self {
        Self::from_parts(self.slot(), flags, self.meta())
    }

    /// Same content slot and flags, with `meta` substituted. O(1), no
    /// arena hit.
    #[inline]
    #[must_use]
    pub fn with_meta(self, meta: u8) -> Self {
        Self::from_parts(self.slot(), self.flags(), meta)
    }

    /// `true` iff `self` and `other` refer to the same content slot,
    /// regardless of flags or meta.
    #[inline]
    pub const fn content_eq(self, other: Self) -> bool {
        self.slot() == other.slot()
    }

    /// Resolve this handle to its [`Type`](crate::Type) value via the
    /// process-global interner. The returned [`Type`] holds only the
    /// element set; flags travel on the handle, not in the arena.
    ///
    /// # Panics
    ///
    /// Panics if the slot is not present, which can only happen when the
    /// handle was forged or constructed before the boot routine ran.
    #[allow(clippy::should_implement_trait)]
    #[inline]
    pub fn as_ref(self) -> &'static Type {
        crate::interner::interner().get_type(self)
    }

    /// Build a singleton union from one element, with empty flow flags.
    #[inline]
    pub fn singleton(element: ElementId) -> Self {
        Self::union(&[element])
    }

    /// Build a union from a slice of elements, with empty flow flags.
    ///
    /// The interner sorts and deduplicates the slice for canonical
    /// handle identity, but applies **no merge rules**: `[TRUE, FALSE]`
    /// does not collapse to `BOOL`, dominator absorption does not
    /// fire, range unions are not merged. Callers that want the
    /// lattice-canonical form route through
    /// [`crate::join::compute`] explicitly.
    ///
    /// Empty input collapses to `[never]`.
    #[inline]
    pub fn union(elements: &[ElementId]) -> Self {
        crate::interner::interner().intern_type(elements, FlowFlags::EMPTY)
    }

    /// Singleton type wrapping a literal integer.
    #[inline]
    pub fn int_literal(value: i64) -> Self {
        Self::singleton(ElementId::int_literal(value))
    }

    /// Singleton type wrapping an integer range. Either bound may be `None`
    /// for open (`-∞` / `+∞`).
    #[inline]
    pub fn int_range(lower: Option<i64>, upper: Option<i64>) -> Self {
        Self::singleton(ElementId::int_range(lower, upper))
    }

    /// Singleton type wrapping a literal float.
    #[inline]
    pub fn float_literal(value: f64) -> Self {
        Self::singleton(ElementId::float_literal(value))
    }

    /// Singleton type wrapping a literal string.
    #[inline]
    pub fn string_literal(value: &str) -> Self {
        Self::singleton(ElementId::string_literal(value))
    }
}

define_handle! {
    /// Handle to an interned `&'static [TypeId]`. Used by payloads that
    /// carry a sequence of type arguments (object generic args, callable
    /// parameter type lists, conditional/derived input lists).
    TypeListId
}

impl std::fmt::Display for TypeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.as_ref(), f)
    }
}

impl std::fmt::Debug for TypeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            return f.debug_struct("TypeId")
                .field("raw", &format_args!("{:#018x}", self.0.get()))
                .field("slot", &self.slot())
                .field("flags", &self.flags())
                .field("meta", &self.meta())
                .field("display", &format_args!("{}", self))
                .finish();
        }
        write!(f, "TypeId({:#018x}", self.0.get())?;
        if self.flags() != FlowFlags::EMPTY {
            write!(f, ", flags={:?}", self.flags())?;
        }
        if self.meta() != 0 {
            write!(f, ", meta={:#x}", self.meta())?;
        }
        write!(f, ": {})", self)
    }
}

impl crate::typed::Typed for TypeId {
    fn pretty_with_indent(&self, indent: usize) -> String {
        crate::typed::Typed::pretty_with_indent(self.as_ref(), indent)
    }

    fn intersection_types(&self) -> &'static [crate::ElementId] {
        &[]
    }

    fn has_intersection_types(&self) -> bool {
        false
    }

    fn can_be_intersected(&self) -> bool {
        false
    }

    fn is_complex(&self) -> bool {
        crate::typed::Typed::is_complex(self.as_ref())
    }
}
