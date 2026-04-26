use std::hash::Hash;

use dashmap::DashMap;
use dashmap::Entry;

/// Append-only interner for whole values of `T`.
///
/// Each unique value gets a stable 1-based slot. Re-interning the same value
/// returns the same slot. The `'static` lifetime on stored references is
/// upheld by placing the arena inside a process-global `OnceLock`; values are
/// never moved or freed.
///
/// `T: Eq + Hash + Clone + Send + Sync + 'static` so the dedup map can hash
/// and compare entries, the storage can keep one copy and the dedup another,
/// and the arena itself can live in a global static.
pub struct Arena<T: Eq + Hash + Clone + Send + Sync + 'static> {
    storage: boxcar::Vec<T>,
    dedup: DashMap<T, u32>,
}

impl<T: Eq + Hash + Clone + Send + Sync + 'static> Arena<T> {
    /// A fresh, empty arena. Slot indices start at 1 (slot `0` is reserved as
    /// the niche so handle types can be `NonZeroU32`).
    #[inline]
    pub fn new() -> Self {
        Self { storage: boxcar::Vec::new(), dedup: DashMap::new() }
    }

    /// Intern `value`, returning its 1-based slot.
    ///
    /// If the same value has already been interned, the original slot is
    /// returned. Otherwise the value is appended to storage and assigned a
    /// fresh slot.
    ///
    /// Concurrent calls with the same value race on the dedup entry, but only
    /// one wins the insert; the others see the already-interned slot.
    pub fn intern(&self, value: T) -> u32 {
        match self.dedup.entry(value) {
            Entry::Occupied(occupied) => *occupied.get(),
            Entry::Vacant(vacant) => {
                let stored = vacant.key().clone();
                let zero_based = self.storage.push(stored) as u32;
                let slot = zero_based + 1;
                vacant.insert(slot);
                slot
            }
        }
    }

    /// Look up a slot. Returns `None` if `slot` is `0` or out of range.
    ///
    /// The returned reference is `'static` whenever the arena itself is in a
    /// `'static` location (e.g., inside a `OnceLock`). The compiler propagates
    /// the lifetime from the arena's outer borrow.
    #[inline]
    pub fn get(&self, slot: u32) -> Option<&T> {
        if slot == 0 {
            return None;
        }
        self.storage.get((slot - 1) as usize)
    }

    /// How many distinct values have been interned. Mostly useful for tests
    /// and diagnostics.
    #[inline]
    pub fn len(&self) -> usize {
        self.storage.count()
    }

    /// `true` when no values have been interned yet.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.storage.count() == 0
    }
}

impl<T: Eq + Hash + Clone + Send + Sync + 'static> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Append-only interner for slices of `T`.
///
/// Each unique slice content gets a stable 1-based slot. Re-interning the
/// same slice content returns the same slot. The stored slice is leaked to
/// `'static` (well-defined: the arena itself lives for the process lifetime),
/// so callers can hold `&'static [T]` references freely.
///
/// `T: Eq + Hash + Clone + Send + Sync + 'static` for the same reasons as
/// [`Arena`].
pub struct SliceArena<T: Eq + Hash + Clone + Send + Sync + 'static> {
    storage: boxcar::Vec<&'static [T]>,
    dedup: DashMap<Box<[T]>, u32>,
}

