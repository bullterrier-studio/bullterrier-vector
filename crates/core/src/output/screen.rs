//! The dot screen of a halftone fill: which dots of the lattice a shape gets, and their polygons. Every output draws a
//! halftone from this one answer, in design pixels, so that the SVG, the pixels and the mesh show the same dots.
//!
//! The lattice is square, `pitch` apart, turned `angle` degrees clockwise (`y` down) about the origin of the design
//! frame, and a dot sits on each lattice point. It is anchored to the frame and not to the shape, so the dots stay put
//! when the shape moves. A dot is a regular polygon of the area `(1 - tone)` of a cell, as many corners as keep the
//! polygon within [`FLATNESS`] of the circle; a dot of a tone below `1 - pi / 4` (about 0.21) overlaps its neighbours,
//! where the union covers less than `(1 - tone)` of the cell. Tone 0 is solid, which no circles can be (they leave the corners
//! of the cell bare), and tone 1 has no dots.

use crate::Error;
use crate::geometry::Point;
use crate::math::{sin_cos, sin_cos_deg};
use crate::path::{FLATNESS, Polyline};
use crate::shape::HalftoneStyle;
use std::f64::consts::{PI, TAU};

/// What a halftone paints over a shape.
pub(super) enum Screen {
    /// The whole shape, in the colour of the dots: the tone is 0.
    Solid,
    /// The dots that touch the shape.
    Dots(Dots),
}

/// The dots of a screen that touch a shape, as closed polygons in design pixels, all running clockwise on the screen
/// (`y` down), so that the nonzero rule paints their union.
pub(super) struct Dots {
    /// The dots that lie wholly inside the shape.
    pub(super) inside: Vec<Vec<Point>>,
    /// The dots that cross the outline, or may: they are cut to the shape where the outline is not a clip.
    pub(super) crossing: Vec<Vec<Point>>,
}

impl Dots {
    /// Whether there are no dots.
    pub(super) fn is_empty(&self) -> bool {
        self.inside.is_empty() && self.crossing.is_empty()
    }

    /// Every dot, inside ones first.
    pub(super) fn all(&self) -> impl Iterator<Item = &Vec<Point>> {
        self.inside.iter().chain(&self.crossing)
    }
}

/// Where a dot lies against a shape.
enum Place {
    Inside,
    Outside,
    Crossing,
}

/// The screen `halftone` makes over the shape whose outline is `outline`, in design pixels.
///
/// The work is charged to `charge` before any dot is made: for each lattice point near the shape, the edges of the
/// outline it is tested against and the corners of its dot, so that a tiny pitch on a big shape is refused rather than
/// allocated.
///
/// # Errors
/// What `charge` returns.
pub(super) fn screen(
    halftone: &HalftoneStyle,
    outline: &[Polyline],
    charge: &mut dyn FnMut(usize) -> Result<(), Error>,
) -> Result<Screen, Error> {
    let none = || {
        Screen::Dots(Dots {
            inside: Vec::new(),
            crossing: Vec::new(),
        })
    };
    let (tone, pitch) = (halftone.tone(), halftone.pitch());
    // the tone is within 0 to 1: its ends are solid and bare by definition
    if tone <= 0.0 {
        return Ok(Screen::Solid);
    }
    let bare = tone >= 1.0;
    let edges = edges(outline);
    let Some(bounds) = bounds(outline).filter(|_| !bare && !edges.is_empty()) else {
        return Ok(none());
    };
    let radius = pitch * ((1.0 - tone) / PI).sqrt();
    // an arc of angle s is within radius * s^2 / 8 of its chord; a triangle is the fewest corners
    let corners = (TAU / (8.0 * FLATNESS / radius).sqrt()).ceil().max(3.0);
    // the lattice points whose dots can reach the box: its corners in lattice units, a radius more on every side
    let (sin, cos) = sin_cos_deg(halftone.angle());
    let lattice = |x: f64, y: f64| ((x * cos + y * sin) / pitch, (y * cos - x * sin) / pitch);
    let found = [
        lattice(bounds.0, bounds.1),
        lattice(bounds.2, bounds.1),
        lattice(bounds.0, bounds.3),
        lattice(bounds.2, bounds.3),
    ];
    let reach = radius / pitch;
    let span = |along: fn(&(f64, f64)) -> f64| {
        let least = found.iter().map(along).fold(f64::INFINITY, f64::min);
        let most = found.iter().map(along).fold(f64::NEG_INFINITY, f64::max);
        let first = (least - reach).floor();
        (first, (most + reach).ceil() - first + 1.0)
    };
    let ((first_i, across), (first_j, down)) = (span(|corner| corner.0), span(|corner| corner.1));
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "a count of work is a whole number, and one beyond usize saturates, which any limit refuses"
    )]
    {
        // a lattice coordinate beyond a float has no span to count: infinitely many dots
        let work = if found.iter().all(|(i, j)| i.is_finite() && j.is_finite()) {
            (across * down * (edges.len() as f64 + corners)) as usize
        } else {
            usize::MAX
        };
        charge(work)?;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "charged above, so at most the limit of edges: whole numbers far below usize"
    )]
    let (corners, across, down) = (corners as usize, across as usize, down as usize);
    let unit: Vec<(f64, f64)> = (0..corners)
        .map(|corner| {
            #[expect(clippy::cast_precision_loss, reason = "a count below the limit is exact")]
            let (sin, cos) = sin_cos(TAU * corner as f64 / corners as f64);
            (cos * radius, sin * radius)
        })
        .collect();
    let mut dots = Dots {
        inside: Vec::new(),
        crossing: Vec::new(),
    };
    for row in 0..down {
        for column in 0..across {
            #[expect(clippy::cast_precision_loss, reason = "a count below the limit is exact")]
            let (i, j) = ((first_i + column as f64) * pitch, (first_j + row as f64) * pitch);
            let centre = Point::new(i * cos - j * sin, i * sin + j * cos);
            let list = match place(centre, radius, &edges) {
                Place::Outside => continue,
                Place::Inside => &mut dots.inside,
                Place::Crossing => &mut dots.crossing,
            };
            list.push(
                unit.iter()
                    .map(|(x, y)| Point::new(centre.x + x, centre.y + y))
                    .collect(),
            );
        }
    }
    Ok(Screen::Dots(dots))
}

