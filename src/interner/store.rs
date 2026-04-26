use std::sync::OnceLock;

use crate::ElementId;
use crate::ElementKind;
use crate::ElementListId;
use crate::FlowFlags;
use crate::Type;
use crate::TypeId;
use crate::TypeListId;
use crate::element::payload::AliasInfo;
use crate::element::payload::CallableAlias;
use crate::element::payload::CallableAliasId;
use crate::element::payload::CallableInfo;
use crate::element::payload::ClassLikeStringInfo;
use crate::element::payload::ConditionalInfo;
use crate::element::payload::DefiningEntity;
use crate::element::payload::DefiningEntityId;
use crate::element::payload::DerivedInfo;
use crate::element::payload::EnumInfo;
use crate::element::payload::GenericParameterInfo;
use crate::element::payload::GlobalReference;
use crate::element::payload::HasMethodInfo;
use crate::element::payload::HasPropertyInfo;
use crate::element::payload::IntRange;
use crate::element::payload::IntRangeId;
use crate::element::payload::IterableInfo;
use crate::element::payload::KeyedArrayInfo;
use crate::element::payload::KnownElementEntry;
use crate::element::payload::KnownElementsId;
use crate::element::payload::KnownItemEntry;
use crate::element::payload::KnownItemsId;
use crate::element::payload::KnownPropertiesId;
use crate::element::payload::KnownPropertyEntry;
use crate::element::payload::ListInfo;
use crate::element::payload::MemberReference;
use crate::element::payload::MixedInfo;
use crate::element::payload::ObjectInfo;
use crate::element::payload::ObjectShapeInfo;
use crate::element::payload::ParamInfo;
use crate::element::payload::ParamListId;
use crate::element::payload::ResourceInfo;
use crate::element::payload::Signature;
use crate::element::payload::SignatureId;
use crate::element::payload::SymbolReference;
use crate::element::payload::VariableInfo;
use crate::element::payload::scalar::FloatInfo;
use crate::element::payload::scalar::IntInfo;
use crate::element::payload::scalar::StringInfo;
use crate::interner::Arena;
use crate::interner::SliceArena;
use crate::well_known::NEVER;

/// Process-global interner that owns one storage backend per payload family.
///
/// Twenty-two per-kind [`Arena`]s (one for every [`ElementKind`] variant that
/// carries a payload), four side-table arenas for the secondary handles
/// ([`IntRange`], [`DefiningEntity`], [`Signature`], [`CallableAlias`]), and
/// six [`SliceArena`]s for the list handles.
///
/// The interner lives inside a process-global `OnceLock` (see [`interner`]).
/// Its arenas are append-only, so a slot once assigned never moves; that is
/// what lets the lookup methods hand out plain `&'static T` references.
pub struct Interner {
    mixed: Arena<MixedInfo>,
    int: Arena<IntInfo>,
    float: Arena<FloatInfo>,
    string: Arena<StringInfo>,
    class_like_string: Arena<ClassLikeStringInfo>,
    object: Arena<ObjectInfo>,
    enumeration: Arena<EnumInfo>,
    object_shape: Arena<ObjectShapeInfo>,
    has_method: Arena<HasMethodInfo>,
    has_property: Arena<HasPropertyInfo>,
    array: Arena<KeyedArrayInfo>,
    list: Arena<ListInfo>,
    iterable: Arena<IterableInfo>,
    callable: Arena<CallableInfo>,
    resource: Arena<ResourceInfo>,
    generic_parameter: Arena<GenericParameterInfo>,
    variable: Arena<VariableInfo>,
    reference: Arena<SymbolReference>,
    member_reference: Arena<MemberReference>,
    global_reference: Arena<GlobalReference>,
    alias: Arena<AliasInfo>,
    conditional: Arena<ConditionalInfo>,
    derived: Arena<DerivedInfo>,
    int_range: Arena<IntRange>,
    defining_entity: Arena<DefiningEntity>,
    signature: Arena<Signature>,
    callable_alias: Arena<CallableAlias>,
    type_list: SliceArena<TypeId>,
    element_list: SliceArena<ElementId>,
    known_items: SliceArena<KnownItemEntry>,
    known_elements: SliceArena<KnownElementEntry>,
    known_properties: SliceArena<KnownPropertyEntry>,
    param_list: SliceArena<ParamInfo>,
    types: Arena<Type>,
}

