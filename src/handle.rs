//! The `define_handle!` macro: boilerplate for declaring an interned-handle
//! newtype around `NonZeroU32`.
//!
//! Each handle lives next to the payload it points at (e.g. `IntRangeId` is
//! defined in the same file as `IntRange`, `KnownItemsId` next to
//! `KnownItemEntry`). This file only carries the macro that mints them.
//!
//! The macro generates:
//! - a `pub struct $name(NonZeroU32)` newtype,
//! - the standard derive set (`Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`,
//!   `Hash`, `PartialOrd`, `Ord`),
//! - a compile-time size assertion (`size_of::<$name>() == 4`).
//!
//! Use it like:
//! ```ignore
//! use crate::handle::define_handle;
//!
//! define_handle! {
//!     /// Handle to ...
//!     KnownItemsId
//! }
//! ```

macro_rules! define_handle {
    ($(#[$attr:meta])* $name:ident) => {
        $(#[$attr])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(::std::num::NonZeroU32);

        const _: () = assert!(::std::mem::size_of::<$name>() == 4);
        const _: () = assert!(::std::mem::size_of::<Option<$name>>() == 4);
    };
}

pub(crate) use define_handle;
