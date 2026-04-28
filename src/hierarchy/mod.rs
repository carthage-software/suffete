//! Cross-hierarchy template-argument propagation.
//!
//! When a class extends or implements a parameterised parent, it passes
//! type-arguments to the parent — and those arguments may mention the
//! child's templates. The analyzer needs an O(1) answer to "what does
//! `child` ultimately pass to `ancestor`'s `position`-th type parameter"
//! for any direct or transitive ancestor.
//!
//! This module precomputes the closure once and exposes it through
//! [`Hierarchy::arg`] / [`Hierarchy::args`]. Plug it into a [`World`]
//! implementation's [`World::inherited_template_argument`] and the
//! O(depth × arity) cost vanishes from every query.
//!
//! Construction is two-step:
//!
//! ```ignore
//! let mut builder = HierarchyBuilder::new();
//! builder.add_edge(child, parent, args_in_child_namespace);
//! let hierarchy = builder.build(&world);
//! let arg = hierarchy.arg(child, ancestor, position);
//! ```
//!
//! `build` walks the registered direct edges; for each transitive
//! `(child, ancestor)` pair it composes the chain by substituting each
//! intermediate parent's templates with the child's actual arguments to
//! that parent. The substitution algorithm is [`crate::template::substitute`].

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use mago_atom::Atom;

use crate::TypeId;
use crate::element::payload::DefiningEntity;
use crate::element::payload::GenericParameterInfo;
use crate::interner::interner;
use crate::template::substitute;
use crate::world::World;

/// Builder collecting direct parent edges before transitive composition.
#[derive(Debug, Default, Clone)]
pub struct HierarchyBuilder {
    edges: BTreeMap<(Atom, Atom), Vec<TypeId>>,
}

impl HierarchyBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register `child extends/implements parent<args>` where `args` is
    /// expressed in `child`'s template namespace. Idempotent on
    /// `(child, parent)`; the latest call wins.
    pub fn add_edge(&mut self, child: Atom, parent: Atom, args: Vec<TypeId>) {
        self.edges.insert((child, parent), args);
    }

    /// Compute the transitive closure of inherited template arguments.
    /// `world` supplies template-name-to-position lookups for each
    /// intermediate class via [`World::template_parameter_index`].
    pub fn build<W: World>(self, world: &W) -> Hierarchy {
        let mut parents_of: BTreeMap<Atom, Vec<Atom>> = BTreeMap::new();
        for &(child, parent) in self.edges.keys() {
            parents_of.entry(child).or_default().push(parent);
        }

        let mut composed: BTreeMap<(Atom, Atom), Vec<TypeId>> = self.edges.clone();

        let children: Vec<Atom> = parents_of.keys().copied().collect();
        for child in children {
            let mut visiting: BTreeSet<Atom> = BTreeSet::new();
            walk(child, &self.edges, &parents_of, &mut composed, &mut visiting, world);
        }

        Hierarchy { composed }
    }
}

fn walk<W: World>(
    child: Atom,
    edges: &BTreeMap<(Atom, Atom), Vec<TypeId>>,
    parents_of: &BTreeMap<Atom, Vec<Atom>>,
    composed: &mut BTreeMap<(Atom, Atom), Vec<TypeId>>,
    visiting: &mut BTreeSet<Atom>,
    world: &W,
) {
    if !visiting.insert(child) {
        return;
    }
    let Some(parents) = parents_of.get(&child) else {
        visiting.remove(&child);
        return;
    };

    for &parent in parents {
        walk(parent, edges, parents_of, composed, visiting, world);

        let Some(child_to_parent) = edges.get(&(child, parent)).cloned() else {
            continue;
        };

        let parent_entity = interner().intern_defining_entity(DefiningEntity::ClassLike(parent));

        let grandparents: Vec<Atom> = composed.keys().filter(|(c, _)| *c == parent).map(|(_, gp)| *gp).collect();

        for grandparent in grandparents {
            if grandparent == child || grandparent == parent {
                continue;
            }
            if composed.contains_key(&(child, grandparent)) {
                continue;
            }
            let Some(parent_to_grandparent) = composed.get(&(parent, grandparent)).cloned() else {
                continue;
            };

            let composed_args: Vec<TypeId> = parent_to_grandparent
                .into_iter()
                .map(|arg| {
                    substitute(arg, &|info: &GenericParameterInfo| -> Option<TypeId> {
                        if info.defining_entity != parent_entity {
                            return None;
                        }
                        let pos = world.template_parameter_index(parent, info.name)?;
                        child_to_parent.get(pos).copied()
                    })
                })
                .collect();

            composed.insert((child, grandparent), composed_args);
        }
    }

    visiting.remove(&child);
}

/// Precomputed transitive closure of cross-hierarchy template arguments.
/// O(1) lookup keyed on `(child, ancestor)`.
#[derive(Debug, Clone, Default)]
pub struct Hierarchy {
    composed: BTreeMap<(Atom, Atom), Vec<TypeId>>,
}

impl Hierarchy {
    /// Composed type-argument list `child` passes to `ancestor`, in
    /// `ancestor`'s declaration order, expressed in `child`'s template
    /// namespace. `None` when `child` does not descend from `ancestor`
    /// or no edges were registered along the path.
    pub fn args(&self, child: Atom, ancestor: Atom) -> Option<&[TypeId]> {
        self.composed.get(&(child, ancestor)).map(Vec::as_slice)
    }

    /// Single positional argument; convenience for [`Hierarchy::args`]
    /// followed by `[position]`.
    pub fn arg(&self, child: Atom, ancestor: Atom, position: usize) -> Option<TypeId> {
        self.args(child, ancestor).and_then(|args| args.get(position).copied())
    }

    /// Iterate every `((child, ancestor), args)` triple recorded in the
    /// closure. Useful for building reverse indexes or for a wrapper
    /// [`World`] that delegates [`World::descends_from`].
    pub fn iter(&self) -> impl Iterator<Item = ((Atom, Atom), &[TypeId])> {
        self.composed.iter().map(|(&k, v)| (k, v.as_slice()))
    }
}