impl Interner {
    /// A fresh, empty interner. The well-known slots are NOT pre-populated by
    /// this constructor; that responsibility belongs to the boot routine
    /// (added in a later layer).
    pub fn new() -> Self {
        Self {
            mixed: Arena::new(),
            int: Arena::new(),
            float: Arena::new(),
            string: Arena::new(),
            class_like_string: Arena::new(),
            object: Arena::new(),
            enumeration: Arena::new(),
            object_shape: Arena::new(),
            has_method: Arena::new(),
            has_property: Arena::new(),
            array: Arena::new(),
            list: Arena::new(),
            iterable: Arena::new(),
            callable: Arena::new(),
            resource: Arena::new(),
            generic_parameter: Arena::new(),
            variable: Arena::new(),
            reference: Arena::new(),
            member_reference: Arena::new(),
            global_reference: Arena::new(),
            alias: Arena::new(),
            conditional: Arena::new(),
            derived: Arena::new(),
            int_range: Arena::new(),
            defining_entity: Arena::new(),
            signature: Arena::new(),
            callable_alias: Arena::new(),
            type_list: SliceArena::new(),
            element_list: SliceArena::new(),
            known_items: SliceArena::new(),
            known_elements: SliceArena::new(),
            known_properties: SliceArena::new(),
            param_list: SliceArena::new(),
            types: Arena::new(),
        }
    }

    /// Intern a [`Type`] from a slice of elements and a set of flow flags.
    ///
    /// Elements are sorted and deduplicated; empty input collapses to
    /// `[NEVER]`. No structural canonicalization is applied: this method
    /// stores whatever multiset the caller hands in.
    ///
    /// Callers that want a canonical union should run the input through
    /// [`combiner::combine`](crate::combiner::combine) first, or use the
    /// sugar constructors on [`TypeId`] which do that for you. The interner
    /// stays decoupled from canonicalization because the subtype lattice is
    /// preserved under combination, so the comparator answers the same
    /// questions either way.
    pub fn intern_type(&self, elements: &[ElementId], flags: FlowFlags) -> TypeId {
        let mut sorted: Vec<ElementId> = if elements.is_empty() { vec![NEVER] } else { elements.to_vec() };
        sorted.sort_unstable();
        sorted.dedup();

        let list_id = self.element_list.intern(&sorted);
        let static_slice = self.element_list.get(list_id).expect("just-interned slice resolves");
        let value = Type { elements: static_slice, flags };
        TypeId::from_slot(self.types.intern(value))
    }

    /// Look up the [`Type`] behind a [`TypeId`].
    ///
    /// # Panics
    ///
    /// Panics if the slot is not present (forged handle, or constructed
    /// before the boot routine ran for a well-known constant).
    #[inline]
    pub fn get_type(&self, id: TypeId) -> &Type {
        self.types.get(id.slot()).expect("invalid TypeId slot")
    }
}

impl Default for Interner {
    fn default() -> Self {
        Self::new()
    }
}

macro_rules! element_arena_methods {
    (
        $(
            $kind:ident, $field:ident, $payload:ty,
            $intern_fn:ident, $get_fn:ident
        );* $(;)?
    ) => {
        impl Interner {
            $(
                #[doc = concat!("Intern a [`", stringify!($payload), "`] and return the corresponding [`ElementId`].")]
                #[inline]
                pub fn $intern_fn(&self, info: $payload) -> ElementId {
                    let one_based = self.$field.intern(info);
                    ElementId::new(ElementKind::$kind, one_based - 1)
                }

                #[doc = concat!("Look up the [`", stringify!($payload), "`] behind an [`ElementId`] of kind `", stringify!($kind), "`.")]
                ///
                /// The returned reference is `'static` whenever this method is
                /// called on the process-global [`interner`], since lifetime
                /// inference promotes `&self` to `&'static` from that callsite.
                ///
                /// # Panics
                ///
                /// Panics if `id.kind()` is not the expected kind, or if the slot is
                /// not present (which can only happen if the handle was forged).
                #[inline]
                pub fn $get_fn(&self, id: ElementId) -> &$payload {
                    debug_assert_eq!(
                        id.kind(),
                        ElementKind::$kind,
                        concat!("expected ElementKind::", stringify!($kind))
                    );
                    self.$field
                        .get(id.slot() + 1)
                        .expect(concat!("invalid slot for ElementKind::", stringify!($kind)))
                }
            )*
        }
    };
}

