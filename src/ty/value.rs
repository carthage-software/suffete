/// A union of one or more [`Element`](crate::Element)s, plus flow flags.
///
/// **Stub.** The real shape (a sorted, deduplicated `&'static [ElementId]`
/// plus a [`FlowFlags`](crate::FlowFlags) bitfield) comes when the interner
/// lands. For now this is a marker type so the public API can compile.
#[derive(Debug)]
pub struct Type {
    _stub: (),
}
