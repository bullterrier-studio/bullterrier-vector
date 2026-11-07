//! The steps that reshape the list: `split` and `merge`.

use super::Drawing;
use crate::node::{Given, items};

impl Drawing {
    /// This drawing with every shape split into its parts: each closed outline together with the holes inside it, and
    /// each open line (as [`Path::split`](crate::Path::split)). A part is a shape of its own that keeps the attributes
    /// and style of its shape and has `part` (from 0, counting within the shape it came from) and the middle of its
    /// own bounds as `origin`, replacing one the shape carried (a glyph's). `order` survives; the list is renumbered.
    /// A shape with nothing in it stays, with `part` 0.
    #[must_use]
    pub fn split(&self) -> Self {
        self.then(&items::Split, Vec::new())
    }

    /// This drawing with all its shapes merged into one: every sub-path, in list order, with the attributes and style
    /// of the first shape. The sub-paths are joined, not united: overlaps stay and fill as the nonzero rule says.
    #[must_use]
    pub fn merge(&self) -> Self {
        self.then(&items::Merge, Vec::new())
    }

    /// This drawing with the shapes whose attribute `by` is equal merged into one shape each, as [`Drawing::merge`]
    /// does for all of them. The merged shape sits where its first member was, so the groups follow in the order
    /// their first members appeared; shapes without the attribute are one group. `by` is one of the names
    /// [`Select`](crate::Select) takes (`index` included).
    ///
    /// An unknown name leaves the drawing with an [`error`](Drawing::error). An attribute that no shape of the list
    /// carries is refused when the drawing is drawn, naming it (`step 2 (merge).by`); an empty list has nothing to
    /// ask.
    #[must_use]
    pub fn merge_by(&self, by: &str) -> Self {
        self.then(&items::Merge, vec![("by", Given::Text(by.to_owned()))])
    }
}
