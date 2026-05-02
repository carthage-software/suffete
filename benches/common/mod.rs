//! Shared helpers for the workload benchmark suite.
//!
//! Each bench in `benches/` is one focused workload running ~500ms of
//! simulated work per iteration. They share:
//!
//! - [`Rng`]: zero-dep deterministic PRNG (xorshift64*) so a given
//!   bench run produces identical inputs across measurements.
//! - [`TypePool`]: a pre-built fixture of representative `TypeId`s
//!   covering the kinds and shapes that turn up in real PHP code, with
//!   a population mix tuned to match what Mago actually feeds in (mostly
//!   singletons, occasional unions, rare deep nesting).
//! - [`bench_world`]: a `NullWorld` for benches that don't need a class
//!   hierarchy.

#![allow(
    dead_code,
    clippy::missing_docs_in_private_items,
    clippy::missing_inline_in_public_items,
    clippy::must_use_candidate,
    clippy::absolute_paths,
    clippy::std_instead_of_alloc,
    clippy::std_instead_of_core,
    clippy::missing_assert_message,
    clippy::arithmetic_side_effects,
    clippy::wildcard_imports,
    clippy::missing_const_for_fn,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::needless_pass_by_value
)]

use suffete::ElementId;
use suffete::FlowFlags;
use suffete::TypeId;
use suffete::interner::interner;
use suffete::prelude;
use suffete::world::NullWorld;

/// World instance used across benches.
#[inline]
pub fn bench_world() -> NullWorld {
    NullWorld
}

// -- Deterministic PRNG -------------------------------------------------------

/// Xorshift64* RNG: tiny, deterministic, non-cryptographic. Fine for
/// benches because we want reproducibility, not entropy.
#[derive(Clone, Copy)]
pub struct Rng(u64);

impl Rng {
    pub const fn new(seed: u64) -> Self {
        // Avoid the all-zero state by OR-ing in a fixed bit.
        Self(seed | 0x1)
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    /// Uniform pick from `0..n` (n must be >= 1).
    #[inline]
    pub fn pick(&mut self, n: usize) -> usize {
        (self.next_u64() as usize) % n.max(1)
    }

    /// Pick one element from `slice` uniformly. Panics if empty.
    #[inline]
    pub fn pick_from<T: Copy>(&mut self, slice: &[T]) -> T {
        slice[self.pick(slice.len())]
    }

    /// Roll a true with probability `p` (0..=100).
    #[inline]
    pub fn chance(&mut self, p: u32) -> bool {
        (self.next_u32() % 100) < p
    }
}

// -- Element constructors -----------------------------------------------------

#[inline]
pub fn ut(elem: ElementId) -> TypeId {
    interner().intern_type(&[elem], FlowFlags::EMPTY)
}

#[inline]
pub fn um(elems: &[ElementId]) -> TypeId {
    TypeId::union(elems)
}

// -- TypePool -----------------------------------------------------------------

/// A pre-built fixture of representative types. Generated once at bench
/// startup, then drawn from in the hot loop. The mix is roughly
/// 60% singletons (atomic kinds, named objects, literals),
/// 25% small unions (2-4 elements),
/// 10% wide unions (8-32 elements),
/// 5%  deeply nested (lists / arrays / iterables of unions of …).
pub struct TypePool {
    pub singletons: Vec<TypeId>,
    pub small_unions: Vec<TypeId>,
    pub wide_unions: Vec<TypeId>,
    pub deep_nested: Vec<TypeId>,
    /// Flattened pick distribution: each TypeId appears proportional to its
    /// category weight, so `pool.pick(rng)` matches the realistic mix above.
    pub weighted: Vec<TypeId>,
}

impl TypePool {
    pub fn new(seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let singletons = build_singletons(&mut rng);
        let small_unions = build_small_unions(&mut rng, &singletons);
        let wide_unions = build_wide_unions(&mut rng, &singletons);
        let deep_nested = build_deep_nested(&mut rng, &singletons);

        let mut weighted: Vec<TypeId> = Vec::with_capacity(1000);
        for _ in 0..600 {
            weighted.push(rng_pick(&mut rng, &singletons));
        }
        for _ in 0..250 {
            weighted.push(rng_pick(&mut rng, &small_unions));
        }
        for _ in 0..100 {
            weighted.push(rng_pick(&mut rng, &wide_unions));
        }
        for _ in 0..50 {
            weighted.push(rng_pick(&mut rng, &deep_nested));
        }

        Self { singletons, small_unions, wide_unions, deep_nested, weighted }
    }