element_arena_methods! {
    Mixed,            mixed,             MixedInfo,            intern_mixed,             get_mixed;
    Int,              int,               IntInfo,              intern_int,               get_int;
    Float,            float,             FloatInfo,            intern_float,             get_float;
    String,           string,            StringInfo,           intern_string,            get_string;
    ClassLikeString,  class_like_string, ClassLikeStringInfo,  intern_class_like_string, get_class_like_string;
    Object,           object,            ObjectInfo,           intern_object,            get_object;
    Enum,             enumeration,       EnumInfo,             intern_enum,              get_enum;
    ObjectShape,      object_shape,      ObjectShapeInfo,      intern_object_shape,      get_object_shape;
    HasMethod,        has_method,        HasMethodInfo,        intern_has_method,        get_has_method;
    HasProperty,      has_property,      HasPropertyInfo,      intern_has_property,      get_has_property;
    Array,            array,             KeyedArrayInfo,       intern_array,             get_array;
    List,             list,              ListInfo,             intern_list,              get_list;
    Iterable,         iterable,          IterableInfo,         intern_iterable,          get_iterable;
    Callable,         callable,          CallableInfo,         intern_callable,          get_callable;
    Resource,         resource,          ResourceInfo,         intern_resource,          get_resource;
    GenericParameter, generic_parameter, GenericParameterInfo, intern_generic_parameter, get_generic_parameter;
    Variable,         variable,          VariableInfo,         intern_variable,          get_variable;
    Reference,        reference,         SymbolReference,      intern_reference,         get_reference;
    MemberReference,  member_reference,  MemberReference,      intern_member_reference,  get_member_reference;
    GlobalReference,  global_reference,  GlobalReference,      intern_global_reference,  get_global_reference;
    Alias,            alias,             AliasInfo,            intern_alias,             get_alias;
    Conditional,      conditional,       ConditionalInfo,      intern_conditional,       get_conditional;
    Derived,          derived,           DerivedInfo,          intern_derived,           get_derived;
}

macro_rules! side_table_methods {
    (
        $(
            $field:ident, $payload:ty, $handle:ty,
            $intern_fn:ident, $get_fn:ident
        );* $(;)?
    ) => {
        impl Interner {
            $(
                #[doc = concat!("Intern a [`", stringify!($payload), "`] and return its handle.")]
                #[inline]
                pub fn $intern_fn(&self, value: $payload) -> $handle {
                    <$handle>::from_slot(self.$field.intern(value))
                }

                #[doc = concat!("Look up the [`", stringify!($payload), "`] behind a [`", stringify!($handle), "`].")]
                #[inline]
                pub fn $get_fn(&self, id: $handle) -> &$payload {
                    self.$field
                        .get(id.slot())
                        .expect(concat!("invalid slot for ", stringify!($handle)))
                }
            )*
        }
    };
}

side_table_methods! {
    int_range,       IntRange,       IntRangeId,       intern_int_range,       get_int_range;
    defining_entity, DefiningEntity, DefiningEntityId, intern_defining_entity, get_defining_entity;
    signature,       Signature,      SignatureId,      intern_signature,       get_signature;
    callable_alias,  CallableAlias,  CallableAliasId,  intern_callable_alias,  get_callable_alias;
}

macro_rules! slice_arena_methods {
    (
        $(
            $field:ident, $element:ty, $handle:ty,
            $intern_fn:ident, $get_fn:ident
        );* $(;)?
    ) => {
        impl Interner {
            $(
                #[doc = concat!("Intern a `&[", stringify!($element), "]` and return its handle.")]
                #[inline]
                pub fn $intern_fn(&self, slice: &[$element]) -> $handle {
                    <$handle>::from_slot(self.$field.intern(slice))
                }

                #[doc = concat!("Look up the `&'static [", stringify!($element), "]` behind a [`", stringify!($handle), "`].")]
                #[inline]
                pub fn $get_fn(&self, id: $handle) -> &'static [$element] {
                    self.$field
                        .get(id.slot())
                        .expect(concat!("invalid slot for ", stringify!($handle)))
                }
            )*
        }
    };
}

slice_arena_methods! {
    type_list,        TypeId,                TypeListId,        intern_type_list,        get_type_list;
    element_list,     ElementId,             ElementListId,     intern_element_list,     get_element_list;
    known_items,      KnownItemEntry,        KnownItemsId,      intern_known_items,      get_known_items;
    known_elements,   KnownElementEntry,     KnownElementsId,   intern_known_elements,   get_known_elements;
    known_properties, KnownPropertyEntry,    KnownPropertiesId, intern_known_properties, get_known_properties;
    param_list,       ParamInfo,             ParamListId,       intern_param_list,       get_param_list;
}

static INTERNER: OnceLock<Interner> = OnceLock::new();

