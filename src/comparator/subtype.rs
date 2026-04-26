use crate::ElementId;
use crate::TypeId;
use crate::comparator::Codebase;
use crate::comparator::SubtypeContext;
use crate::well_known::MIXED;
use crate::well_known::NEVER;

/// Decide whether `input <: container` under the spec's subtype relation.
///
/// Returns the boolean answer; richer information (coercions, template
/// bounds, replacement suggestions) accumulates in `ctx`. Implementations
/// that only need yes/no can pass `&mut SubtypeContext::new()`.
///
/// The current implementation covers the universal axioms
/// (refl / Bot / Top) and the union dispatch (Union-L / Union-R from spec
/// §4.3); element-vs-element rules for the scalar lattice, objects,
/// callables, etc., are added incrementally and use the `codebase` parameter
/// for ancestor / member / template queries.
pub fn is_subtype<C: Codebase>(input: TypeId, container: TypeId, ctx: &mut SubtypeContext, codebase: &C) -> bool {
    if input == container {
        return true;
    }

    let input_type = input.as_ref();
    let container_type = container.as_ref();

    input_type.elements.iter().all(|input_elem| {
        container_type
            .elements
            .iter()
            .any(|container_elem| element_is_subtype(*input_elem, *container_elem, ctx, codebase))
    })
}

/// Decide whether one element is a subtype of another, ignoring flow flags.
///
/// Currently implements only:
///
/// - element reflexivity (`a == a → true`),
/// - Bot (`never <: anything → true`),
/// - Top (`anything <: vanilla mixed → true`).
///
/// Family-specific rules (scalar lattice cross-edges, object hierarchy via
/// codebase, callable contravariance, template-parameter relational identity,
/// etc.) layer in as the comparator grows. Anything not yet handled returns
/// `false` conservatively; this is sound (we never accept a wrong subtype
/// edge), only incomplete (we may reject true edges).
fn element_is_subtype<C: Codebase>(
    input: ElementId,
    container: ElementId,
    _ctx: &mut SubtypeContext,
    _codebase: &C,
) -> bool {
    if input == container {
        return true;
    }

    if input == NEVER {
        return true;
    }

    if container == MIXED {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ElementId;
    use crate::FlowFlags;
    use crate::comparator::NullCodebase;
    use crate::interner::interner;
    use crate::well_known::FLOAT;
    use crate::well_known::INT;
    use crate::well_known::STRING;
    use crate::well_known::TYPE_FLOAT;
    use crate::well_known::TYPE_INT;
    use crate::well_known::TYPE_INT_OR_FLOAT;
    use crate::well_known::TYPE_INT_OR_STRING;
    use crate::well_known::TYPE_MIXED;
    use crate::well_known::TYPE_NEVER;
    use crate::well_known::TYPE_NULL;
    use crate::well_known::TYPE_STRING;

    fn check(input: TypeId, container: TypeId) -> bool {
        let mut ctx = SubtypeContext::new();
        is_subtype(input, container, &mut ctx, &NullCodebase)
    }

    #[test]
    fn reflexivity_holds_for_well_known_types() {
        assert!(check(TYPE_INT, TYPE_INT));
        assert!(check(TYPE_NULL, TYPE_NULL));
        assert!(check(TYPE_NEVER, TYPE_NEVER));
        assert!(check(TYPE_MIXED, TYPE_MIXED));
        assert!(check(TYPE_INT_OR_STRING, TYPE_INT_OR_STRING));
    }

    #[test]
    fn bot_axiom_never_is_subtype_of_anything() {
        assert!(check(TYPE_NEVER, TYPE_INT));
        assert!(check(TYPE_NEVER, TYPE_NULL));
        assert!(check(TYPE_NEVER, TYPE_MIXED));
        assert!(check(TYPE_NEVER, TYPE_INT_OR_STRING));
    }

    #[test]
    fn top_axiom_anything_is_subtype_of_vanilla_mixed() {
        assert!(check(TYPE_INT, TYPE_MIXED));
        assert!(check(TYPE_NULL, TYPE_MIXED));
        assert!(check(TYPE_STRING, TYPE_MIXED));
        assert!(check(TYPE_INT_OR_STRING, TYPE_MIXED));
    }

    #[test]
    fn union_left_every_input_element_must_fit_some_container_element() {
        assert!(check(TYPE_INT_OR_STRING, TYPE_MIXED));
        assert!(check(TYPE_INT_OR_FLOAT, TYPE_INT_OR_FLOAT));
        assert!(!check(TYPE_INT_OR_STRING, TYPE_INT));
    }

    #[test]
    fn union_right_singleton_input_fits_member_of_union() {
        assert!(check(TYPE_INT, TYPE_INT_OR_STRING));
        assert!(check(TYPE_STRING, TYPE_INT_OR_STRING));
        assert!(!check(TYPE_FLOAT, TYPE_INT_OR_STRING));
    }

    #[test]
    fn unrelated_types_are_not_subtypes_yet() {
        assert!(!check(TYPE_INT, TYPE_FLOAT));
        assert!(!check(TYPE_INT, TYPE_STRING));
        assert!(!check(TYPE_NULL, TYPE_INT));
    }

    #[test]
    fn fresh_int_literal_is_subtype_of_mixed_via_top_axiom() {
        let lit = ElementId::int_literal(42);
        let lit_type = interner().intern_type(&[lit], FlowFlags::EMPTY);

        assert!(check(lit_type, TYPE_MIXED));
        // But not yet a subtype of TYPE_INT (needs the "Literal(N) <: Unspecified"
        // rule, which lives in the scalar lattice family).
        assert!(!check(lit_type, TYPE_INT));
    }

    #[test]
    fn input_int_or_float_fits_int_or_float_via_union_dispatch() {
        let mixed_lit = interner().intern_type(&[INT, FLOAT], FlowFlags::EMPTY);
        // INT element fits via INT in the container; FLOAT fits via FLOAT.
        assert!(check(mixed_lit, TYPE_INT_OR_FLOAT));

        // Adding STRING in the input means STRING has to fit something, which
        // it doesn't (no scalar rules yet, and TYPE_INT_OR_FLOAT doesn't
        // contain STRING).
        let with_string = interner().intern_type(&[INT, FLOAT, STRING], FlowFlags::EMPTY);
        assert!(!check(with_string, TYPE_INT_OR_FLOAT));
    }
}
