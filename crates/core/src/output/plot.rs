//! `Drawing::to_plot_svg`: the drawing as strokes only, for a pen plotter.

use super::Size;
use super::order::{Stroke, nearest_neighbour};
use super::view::View;
use crate::field::length;
use crate::geometry::{MAX_COORDINATE, Point};
use crate::limits::Work;
use crate::path::{Polyline, number};
use crate::{Blend, Drawing, Error, Flow, Hatch, Paint, Shape, Values};
use std::fmt::Write as _;

/// The most distance tests the simplification of one call may make.
// constant: a billion tests of a few instructions is a few seconds; Douglas and Peucker's worst case (a spiral) is
// quadratic, so a call that would pass it is refused instead of running for minutes
const MAX_SIMPLIFY_WORK: f64 = 1e9;

/// The share of the pen width that is the default tolerance.
// constant: a quarter of the pen width is a deviation the line of the pen already covers
const DEFAULT_TOLERANCE: f64 = 0.25;

/// How to write a plot: the width of the pen, and how closely the lines follow the drawing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plot {
    pen_width: f64,
    tolerance: Option<f64>,
}

impl Plot {
    /// A plot for a pen `pen_width` design pixels wide: a solid is hatched with lines that far apart, so that the
    /// strokes touch. Lines are simplified within a quarter of the pen width; see [`Plot::tolerance`].
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `pen_width` unless it is above 0 and within 1e100.
    pub fn new(pen_width: f64) -> Result<Self, Error> {
        if !(pen_width.is_finite() && pen_width > 0.0 && pen_width <= MAX_COORDINATE) {
            return Err(Error::parameter(
                "pen_width",
                format!("must be above 0, got {pen_width}"),
            ));
        }
        Ok(Self {
            pen_width,
            tolerance: None,
        })
    }

    /// Simplify every line to within `tolerance` design pixels of the line the drawing has (0: keep every point). Checked
    /// where the plot is written: it must be at least 0 and within 1e100.
    #[must_use]
    pub fn tolerance(mut self, tolerance: f64) -> Self {
        self.tolerance = Some(tolerance);
        self
    }
}

/// One pen: its colour and what it draws.
struct Pen {
    colour: [u8; 3],
    strokes: Vec<Stroke>,
}

impl Drawing {
    /// The drawing as an SVG document for a pen plotter, of `size` as [`Drawing::to_svg`] has it, with the Params at
    /// `values`: strokes only, never a fill, one layer (`<g inkscape:groupmode="layer">`, named for its pen) for each pen,
    /// the pens being the colours of the strokes and fills in the order they first appear, with the pen's colour and
    /// `plot`'s width on the group.
    ///
    /// Every shape's outline is drawn along its centre, the shape's own stroke width ignored (the pen draws its own);
    /// every fill is drawn by pen-width hatching at 45 degrees inside its outline (its outline when it is too small
    /// for a line); a translucent colour is drawn as its opaque one and opacity is ignored. A shape drawn later does not
    /// hide one drawn earlier: draw what hides in the drawing, not on top. Curves are flattened within the library's
    /// flatness and every line simplified (Douglas and Peucker) to within `plot`'s tolerance; a closed outline is
    /// written as a line that comes back to its start.
    ///
    /// Within a layer the lines are ordered by nearest neighbour to cut pen-up travel: the pen starts at the top left
    /// corner, draws the line with the nearest end (a closed one entered at its nearest point), and goes on from where it
    /// ends. The same drawing gives the same plot.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `width` or `height` if a side of `size` is not a whole number from 1 to 100,000;
    /// naming `tolerance` if it is not at least 0 or the simplification would take over a billion distance tests;
    /// naming the shape (`shape 3`) if it has a gradient fill or stroke, a halftone, a shadow, a clip, a mask, a
    /// knockout or a blend, which a pen cannot draw; naming `pen_width` past `MAX_SEGMENTS` points; otherwise what
    /// [`Drawing::shapes`] raises.
    pub fn to_plot_svg(&self, size: Size, values: &Values, plot: &Plot) -> Result<String, Error> {
        let canvas = size.checked()?;
        let tolerance = plot.tolerance.unwrap_or(DEFAULT_TOLERANCE * plot.pen_width);
        if !(tolerance.is_finite() && (0.0..=MAX_COORDINATE).contains(&tolerance)) {
            return Err(Error::parameter(
                "tolerance",
                format!("must be at least 0, got {tolerance}"),
            ));
        }
        let shapes = self.shapes(values)?;
        let view = View::fit(self.size(), canvas);
        let work = Work::new();
        let mut budget = MAX_SIMPLIFY_WORK;
        let mut pens: Vec<Pen> = Vec::new();
        for shape in &shapes {
            let place = format!("shape {}", shape.index());
            refuse_what_a_pen_cannot_draw(shape, &place)?;
            let style = shape.style();
            let mut charge = |count| work.made_segments(count, "pen_width");
            let mut add = |colour: [u8; 3], lines: Vec<Polyline>| -> Result<(), Error> {
                let at = pens.iter().position(|pen| pen.colour == colour).unwrap_or_else(|| {
                    pens.push(Pen {
                        colour,
                        strokes: Vec::new(),
                    });
                    pens.len() - 1
                });
                for line in lines.into_iter().filter(|line| line.drawn && !line.points.is_empty()) {
                    let points = simplified(&line, tolerance, &mut budget)?;
                    let mapped = points.into_iter().map(|p| view.point(p)).collect();
                    pens[at].strokes.push(Stroke {
                        points: mapped,
                        closed: line.closed && line.points.len() > 2,
                    });
                }
                Ok(())
            };
            if let Paint::Color(colour) = style.fill()
                && let Some(rgba) = colour.rgba()
            {
                let hatch = Hatch::new(plot.pen_width).flow(Flow::fixed(45.0));
                let lines = shape
                    .path()
                    .hatch_charged(&hatch, &[], &mut charge)?
                    .flattened(&mut charge)?;
                let lines = if lines.iter().any(|line| line.drawn) {
                    lines
                } else {
                    shape.path().flattened(&mut charge)?
                };
                add([rgba[0], rgba[1], rgba[2]], lines)?;
            }
            if let Some(stroke) = style.stroke()
                && let Paint::Color(colour) = stroke.paint()
                && let Some(rgba) = colour.rgba()
            {
                add([rgba[0], rgba[1], rgba[2]], shape.path().flattened(&mut charge)?)?;
            }
        }
        let (width, height) = (number(canvas.0), number(canvas.1));
        let pen_width = number(view.length(plot.pen_width)?);
        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:inkscape=\"http://www.inkscape.org/namespaces/inkscape\" \
             width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n"
        );
        for (number_of, pen) in pens.into_iter().enumerate() {
            let hex = format!("#{:02x}{:02x}{:02x}", pen.colour[0], pen.colour[1], pen.colour[2]);
            let _ = writeln!(
                svg,
                "<g id=\"pen-{n}\" inkscape:groupmode=\"layer\" inkscape:label=\"pen {n} {hex}\" fill=\"none\" stroke=\"{hex}\" \
                 stroke-width=\"{pen_width}\" stroke-linecap=\"round\" stroke-linejoin=\"round\">",
                n = number_of + 1
            );
            for line in nearest_neighbour(&pen.strokes, Point::new(0.0, 0.0)) {
                svg.push_str("<path d=\"");
                for (k, p) in line.iter().enumerate() {
                    let _ = write!(svg, "{}{} {}", if k == 0 { "M" } else { "L" }, number(p.x), number(p.y));
                }
                svg.push_str("\"/>\n");
            }
            svg.push_str("</g>\n");
        }
        svg.push_str("</svg>\n");
        Ok(svg)
    }
}

