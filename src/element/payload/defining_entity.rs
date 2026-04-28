use std::mem::size_of;

use mago_atom::Atom;

use crate::handle::define_handle;

define_handle! {
    /// Handle to an interned [`DefiningEntity`].
    ///
    /// Heavy reuse: every `@template T` declared in the same class or method
    /// shares one entity, and every reference to such a template
    /// ([`GenericParameterInfo`](crate::payload::GenericParameterInfo),
    /// [`ClassLikeStringSpecifier::Generic`](crate::payload::ClassLikeStringSpecifier))
    /// dedupes to that one handle.
    DefiningEntityId
}

/// The class-or-function in whose scope a generic parameter or class-like-
/// string template is declared.
///
/// `Function` covers free functions; `Method` covers methods on a class
/// (`(class, method)` pair); `ClassLike` covers a class/interface/trait/enum
/// declaring class-level templates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DefiningEntity {
    ClassLike(Atom),
    Method { class: Atom, method: Atom },
    Function(Atom),
}

const _: () = assert!(size_of::<DefiningEntity>() <= 24);
