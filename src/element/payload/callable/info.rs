use std::mem::size_of;

use super::CallableAliasId;
use super::SignatureId;

/// `callable`, `Closure(int): string`, `callable(int, string=): void`,
/// `pure-callable(int): int`, and known-function/method/closure aliases.
///
/// All variants stay 4-byte handle-shaped: heavy data (signatures, alias
/// identifiers) lives in side-table interners. Largest payload here is one
/// `NonZeroU32`, so the whole enum lands at 8 bytes.
///
/// Note: a "bare `\Closure`" type (the class without a known signature) is
/// represented as [`ObjectInfo`](crate::payload::ObjectInfo) `Named(\Closure)`,
/// not as `Closure(...)` here. `Closure(σ)` is reserved for the case where
/// the signature `σ` is known.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CallableInfo {
    /// Just `callable`, no signature info.
    Any,

    /// `\Closure` with a known signature (e.g. `Closure(int): string`).
    /// Subtype of both `Callable` and `Object::Named(\Closure)`; the latter
    /// relationship is enforced at subtype time, not here.
    Closure(SignatureId),

    /// An anonymous callable signature: `callable(...)`.
    Signature(SignatureId),

    /// A reference to a known function, method, or closure expression.
    Alias(CallableAliasId),
}

const _: () = assert!(size_of::<CallableInfo>() <= 8);