/// The box (left, top, right, bottom) that holds the points of `outline`, if it has any.
fn bounds(outline: &[Polyline]) -> Option<(f64, f64, f64, f64)> {
    let mut points = outline.iter().flat_map(|polyline| &polyline.points);
    let first = points.next()?;
    Some(points.fold(
        (first.x, first.y, first.x, first.y),
        |(left, top, right, bottom), at| (left.min(at.x), top.min(at.y), right.max(at.x), bottom.max(at.y)),
    ))
}

/// The edges of `outline`, an open sub-path closed as a fill closes it; a lone point has none.
fn edges(outline: &[Polyline]) -> Vec<(Point, Point)> {
    let mut edges = Vec::new();
    for polyline in outline.iter().filter(|polyline| polyline.points.len() > 1) {
        let points = &polyline.points;
        edges.extend(
            points
                .iter()
                .enumerate()
                .map(|(at, from)| (*from, points[(at + 1) % points.len()])),
        );
    }
    edges
}

/// Where a dot of this `radius` about `centre` lies against the shape with these `edges`: inside if the centre is
/// inside (by the nonzero rule) and no edge comes closer than the radius, outside if the centre is outside and none does,
/// else crossing.
fn place(centre: Point, radius: f64, edges: &[(Point, Point)]) -> Place {
    let limit = radius * radius;
    let mut winding = 0_i32;
    for &(from, to) in edges {
        // an edge whose box is a radius away from the centre is not nearer than one
        let far = from.x.max(to.x) < centre.x - radius
            || from.x.min(to.x) > centre.x + radius
            || from.y.max(to.y) < centre.y - radius
            || from.y.min(to.y) > centre.y + radius;
        if !far && nearness(centre, from, to) < limit {
            return Place::Crossing;
        }
        // the sign of the cross product says which side of the edge the centre is on
        let side = (to.x - from.x) * (centre.y - from.y) - (centre.x - from.x) * (to.y - from.y);
        if from.y <= centre.y && to.y > centre.y && side > 0.0 {
            winding += 1;
        } else if from.y > centre.y && to.y <= centre.y && side < 0.0 {
            winding -= 1;
        }
    }
    if winding == 0 { Place::Outside } else { Place::Inside }
}

/// The square of the distance from `at` to the segment from `from` to `to`.
fn nearness(at: Point, from: Point, to: Point) -> f64 {
    let (along_x, along_y) = (to.x - from.x, to.y - from.y);
    let length = along_x * along_x + along_y * along_y;
    // an edge of length 0 is its point
    let share = if length > 0.0 {
        (((at.x - from.x) * along_x + (at.y - from.y) * along_y) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (x, y) = (from.x + along_x * share - at.x, from.y + along_y * share - at.y);
    x * x + y * y
}
