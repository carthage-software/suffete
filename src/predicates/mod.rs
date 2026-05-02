//! Single-call predicates over a [`TypeId`].
//!
//! Each function answers one structural question. The naming is
//! consistent across the module:
//!
//! - **`is_X`** ; *guaranteed*: every element of the type is in
//!   family `X`. Conservative: returns `false` when any element is
//!   outside `X` (including `never` for the all-bottom type).
//! - **`contains_X`** ; *possible at the top level*: at least one
//!   top-level element of the type is in family `X`.
//! - **`is_truthy` / `is_falsy`** ; every element guaranteed
//!   truthy / falsy at runtime.
//! - **`could_be_truthy` / `could_be_falsy`** ; at least one element
//!   could be truthy / falsy.
//! - **`*_anywhere`** ; the question recurses through every nested-type
//!   carrier. Use these for "does this tree contain any unresolved
//!   atom" / "is there a free template anywhere" / etc.
//!
//! All predicates are pure functions of the [`TypeId`] (no `World`,
//! no options). Predicates that need the world (e.g. "is this
//! *effectively* an int after unwrapping generic constraints") belong
//! to the effective-type queries module, not here.

pub(crate) mod element;

use crate::ElementKind;
use crate::TypeId;
use crate::inspect;
use crate::prelude::TYPE_MIXED;
use crate::prelude::TYPE_NEVER;

/// `true` iff `ty` is the bottom type (no values).
#[inline]
#[must_use] 
pub fn is_never(ty: TypeId) -> bool {
    ty == TYPE_NEVER
}

/// `true` iff `ty` is the unconstrained top (`mixed` with no axes).
#[inline]
#[must_use] 
pub fn is_mixed(ty: TypeId) -> bool {
    ty == TYPE_MIXED
}

/// `true` iff `ty` is a single-element union.
#[inline]
#[must_use] 
pub fn is_singleton(ty: TypeId) -> bool {
    ty.as_ref().elements.len() == 1
}

/// `true` iff `ty` is a multi-element union.
#[inline]
#[must_use] 
pub fn is_union(ty: TypeId) -> bool {
    ty.as_ref().elements.len() > 1
}

/// Generates an `is_X` predicate over a top-level element-kind set.
macro_rules! is_kind {
    ($name:ident, $($kind:pat),+ $(,)?) => {
        #[inline]
        pub fn $name(ty: TypeId) -> bool {
            let elems = ty.as_ref().elements;
            !elems.is_empty() && elems.iter().all(|e| matches!(e.kind(), $($kind)|+))
        }
    };
}

is_kind!(is_int, ElementKind::Int);
is_kind!(is_float, ElementKind::Float);
is_kind!(is_string, ElementKind::String);
is_kind!(is_bool, ElementKind::Bool | ElementKind::True | ElementKind::False);
is_kind!(is_null, ElementKind::Null);
is_kind!(is_void, ElementKind::Void);
is_kind!(is_list, ElementKind::List);
is_kind!(is_keyed_array, ElementKind::Array);
is_kind!(is_array, ElementKind::Array | ElementKind::List);
is_kind!(is_iterable, ElementKind::Iterable);
is_kind!(
    is_object,
    ElementKind::Object
        | ElementKind::Enum
        | ElementKind::ObjectShape
        | ElementKind::HasMethod
        | ElementKind::HasProperty
        | ElementKind::ObjectAny
);
is_kind!(is_resource, ElementKind::Resource);
is_kind!(is_callable, ElementKind::Callable);
is_kind!(is_array_key, ElementKind::ArrayKey);
is_kind!(
    is_scalar,
    ElementKind::Scalar
        | ElementKind::Int
        | ElementKind::Float
        | ElementKind::String
        | ElementKind::Bool
        | ElementKind::True
        | ElementKind::False
        | ElementKind::ClassLikeString
        | ElementKind::Numeric
        | ElementKind::ArrayKey
);
is_kind!(is_numeric, ElementKind::Numeric | ElementKind::Int | ElementKind::Float);

/// Generates a `contains_X` predicate over a top-level element-kind set.
macro_rules! contains_kind {
    ($name:ident, $($kind:pat),+ $(,)?) => {
        #[inline]
        pub fn $name(ty: TypeId) -> bool {
            ty.as_ref().elements.iter().any(|e| matches!(e.kind(), $($kind)|+))
        }
    };
}

