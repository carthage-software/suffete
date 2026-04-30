//! PHP cast semantics (report §20).
//!
//! [`cast`] computes the result type of an explicit PHP cast operator —
//! `(int)`, `(float)`, `(string)`, `(bool)`, `(array)`, `(object)` —
//! applied to an arbitrary input [`TypeId`]. The result is a
//! [`CastResult`] pairing the post-cast type with [`CastFlags`] that
//! record whether the cast lost information ([`CastFlags::LOSSY`]) or
//! could throw at runtime ([`CastFlags::MAY_THROW`]).
//!
//! The implementation distributes over the input union and runs a
//! per-element rule. Element classifications combine via bitwise-or on
//! the flags and union on the result types.
//!
//! # Accuracy
//!
//! This is a first-cut. Literal preservation is partial: integer and
//! string literals round-trip across `(int)` and `(string)` losslessly,
//! and the falsy literals (`0`, `0.0`, `""`, `"0"`, `null`) collapse to
//! `false` under `(bool)`. Float-to-int truncation, complex
//! `(array)`/`(object)` shape construction, and `__toString`-aware
//! object-to-string casts are intentionally coarse — the analyzer can
//! still validate the operator sites; precision improvements land later.

use crate::ElementId;
use crate::ElementKind;
use crate::TypeId;
use crate::element::payload::scalar::FloatInfo;
use crate::element::payload::scalar::IntInfo;
use crate::element::payload::scalar::StringLiteral;
use crate::interner::interner;
use crate::prelude::EMPTY_ARRAY;
use crate::prelude::EMPTY_STRING;
use crate::prelude::FALSE;
use crate::prelude::INT_ONE;
use crate::prelude::INT_ZERO;
use crate::prelude::TRUE;
use crate::prelude::TYPE_BOOL;
use crate::prelude::TYPE_FLOAT;
use crate::prelude::TYPE_INT;
use crate::prelude::TYPE_MIXED;
use crate::prelude::TYPE_STRING;
use crate::world::World;

/// One of PHP's six explicit cast operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CastTarget {
    Int,
    Float,
    String,
    Bool,
    Array,
    Object,
}

/// Result of [`cast`]: the post-cast type plus diagnostic flags.
#[derive(Debug, Clone, Copy)]
pub struct CastResult {
    pub ty: TypeId,
    pub flags: CastFlags,
}

impl CastResult {
    #[inline]
    pub const fn lossless(ty: TypeId) -> Self {
        Self { ty, flags: CastFlags::NONE }
    }

    #[inline]
    pub const fn lossy(ty: TypeId) -> Self {
        Self { ty, flags: CastFlags::LOSSY }
    }

    #[inline]
    pub const fn may_throw(ty: TypeId) -> Self {
        Self { ty, flags: CastFlags::MAY_THROW }
    }
}

/// Bitset of cast diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CastFlags(u8);

impl CastFlags {
    pub const NONE: Self = Self(0);
    /// The cast discarded information from the input (e.g. float
    /// truncation, non-numeric string to `int = 0`).
    pub const LOSSY: Self = Self(1 << 0);
    /// The cast may emit an error or throw at runtime (e.g. casting an
    /// array to a string in PHP 8+, or an object lacking `__toString`).
    pub const MAY_THROW: Self = Self(1 << 1);

    #[inline]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    #[inline]
    pub const fn lossy(self) -> bool {
        self.contains(Self::LOSSY)
    }

    #[inline]
    pub const fn may_throw(self) -> bool {
        self.contains(Self::MAY_THROW)
    }

