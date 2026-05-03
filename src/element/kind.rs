/// The top-level dispatch tag for a single [`Element`](crate::Element) of a type.
///
/// Every [`ElementId`](crate::ElementId) packs an `ElementKind` in its high bits, so checking
/// "what family does this element belong to?" is one bit-mask away from no arena lookup at all.
///
/// Discriminants start at `1`. Discriminant `0` is reserved as the niche that makes
/// [`ElementId`](crate::ElementId) (and [`TypeId`](crate::TypeId)) `NonZero`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ElementKind {
    Null = 1,
    Never,
    Void,
    Placeholder,
    Mixed,
    Bool,
    True,
    False,
    Int,
    Float,
    String,
    ClassLikeString,
    Scalar,
    Numeric,
    ArrayKey,
    Object,
    Enum,
    ObjectShape,
    HasMethod,
    HasProperty,
    Array,
    List,
    Iterable,
    Callable,
    Resource,
    GenericParameter,
    Variable,
    Reference,
    MemberReference,
    GlobalReference,
    Alias,
    Conditional,
    Derived,
    ObjectAny,
    /// `!T`, the complement of `T` against the universal type
    /// (`mixed`). Carries a single inner [`TypeId`].
    Negated,
    /// `head & conj1 & conj2 & ...`. The universal intersection
    /// wrapper: any element kind can carry conjunct narrowings via
    /// this element. Carries a `head` [`ElementId`] plus a non-empty
    /// list of `conjuncts`. The interner enforces canonicalization
    /// (sorted/deduped conjuncts, head extraction when only one
    /// element is present, never produced with empty conjuncts).
    Intersected,
}

impl ElementKind {
    /// `true` when the kind is fully described by its tag, with no arena slot
    /// needed and no payload to look up. Trivial elements have a single
    /// canonical instance.
    #[inline]
    #[must_use]
    pub const fn is_trivial(self) -> bool {
        matches!(
            self,
            Self::Null
                | Self::Never
                | Self::Void
                | Self::Placeholder
                | Self::Bool
                | Self::True
                | Self::False
                | Self::Scalar
                | Self::Numeric
                | Self::ArrayKey
                | Self::ObjectAny
        )
    }
}
