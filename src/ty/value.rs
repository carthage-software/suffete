use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::mem::size_of;

use crate::ElementId;
use crate::ElementKind;
use crate::typed::Typed;

/// A union of one or more [`Element`](crate::Element)s.
///
/// `elements` is sorted, deduplicated, and lives in the slice arena, so two
/// types with the same element set share one slice.
///
/// Flow flags do **not** live here — they ride on the [`TypeId`](crate::TypeId)
/// itself, so the same content shares a single arena slot regardless of
/// the flag combinations the consumer wraps it in. Read flags via
/// [`TypeId::flags`](crate::TypeId::flags); the [`Type`] value behind the
/// handle is content-only.
///
/// Construct via [`Interner::intern_type`](crate::interner::Interner::intern_type)
/// (or the wrappers on [`TypeId`](crate::TypeId)). Read via
/// [`TypeId::as_ref`](crate::TypeId::as_ref).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Type {
    pub elements: &'static [ElementId],
}

const _: () = assert!(size_of::<Type>() <= 16);

impl Display for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let len = self.elements.len();
        if len == 0 {
            return f.write_str("never");
        }
        if len == 1 {
            return Display::fmt(&self.elements[0], f);
        }
        let mut ids: Vec<String> = self
            .elements
            .iter()
            .map(|elem| {
                let s = elem.to_string();
                if elem.kind() == ElementKind::GenericParameter || (elem.has_intersection_types() && len > 1) {
                    format!("({s})")
                } else {
                    s
                }
            })
            .collect();
        ids.sort_unstable();
        let mut first = true;
        for id in &ids {
            if !first {
                f.write_str("|")?;
            }
            first = false;
            f.write_str(id)?;
        }
        Ok(())
    }
}

impl Typed for Type {
    fn pretty_with_indent(&self, indent: usize) -> String {
        let len = self.elements.len();
        if len == 0 {
            return String::from("never");
        }
        if len == 1 {
            return self.elements[0].pretty_with_indent(indent);
        }
        if len > 3 {
            let mut ids: Vec<String> = self
                .elements
                .iter()
                .map(|elem| {
                    let s = elem.pretty_with_indent(indent + 2);
                    if elem.has_intersection_types() { format!("({s})") } else { s }
                })
                .collect();
            ids.sort_unstable();
            let pad = " ".repeat(indent);
            let mut out = ids[0].clone();
            for id in &ids[1..] {
                out.push('\n');
                out.push_str(&pad);
                out.push_str("| ");
                out.push_str(id);
            }
            out
        } else {
            let mut ids: Vec<String> = self
                .elements
                .iter()
                .map(|elem| {
                    let s = elem.pretty_with_indent(indent);
                    if elem.has_intersection_types() && len > 1 { format!("({s})") } else { s }
                })
                .collect();
            ids.sort_unstable();
            ids.join(" | ")
        }
    }

    fn intersection_types(&self) -> &'static [ElementId] {
        &[]
    }

    fn has_intersection_types(&self) -> bool {
        false
    }

    fn can_be_intersected(&self) -> bool {
        false
    }

    fn is_complex(&self) -> bool {
        self.elements.len() > 3 || self.elements.iter().any(|e| e.is_complex())
    }
}
