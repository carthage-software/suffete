#![allow(clippy::pub_use)]

//! Callable payloads: any-callable, closure (PHP `\Closure` instance),
//! anonymous signature, and known-function/method/closure aliases.

mod alias;
mod info;
mod signature;

pub use self::alias::CallableAlias;
pub use self::alias::CallableAliasId;
pub use self::info::CallableInfo;
pub use self::signature::ParamFlags;
pub use self::signature::ParamInfo;
pub use self::signature::ParamListId;
pub use self::signature::Signature;
pub use self::signature::SignatureFlags;
pub use self::signature::SignatureId;