contains_kind!(contains_int, ElementKind::Int);
contains_kind!(contains_float, ElementKind::Float);
contains_kind!(contains_string, ElementKind::String);
contains_kind!(contains_bool, ElementKind::Bool | ElementKind::True | ElementKind::False);
contains_kind!(contains_null, ElementKind::Null);
contains_kind!(contains_void, ElementKind::Void);
contains_kind!(contains_array, ElementKind::Array | ElementKind::List);
contains_kind!(contains_iterable, ElementKind::Iterable);
contains_kind!(
    contains_object,
    ElementKind::Object
        | ElementKind::Enum
        | ElementKind::ObjectShape
        | ElementKind::HasMethod
        | ElementKind::HasProperty
        | ElementKind::ObjectAny
);
contains_kind!(contains_resource, ElementKind::Resource);
contains_kind!(contains_callable, ElementKind::Callable);
contains_kind!(contains_mixed, ElementKind::Mixed);

/// `true` iff every element of `ty` is guaranteed truthy at runtime.
/// Vacuously `false` for the empty type (`never`).
#[inline]
#[must_use] 
pub fn is_truthy(ty: TypeId) -> bool {
    let elems = ty.as_ref().elements;
    !elems.is_empty() && elems.iter().all(|&e| element::is_truthy(e))
}

/// `true` iff every element of `ty` is guaranteed falsy at runtime.
/// Vacuously `false` for the empty type (`never`).
#[inline]
#[must_use] 
pub fn is_falsy(ty: TypeId) -> bool {
    let elems = ty.as_ref().elements;
    !elems.is_empty() && elems.iter().all(|&e| element::is_falsy(e))
}

/// `true` iff at least one element of `ty` could be truthy at
/// runtime. `never` and `void` cannot be truthy; everything else that
/// isn't *guaranteed* falsy could be.
#[inline]
#[must_use] 
pub fn could_be_truthy(ty: TypeId) -> bool {
    ty.as_ref().elements.iter().any(|&e| element::could_be_truthy(e))
}

/// `true` iff at least one element of `ty` could be falsy at runtime.
/// `never` cannot be anything; `void` is treated as falsy per PHP
/// semantics.
#[inline]
#[must_use] 
pub fn could_be_falsy(ty: TypeId) -> bool {
    ty.as_ref().elements.iter().any(|&e| element::could_be_falsy(e))
}

/// `true` iff every element of `ty` is a literal-shaped value
/// (specific int / float / string literal, `true`, `false`, `null`,
/// `void`).
#[inline]
#[must_use] 
pub fn is_literal(ty: TypeId) -> bool {
    let elems = ty.as_ref().elements;
    !elems.is_empty() && elems.iter().all(|&e| element::is_literal(e))
}

/// `true` iff `ty` is a single literal element. Equivalent to
/// `is_literal(ty) && is_singleton(ty)`. The most useful "can I
/// constant-fold this?" check.
#[inline]
#[must_use] 
pub fn is_constant_foldable(ty: TypeId) -> bool {
    is_singleton(ty) && is_literal(ty)
}

/// `true` iff any element anywhere in `ty`'s tree is a `mixed`
/// (the family-level top, including narrowed mixed variants).
#[inline]
#[must_use] 
pub fn contains_mixed_anywhere(ty: TypeId) -> bool {
    inspect::any(ty, |e| e.kind() == ElementKind::Mixed)
}

/// `true` iff any element anywhere in `ty`'s tree is a free template
/// parameter.
#[inline]
#[must_use] 
pub fn contains_template_anywhere(ty: TypeId) -> bool {
    inspect::any(ty, |e| e.kind() == ElementKind::GenericParameter)
}

/// `true` iff any element anywhere in `ty`'s tree is a placeholder
/// (the inference-time hole `?`).
#[inline]
#[must_use] 
pub fn contains_placeholder_anywhere(ty: TypeId) -> bool {
    inspect::any(ty, |e| e.kind() == ElementKind::Placeholder)
}

/// `true` iff any element anywhere in `ty`'s tree is unresolved.
///
/// Unresolved means `Alias`, `Reference`, `MemberReference`,
/// `GlobalReference`, `Conditional`, or `Derived`. The analyser
/// typically needs to call `expand` on such a type before doing
/// further reasoning.
#[inline]
#[must_use] 
pub fn contains_unresolved_anywhere(ty: TypeId) -> bool {
    inspect::any(ty, |e| {
        matches!(
            e.kind(),
            ElementKind::Alias
                | ElementKind::Reference
                | ElementKind::MemberReference
                | ElementKind::GlobalReference
                | ElementKind::Conditional
                | ElementKind::Derived
        )
    })
}

/// `true` iff `ty`'s tree contains no unresolved atom (no `Alias`,
/// `Reference`, `MemberReference`, `GlobalReference`, `Conditional`,
/// or `Derived` at any depth).
#[inline]
#[must_use] 
pub fn is_fully_resolved(ty: TypeId) -> bool {
    !contains_unresolved_anywhere(ty)
}
