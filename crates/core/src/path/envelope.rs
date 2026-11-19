//! Envelopes: [`Path::envelope`] and [`Path::arch`], the map of a rectangle onto the region between two curves. Both
//! are one map ([`lattice`]) with a different pair of edge functions, run through the warp kernel ([`Path::warp`]).

use super::Path;
use super::bend::Guide;
use super::curve::lerp_number;
use super::warp::segment_budget;
use crate::Error;
use crate::geometry::{Point, Rect, coordinate, finite};

/// The pieces of the frame's width an edge is cut into at most.
// constant: a chord of an edge curve cut at a 256th of the frame's width strays from it by at most `curvature (L /
// 256)^2 / 8` (L the curve's length): for a half circle 0.0001 of its radius times pi squared (0.9 pixel in 10,000), and
// for a diagonal edge of a wide frame (bilinear in the frame) at most `slope W^2 / (4 * 256^2 H)`, 0.04 pixel for a frame
// 1000 by 100; 128 pieces put a curve's chords 0.01 pixel off it, which the test of the top edge refuses
const PIECES: f64 = 256.0;

/// The most a chord of an edge may stray from a curve, as a share of the frame's width, at the cut of `PIECES`.
// constant: the 0.9 pixel in 10,000 the comment above works out for a half circle, rounded up to a ten-thousandth; a
// curve of more curvature than a half circle across the frame is cut by the same rule and shows its facets
const CURVE_ERROR: f64 = 1e-4;

/// A frame the deformers map: its corner and its size, both above 0 and within ±1e100.
#[derive(Clone, Copy)]
struct Frame {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl Frame {
    /// The frame `rect` is, or the error naming `frame`.
    fn new(rect: Rect) -> Result<Self, Error> {
        let Rect { x, y, width, height } = rect;
        let beyond = |value: f64| coordinate("frame", value);
        let (right, bottom) = (beyond(x)? + beyond(width)?, beyond(y)? + beyond(height)?);
        beyond(right)?;
        beyond(bottom)?;
        if width > 0.0 && height > 0.0 {
            Ok(Self { x, y, width, height })
        } else {
            Err(Error::parameter(
                "frame",
                format!(
                    "must have a width and a height above 0, got {width} by {height}; give `frame` for a shape with none"
                ),
            ))
        }
    }

    /// The point `u` of the way along the edge `guide` (the frame's own straight edge if none) blended by `fraction` with the
    /// frame's top edge (`down` 0) or bottom one (`down` 1).
    fn edge(&self, guide: Option<&Guide>, down: f64, fraction: f64, u: f64) -> Point {
        let straight = self.straight(u, down);
        guide.map_or(straight, |guide| {
            let curve = guide.frame(u * guide.length).0;
            Point::new(
                lerp_number(straight.x, curve.x, fraction),
                lerp_number(straight.y, curve.y, fraction),
            )
        })
    }

    /// The point `u` of the way across the frame's top edge, or of the bottom one (`down` 1).
    fn straight(&self, u: f64, down: f64) -> Point {
        Point::new(self.x + u * self.width, self.y + down * self.height)
    }
}

/// The map `(x, y) -> lerp(top(u), bottom(u), v)`, `u` and `v` the shares of the frame's width and height `(x, y)` is
/// at. The one place the logic of an envelope is.
fn lattice<'a>(
    frame: Frame,
    top: &'a dyn Fn(f64) -> Point,
    bottom: &'a dyn Fn(f64) -> Point,
) -> impl Fn(Point) -> Point + 'a {
    move |point| {
        let (u, v) = ((point.x - frame.x) / frame.width, (point.y - frame.y) / frame.height);
        let (above, below) = (top(u), bottom(u));
        Point::new(lerp_number(above.x, below.x, v), lerp_number(above.y, below.y, v))
    }
}

