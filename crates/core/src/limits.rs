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
// constant: formats.md "Limits and security" sets MAX_DEPTH to 128 for documents; expressions written in code follow
// it. A stack takes about four JSON levels, so a code drawing nested over about 31 stacks builds but `to_json` refuses it
pub(crate) const MAX_DEPTH: usize = 128;

/// The most nodes an expression may have, a shared part counted each time it is used, as evaluation visits it. It
/// bounds the work of one number: an expression that adds itself to itself 28 times is 2^29 nodes.
// constant: neither formats.md nor api.md names a limit on expressions; 10,000 nodes is far beyond any expression
// written by hand and evaluates in microseconds
pub(crate) const MAX_EXPR_NODES: usize = 10_000;

/// The most straight edges one draw's pixel output may flatten curves into, counted as they are made; also the most
/// that the outline of one shape's stroke may have.
// constant: formats.md "Limits and security" sets MAX_SEGMENTS to 10,000,000
pub(crate) const MAX_SEGMENTS: usize = 10_000_000;

/// The most tessellation work one draw's mesh may ask for: for each vertex the tessellator makes, the most edges of its
/// layer that a sweep line crosses at once.
// constant: measured on this machine in release builds, tessellating costs about 4 ns a unit however the outline is
// built (a star polygon, a stroked one, a comb), so 250 million is about a second; a shape of ordinary size (a circle,
// a glyph, a stroke around one) crosses 2 to 6 edges at a time and spends under a tenth of this on a million vertices
pub(crate) const MAX_TESSELLATION_WORK: usize = 250_000_000;

/// The longest document `from_json` reads and `to_json` writes, in bytes of UTF-8.
// constant: formats.md "Limits and security" sets MAX_DOCUMENT_BYTES to 16 MiB
pub(crate) const MAX_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;

/// The most call objects (sources, steps and expressions) a document may hold.
// constant: formats.md "Limits and security" sets MAX_CALLS to 100,000
pub(crate) const MAX_CALLS: usize = 100_000;

/// The most distinct Params a drawing may read, those a `with_values` fixed included, and the most entries of a
/// document's `params`.
// constant: formats.md "Limits and security" sets MAX_PARAMS to 1,000; building a step costs time in proportion to the
// Params already read, so a drawing of 8,000 stalls for seconds, and no drawing a person made has a tenth of that
pub(crate) const MAX_PARAMS: usize = 1_000;

/// The longest font `Font::new` reads, in bytes: the file given, and the font a WOFF2 file unpacks to (a file of a few
/// hundred kilobytes can unpack to many megabytes; the decoder stops at 128 MiB and a ratio of 100 to 1 by itself).
// constant: a font with every glyph of Unicode in one file is about 20 MB (Noto Sans CJK, 16 MB per weight); this
// leaves room for it and for variable fonts of 30 MB, and stops a hostile input from asking for more
pub(crate) const MAX_FONT_BYTES: usize = 32 * 1024 * 1024;

/// The most glyphs one `text` call shapes (spaces included), counted from its characters before it shapes.
// constant: a page of dense text is 5,000 characters; 100,000 is a book chapter. The glyph outlines it draws are
// charged to `MAX_OUTLINE_SEGMENTS` and the shapes to `MAX_SHAPES`
pub(crate) const MAX_GLYPHS: usize = 100_000;

/// The most axes, and the most features, one `text` call gives.
// constant: the fonts that exist have under 20 axes (Roboto Flex 13) and a few dozen features; an axis costs memory in
// every run of a text and a feature time in every shaping, so a call may not name thousands
pub(crate) const MAX_SETTINGS: usize = 64;

/// The most segments the glyph outlines of one draw may have, counted as they are put into shapes.
// constant: an ordinary glyph has 20 to 60 segments, so 10 million is 150,000 glyphs; the same as `MAX_SEGMENTS`
pub(crate) const MAX_OUTLINE_SEGMENTS: usize = 10_000_000;

/// What one draw has made so far. A node charges what it creates as it creates it, before it allocates.
#[derive(Debug, Default)]
pub(crate) struct Work {
    shapes: Cell<usize>,
    segments: Cell<usize>,
    tessellation: Cell<usize>,
    outline: Cell<usize>,
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
    /// Charges `count` straight edges made for `place`.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `place`, the counter and its limit, if the draw has now made more than
    /// [`MAX_SEGMENTS`].
    pub(crate) fn made_segments(&self, count: usize, place: &str) -> Result<(), Error> {
        let total = self.segments.get().saturating_add(count);
        self.segments.set(total);
        if total > MAX_SEGMENTS {
            return Err(Error::parameter(place, format!("MAX_SEGMENTS {MAX_SEGMENTS} passed")));
        }
        Ok(())
    }

    /// Charges `count` units of tessellation work made for `place`.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `place`, the counter and its limit, if the draw has now made more than
    /// [`MAX_TESSELLATION_WORK`].
    pub(crate) fn made_tessellation(&self, count: usize, place: &str) -> Result<(), Error> {
        let total = self.tessellation.get().saturating_add(count);
        self.tessellation.set(total);
        if total > MAX_TESSELLATION_WORK {
            return Err(Error::parameter(
                place,
                format!("MAX_TESSELLATION_WORK {MAX_TESSELLATION_WORK} passed"),
            ));
        }
        Ok(())
    }

    /// Charges `count` segments of glyph outlines made for `place`.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `place`, the counter and its limit, if the draw has now made more than
    /// [`MAX_OUTLINE_SEGMENTS`].
    pub(crate) fn made_outline(&self, count: usize, place: &Place) -> Result<(), Error> {
        let total = self.outline.get().saturating_add(count);
        self.outline.set(total);
        if total > MAX_OUTLINE_SEGMENTS {
            return Err(Error::parameter(
                &place.to_string(),
                format!("MAX_OUTLINE_SEGMENTS {MAX_OUTLINE_SEGMENTS} passed"),
            ));
        }
        Ok(())
    }
}
