//! The dot screen of a halftone fill: which dots of the layout a shape gets, and their polygons. Every output draws a
//! halftone from this one answer, in design pixels, so that the SVG, the pixels, the mesh and the plot show the same
//! dots.
//!
//! A screen is a layout of cells (`lines/layout.rs` places them: the straight rows of a grid or a wave, the rings, the
//! spiral or the rays `bands` draws lines along) and a dot in each. A cell has a centre, the direction of its row, and a
//! length along the row and a width across it, whose product is the cell's area; a dot is the polygon of its mark
//! (`shape/dot.rs`) of the area `(1 - tone)` of the unit cell, stretched to the cell, turned to the row and put on the
//! cell's centre, so it covers `(1 - tone)` of the area of its own cell whatever the mark and the screen. The tone is
//! read at the cell's centre. A grid and a wave are anchored to the origin of the design frame, not to the shape, so
//! the dots stay put when the shape moves; the others are about their centre.
//!
//! A round mark (dot, ellipse, a caller's outline) of a tone below `1 - pi / 4` (about 0.21) overlaps its neighbours,
//! where the union covers less than `(1 - tone)` of the cell; the square, the diamond and the euclidean mark stay in
//! their cell. A tone of the number 0 is solid, which no circles can be, and a tone of 1 has no dots.

#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "the counts of cells are worked out in floats and checked against the work limit before they are used"
)]

use crate::geometry::Point;
use crate::lines::{Distances, Family, Layout, corners, middle, polar};
use crate::math::sin_cos_deg;
use crate::path::Polyline;
use crate::shape::{Dot, HalftoneStyle, Screen as Arrangement};
use crate::{Error, Rect};
use std::f64::consts::{PI, TAU};

/// The widest a cell may be, in pitches, that the reach of a dot and the corners of a curve are worked out for: a
/// wedge of the rays near their centre, which are at most a pitch wide.
// constant: the widest ray cell is a pitch (tests/it/halftone_screens.rs measures it), and 2 leaves room
const WIDEST_CELL: f64 = 2.0;

/// The narrowest a ray cell may be at its middle, in pitches: the rays merge in pairs (a power of two at a time) until
/// none is narrower.
// constant: below half a pitch the cells of the rays are about as wide as a dot is thick at tone 0.5, which draws a
// blob; bullterrier-image keeps the same bound (`NARROWEST_RAY`)
const NARROWEST_RAY: f64 = 0.5;

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

/// The screen `halftone` makes over the shape whose outline is `outline`, in design pixels.
///
/// The work is charged to `charge` before any dot is made, as [`charged`] does.
///
/// # Errors
/// What `charge` returns, and [`Error::Parameter`] naming `mark` for an outline that cannot be a dot.
pub(super) fn screen(
    halftone: &HalftoneStyle,
    outline: &[Polyline],
    charge: &mut dyn FnMut(usize) -> Result<(), Error>,
) -> Result<Screen, Error> {
    Ok(match plan(halftone, outline, charge)? {
        Plan::Solid => Screen::Solid,
        Plan::Bare => Screen::Dots(Dots {
            inside: Vec::new(),
            crossing: Vec::new(),
        }),
        Plan::Cells(cells) => Screen::Dots(cells.dots(halftone)),
    })
}

/// Charges `charge` what [`screen`] would, for each cell near the shape the edges of the outline it is tested against
/// and the corners of its dot, and makes no dot: a pitch too small for its shape is refused rather than allocated, and
/// a shape that is not drawn is refused the same.
///
/// # Errors
/// As [`screen`].
pub(super) fn charged(
    halftone: &HalftoneStyle,
    outline: &[Polyline],
    charge: &mut dyn FnMut(usize) -> Result<(), Error>,
) -> Result<(), Error> {
    plan(halftone, outline, charge).map(|_| ())
}

/// What a halftone makes of a shape, once its work is charged.
enum Plan {
    /// The tone is the number 0.
    Solid,
    /// No dot touches the shape.
    Bare,
    /// The cells to make dots in.
    Cells(Cells),
}