impl Path {
    /// This path with the frame mapped onto the region between the curve `top` and the curve `bottom`: a point `(x, y)`
    /// goes to `lerp(T(u), B(u), v)`, `u = (x - x0) / width` and `v = (y - y0) / height` its shares of the `frame`
    /// `(x0, y0, width, height)`, `T(u)` and `B(u)` the points `u` of the way along `top` and `bottom` by arc length.
    /// A column of the frame (a fixed `u`) stays a straight line from `T(u)` to `B(u)`; a row follows the curves.
    /// The classic banner is a word between a curve and the same curve lower.
    ///
    /// `T` and `B` are walked as `along` walks a guide: sub-paths in order as one length, `u` outside 0 to 1 going on
    /// straight past an open curve's ends, a closed curve wrapping, its seam at its start (so `u` = 0 and `u` = 1 are the
    /// same point, a circle's top). `bottom` left out is the frame's own bottom edge, the straight line `y = y0 + height`.
    /// `fraction` blends each edge between the frame's own straight edge (0) and the curve (1): `T_eff(u) = lerp(straight(u),
    /// T(u), fraction)`, so that `fraction` animates a bend although the curves are fixed; any finite number is accepted, one
    /// above 1 exaggerating the curve and one below 0 bending it the other way. `frame` left out is the bounds of this
    /// path.
    ///
    /// The path is flattened and each edge cut into equal pieces of at most a 256th of the frame's width, every vertex of
    /// the flattening kept (corners stay corners); every point of the result is within 0.05 pixel and 0.2% of how far the
    /// map moved it of the image it stands for, for the shapes the tests hold it to (bound worked at `PIECES`), and a frame wide enough for that to be over 0.05 pixel is cut finer so that it is not. The
    /// result is made of straight lines, each sub-path keeping its closedness; a lone point is moved, an empty path stays
    /// empty.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `frame` if a number of it is NaN, infinite or beyond ±1e100, its width or height is not
    /// above 0 (a path with no width or height has no frame of its own to default to), or the pieces the cut would make,
    /// over all the sub-paths, are more than `MAX_SEGMENTS` (10,000,000), counted before any is made; naming `fraction`
    /// if it is NaN or infinite, or a point would go beyond ±1e100; [`Error::Geometry`] if `top` or `bottom` is empty
    /// or has no length, and so no direction.
    pub fn envelope(
        &self,
        top: &Self,
        bottom: Option<&Self>,
        frame: Option<Rect>,
        fraction: f64,
    ) -> Result<Self, Error> {
        let top = Guide::new("envelope", top, None, &mut |_| Ok(()))?;
        let bottom = bottom
            .map(|bottom| Guide::new("envelope", bottom, None, &mut |_| Ok(())))
            .transpose()?;
        self.enveloped(&top, bottom.as_ref(), frame, fraction, &mut segment_budget("frame"))
    }

    /// As [`Path::envelope`] on curves already walked, calling `charge` with the number of points the result would have,
    /// before any is made; its error stops the verb.
    pub(crate) fn enveloped(
        &self,
        top: &Guide,
        bottom: Option<&Guide>,
        frame: Option<Rect>,
        fraction: f64,
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<Self, Error> {
        let fraction = finite("fraction", fraction)?;
        let Some(frame) = self.frame_of(frame)? else {
            return Ok(self.clone());
        };
        let above = |u: f64| frame.edge(Some(top), 0.0, fraction, u);
        let below = |u: f64| frame.edge(bottom, 1.0, fraction, u);
        self.warp(
            "fraction",
            frame.width / PIECES,
            frame.width * CURVE_ERROR,
            &lattice(frame, &above, &below),
            charge,
        )
    }

    /// This path with an arch put in it: the whole band of the frame lifted by a parabola through the frame's two
    /// corners whose middle is `bend * width` above them (`bend` negative: below them, a sag; 0: no change), the top and
    /// the bottom edge by the same lift, so the frame's height is kept at every column and `-bend` is the mirror of `bend`.
    /// A point `(x, y)` goes to `(x, y - 4 bend width u (1 - u))`, `u` its share of the `frame` (the bounds of this path if
    /// left out); it is the map of [`Path::envelope`] between a parabola and the same parabola lower by the frame's height.
    /// `y` is down, so a positive `bend` lifts the middle up the screen. A flat baseline under a curved top is
    /// [`Path::envelope`] with a `top` alone.
    ///
    /// The parabola is given in closed form, so no arc length is walked: `u` follows `x` (the horizontal spacing of the
    /// letters of a word is kept, which a curve walked by arc length does not give). Cut and bounded as for
    /// [`Path::envelope`]; every point of the result is within `0.05` pixel of the map for a `bend` of 1 or less on
    /// a frame as tall as it is wide.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `frame` as for [`Path::envelope`]; naming `bend` if it is NaN or infinite, or a
    /// point would go beyond ±1e100.
    pub fn arch(&self, bend: f64, frame: Option<Rect>) -> Result<Self, Error> {
        self.arched(bend, frame, &mut segment_budget("frame"))
    }

    /// As [`Path::arch`], calling `charge` with the number of points the result would have, before any is made.
    pub(crate) fn arched(
        &self,
        bend: f64,
        frame: Option<Rect>,
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<Self, Error> {
        let bend = finite("bend", bend)?;
        let Some(frame) = self.frame_of(frame)? else {
            return Ok(self.clone());
        };
        let lifted = |u: f64, down: f64| {
            let corner = frame.straight(u, down);
            Point::new(corner.x, corner.y - 4.0 * bend * frame.width * u * (1.0 - u))
        };
        let above = |u: f64| lifted(u, 0.0);
        let below = |u: f64| lifted(u, 1.0);
        self.warp(
            "bend",
            frame.width / PIECES,
            bend.abs() * frame.width / PIECES / PIECES,
            &lattice(frame, &above, &below),
            charge,
        )
    }

    /// The frame a deformer maps by: `frame` checked, else the bounds of this path; `None` for an empty path, which has
    /// nothing to map (an explicit frame is checked all the same).
    fn frame_of(&self, frame: Option<Rect>) -> Result<Option<Frame>, Error> {
        let rect = match frame {
            Some(rect) => rect,
            None if self.subpaths.is_empty() => return Ok(None),
            None => self.bounds()?,
        };
        let frame = Frame::new(rect)?;
        Ok((!self.subpaths.is_empty()).then_some(frame))
    }
}
