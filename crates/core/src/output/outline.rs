//! The straight-line geometry that the pixel and mesh outputs are both drawn from: the outline of a stroke, the dots
//! of its round and square ends, and the one limit on how far from the origin a coordinate may lie. With `raster` and
//! `mesh` the only users of `kurbo`, nothing of it reaches the API.

use crate::limits::Work;
use crate::path::Polyline;
use crate::{Cap, Error, Join, Path, Point, StrokeStyle};
use kurbo::{BezPath, StrokeOpts};
use std::cell::Cell;

/// The largest coordinate, and the widest stroke, that is handed to the rasteriser or the tessellator, in the pixels
/// the output is measured in.
// constant: both compute in f32, which holds whole numbers exactly up to 16,777,216; 10 million pixels is far beyond
// any canvas (at most 65,000 across) and keeps the arithmetic of a join or a curve inside that range
pub(super) const MAX_DEVICE: f64 = 1.0e7;

/// `value` (a coordinate or a width, `what`), or the error unless it is within [`MAX_DEVICE`].
pub(super) fn within_device(what: &str, value: f64) -> Result<(), Error> {
    if value <= MAX_DEVICE {
        Ok(())
    } else {
        Err(Error::parameter(
            "device",
            format!("{what} of {value} device pixels is beyond {MAX_DEVICE}"),
        ))
    }
}

/// The outline of the stroke `width` wide that `stroke` draws along `centre`, as straight lines to within `flatness`
/// (the outline has arcs for the round joins and ends; the rasteriser and the tessellator are given lines only).
///
/// The edges of the outline have a count of their own, `MAX_SEGMENTS` for one shape: a drawing of thousands of
/// stroked shapes makes millions of them. Each is charged to `name` before it is kept, and the walk ends at the first
/// refusal, so that a join of a very wide stroke can fill neither the memory nor the time.
///
/// # Errors
/// [`Error::Parameter`] naming `name` once the outline has more edges than `MAX_SEGMENTS`.
pub(super) fn stroke_outline(
    centre: &BezPath,
    stroke: &StrokeStyle,
    width: f64,
    flatness: f64,
    name: &str,
) -> Result<BezPath, Error> {
    let expanded = kurbo::Stroke::new(width)
        .with_join(join(stroke.join()))
        .with_caps(cap(stroke.cap()))
        .with_miter_limit(stroke.miter_limit());
    let curved = kurbo::stroke(
        centre.elements().iter().copied(),
        &expanded,
        &StrokeOpts::default(),
        flatness,
    );
    let mut lines = BezPath::new();
    let mut refused = None;
    let stopped = Cell::new(false);
    let made = Work::new();
    let walked = curved.into_iter().take_while(|_| !stopped.get());
    kurbo::flatten(walked, flatness, |element| {
        if let Err(error) = made.made_segments(1, name) {
            refused.get_or_insert(error);
            stopped.set(true);
        } else if !stopped.get() {
            lines.push(element);
        }
    });
    match refused {
        Some(error) => Err(error),
        None => Ok(lines),
    }
}

/// The shapes that the sub-paths which never leave their start leave under a round or square end of a stroke `width`
/// wide: a disc or a square about the point, as SVG draws them. The stroker draws nothing for them, as it does for a
/// butt end; a lone move leaves nothing.
pub(super) fn dots(
    polylines: &[Polyline],
    cap: Cap,
    width: f64,
    charge: &mut dyn FnMut(usize) -> Result<(), Error>,
) -> Result<Vec<Vec<Polyline>>, Error> {
    let half = width / 2.0;
    let mut dots = Vec::new();
    for polyline in polylines {
        let [at] = polyline.points[..] else { continue };
        if !polyline.drawn || cap == Cap::Butt {
            continue;
        }
        let shape = match cap {
            Cap::Square => Path::rect(at.x - half, at.y - half, width, width, 0.0)?,
            _ => Path::circle(at, half)?,
        };
        dots.push(shape.flattened(charge)?);
    }
    Ok(dots)
}

fn join(join: Join) -> kurbo::Join {
    match join {
        Join::Round => kurbo::Join::Round,
        Join::Miter => kurbo::Join::Miter,
        Join::Bevel => kurbo::Join::Bevel,
    }
}

fn cap(cap: Cap) -> kurbo::Cap {
    match cap {
        Cap::Butt => kurbo::Cap::Butt,
        Cap::Round => kurbo::Cap::Round,
        Cap::Square => kurbo::Cap::Square,
    }
}

/// `polylines` as a path of straight lines.
pub(super) fn bez_path(polylines: &[Polyline]) -> BezPath {
    let at = |point: Point| kurbo::Point::new(point.x, point.y);
    let mut outline = BezPath::new();
    for polyline in polylines {
        let mut points = polyline.points.iter();
        if let Some(start) = points.next() {
            outline.move_to(at(*start));
        }
        for point in points {
            outline.line_to(at(*point));
        }
        if polyline.closed {
            outline.close_path();
        }
    }
    outline
}
