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
#[non_exhaustive]
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

impl std::fmt::Display for CallableInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let i = crate::interner::interner();
        match self {
            CallableInfo::Any => f.write_str("callable"),
            CallableInfo::Signature(sid) => render_signature(i.get_signature(*sid), false, f),
            CallableInfo::Closure(sid) => render_signature(i.get_signature(*sid), true, f),
            CallableInfo::Alias(aid) => std::fmt::Display::fmt(i.get_callable_alias(*aid), f),
        }
    }
}

impl CallableInfo {
    pub(crate) fn pretty_with_indent(&self, indent: usize) -> String {
        use crate::typed::Typed;
        let i = crate::interner::interner();
        match self {
            CallableInfo::Signature(sid) | CallableInfo::Closure(sid) => {
                let sig = i.get_signature(*sid);
                let is_closure = matches!(self, CallableInfo::Closure(_));
                let params = sig.parameters.map(|pid| i.get_param_list(pid)).unwrap_or(&[] as _);
                if params.len() <= 2 {
                    return self.to_string();
                }
                let mut out = String::from("(");
                if sig.flags.is_pure() {
                    out.push_str("pure-");
                }
                out.push_str(if is_closure { "closure(\n" } else { "callable(\n" });
                let inner = indent + 2;
                let pad = " ".repeat(inner);
                for (idx, p) in params.iter().enumerate() {
                    if idx > 0 {
                        out.push_str(",\n");
                    }
                    out.push_str(&pad);
                    if p.flags.variadic() {
                        out.push_str("...");
                    }
                    out.push_str(&p.type_.pretty_with_indent(inner));
                    if p.flags.has_default() {
                        out.push('=');
                    }
                }
                out.push('\n');
                out.push_str(&" ".repeat(indent));
                out.push_str("): ");
                out.push_str(&sig.return_type.pretty_with_indent(indent));
                out.push(')');
                out
            }
            _ => self.to_string(),
        }
    }
}

fn render_signature(sig: &super::Signature, is_closure: bool, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let i = crate::interner::interner();
    f.write_str("(")?;
    if sig.flags.is_pure() {
        f.write_str("pure-")?;
    }
    f.write_str(if is_closure { "closure(" } else { "callable(" })?;
    if let Some(pid) = sig.parameters {
        for (idx, p) in i.get_param_list(pid).iter().enumerate() {
            if idx > 0 {
                f.write_str(", ")?;
            }
            if p.flags.variadic() {
                f.write_str("...")?;
            }
            std::fmt::Display::fmt(&p.type_, f)?;
            if p.flags.has_default() {
                f.write_str("=")?;
            }
        }
    }
    write!(f, "): {})", sig.return_type)
}
