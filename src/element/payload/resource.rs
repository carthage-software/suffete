use std::mem::size_of;

/// PHP `resource`, optionally narrowed to open or closed state.
///
/// Subtyping on resources is purely on this state plus the kind, never
/// structural.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
#[non_exhaustive]
pub enum ResourceInfo {
    /// `resource`: any resource, regardless of state.
    Any,
    /// `open-resource`.
    Open,
    /// `closed-resource`.
    Closed,
}

const _: () = assert!(size_of::<ResourceInfo>() == 1);