impl<T: Eq + Hash + Clone + Send + Sync + 'static> SliceArena<T> {
    #[inline]
    pub fn new() -> Self {
        Self { storage: boxcar::Vec::new(), dedup: DashMap::new() }
    }

    /// Intern `slice`, returning its 1-based slot.
    ///
    /// The slice contents are compared by value; a fresh `Box<[T]>` is
    /// allocated and leaked to `'static` only on first sight of a given
    /// content. Subsequent interns of slices with identical contents return
    /// the original slot and allocate nothing.
    pub fn intern(&self, slice: &[T]) -> u32 {
        let key: Box<[T]> = slice.into();

        match self.dedup.entry(key) {
            Entry::Occupied(occupied) => *occupied.get(),
            Entry::Vacant(vacant) => {
                // Leak the boxed slice to obtain a `'static` reference. This
                // is sound because the arena lives for the process lifetime;
                // the leaked memory is released when the process exits.
                let leaked: &'static [T] = Box::leak(vacant.key().clone());
                let zero_based = self.storage.push(leaked) as u32;
                let slot = zero_based + 1;
                vacant.insert(slot);
                slot
            }
        }
    }

    /// Look up a slot. Returns `None` if `slot` is `0` or out of range.
    #[inline]
    pub fn get(&self, slot: u32) -> Option<&'static [T]> {
        if slot == 0 {
            return None;
        }
        self.storage.get((slot - 1) as usize).copied()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.storage.count()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.storage.count() == 0
    }
}

impl<T: Eq + Hash + Clone + Send + Sync + 'static> Default for SliceArena<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn arena_dedups_values() {
        let arena: Arena<i64> = Arena::new();

        let a = arena.intern(42);
        let b = arena.intern(7);
        let c = arena.intern(42);

        assert_eq!(a, c, "interning the same value returns the same slot");
        assert_ne!(a, b, "different values get different slots");
        assert_eq!(arena.len(), 2);
    }

    #[test]
    fn arena_slots_are_one_based_and_get_roundtrips() {
        let arena: Arena<i64> = Arena::new();

        let slot = arena.intern(100);
        assert_eq!(slot, 1, "first intern returns slot 1, never 0");
        assert_eq!(arena.get(slot), Some(&100));
        assert_eq!(arena.get(0), None, "slot 0 is reserved as the niche");
        assert_eq!(arena.get(999), None, "out-of-range slot returns None");
    }

    #[test]
    fn arena_handles_concurrent_inserts_without_duplicates() {
        let arena: Arc<Arena<i64>> = Arc::new(Arena::new());
        let threads = 16;
        let inserts_per_thread = 200;

        let handles: Vec<_> = (0..threads)
            .map(|t| {
                let arena = Arc::clone(&arena);
                thread::spawn(move || {
                    for i in 0..inserts_per_thread {
                        // Overlapping value space across threads forces dedup races.
                        arena.intern((t + i) as i64);
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        // Distinct values produced: union of (t + i) for t in 0..16, i in 0..200.
        // That is the integers 0..(threads + inserts_per_thread - 1) inclusive.
        let expected_unique = (threads + inserts_per_thread - 1) as usize;
        assert_eq!(arena.len(), expected_unique);

        // Re-interning any of those values must not allocate a new slot.
        let baseline = arena.len();
        for t in 0..threads {
            for i in 0..inserts_per_thread {
                arena.intern((t + i) as i64);
            }
        }
        assert_eq!(arena.len(), baseline, "re-interning must be a no-op");
    }

    #[test]
    fn slice_arena_dedups_by_content() {
        let arena: SliceArena<u32> = SliceArena::new();

        let a = arena.intern(&[1, 2, 3]);
        let b = arena.intern(&[4, 5]);
        let c = arena.intern(&[1, 2, 3]);

        assert_eq!(a, c, "identical slice contents share a slot");
        assert_ne!(a, b);
        assert_eq!(arena.len(), 2);
    }

    #[test]
    fn slice_arena_returns_static_slices_with_correct_contents() {
        let arena: SliceArena<u32> = SliceArena::new();
        let slot = arena.intern(&[10, 20, 30]);

        let stored: &'static [u32] = arena.get(slot).expect("just-interned slot resolves");
        assert_eq!(stored, &[10, 20, 30]);
        assert_eq!(arena.get(0), None);
        assert_eq!(arena.get(999), None);
    }

    #[test]
    fn slice_arena_distinguishes_empty_from_singleton() {
        let arena: SliceArena<u32> = SliceArena::new();

        let empty = arena.intern(&[]);
        let one = arena.intern(&[0]);

        assert_ne!(empty, one);
        assert_eq!(arena.get(empty), Some(&[][..]));
        assert_eq!(arena.get(one), Some(&[0][..]));
    }
}
