//! The typed sources that trace a map.

use super::Drawing;
use crate::node::{Given, trace};
use crate::{Expr, Field};

impl Drawing {
    /// A drawing of a map of square cells traced exactly (a dithered picture): `map` is a [`Field::map`] whose bytes are
    /// palette indexes, laid out as cells of `cell` pixels each (the map's width and height are multiples of it and every
    /// cell is one byte). The drawing has one shape for each colour, in order of its byte, with the byte as its `level`
    /// attribute and no paint (set the fill after, per colour: `only(level=..)`). A shape's edges lie on cell sides:
    /// cells of one colour that touch along a side are merged, holes are kept, nothing is rounded.
    ///
    /// A `cell` that is not a whole number from 1, or `max_colors` that is not one from 1 to 256, leaves the drawing with
    /// an [`error`](Drawing::error) naming the step; a map that is not a [`Field::map`], that does not divide into
    /// cells, whose cells are not one colour each, or that has more than `max_colors` colours, is refused when drawn.
    #[must_use]
    pub fn trace_cells(map: &Field, cell: usize, max_colors: usize) -> Self {
        let number = |value: usize| {
            #[expect(clippy::cast_precision_loss, reason = "a count of pixels is far below 2^53")]
            Given::Number(Expr::from(value as f64))
        };
        Self::source(
            &trace::TraceCells,
            vec![
                ("map", Given::Field(map.clone())),
                ("cell", number(cell)),
                ("max_colors", number(max_colors)),
            ],
        )
    }
}