/// The rows of cells a screen has near a shape.
enum Rows {
    /// The first lattice point, and how many there are across and down, in the frame of the rows.
    Straight {
        first: (f64, f64),
        across: usize,
        down: usize,
    },
    /// Rings `first` to `last`.
    Rings { first: f64, last: f64 },
    /// The arms of the spiral `first` to `last`.
    Spiral { first: f64, last: f64 },
    /// The cells `first` to `last` out along `rays` rays at the widest.
    Rays { first: f64, last: f64, rays: f64 },
}

/// The polygon of a dot made for an area and a size (their bits): a number's dots are all one.
type Made = (u64, u64, Vec<(f64, f64)>);

/// One cell of a screen: where it is, which way its row runs, and its length along the row and its width across.
struct Cell {
    centre: Point,
    /// The sine and cosine of the direction of the row.
    turn: (f64, f64),
    along: f64,
    across: f64,
}

/// The cells whose dots can reach a shape, and what a dot is.
struct Cells {
    edges: Vec<(Point, Point)>,
    bounds: (f64, f64, f64, f64),
    layout: Layout,
    centre: Point,
    rows: Rows,
    dot: Dot,
    /// How far a dot reaches from its cell's centre, in cells of the size of the cell.
    reach: f64,
}

/// The plan for `halftone` over `outline`, with its work charged to `charge`.
///
/// # Errors
/// As [`screen`].
fn plan(
    halftone: &HalftoneStyle,
    outline: &[Polyline],
    charge: &mut dyn FnMut(usize) -> Result<(), Error>,
) -> Result<Plan, Error> {
    let level = halftone.tone().level();
    // the number 0 and 1 are solid and bare by definition
    if level.is_some_and(|tone| tone <= 0.0) {
        return Ok(Plan::Solid);
    }
    let bare = level.is_some_and(|tone| tone >= 1.0);
    let edges = edges(outline);
    let Some(bounds) = bounds(outline).filter(|_| !bare && !edges.is_empty()) else {
        return Ok(Plan::Bare);
    };
    let pitch = halftone.pitch();
    let dot = Dot::new(halftone.mark(), halftone.aspect())?;
    let size = pitch * WIDEST_CELL;
    let corners_of_dot = dot.corners(size);
    let reach = dot.reach();
    let reach_px = reach * size;
    let screen = halftone.screen();
    let layout = Layout {
        family: match screen {
            Arrangement::Grid => Family::Lines,
            Arrangement::Wave => Family::Waves,
            Arrangement::Rings => Family::Rings,
            Arrangement::Spiral => Family::Spiral,
            Arrangement::Rays => Family::Rays,
        },
        spacing: pitch,
        // rings have no direction
        angle: if screen == Arrangement::Rings {
            0.0
        } else {
            halftone.angle()
        },
        wave: Some((halftone.wavelength() * pitch, halftone.amplitude() * pitch)),
    };
    let rect = Rect {
        x: bounds.0,
        y: bounds.1,
        width: bounds.2 - bounds.0,
        height: bounds.3 - bounds.1,
    };
    let centre = halftone.centre().unwrap_or_else(|| middle(rect));
    let (rows, cells) = rows(&layout, rect, centre, reach_px);
    // a cell is a lattice point or a wedge: the work is its edges tested and its corners made
    let work = if cells.is_finite() {
        (cells * (edges.len() + corners_of_dot) as f64) as usize
    } else {
        usize::MAX
    };
    charge(work)?;
    if let Some(field) = halftone.tone().field() {
        field.charge_reads("pitch", "halftone", cells)?;
    }
    Ok(Plan::Cells(Cells {
        edges,
        bounds,
        layout,
        centre,
        rows,
        dot,
        reach,
    }))
}

