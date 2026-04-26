//! Bool family: `bool`, `true`, `false`.
//!
//! `true` and `false` are both subtypes of `bool`. They are not subtypes of
//! each other. `bool` is not a subtype of `true` or `false` (a `bool` could
//! be either at runtime).

use crate::ElementId;
use crate::prelude::FALSE;
use crate::prelude::TRUE;

/// `true | false <: bool`. The dispatcher passes `container` for symmetry
/// with other families; here it must be `BOOL`. Reflexivity (`bool <: bool`)
/// is the dispatcher's responsibility.
pub fn refines(input: ElementId, _container: ElementId) -> bool {
    input == TRUE || input == FALSE
}
