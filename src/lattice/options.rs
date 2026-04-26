use crate::TypeId;

/// Caller-controlled options for the lattice operations
/// ([`refines`](crate::lattice::refines),
/// [`generalizes`](crate::lattice::generalizes),
/// [`intersects`](crate::lattice::intersects)).
///
/// Each field tweaks the algorithm without changing its meaning at the
/// type level. Defaults are all `false`. Use [`LatticeOptions::default`]
/// for the common case and chain `with_*` builders for any flags you
/// need; or derive options from a type's [`FlowFlags`](crate::FlowFlags)
/// via [`LatticeOptions::of_type`] / [`LatticeOptions::assertion_of_type`].
///
/// `LatticeOptions` is `Copy` and small enough to pass by value; the
/// operations take it that way.
#[derive(Debug, Clone, Copy, Default)]
pub struct LatticeOptions {
    /// Skip `null` elements in the input union when refining. Used by
    /// nullsafe-aware analyzers: a `?int` argument can be passed to an
    /// `int` parameter under this flag without a "null leak" diagnostic.
    pub ignore_null: bool,
    /// Skip the `false` element in the input union when refining. Used by
    /// `int|false` style return values that the caller has narrowed away
    /// from `false`.
    pub ignore_false: bool,
    /// The refinement is being checked inside a runtime assertion (e.g.
    /// `assert($x instanceof Foo)`). Some rules become more permissive
    /// in this mode.
    pub inside_assertion: bool,
}

impl LatticeOptions {
    /// Derive options from a type's [`FlowFlags`](crate::FlowFlags):
    ///
    /// `ignore_null` mirrors `flags.ignore_nullable_issues()` and
    /// `ignore_false` mirrors `flags.ignore_falsable_issues()`.
    /// `inside_assertion` stays `false`.
    pub fn of_type(ty: TypeId) -> Self {
        let f = ty.as_ref().flags;
        Self {
            ignore_null: f.ignore_nullable_issues(),
            ignore_false: f.ignore_falsable_issues(),
            inside_assertion: false,
        }
    }

    /// Same as [`of_type`](Self::of_type), but with `inside_assertion` set.
    pub fn assertion_of_type(ty: TypeId) -> Self {
        Self::of_type(ty).inside_assertion()
    }

    /// Set [`ignore_null`](Self::ignore_null) to `true`.
    #[must_use]
    pub const fn with_ignore_null(mut self) -> Self {
        self.ignore_null = true;
        self
    }

    /// Set [`ignore_false`](Self::ignore_false) to `true`.
    #[must_use]
    pub const fn with_ignore_false(mut self) -> Self {
        self.ignore_false = true;
        self
    }

    /// Set [`inside_assertion`](Self::inside_assertion) to `true`.
    #[must_use]
    pub const fn inside_assertion(mut self) -> Self {
        self.inside_assertion = true;
        self
    }
}
