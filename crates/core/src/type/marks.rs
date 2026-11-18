//! `marks`: the typed call of overlay marks, brackets round boxes and crosshairs on points, with labels.

use super::font::Font;
use crate::node::marks::{BOX, MarksNode, POINT};
use crate::node::{Args, Given};
use crate::{Drawing, Expr};

/// A box to bracket: the place of its top left corner, its width and height, and a label if it has one. The numbers
/// are in design pixels, each a number, a `Param` or an expression read once.
#[derive(Debug, Clone)]
pub struct BoxMark {
    numbers: [Expr; 4],
    label: Option<String>,
}

impl BoxMark {
    /// The box with its top left corner at (`x`, `y`), `width` wide and `height` high (both above 0).
    #[must_use]
    pub fn new(x: impl Into<Expr>, y: impl Into<Expr>, width: impl Into<Expr>, height: impl Into<Expr>) -> Self {
        Self {
            numbers: [x.into(), y.into(), width.into(), height.into()],
            label: None,
        }
    }

    /// This box with `label`, one line of printable characters, on a leader from its top right corner. An empty label
    /// is none.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    fn args(self) -> Args {
        let [x, y, width, height] = self.numbers;
        let mut given = vec![
            ("x", Given::Number(x)),
            ("y", Given::Number(y)),
            ("width", Given::Number(width)),
            ("height", Given::Number(height)),
        ];
        given.extend(self.label.map(|label| ("label", Given::Text(label))));
        Args::new(&BOX, given)
    }
}

/// A point to mark with a crosshair: its place and a label if it has one.
#[derive(Debug, Clone)]
pub struct PointMark {
    numbers: [Expr; 2],
    label: Option<String>,
}

impl PointMark {
    /// The point (`x`, `y`) in design pixels.
    #[must_use]
    pub fn new(x: impl Into<Expr>, y: impl Into<Expr>) -> Self {
        Self {
            numbers: [x.into(), y.into()],
            label: None,
        }
    }

    /// This point with `label`, one line of printable characters, on a leader from the point. An empty label is none.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    fn args(self) -> Args {
        let [x, y] = self.numbers;
        let mut given = vec![("x", Given::Number(x)), ("y", Given::Number(y))];
        given.extend(self.label.map(|label| ("label", Given::Text(label))));
        Args::new(&POINT, given)
    }
}

/// The options of [`marks`]: the font of the labels, the marks, and what is not the default of the rest.
#[derive(Debug, Clone)]
pub struct Marks {
    font: Font,
    boxes: Vec<BoxMark>,
    points: Vec<PointMark>,
    bracket: Option<Expr>,
    cross: Option<Expr>,
    label_size: Option<Expr>,
}

impl Marks {
    /// No marks yet; labels are set in `font`.
    #[must_use]
    pub fn new(font: &Font) -> Self {
        Self {
            font: font.clone(),
            boxes: Vec::new(),
            points: Vec::new(),
            bracket: None,
            cross: None,
            label_size: None,
        }
    }

    /// These marks with `boxes`, each bracketed in its four corners.
    #[must_use]
    pub fn boxes(mut self, boxes: impl IntoIterator<Item = BoxMark>) -> Self {
        self.boxes = boxes.into_iter().collect();
        self
    }

    /// These marks with `points`, each a crosshair.
    #[must_use]
    pub fn points(mut self, points: impl IntoIterator<Item = PointMark>) -> Self {
        self.points = points.into_iter().collect();
        self
    }

    /// These marks with a bracket arm `bracket` times the smaller side of its box long: above 0, at most 0.5 (the
    /// arms of two corners then meet). Default 0.25.
    #[must_use]
    pub fn bracket(mut self, bracket: impl Into<Expr>) -> Self {
        self.bracket = Some(bracket.into());
        self
    }

    /// These marks with each of a crosshair's lines reaching `cross` pixels either side of its point: above 0.
    /// Default 12.
    #[must_use]
    pub fn cross(mut self, cross: impl Into<Expr>) -> Self {
        self.cross = Some(cross.into());
        self
    }

    /// These marks with labels `label_size` pixels to the em: above 0. Default 12.
    #[must_use]
    pub fn label_size(mut self, label_size: impl Into<Expr>) -> Self {
        self.label_size = Some(label_size.into());
        self
    }
}

/// A drawing of overlay marks in the look of a heads-up display, in design pixels, as the options say.
///
/// Each box has a bracket in each corner: an open line of three points (arm, corner, arm) whose arms are `bracket`
/// times the box's smaller side long. Each point has a crosshair: two open lines through it, each `cross` either way.
/// A mark with a label has a leader, an open line from the box's top right corner or from the point, up and to the
/// right by `cross` plus `label_size` in each direction, and the label, `label_size` pixels to the em in the font of
/// the options, one line: the glyphs `text` makes of it, standing to the right of the leader's end with the middle of
/// a capital on its height. So a label never lies on its own box (it is to the right of the corner) or crosshair (it
/// is beyond its reach). No layout is solved: two labels can overlap.
///
/// Every shape has `role` (`bracket`, `cross`, `leader` or `label`) and `mark` (which mark it is of: the boxes in the
/// order given, then the points), and no paint, so `only(Select::new().attribute("role", "label"))` picks what to
/// style; a label's glyphs also have what `text` writes. Brackets, crosshairs and leaders are open lines, ready for
/// a plotter.
///
/// A bracket not above 0 and at most 0.5, a cross or label size not above 0, a box with a width or height not above
/// 0, a number that is not finite or beyond 1e100, a label with a control character, more than 10,000 marks or more
/// than 100,000 characters of labels leave the drawing with an [`error`](Drawing::error); for numbers that read a
/// `Param` the draw is refused.
#[must_use]
pub fn marks(options: Marks) -> Drawing {
    let mut given = vec![("font", Given::Font(options.font))];
    if !options.boxes.is_empty() {
        given.push((
            "boxes",
            Given::Records(options.boxes.into_iter().map(BoxMark::args).collect()),
        ));
    }
    if !options.points.is_empty() {
        given.push((
            "points",
            Given::Records(options.points.into_iter().map(PointMark::args).collect()),
        ));
    }
    for (name, number) in [
        ("bracket", options.bracket),
        ("cross", options.cross),
        ("label_size", options.label_size),
    ] {
        if let Some(number) = number {
            given.push((name, Given::Number(number)));
        }
    }
    Drawing::source(&MarksNode, given)
}
