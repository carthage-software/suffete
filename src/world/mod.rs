//! The boundary between suffete's pure type system and the analyzer's
//! view of the codebase being analyzed.
//!
//! Suffete answers questions about types in isolation: "is `int` a
//! subtype of `float`?", "what is the join of `int|null` and
//! `string|null`?". Many real-world questions also depend on facts the
//! surrounding analyzer knows: "does class `Foo` extend `Bar`?", "what
//! type parameters does `Container` declare?", "what type does `Box<T>
//! extends Wrapper<T>` pass to `Wrapper`'s template?".
//!
//! Those facts live in the analyzer (a static analyzer, a language
//! server, mock fixtures for tests) and are exposed to suffete via the
//! [`World`] trait. Each lattice operation, future narrowing operation,
//! and structural analysis takes a `&impl World` so the type system can
//! ask whatever it needs without knowing how the analyzer stores it.
//!
//! "World" is the universe of class-likes / functions / templates the
//! analyzer has scanned. A `MockWorld` in tests carries a hand-built
//! hierarchy; a real analyzer's impl reads from its symbol table.

mod template;

use mago_atom::Atom;

pub use self::template::TemplateParameter;
pub use self::template::Variance;

use crate::TypeId;
use crate::element::payload::Visibility;

/// What suffete needs to know about the codebase being analyzed.
///
/// All methods are queries — single-purpose lookups, never returning
/// collections. This lets implementations store metadata however they
/// like (`HashMap`, indexed `Vec`, persistent tree, database) without
/// suffete forcing a particular shape. It also keeps the trait
/// dyn-compatible.
///
/// All methods are required: the trait gives no defaults, so an
/// implementation can't accidentally leave a query unanswered. A "this
/// world knows nothing" implementation should return `false` / `0` /
/// `None` explicitly (see [`NullWorld`]).
pub trait World {
    /// `true` iff `child` is the same class-like as `ancestor`, or
    /// extends / implements / uses-trait it transitively.
    fn descends_from(&self, child: Atom, ancestor: Atom) -> bool;

    /// `true` iff `class` directly `use`s `trait_` (the trait appears
    /// in `class`'s body as `use TraitName;`).
    ///
    /// Asymmetric vs [`descends_from`](Self::descends_from), which
    /// closes over inheritance: `descends_from` returns `true` for any
    /// trait in the chain, but `uses_trait` only for direct usage.
    fn uses_trait(&self, class: Atom, trait_: Atom) -> bool;

    /// How many type parameters `class` declares. `0` for unknown or
    /// non-generic classes.
    fn template_parameter_arity(&self, class: Atom) -> usize;

    /// The type parameter at `position` in `class`'s declaration, or
    /// `None` if `position >= template_parameter_arity(class)`.
    fn template_parameter_at(&self, class: Atom, position: usize) -> Option<TemplateParameter>;

    /// The position of `class`'s type parameter named `name`, or `None`
    /// if no such parameter exists.
    fn template_parameter_index(&self, class: Atom, name: Atom) -> Option<usize>;

    /// The type `child` passes to `ancestor`'s `position`-th type
    /// parameter, expressed in `child`'s template namespace.
    ///
    /// For `class B<T> extends A<string>` with `inherited_template_argument(B,
    /// A, 0)`, returns `Some(string)`. For `class B<T> extends
    /// A<List<T>>`, returns `Some(List<T>)` — caller substitutes
    /// `child`'s actual arguments to fully resolve.
    ///
    /// Returns `None` when `child` does not descend from `ancestor`,
    /// or when `position >= template_parameter_arity(ancestor)`.
    fn inherited_template_argument(&self, child: Atom, ancestor: Atom, position: usize) -> Option<TypeId>;

    /// `true` iff `class` declares or inherits a method named `method`.
    /// Mirrors PHP's `method_exists()` semantics: walks the inheritance
    /// closure (parent classes, implemented interfaces, used traits).
    fn class_has_method(&self, class: Atom, method: Atom) -> bool;

    /// The declared type of `property` on `class`, walking the
    /// inheritance closure. `None` when the property is absent or its
    /// type is unknown.
    ///
    /// Used by [`crate::lattice`] for object-shape compatibility:
    /// `Named(C) <: object{p: T}` requires `C` to declare `p` with a
    /// type that refines `T`.
    fn class_property_type(&self, class: Atom, property: Atom) -> Option<TypeId>;

