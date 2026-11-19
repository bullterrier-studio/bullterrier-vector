//! The typed sources that trace a map.

use super::{Drawing, Failure, Shared};
use crate::node::{Given, Place, trace};
use crate::{Error, Expr, Field, Trace};

impl Drawing {
    /// A drawing of a map of square cells traced exactly (a dithered picture): `map` is a [`Field::map`] whose bytes are
    /// palette indexes, laid out as cells of `cell` map pixels each (the map's width and height are multiples of it and every
    /// cell is one byte). The drawing has one shape for each colour, in order of its byte, with the byte as its `label`
    /// attribute and no paint (set the fill after, per colour: `only(label=..)`). A shape's edges lie on cell sides:
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

    /// A drawing of a field traced at `levels` (numbers, Params or expressions, read once; a map reads 0 black to 1
    /// white): one shape for each level the field reaches, in the order given, the region where the field is at or above
    /// it, with the level as its `level` attribute and no paint. Stacked in order from the lowest, each painted by its
    /// level, the shapes are a posterized picture of filled cut-outs. The field is read at the centres of the trace's
    /// [`frame`](Trace::frame) cut into equal cells no wider than its [`cell`](Trace::cell) (design pixels); left out,
    /// they are a [`Field::map`]'s own box and one map pixel, which any other field refuses. `trace` also says how the
    /// outlines are tidied. A shape that meets the frame's border goes on
    /// past it and closes outside, so no edge lies along the border. A field that reads Params or drawings reads them
    /// with the values of the draw.
    ///
    /// No level or more than 256, a level that is not finite, a `frame` or `cell` that is not finite, or a tidying number
    /// that is below 0 or past 1e100, leaves the drawing with an [`error`](Drawing::error) naming the step; a `frame` or
    /// `cell` left out for a field that is not a map, a grid of more than 2^24 cells, or a trace that would pass
    /// `MAX_SEGMENTS` points, is refused when drawn.
    #[must_use]
    pub fn trace_levels<E: Into<Expr>>(map: &Field, levels: impl IntoIterator<Item = E>, trace: &Trace) -> Self {
        let mut given = vec![
            ("map", Given::Field(map.clone())),
            ("levels", Given::Numbers(levels.into_iter().map(Into::into).collect())),
            (
                "box",
                trace.frame.map_or(Given::Unset, |f| {
                    Given::Numbers([f.x, f.y, f.width, f.height].into_iter().map(Expr::from).collect())
                }),
            ),
            (
                "cell",
                trace.cell.map_or(Given::Unset, |cell| Given::Number(Expr::from(cell))),
            ),
        ];
        given.extend(tidy(trace));
        Self::source(&trace::TraceLevels, given)
    }

    /// A drawing of a map of colour labels: one shape for each byte the map holds, the region of the pixels with that
    /// byte, with the byte as its `label` attribute and no paint. `trace` says how the outlines are tidied and
    /// `max_colors` how many labels are allowed.
    ///
    /// The shapes tile the frame and its padding: they never overlap and leave no gap, whatever the tidying, so two inks
    /// never lie on one spot. The commonest label (the lowest byte of a tie) comes first and is whatever the others
    /// leave; the others follow in order of the byte, each its traced and tidied region without the labels after it.
    /// Tidied shapes are polylines within 0.01 design pixels of their curves, because they are cut by booleans.
    ///
    /// Refusals as for [`Drawing::trace_levels`]; a `trace` with a [`frame`](Trace::frame) or a [`cell`](Trace::cell)
    /// (labels are read at the map's own pixels) leaves the drawing with an [`error`](Drawing::error) naming it; a map
    /// with more than `max_colors` labels is refused when drawn, and so is one whose outlines are too many to cut apart
    /// within the draw's segment budget (naming `max_colors`).
    #[must_use]
    pub fn trace_labels(map: &Field, max_colors: usize, trace: &Trace) -> Self {
        let unread = [("box", trace.frame.is_some()), ("cell", trace.cell.is_some())];
        if let Some((name, _)) = unread.into_iter().find(|(_, set)| *set) {
            let place = Place {
                step: 0,
                verb: trace::TRACE_LABELS.name,
            };
            let error = Error::parameter(
                name,
                "is read only by trace_levels: labels are read at the map's own pixels",
            );
            return Self {
                last: Err(Failure::from(error.at_argument(&place, None))),
                cache: Shared::default(),
            };
        }
        #[expect(clippy::cast_precision_loss, reason = "a count of at most 256 is far below 2^53")]
        let colors = Given::Number(Expr::from(max_colors as f64));
        let mut given = vec![("map", Given::Field(map.clone())), ("max_colors", colors)];
        given.extend(tidy(trace));
        Self::source(&trace::TraceLabels, given)
    }
}

/// The tidying numbers of `trace`, as arguments.
fn tidy(trace: &Trace) -> Vec<(&'static str, Given)> {
    [
        ("smooth", &trace.smooth),
        ("tolerance", &trace.tolerance),
        ("open", &trace.open),
        ("close", &trace.close),
        ("min_area", &trace.min_area),
        ("min_hole", &trace.min_hole),
    ]
    .into_iter()
    .map(|(name, expr)| (name, Given::Number(expr.clone())))
    .collect()
}
