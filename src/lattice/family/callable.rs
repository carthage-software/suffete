//! Callable family: `callable`, `Closure(...)`, anonymous signatures, and
//! known-callable aliases.
//!
//! Within the family:
//!
//! - `callable` (the `Any` variant) accepts any other callable.
//! - `Closure(σ) <: Signature(σ')` when the signatures match (a `Closure`
//!   is a refinement of an anonymous callable).
//! - `Signature(σ) <: Closure(σ')` does NOT hold (the input might be a
//!   non-`\Closure` callable).
//!
//! Cross-family inputs:
//!
//! - A string with the `is_callable` refinement flag refines `callable`
//!   (a `callable-string` is a callable name).
//! - `\Closure` named-object refinement is decided by the object family,
//!   not here.
//!
//! Signature-vs-signature comparison (parameter contravariance, return
//! covariance) is not yet implemented; for now signatures are treated as
//! matching iff they are the same handle (refl).

use crate::ElementId;
use crate::ElementKind;
use crate::element::payload::CallableInfo;
use crate::interner::interner;

pub fn refines(input: ElementId, container: ElementId) -> bool {
    let i = interner();

    // Cross-family: callable-string refines `callable`.
    if input.kind() == ElementKind::String {
        let info = i.get_string(input);
        if info.flags.is_callable() {
            return true;
        }
    }

    if input.kind() != ElementKind::Callable {
        return false;
    }

    let container_info = *i.get_callable(container);
    let input_info = *i.get_callable(input);

    match (input_info, container_info) {
        // `Any` container accepts any callable input.
        (_, CallableInfo::Any) => true,
        // Closure refines an anonymous Signature container if the signatures
        // match. Reflexivity already handled equal handles upstream; for
        // distinct signatures we conservatively require handle equality
        // (real variance comparison is a follow-up).
        (CallableInfo::Closure(s_in), CallableInfo::Signature(s_out)) => s_in == s_out,
        // Same-shape variants: handle equality (refl handles equal handles).
        _ => false,
    }
}
