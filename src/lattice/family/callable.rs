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
//! Signature comparison is contravariant on
//! parameters and covariant on return: `Sig(P̄_in, R_in)` refines
//! `Sig(P̄_out, R_out)` iff every container parameter at position `i`
//! refines the corresponding input parameter (`P̄_out[i] <: P̄_in[i]`),
//! `R_in <: R_out`, and the input is at least as pure as the container
//! demands. A container with `parameters: None` (an unspecified
//! signature) accepts any signature.

use crate::ElementId;
use crate::ElementKind;
use crate::element::payload::CallableInfo;
use crate::element::payload::Signature;
use crate::element::payload::SignatureId;
use crate::interner::interner;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::lattice::refines::refines as type_refines;
use crate::world::World;

#[inline]
pub fn refines<W: World>(
    input: ElementId,
    container: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let i = interner();

    if input.kind() == ElementKind::String {
        let info = i.get_string(input);
        if info.flags.is_callable() {
            return true;
        }
    }

    if input.kind() != ElementKind::Callable {
        return false;
    }

    let input_info = *i.get_callable(input);
    let container_info = *i.get_callable(container);

    match (input_info, container_info) {
        (_, CallableInfo::Any) => true,
        (CallableInfo::Any, _) => false,
        (CallableInfo::Signature(s_in), CallableInfo::Signature(s_out))
        | (CallableInfo::Closure(s_in), CallableInfo::Signature(s_out))
        | (CallableInfo::Closure(s_in), CallableInfo::Closure(s_out)) => {
            signature_refines(s_in, s_out, world, options, report)
        }
        (CallableInfo::Signature(_), CallableInfo::Closure(_)) => false,
        (CallableInfo::Alias(a), CallableInfo::Alias(b)) => a == b,
        (CallableInfo::Alias(_), _) | (_, CallableInfo::Alias(_)) => false,
    }
}

#[inline]
fn signature_refines<W: World>(
    in_id: SignatureId,
    out_id: SignatureId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    if in_id == out_id {
        return true;
    }

    let i = interner();
    let s_in = *i.get_signature(in_id);
    let s_out = *i.get_signature(out_id);

    if !type_refines(s_in.return_type, s_out.return_type, world, options, report) {
        return false;
    }

    if s_out.flags.is_pure() && !s_in.flags.is_pure() {
        return false;
    }

    if !throws_refines(s_in.throws, s_out.throws, world, options, report) {
        return false;
    }

    parameters_refine(s_in, s_out, world, options, report)
}

/// Container's `throws` constrains input's: input's exceptions must fit
/// within the container's set. `None` on the container means no
/// constraint; `None` on the input means "throws anything", which is too
/// loose for any constrained container.
#[inline]
fn throws_refines<W: World>(
    input: Option<crate::TypeId>,
    container: Option<crate::TypeId>,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    match (input, container) {
        (_, None) => true,
        (None, Some(_)) => false,
        (Some(in_throws), Some(out_throws)) => type_refines(in_throws, out_throws, world, options, report),
    }
}

#[inline]
fn parameters_refine<W: World>(
    s_in: Signature,
    s_out: Signature,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let i = interner();

    let Some(out_id) = s_out.parameters else {
        return true;
    };
    let Some(in_id) = s_in.parameters else {
        return false;
    };

    let in_params = i.get_param_list(in_id);
    let out_params = i.get_param_list(out_id);

    let in_required = required_count(in_params);
    let out_required = required_count(out_params);
    if in_required > out_required {
        return false;
    }

    let in_variadic = s_in.flags.is_variadic();
    let out_variadic = s_out.flags.is_variadic();
    if out_variadic && !in_variadic {
        return false;
    }

    if !in_variadic && in_params.len() < out_params.len() {
        return false;
    }

    for (idx, out_param) in out_params.iter().enumerate() {
        let in_type = match in_params.get(idx) {
            Some(p) => p.type_,
            None => match in_params.last() {
                Some(last) if in_variadic => last.type_,
                _ => return false,
            },
        };
        if !type_refines(out_param.type_, in_type, world, options, report) {
            return false;
        }
    }

    true
}

#[inline]
fn required_count(params: &[crate::element::payload::ParamInfo]) -> usize {
    params.iter().take_while(|p| !p.flags.has_default() && !p.flags.variadic()).count()
}
