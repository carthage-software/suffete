use crate::element::payload::AliasInfo;
use crate::element::payload::CallableInfo;
use crate::element::payload::ClassLikeStringInfo;
use crate::element::payload::ConditionalInfo;
use crate::element::payload::DerivedInfo;
use crate::element::payload::EnumInfo;
use crate::element::payload::GenericParameterInfo;
use crate::element::payload::GlobalReference;
use crate::element::payload::HasMethodInfo;
use crate::element::payload::HasPropertyInfo;
use crate::element::payload::IterableInfo;
use crate::element::payload::KeyedArrayInfo;
use crate::element::payload::ListInfo;
use crate::element::payload::MemberReference;
use crate::element::payload::MixedInfo;
use crate::element::payload::ObjectInfo;
use crate::element::payload::ObjectShapeInfo;
use crate::element::payload::ResourceInfo;
use crate::element::payload::SymbolReference;
use crate::element::payload::VariableInfo;
use crate::element::payload::scalar::FloatInfo;
use crate::element::payload::scalar::IntInfo;
use crate::element::payload::scalar::StringInfo;

/// A borrowed view into an [`ElementId`](crate::ElementId)'s payload.
///
/// Returned by [`ElementId::view`](crate::ElementId::view). Trivial-kind
/// variants (no payload) are unit-like; payload-bearing variants carry a
/// `&'static` reference into the matching per-kind arena.
///
/// `Element` is `Copy` (every variant is either tag-only or a thin pointer),
/// so consumers can match-then-discard freely.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Element {
    Null,
    Never,
    Void,
    Placeholder,
    Bool,
    True,
    False,
    Scalar,
    Numeric,
    ArrayKey,
    ObjectAny,
    Mixed(&'static MixedInfo),
    Int(&'static IntInfo),
    Float(&'static FloatInfo),
    String(&'static StringInfo),
    ClassLikeString(&'static ClassLikeStringInfo),
    Object(&'static ObjectInfo),
    Enum(&'static EnumInfo),
    ObjectShape(&'static ObjectShapeInfo),
    HasMethod(&'static HasMethodInfo),
    HasProperty(&'static HasPropertyInfo),
    Array(&'static KeyedArrayInfo),
    List(&'static ListInfo),
    Iterable(&'static IterableInfo),
    Callable(&'static CallableInfo),
    Resource(&'static ResourceInfo),
    GenericParameter(&'static GenericParameterInfo),
    Variable(&'static VariableInfo),
    Reference(&'static SymbolReference),
    MemberReference(&'static MemberReference),
    GlobalReference(&'static GlobalReference),
    Alias(&'static AliasInfo),
    Conditional(&'static ConditionalInfo),
    Derived(&'static DerivedInfo),
}