/// The rows of cells of `layout` that can reach the box `rect` with a dot that reaches `reach_px`, and an upper bound of
/// how many cells there are (not a number if the work is not).
fn rows(layout: &Layout, rect: Rect, centre: Point, reach_px: f64) -> (Rows, f64) {
    let pitch = layout.spacing;
    let box_corners = corners(rect);
    if !layout.family.is_round() {
        // the lattice points whose dots can reach the box: its corners in lattice units, a reach more on every side (and a
        // wave's amplitude more across, where its rows swing)
        let found = box_corners.map(|corner| {
            let (along, across) = layout.frame(corner);
            (along / pitch, across / pitch)
        });
        let swing = layout
            .wave
            .filter(|_| layout.family == Family::Waves)
            .map_or(0.0, |(_, height)| height.abs());
        let span = |select: fn(&(f64, f64)) -> f64, reach: f64| {
            let least = found.iter().map(select).fold(f64::INFINITY, f64::min);
            let most = found.iter().map(select).fold(f64::NEG_INFINITY, f64::max);
            let first = (least - reach).floor();
            (first, (most + reach).ceil() - first + 1.0)
        };
        let reach = reach_px / pitch;
        let ((first_i, across), (first_j, down)) = (
            span(|corner| corner.0, reach),
            span(|corner| corner.1, reach + swing / pitch),
        );
        // a lattice coordinate beyond a float has no span to count: infinitely many cells
        let cells =
            if found.iter().all(|(i, j)| i.is_finite() && j.is_finite()) && across.is_finite() && down.is_finite() {
                across * down
            } else {
                f64::INFINITY
            };
        return (
            Rows::Straight {
                first: (first_i, first_j),
                across: across as usize,
                down: down as usize,
            },
            cells,
        );
    }
    let distances = Distances::of(centre, rect, &box_corners);
    let widened = Distances {
        near: (distances.near - reach_px).max(0.0),
        far: distances.far + reach_px,
    };
    let (first, last) = layout.rings_within(widened);
    let rows = (last - first + 1.0).max(0.0);
    match layout.family {
        Family::Rings => (
            Rows::Rings { first, last },
            // a ring of radius (k + 1/2) has at most 2 pi (k + 1/2) + 2 cells
            if rows > 0.0 {
                PI * ((last + 1.0) * (last + 1.0) - first * first) + 2.0 * rows
            } else {
                0.0
            },
        ),
        Family::Spiral => {
            let first = (widened.near / pitch - 1.5).floor().max(0.0);
            let last = (widened.far / pitch - 0.5).floor();
            let arms = (last - first + 1.0).max(0.0);
            (
                Rows::Spiral { first, last },
                if arms > 0.0 {
                    PI * ((last + 1.0) * (last + 2.0) - first * (first + 1.0)) + 2.0 * arms
                } else {
                    0.0
                },
            )
        }
        _ => {
            let rays = Layout::ray_count(distances.far / pitch);
            (Rows::Rays { first, last, rays }, rows * rays)
        }
    }
}

/// The cells of the ring or arm whose cells are about a cell long when it is `middle` pitches out: an even number, so
/// that a half turn maps a cell to a cell.
fn ring_cells(middle: f64) -> usize {
    2 * (PI * middle).round().max(1.0) as usize
}

/// The wedges at `middle` pitches out among `rays` rays at the widest: the rays merge in pairs, a power of two at a
/// time, until none is narrower than [`NARROWEST_RAY`].
fn wedges(rays: f64, middle: f64) -> usize {
    let mut step = 1.0;
    while step < rays && TAU * middle * step / rays < NARROWEST_RAY {
        step *= 2.0;
    }
    (rays / step).floor().max(1.0) as usize
}

