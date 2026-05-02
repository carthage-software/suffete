//! Compatibility relations: can two types refer to the same value?
//!
//! Two questions, two functions:
//!
//! - [`statically_compatible`]: at the type-system level, does there
//!   exist a runtime value the type system admits as inhabiting both
//!   types? Identical to [`crate::lattice::overlaps`]; re-exported here
//!   so callers can find both relations in one place.
//! - [`runtime_compatible`]: at the PHP runtime level, could a single
//!   value be a member of both types after PHP erases the information
//!   the type system tracks but the runtime does not (object generic
//!   arguments, intersection conjuncts beyond the head class, etc.)?
//!   More permissive than [`statically_compatible`].
//!
//! Concrete differences:
//!
//! - `Cell<int>` vs `Cell<string>` ; statically disjoint under invariance,
//!   runtime-compatible because PHP cannot tell two `Cell` instances
//!   apart by their generic argument.
//! - `Foo&Bar` vs `Foo` ; runtime-compatible: an instance of `Foo&Bar`
//!   is also an instance of `Foo` for `instanceof`.
//! - `int` vs `string` ; incompatible under both relations.
//!
//! Both functions write into a [`crate::lattice::LatticeReport`] for
//! parity with the rest of the lattice surface, although the runtime
//! variant currently records nothing.

use mago_atom::Atom;

use crate::ElementId;
use crate::ElementKind;
use crate::FlowFlags;
use crate::TypeId;
use crate::interner::interner;
use crate::lattice;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::prelude::MIXED;
use crate::prelude::NEVER;
use crate::prelude::PLACEHOLDER;
use crate::world::World;

/// Static compatibility: a value the type system admits in `a` is also
/// admitted in `b`. Equivalent to [`lattice::overlaps`].
#[inline]
pub fn statically_compatible<W: World>(
    a: TypeId,
    b: TypeId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    lattice::overlaps(a, b, world, options, report)
}

/// Runtime compatibility: a single PHP runtime value could inhabit both
/// `a` and `b` once the runtime has erased the information the type
/// system tracks but the PHP engine does not.
///
/// More permissive than [`statically_compatible`]: same-class objects
/// with disjoint generic arguments are compatible, and intersection
/// conjuncts beyond the head class are ignored.
#[inline]
pub fn runtime_compatible<W: World>(
    a: TypeId,
    b: TypeId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let a_view = a.as_ref();
    let b_view = b.as_ref();
    a_view
        .elements
        .iter()
        .any(|x| b_view.elements.iter().any(|y| element_runtime_compatible(*x, *y, world, options, report)))
}

#[inline]
fn element_runtime_compatible<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    if a == NEVER || b == NEVER {
        return false;
    }
    if a == b {
        return true;
    }
    if a == MIXED || b == MIXED || a == PLACEHOLDER || b == PLACEHOLDER {
        return true;
    }

    let a_obj = is_object_family(a.kind());
    let b_obj = is_object_family(b.kind());
    if a_obj && b_obj {
        return objects_runtime_compatible(a, b, world);
    }
    if a_obj != b_obj {
        return false;
    }

    let i = interner();
    let at = i.intern_type(&[a], FlowFlags::EMPTY);
    let bt = i.intern_type(&[b], FlowFlags::EMPTY);
    lattice::overlaps(at, bt, world, options, report)
}

/// `true` iff some nominal class on one side is related (either direction
/// of `descends_from`) to some nominal class on the other. An empty
/// nominal set on a side means "any class", which is compatible with
/// anything in the family.
#[inline]
fn objects_runtime_compatible<W: World>(a: ElementId, b: ElementId, world: &W) -> bool {
    let a_classes = nominal_classes(a);
    let b_classes = nominal_classes(b);

    if a_classes.is_empty() || b_classes.is_empty() {
        return true;
    }

    a_classes.iter().any(|ac| b_classes.iter().any(|bc| world.descends_from(*ac, *bc) || world.descends_from(*bc, *ac)))
}

/// Collect the nominal class names an object-family element identifies
/// at runtime. Empty when the element is purely structural (`object`,
/// `object{...}`, `has-method`, `has-property`) and therefore matches any
/// class.
#[inline]
fn nominal_classes(elem: ElementId) -> Vec<Atom> {
    let i = interner();
    match elem.kind() {
        ElementKind::Object => {
            let info = i.get_object(elem);
            let mut out = vec![info.name];
            if let Some(id) = info.intersections {
                for &conjunct in i.get_element_list(id) {
                    out.extend(nominal_classes(conjunct));
                }
            }
            out
        }
        ElementKind::Enum => vec![i.get_enum(elem).name],
        _ => Vec::new(),
    }
}

#[inline]
const fn is_object_family(kind: ElementKind) -> bool {
    matches!(
        kind,
        ElementKind::Object
            | ElementKind::Enum
            | ElementKind::ObjectShape
            | ElementKind::HasMethod
            | ElementKind::HasProperty
            | ElementKind::ObjectAny
    )
}
