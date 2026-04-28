use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
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

impl Display for ResourceInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str(match self {
            ResourceInfo::Any => "resource",
            ResourceInfo::Open => "open-resource",
            ResourceInfo::Closed => "closed-resource",
        })
    }
}