/// An error naming `place` if the shape has something a pen cannot draw.
fn refuse_what_a_pen_cannot_draw(shape: &Shape, place: &str) -> Result<(), Error> {
    let style = shape.style();
    let reason = if matches!(style.fill(), Paint::Gradient(_))
        || matches!(style.stroke().map(crate::StrokeStyle::paint), Some(Paint::Gradient(_)))
    {
        "a gradient"
    } else if style.halftone().is_some() {
        "a halftone"
    } else if style.shadow().is_some() {
        "a shadow"
    } else if style.clip().is_some() {
        "a clip"
    } else if style.mask().is_some() {
        "a mask"
    } else if style.knockout() {
        "a knockout"
    } else if style.blend() != Blend::Normal {
        "a blend"
    } else {
        return Ok(());
    };
    Err(Error::parameter(
        place,
        format!("has {reason}, which a pen cannot draw"),
    ))
}

/// The points of `line` simplified to within `tolerance` (Douglas and Peucker, without recursion): the first and the
/// last kept, and for a closed line the start repeated as the last. `budget` is the distance tests left.
fn simplified(line: &Polyline, tolerance: f64, budget: &mut f64) -> Result<Vec<Point>, Error> {
    let mut points = line.points.clone();
    if line.closed && points.len() > 2 {
        points.push(points[0]);
    }
    if tolerance == 0.0 || points.len() < 3 {
        return Ok(if line.closed && points.len() > 2 {
            points.pop();
            points
        } else {
            points
        });
    }
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    let mut pending = vec![(0_usize, points.len() - 1)];
    while let Some((low, high)) = pending.pop() {
        if high <= low + 1 {
            continue;
        }
        #[expect(clippy::cast_precision_loss, reason = "a count of points is far below 2^53")]
        {
            *budget -= (high - low) as f64;
        }
        if *budget < 0.0 {
            return Err(Error::parameter(
                "tolerance",
                format!("would take over {MAX_SIMPLIFY_WORK} distance tests: use a larger one or fewer points"),
            ));
        }
        let (far, distance) = (low + 1..high)
            .map(|k| (k, distance(points[k], points[low], points[high])))
            .fold((low, -1.0), |best, next| if next.1 > best.1 { next } else { best });
        if distance > tolerance {
            keep[far] = true;
            pending.push((low, far));
            pending.push((far, high));
        }
    }
    let kept: Vec<Point> = points.iter().zip(&keep).filter(|(_, k)| **k).map(|(p, _)| *p).collect();
    Ok(if line.closed && kept.len() > 2 {
        kept[..kept.len() - 1].to_vec()
    } else {
        kept
    })
}

/// The distance from `p` to the segment `a` to `b`.
fn distance(p: Point, a: Point, b: Point) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let squared = dx * dx + dy * dy;
    let share = if squared == 0.0 {
        0.0
    } else {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / squared).clamp(0.0, 1.0)
    };
    length(p.x - (a.x + dx * share), p.y - (a.y + dy * share))
}
