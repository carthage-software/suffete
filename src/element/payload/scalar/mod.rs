//! Scalar payloads: `Int`, `Float`, `String`, `ClassLikeString`.
//!
//! `Mixed` is a top type, not a scalar; see [`crate::element::payload::MixedInfo`].

mod class_like_string;
mod float;
mod int;
mod string;

pub use self::class_like_string::ClassLikeKind;
pub use self::class_like_string::ClassLikeStringInfo;
pub use self::class_like_string::ClassLikeStringSpecifier;
pub use self::float::FloatInfo;
pub use self::float::LiteralFloat;
pub use self::int::BoundFlags;
pub use self::int::IntInfo;
pub use self::int::IntRange;
pub use self::int::IntRangeId;
pub use self::string::StringCasing;
pub use self::string::StringInfo;
pub use self::string::StringLiteral;
pub use self::string::StringRefinementFlags;