    /// Random type from the realistic-mix distribution.
    #[inline]
    pub fn pick(&self, rng: &mut Rng) -> TypeId {
        rng.pick_from(&self.weighted)
    }
}

fn rng_pick<T: Copy>(rng: &mut Rng, slice: &[T]) -> T {
    slice[rng.pick(slice.len())]
}

fn build_singletons(rng: &mut Rng) -> Vec<TypeId> {
    let mut out: Vec<TypeId> = vec![
        prelude::TYPE_INT,
        prelude::TYPE_STRING,
        prelude::TYPE_FLOAT,
        prelude::TYPE_BOOL,
        prelude::TYPE_NULL,
        prelude::TYPE_VOID,
        prelude::TYPE_MIXED,
        prelude::TYPE_NEVER,
        prelude::TYPE_OBJECT,
        prelude::TYPE_ARRAY_KEY,
        prelude::TYPE_SCALAR,
        prelude::TYPE_NUMERIC,
    ];
    // Literal ints, varied.
    for _ in 0..30 {
        let v = (rng.next_u64() % 200) as i64 - 100;
        out.push(ut(ElementId::int_literal(v)));
    }
    // Literal strings, varied.
    let words = ["foo", "bar", "baz", "qux", "hello", "world", "abc", "xyz", "0", "1", ""];
    for _ in 0..30 {
        let w = rng.pick_from(&words);
        out.push(ut(ElementId::string_literal(w)));
    }
    // Named objects.
    let classes = ["Foo", "Bar", "Baz", "Qux", "Container", "List", "Map", "Set"];
    for c in classes {
        out.push(ut(ElementId::object_named(c)));
    }
    // Refined ints (ranges).
    for _ in 0..10 {
        let lo = (rng.next_u64() % 100) as i64;
        out.push(ut(ElementId::int_range(Some(lo), Some(lo + 50))));
    }
    out
}

fn build_small_unions(rng: &mut Rng, atoms: &[TypeId]) -> Vec<TypeId> {
    let mut out = Vec::with_capacity(40);
    for _ in 0..40 {
        let n = 2 + (rng.next_u32() % 3) as usize;
        let mut elems: Vec<ElementId> = Vec::with_capacity(n);
        for _ in 0..n {
            let t = rng_pick(rng, atoms);
            elems.extend(t.as_ref().elements);
        }
        out.push(TypeId::union(&elems));
    }
    out
}

fn build_wide_unions(rng: &mut Rng, atoms: &[TypeId]) -> Vec<TypeId> {
    let mut out = Vec::with_capacity(20);
    for _ in 0..20 {
        let n = 8 + (rng.next_u32() % 25) as usize;
        let mut elems: Vec<ElementId> = Vec::with_capacity(n * 2);
        for _ in 0..n {
            let t = rng_pick(rng, atoms);
            elems.extend(t.as_ref().elements);
        }
        out.push(TypeId::union(&elems));
    }
    out
}

fn build_deep_nested(rng: &mut Rng, atoms: &[TypeId]) -> Vec<TypeId> {
    let mut out = Vec::with_capacity(20);
    for _ in 0..15 {
        let inner = rng_pick(rng, atoms);
        // list<list<list<inner>>>
        let l1 = ut(ElementId::list(inner, false));
        let l2 = ut(ElementId::list(l1, false));
        let l3 = ut(ElementId::list(l2, false));
        out.push(l3);
    }
    // Some keyed-array-of-list shapes.
    for _ in 0..5 {
        let value_inner = rng_pick(rng, atoms);
        let list_t = ut(ElementId::list(value_inner, false));
        out.push(ut(ElementId::keyed_unsealed(prelude::TYPE_STRING, list_t, false)));
    }
    out
}