    #[inline]
    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    #[inline]
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Cast `input` to `target`. Distributes over the input union: each
/// element is cast individually, the resulting types are unioned, and
/// the per-element flags are bit-or'd into a single [`CastFlags`].
pub fn cast<W: World>(input: TypeId, target: CastTarget, world: &W) -> CastResult {
    let elements = input.as_ref().elements;
    let mut combined: Vec<ElementId> = Vec::new();
    let mut flags = CastFlags::NONE;
    for &elem in elements {
        let outcome = cast_element(elem, target, world);
        flags.insert(outcome.flags);
        combined.extend_from_slice(outcome.ty.as_ref().elements);
    }
    CastResult { ty: TypeId::union(&combined), flags }
}

fn cast_element<W: World>(elem: ElementId, target: CastTarget, world: &W) -> CastResult {
    match target {
        CastTarget::Int => cast_to_int(elem),
        CastTarget::Float => cast_to_float(elem),
        CastTarget::String => cast_to_string(elem, world),
        CastTarget::Bool => cast_to_bool(elem),
        CastTarget::Array => cast_to_array(elem),
        CastTarget::Object => cast_to_object(elem),
    }
}

fn cast_to_int(elem: ElementId) -> CastResult {
    let i = interner();
    match elem.kind() {
        ElementKind::Int => CastResult::lossless(singleton(elem)),
        ElementKind::True => CastResult::lossless(singleton(INT_ONE)),
        ElementKind::False | ElementKind::Null | ElementKind::Void => CastResult::lossless(singleton(INT_ZERO)),
        ElementKind::Bool => CastResult::lossless(TYPE_INT),
        ElementKind::Float => match *i.get_float(elem) {
            FloatInfo::Literal(lit) => {
                let truncated = lit.value().trunc() as i64;
                CastResult::lossy(singleton(ElementId::int_literal(truncated)))
            }
            _ => CastResult::lossy(TYPE_INT),
        },
        ElementKind::String => match i.get_string(elem).literal {
            StringLiteral::Value(value) => {
                let s = value.as_str();
                if let Some(n) = parse_php_int(s) {
                    CastResult::lossless(singleton(ElementId::int_literal(n)))
                } else {
                    CastResult::lossy(singleton(INT_ZERO))
                }
            }
            _ => CastResult::lossy(TYPE_INT),
        },
        ElementKind::Resource => CastResult::lossless(TYPE_INT),
        ElementKind::Array | ElementKind::List => CastResult::lossy(TYPE_INT),
        ElementKind::Object
        | ElementKind::Enum
        | ElementKind::ObjectShape
        | ElementKind::HasMethod
        | ElementKind::HasProperty
        | ElementKind::ObjectAny => CastResult::may_throw(TYPE_INT),
        _ => CastResult::lossy(TYPE_INT),
    }
}

fn cast_to_float(elem: ElementId) -> CastResult {
    let i = interner();
    match elem.kind() {
        ElementKind::Float => CastResult::lossless(singleton(elem)),
        ElementKind::Int => match *i.get_int(elem) {
            IntInfo::Literal(n) => CastResult::lossless(singleton(ElementId::float_literal(n as f64))),
            _ => CastResult::lossless(TYPE_FLOAT),
        },
        ElementKind::True => CastResult::lossless(singleton(ElementId::float_literal(1.0))),
        ElementKind::False | ElementKind::Null | ElementKind::Void => {
            CastResult::lossless(singleton(ElementId::float_literal(0.0)))
        }
        ElementKind::Bool => CastResult::lossless(TYPE_FLOAT),
        ElementKind::String => match i.get_string(elem).literal {
            StringLiteral::Value(value) => match parse_php_float(value.as_str()) {
                Some(f) => CastResult::lossless(singleton(ElementId::float_literal(f))),
                None => CastResult::lossy(singleton(ElementId::float_literal(0.0))),
            },
            _ => CastResult::lossy(TYPE_FLOAT),
        },
        ElementKind::Object
        | ElementKind::Enum
        | ElementKind::ObjectShape
        | ElementKind::HasMethod
        | ElementKind::HasProperty
        | ElementKind::ObjectAny
        | ElementKind::Array
        | ElementKind::List => CastResult::may_throw(TYPE_FLOAT),
        _ => CastResult::lossy(TYPE_FLOAT),
    }
}

fn cast_to_string<W: World>(elem: ElementId, _world: &W) -> CastResult {
    let i = interner();
    match elem.kind() {
        ElementKind::String => CastResult::lossless(singleton(elem)),
        ElementKind::Int => match *i.get_int(elem) {
            IntInfo::Literal(n) => CastResult::lossless(singleton(ElementId::string_literal(&n.to_string()))),
            _ => CastResult::lossless(TYPE_STRING),
        },
        ElementKind::Float => match *i.get_float(elem) {
            FloatInfo::Literal(lit) => {
                CastResult::lossless(singleton(ElementId::string_literal(&format_php_float(lit.value()))))
            }
            _ => CastResult::lossless(TYPE_STRING),
        },
        ElementKind::True => CastResult::lossless(singleton(ElementId::string_literal("1"))),
        ElementKind::False | ElementKind::Null | ElementKind::Void => CastResult::lossless(singleton(EMPTY_STRING)),
        ElementKind::Bool => CastResult::lossless(TYPE_STRING),
        ElementKind::Resource => CastResult::lossless(TYPE_STRING),
        ElementKind::Object
        | ElementKind::Enum
        | ElementKind::ObjectShape
        | ElementKind::HasMethod
        | ElementKind::HasProperty
        | ElementKind::ObjectAny => CastResult::may_throw(TYPE_STRING),
        ElementKind::Array | ElementKind::List => CastResult::may_throw(TYPE_STRING),
        _ => CastResult::lossy(TYPE_STRING),
    }
}

fn cast_to_bool(elem: ElementId) -> CastResult {
    let i = interner();
    match elem.kind() {
        ElementKind::True => CastResult::lossless(singleton(TRUE)),
        ElementKind::False => CastResult::lossless(singleton(FALSE)),
        ElementKind::Bool => CastResult::lossless(singleton(elem)),
        ElementKind::Null | ElementKind::Void => CastResult::lossless(singleton(FALSE)),
        ElementKind::Int => match *i.get_int(elem) {
            IntInfo::Literal(0) => CastResult::lossless(singleton(FALSE)),
            IntInfo::Literal(_) => CastResult::lossless(singleton(TRUE)),
            _ => CastResult::lossless(TYPE_BOOL),
        },
        ElementKind::Float => match *i.get_float(elem) {
            FloatInfo::Literal(lit) if lit.value() == 0.0 => CastResult::lossless(singleton(FALSE)),
            FloatInfo::Literal(_) => CastResult::lossless(singleton(TRUE)),
            _ => CastResult::lossless(TYPE_BOOL),
        },
        ElementKind::String => match i.get_string(elem).literal {
            StringLiteral::Value(value) => {
                let s = value.as_str();
                if s.is_empty() || s == "0" {
                    CastResult::lossless(singleton(FALSE))
                } else {
                    CastResult::lossless(singleton(TRUE))
                }
            }
            _ => CastResult::lossless(TYPE_BOOL),
        },
        ElementKind::Array | ElementKind::List => CastResult::lossless(TYPE_BOOL),
        ElementKind::Object
        | ElementKind::Enum
        | ElementKind::ObjectShape
        | ElementKind::HasMethod
        | ElementKind::HasProperty
        | ElementKind::ObjectAny
        | ElementKind::Resource
        | ElementKind::Callable => CastResult::lossless(singleton(TRUE)),
        _ => CastResult::lossless(TYPE_BOOL),
    }
}

fn cast_to_array(elem: ElementId) -> CastResult {
    match elem.kind() {
        ElementKind::Array | ElementKind::List => CastResult::lossless(singleton(elem)),
        ElementKind::Null | ElementKind::Void => CastResult::lossless(singleton(EMPTY_ARRAY)),
        ElementKind::Int
        | ElementKind::Float
        | ElementKind::String
        | ElementKind::Bool
        | ElementKind::True
        | ElementKind::False
        | ElementKind::Resource
        | ElementKind::Callable => CastResult::lossy(generic_array_type()),
        ElementKind::Object
        | ElementKind::Enum
        | ElementKind::ObjectShape
        | ElementKind::HasMethod
        | ElementKind::HasProperty
        | ElementKind::ObjectAny => CastResult::lossy(generic_array_type()),
        _ => CastResult::lossy(generic_array_type()),
    }
}

fn cast_to_object(elem: ElementId) -> CastResult {
    match elem.kind() {
        ElementKind::Object
        | ElementKind::Enum
        | ElementKind::ObjectShape
        | ElementKind::HasMethod
        | ElementKind::HasProperty
        | ElementKind::ObjectAny => CastResult::lossless(singleton(elem)),
        _ => CastResult::lossy(stdclass_type()),
    }
}

fn singleton(elem: ElementId) -> TypeId {
    interner().intern_type(&[elem], crate::FlowFlags::EMPTY)
}

/// `array<array-key, mixed>` — the broadest expressible array type.
/// Used as the conservative fallback for `(array)` casts whose input
/// shape isn't precise enough to construct a more specific result.
fn generic_array_type() -> TypeId {
    use crate::element::payload::KeyedArrayFlags;
    use crate::element::payload::KeyedArrayInfo;
    use crate::prelude::TYPE_ARRAY_KEY;
    let i = interner();
    let info = KeyedArrayInfo {
        key_param: Some(TYPE_ARRAY_KEY),
        value_param: Some(TYPE_MIXED),
        known_items: None,
        flags: KeyedArrayFlags::default(),
    };
    singleton(i.intern_array(info))
}

fn stdclass_type() -> TypeId {
    use crate::element::payload::ObjectFlags;
    use crate::element::payload::ObjectInfo;
    let i = interner();
    let info = ObjectInfo {
        name: mago_atom::atom("stdClass"),
        type_args: None,
        intersections: None,
        excluded: None,
        flags: ObjectFlags::default(),
    };
    singleton(i.intern_object(info))
}

/// Parse a string the way PHP's `(int)` does: leading whitespace +
/// optional sign + decimal digits, stopping at the first non-digit.
/// Returns `None` when no leading-decimal prefix exists; `Some(0)` is
/// reserved for the explicit literal "0".
fn parse_php_int(s: &str) -> Option<i64> {
    let trimmed = s.trim_start();
    let bytes = trimmed.as_bytes();
    let mut i = 0;
    if matches!(bytes.first(), Some(b'-' | b'+')) {
        i = 1;
    }
    let digits_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == digits_start {
        return None;
    }
    trimmed[..i].parse::<i64>().ok()
}

fn parse_php_float(s: &str) -> Option<f64> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<f64>().ok()
}

fn format_php_float(value: f64) -> String {
    if value == value.trunc() && value.is_finite() { format!("{}", value as i64) } else { format!("{value}") }
}