impl Cells {
    /// Calls `each` with every cell of the rows.
    fn each_cell(&self, mut each: impl FnMut(Cell)) {
        let layout = &self.layout;
        let pitch = layout.spacing;
        match self.rows {
            Rows::Straight {
                first: (first_i, first_j),
                across,
                down,
            } => {
                let (sin, cos) = sin_cos_deg(layout.angle);
                for row in 0..down {
                    for column in 0..across {
                        let (i, j) = ((first_i + column as f64) * pitch, (first_j + row as f64) * pitch);
                        each(Cell {
                            centre: layout.straight(i, j),
                            turn: (sin, cos),
                            along: pitch,
                            across: pitch,
                        });
                    }
                }
            }
            Rows::Rings { first, last } => {
                let mut k = first;
                while k <= last {
                    let (radius, count) = (layout.ring_radius(k), ring_cells(k + 0.5));
                    for at in 0..count {
                        let angle = TAU * (at as f64 + 0.5) / count as f64;
                        each(Self::wedge(
                            polar(self.centre, radius, angle),
                            angle,
                            radius * TAU / count as f64,
                            pitch,
                            true,
                        ));
                    }
                    k += 1.0;
                }
            }
            Rows::Spiral { first, last } => {
                let mut k = first;
                while k <= last {
                    let count = ring_cells(k + 1.0);
                    for at in 0..count {
                        let turns = (at as f64 + 0.5) / count as f64;
                        let radius = layout.spiral_radius(TAU * (k + 0.5 + turns));
                        let angle = TAU * turns + layout.turn();
                        each(Self::wedge(
                            polar(self.centre, radius, angle),
                            angle,
                            radius * TAU / count as f64,
                            pitch,
                            true,
                        ));
                    }
                    k += 1.0;
                }
            }
            Rows::Rays { first, last, rays } => {
                let mut m = first;
                while m <= last {
                    let radius = layout.ring_radius(m);
                    let count = wedges(rays, m + 0.5);
                    for at in 0..count {
                        let angle = layout.turn() + TAU * (at as f64 + 0.5) / count as f64;
                        each(Self::wedge(
                            polar(self.centre, radius, angle),
                            angle,
                            pitch,
                            radius * TAU / count as f64,
                            false,
                        ));
                    }
                    m += 1.0;
                }
            }
        }
    }

    /// The cell at `centre` of a round layout whose row runs round (`tangent`, a quarter turn from `angle`) or out along
    /// `angle`, `along` long and `across` wide.
    fn wedge(centre: Point, angle: f64, along: f64, across: f64, tangent: bool) -> Cell {
        let heading = if tangent { angle + PI / 2.0 } else { angle };
        let (sin, cos) = crate::math::sin_cos(heading);
        Cell {
            centre,
            turn: (sin, cos),
            along,
            across,
        }
    }

    /// The dots of the cells that touch the shape.
    fn dots(&self, halftone: &HalftoneStyle) -> Dots {
        let tone = halftone.tone();
        let (left, top, right, bottom) = self.bounds;
        let mut dots = Dots {
            inside: Vec::new(),
            crossing: Vec::new(),
        };
        // the polygon of the last area and size: a number's dots are all one
        let mut last: Option<Made> = None;
        self.each_cell(|cell| {
            let size = cell.along.max(cell.across);
            let margin = self.reach * size + 1.0;
            let at = cell.centre;
            if at.x < left - margin || at.x > right + margin || at.y < top - margin || at.y > bottom + margin {
                return;
            }
            let bare = tone.at(at);
            if bare >= 1.0 || bare.is_nan() {
                return;
            }
            let area = (1.0 - bare).min(1.0);
            let keys = (area.to_bits(), size.to_bits());
            if last.as_ref().is_none_or(|(a, s, _)| (*a, *s) != keys) {
                last = Some((keys.0, keys.1, self.dot.polygon(area, size)));
            }
            let unit = last.as_ref().map_or(&[][..], |(_, _, unit)| unit);
            let (sin, cos) = cell.turn;
            let polygon: Vec<Point> = unit
                .iter()
                .map(|(x, y)| {
                    let (x, y) = (x * cell.along, y * cell.across);
                    Point::new(at.x + x * cos - y * sin, at.y + x * sin + y * cos)
                })
                .collect();
            let radius = polygon
                .iter()
                .map(|p| crate::field::length(p.x - at.x, p.y - at.y))
                .fold(0.0, f64::max);
            let target = match place(at, radius, &self.edges) {
                Place::Outside => return,
                Place::Inside => &mut dots.inside,
                Place::Crossing => &mut dots.crossing,
            };
            target.push(polygon);
        });
        dots
    }
}

/// Where a dot lies against a shape.
enum Place {
    Inside,
    Outside,
    Crossing,
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
