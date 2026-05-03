use core::mem::size_of;

use crate::ElementId;
use crate::ElementListId;

/// `head & conj1 & conj2 & …`: the universal intersection wrapper.
/// `conjuncts` is interned, sorted, and non-empty by construction
/// (see [`ElementId::intersected`](crate::ElementId::intersected)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntersectedInfo {
    pub head: ElementId,
    pub conjuncts: ElementListId,
}

const _: () = assert!(size_of::<IntersectedInfo>() <= 8, "size budget exceeded");

impl core::fmt::Display for IntersectedInfo {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let i = crate::interner::interner();
        if self.head.has_intersection_types() {
            write!(f, "({})", self.head)?;
        } else {
            core::fmt::Display::fmt(&self.head, f)?;
        }
        for &conjunct in i.get_element_list(self.conjuncts) {
            if conjunct.has_intersection_types() {
                write!(f, "&({conjunct})")?;
            } else {
                write!(f, "&{conjunct}")?;
            }
        }
        Ok(())
    }
}

impl IntersectedInfo {
    #[inline]
    pub(crate) fn pretty_with_indent(self, indent: usize) -> String {
        use crate::typed::Typed;
        let i = crate::interner::interner();
        let mut out = String::new();
        let head_s = self.head.pretty_with_indent(indent);
        if self.head.has_intersection_types() {
            out.push('(');
            out.push_str(&head_s);
            out.push(')');
        } else {
            out.push_str(&head_s);
        }
        for &conjunct in i.get_element_list(self.conjuncts) {
            let s = conjunct.pretty_with_indent(indent);
            if conjunct.has_intersection_types() {
                out.push_str("&(");
                out.push_str(&s);
                out.push(')');
            } else {
                out.push('&');
                out.push_str(&s);
            }
        }
        out
    }
}
