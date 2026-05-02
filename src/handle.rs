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
        pub struct $name(::core::num::NonZeroU32);

        impl $name {
            /// Construct from a 1-based arena slot. Reserved for use by the
            /// interner; user code never builds these directly.
            #[inline]
            pub(crate) const fn from_slot(slot: u32) -> Self {
                // SAFETY: caller guarantees `slot != 0`. The interner only
                // assigns slots starting at 1.
                Self(unsafe { ::core::num::NonZeroU32::new_unchecked(slot) })
            }

            /// The 1-based arena slot this handle refers to. Used by the
            /// interner for lookups.
            #[inline]
            pub(crate) const fn slot(self) -> u32 {
                self.0.get()
            }
        }

        const _: () = assert!(::core::mem::size_of::<$name>() == 4);
        const _: () = assert!(::core::mem::size_of::<Option<$name>>() == 4);
    };
}

pub(crate) use define_handle;