/// The process-global [`Interner`].
///
/// First call runs [`Interner::boot`], which pre-populates every well-known
/// element and type at the slot the matching `pub const` constant claims.
/// Subsequent calls return the same instance.
#[inline]
pub fn interner() -> &'static Interner {
    INTERNER.get_or_init(Interner::boot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::payload::scalar::IntInfo;
    use crate::well_known::FALSE;
    use crate::well_known::INT;
    use crate::well_known::STRING;
    use crate::well_known::TRUE;
    use crate::well_known::TYPE_INT_OR_STRING;

    #[test]
    fn intern_int_dedupes_and_packs_kind() {
        let i = Interner::new();

        let a = i.intern_int(IntInfo::Literal(42));
        let b = i.intern_int(IntInfo::Literal(7));
        let c = i.intern_int(IntInfo::Literal(42));

        assert_eq!(a, c, "same payload returns same ElementId");
        assert_ne!(a, b);
        assert_eq!(a.kind(), ElementKind::Int);
    }

    #[test]
    fn get_int_resolves_to_the_interned_value() {
        let i = Interner::new();
        let id = i.intern_int(IntInfo::Literal(123));

        assert_eq!(i.get_int(id), &IntInfo::Literal(123));
    }

    #[test]
    fn slot_spaces_are_independent_across_kinds() {
        let i = Interner::new();

        let a = i.intern_int(IntInfo::Unspecified);
        let b = i.intern_float(FloatInfo::Unspecified);

        // Both occupy slot 0 within their respective per-kind arenas, but the
        // packed kind tag makes the ElementIds distinct.
        assert_eq!(a.slot(), 0);
        assert_eq!(b.slot(), 0);
        assert_ne!(a, b);
        assert_eq!(a.kind(), ElementKind::Int);
        assert_eq!(b.kind(), ElementKind::Float);
    }

    #[test]
    fn side_table_intern_returns_typed_handle() {
        let i = Interner::new();

        let r1 = i.intern_int_range(IntRange::new(Some(1), None));
        let r2 = i.intern_int_range(IntRange::new(Some(1), None));
        let r3 = i.intern_int_range(IntRange::new(Some(0), Some(10)));

        assert_eq!(r1, r2, "same range dedupes");
        assert_ne!(r1, r3);
    }

    #[test]
    fn slice_arena_intern_dedupes_by_content() {
        let i = Interner::new();

        let a = i.intern_type_list(&[crate::well_known::TYPE_INT, crate::well_known::TYPE_STRING]);
        let b = i.intern_type_list(&[crate::well_known::TYPE_INT, crate::well_known::TYPE_STRING]);
        let c = i.intern_type_list(&[crate::well_known::TYPE_STRING]);

        assert_eq!(a, b, "identical contents share a slot");
        assert_ne!(a, c);
    }

    #[test]
    fn global_interner_returns_the_same_instance() {
        let a = interner() as *const Interner;
        let b = interner() as *const Interner;
        assert_eq!(a, b);
    }

    #[test]
    fn intern_type_sorts_and_dedupes_elements() {
        let i = interner();
        let a = i.intern_int(IntInfo::Literal(99));
        let b = i.intern_int(IntInfo::Literal(100));

        let t1 = i.intern_type(&[a, b], FlowFlags::EMPTY);
        let t2 = i.intern_type(&[b, a], FlowFlags::EMPTY);
        let t3 = i.intern_type(&[a, b, a, b, a], FlowFlags::EMPTY);

        assert_eq!(t1, t2, "element order does not matter");
        assert_eq!(t1, t3, "duplicate elements collapse");
        assert_eq!(i.get_type(t1).elements.len(), 2);
    }

    #[test]
    fn intern_type_distinguishes_flow_flags() {
        let i = Interner::new();
        let a = i.intern_int(IntInfo::Unspecified);

        let no_flags = i.intern_type(&[a], FlowFlags::EMPTY);
        let with_flag = i.intern_type(&[a], FlowFlags::EMPTY.with_possibly_undefined(true));

        assert_ne!(no_flags, with_flag, "flow flags participate in TypeId interning");
    }

    #[test]
    fn intern_type_empty_input_collapses_to_never() {
        let i = Interner::new();

        let empty = i.intern_type(&[], FlowFlags::EMPTY);
        let just_never = i.intern_type(&[NEVER], FlowFlags::EMPTY);

        assert_eq!(empty, just_never, "empty input is interned as a never-only union");
        assert_eq!(i.get_type(empty).elements, &[NEVER]);
    }

    #[test]
    fn type_id_as_ref_round_trips_through_global_interner() {
        let id = interner().intern_type(&[interner().intern_int(IntInfo::Literal(1729))], FlowFlags::EMPTY);
        let t: &'static Type = id.as_ref();
        assert_eq!(t.elements.len(), 1);
        assert_eq!(t.flags, FlowFlags::EMPTY);
    }

    #[test]
    fn intern_type_does_not_canonicalize_bool() {
        // Raw intern_type stores whatever the caller hands in (modulo
        // sort+dedup). Canonicalization is the combiner's job.
        let raw = interner().intern_type(&[TRUE, FALSE], FlowFlags::EMPTY);
        assert_eq!(raw.as_ref().elements, &[TRUE, FALSE]);
    }

    #[test]
    fn intern_type_preserves_unrelated_unions() {
        // INT and STRING have nothing to canonicalize, so raw intern_type
        // still hits the well-known TYPE_INT_OR_STRING slot.
        let id = interner().intern_type(&[INT, STRING], FlowFlags::EMPTY);
        assert_eq!(id, TYPE_INT_OR_STRING);
    }
}
