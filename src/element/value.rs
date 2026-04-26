/// A single element of a [`Type`](crate::Type).
///
/// **Stub.** The real shape (per-kind structs behind a borrowed view enum,
/// SoA arenas) is the next layer of work. For now this is a marker type that
/// only exists so the public API can compile and `cargo doc` has somewhere to
/// link to. Do not rely on this representation.
#[derive(Debug)]
pub struct Element {
    _stub: (),
}
