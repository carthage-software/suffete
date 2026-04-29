//! `Callable` family meet.
//!
//! For two callable signatures with the same parameter arity:
//!
//! - return type is **covariant** — a value satisfying both must
//!   produce a value compatible with both, so the meet narrows the
//!   return type via [`crate::meet::compute`].
//! - parameter types are **contravariant** — both signatures must
//!   accept any input either accepts, so the meet *widens* each
//!   parameter via [`crate::join::compute`].
//! - purity is conjunctive (`pure ∧ pure → pure`, otherwise impure).
//!
//! When either side carries no signature (the `Any` variant) the
//! subsumption rule has already accepted the more-specific side, so
//! this function never sees that case.

use crate::ElementId;
use crate::element::payload::CallableInfo;
use crate::element::payload::ParamInfo;
use crate::element::payload::Signature;
use crate::element::payload::SignatureFlags;
use crate::interner::interner;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::world::World;

pub(in crate::meet) fn callable_meet<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Option<ElementId> {
    let i = interner();
    let (CallableInfo::Signature(a_id), CallableInfo::Signature(b_id)) = (*i.get_callable(a), *i.get_callable(b))
    else {
        return None;
    };
    let a_sig = *i.get_signature(a_id);
    let b_sig = *i.get_signature(b_id);

    let a_params: &[ParamInfo] = a_sig.parameters.map(|p| i.get_param_list(p)).unwrap_or(&[]);
    let b_params: &[ParamInfo] = b_sig.parameters.map(|p| i.get_param_list(p)).unwrap_or(&[]);
    if a_params.len() != b_params.len() {
        return None;
    }

    let merged_params: Vec<ParamInfo> = a_params
        .iter()
        .zip(b_params.iter())
        .map(|(pa, pb)| {
            let widened = crate::join::compute(&[pa.type_.as_ref().elements, pb.type_.as_ref().elements].concat());
            let type_ = i.intern_type(&widened, crate::FlowFlags::EMPTY);
            ParamInfo { name: pa.name, type_, flags: pa.flags }
        })
        .collect();

    let return_type = crate::meet::compute(a_sig.return_type, b_sig.return_type, world, options, report);

    let throws = match (a_sig.throws, b_sig.throws) {
        (Some(t1), Some(t2)) => {
            let throws_ty = crate::meet::compute(t1, t2, world, options, report);
            Some(throws_ty)
        }
        (Some(t), None) | (None, Some(t)) => Some(t),
        (None, None) => None,
    };

    let pure = a_sig.flags.is_pure() && b_sig.flags.is_pure();
    let variadic = a_sig.flags.is_variadic() && b_sig.flags.is_variadic();
    let flags = SignatureFlags::EMPTY.with_is_pure(pure).with_is_variadic(variadic);

    let parameters = if merged_params.is_empty() { None } else { Some(i.intern_param_list(&merged_params)) };
    let sig_id = i.intern_signature(Signature { parameters, return_type, throws, flags });
    Some(i.intern_callable(CallableInfo::Signature(sig_id)))
}
