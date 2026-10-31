//! The limits that keep a small drawing from asking for unbounded work (formats.md, "Limits and security"), and the
//! counter a draw charges against them. Only the limits a node of this version can reach are here; each other limit
//! arrives with the first step that produces what it counts.

use crate::Error;
use crate::node::Place;
use std::cell::Cell;

/// The most shapes one draw may make, counted as they are made.
// constant: formats.md "Limits and security" sets MAX_SHAPES to 1,000,000; the stack of a drawing shared twice, nested
// 60 times, is a few lines of code that would need more memory than any machine has
pub(crate) const MAX_SHAPES: usize = 1_000_000;

/// The deepest nesting of drawings inside drawings a call accepts, a lone drawing counting as 1; also the deepest
/// expression. It bounds the recursion that draws and prints them. A long chain of steps costs no depth.
// constant: formats.md "Limits and security" sets MAX_DEPTH to 128 for documents; drawings and expressions written in
// code follow it so that what is written in code can be saved and loaded back
pub(crate) const MAX_DEPTH: usize = 128;

/// What one draw has made so far. A node charges what it creates as it creates it, before it allocates.
#[derive(Debug, Default)]
pub(crate) struct Work {
    shapes: Cell<usize>,
}

impl Work {
    /// A draw that has made nothing.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Charges `count` shapes made at `place`.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `place`, the counter and its limit, if the draw has now made more than
    /// [`MAX_SHAPES`].
    pub(crate) fn made_shapes(&self, count: usize, place: &Place) -> Result<(), Error> {
        let total = self.shapes.get().saturating_add(count);
        self.shapes.set(total);
        if total > MAX_SHAPES {
            return Err(Error::parameter(
                &place.to_string(),
                format!("MAX_SHAPES {MAX_SHAPES} passed"),
            ));
        }
        Ok(())
    }
}