    /// What kind of enum `enum_name` is.
    ///
    /// Returns `None` when the enum is unknown (or `enum_name` does not
    /// name an enum). The lattice treats `None` conservatively: a
    /// structural narrowing that depends on the backing (e.g. a
    /// `value` property on an `object{...}` shape) is rejected.
    fn enum_backing(&self, enum_name: Atom) -> Option<EnumBacking>;

    /// The recorded body of `class::alias` (a `@type` alias declared on
    /// the class), or `None` when the alias is unknown. Used by
    /// [`crate::expand`] to substitute alias bodies in place of
    /// `Alias` atoms.
    fn alias_body(&self, class: Atom, alias: Atom) -> Option<TypeId>;

    /// The declared or inferred type of `class::constant`. `None` when
    /// the constant is unknown. Used by [`crate::expand`] to resolve
    /// `MemberReference` atoms with an `Identifier` selector.
    fn class_constant_type(&self, class: Atom, constant: Atom) -> Option<TypeId>;

    /// The declared or inferred type of a global constant or function
    /// signature. `None` when the name is unknown. Used by
    /// [`crate::expand`] to resolve `GlobalReference` atoms.
    fn global_constant_type(&self, name: Atom) -> Option<TypeId>;

    /// How many properties `class` declares or inherits (visible from
    /// `class`'s scope).
    fn class_property_count(&self, class: Atom) -> usize;

    /// The `position`-th property of `class`, walking the inheritance
    /// closure in declaration order. Used by [`crate::expand`] to
    /// build the shape returned by `properties-of<C>`.
    fn class_property_at(&self, class: Atom, position: usize) -> Option<ClassProperty>;
}

/// One declared property of a class-like, returned by
/// [`World::class_property_at`].
#[derive(Debug, Clone, Copy)]
pub struct ClassProperty {
    pub name: Atom,
    pub type_: TypeId,
    pub visibility: Visibility,
}

/// What an enum case carries beyond its `name`. PHP enums are either
/// pure (only `name`) or backed by `int` / `string` (carrying a `value`
/// property of that scalar type).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EnumBacking {
    /// Pure enum (`enum X { case A; }`). Cases expose only `name`.
    Pure,
    /// Backed enum (`enum X: string { case A = 'a'; }`). Cases expose
    /// `name` and `value`. The wrapped [`TypeId`] is the backing type
    /// — typically `int` or `string`.
    Backed(TypeId),
}

/// A no-op [`World`] for queries that don't consult the codebase.
///
/// Every lookup returns the empty / negative answer. Suitable when the
/// input types contain only scalar / trivial elements and no object /
/// generic / reference machinery would be exercised.
pub struct NullWorld;

impl World for NullWorld {
    fn descends_from(&self, _child: Atom, _ancestor: Atom) -> bool {
        false
    }

    fn uses_trait(&self, _class: Atom, _trait_: Atom) -> bool {
        false
    }

    fn template_parameter_arity(&self, _class: Atom) -> usize {
        0
    }

    fn template_parameter_at(&self, _class: Atom, _position: usize) -> Option<TemplateParameter> {
        None
    }

    fn template_parameter_index(&self, _class: Atom, _name: Atom) -> Option<usize> {
        None
    }

    fn inherited_template_argument(&self, _child: Atom, _ancestor: Atom, _position: usize) -> Option<TypeId> {
        None
    }

    fn class_has_method(&self, _class: Atom, _method: Atom) -> bool {
        false
    }

    fn class_property_type(&self, _class: Atom, _property: Atom) -> Option<TypeId> {
        None
    }

    fn enum_backing(&self, _enum_name: Atom) -> Option<EnumBacking> {
        None
    }

    fn alias_body(&self, _class: Atom, _alias: Atom) -> Option<TypeId> {
        None
    }

    fn class_constant_type(&self, _class: Atom, _constant: Atom) -> Option<TypeId> {
        None
    }

    fn global_constant_type(&self, _name: Atom) -> Option<TypeId> {
        None
    }

    fn class_property_count(&self, _class: Atom) -> usize {
        0
    }

    fn class_property_at(&self, _class: Atom, _position: usize) -> Option<ClassProperty> {
        None
    }
}
