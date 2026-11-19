//! The dot screen of a halftone fill: which dots of the layout a shape gets, and their polygons. Every output draws a
//! halftone from this one answer, in design pixels, so that the SVG, the pixels, the mesh and the plot show the same
//! dots.
//!
//! A screen is a layout of cells (`lines/layout.rs` places them: the straight rows of a grid or a wave, the rings, the
//! spiral or the rays `bands` draws lines along) and a dot in each. A cell has a center, the direction of its row, and a
//! length along the row and a width across it, whose product is the cell's area; a dot is the polygon of its mark
//! (`shape/dot.rs`) of the area `(1 - tone)` of the unit cell, stretched to the cell, turned to the row and put on the
//! cell's center, so it covers `(1 - tone)` of the area of its own cell whatever the mark and the screen. The tone is
//! read at the cell's center. A grid and a wave are anchored to the origin of the design frame, not to the shape, so
//! the dots stay put when the shape moves; the others are about their center.
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

use crate::field::Outline;
use crate::geometry::Point;
use crate::lines::{Family, Layout, RoundRows, corners, middle, polar, ray_wedges, ring_cells};
use crate::math::sin_cos_deg;
use crate::path::Polyline;
use crate::shape::{Dot, HalftoneStyle, Screen as Arrangement};
use crate::{Error, Rect};
use std::f64::consts::{PI, TAU};

/// The widest a cell may be, in pitches, that the reach of a dot and the corners of a curve are worked out for: a
/// wedge of the rays near their center, which are at most a pitch wide.
// constant: the widest ray cell is a pitch (tests/it/halftone_screens.rs measures it), and 2 leaves room
const WIDEST_CELL: f64 = 2.0;

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
/// The dots are charged to `charge` before any is made, and the edges each is placed against before they are weighed.
///
/// # Errors
/// What `charge` returns, and [`Error::Parameter`] naming `shape` for an outline that cannot be a dot.
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
        Plan::Cells(cells) => Screen::Dots(cells.dots(halftone, charge)?),
    })
}

/// Charges `charge` what [`screen`] would and keeps no dot: a shape that is not drawn is refused as a drawn one is.
///
/// # Errors
/// As [`screen`].
pub(super) fn charged(
    halftone: &HalftoneStyle,
    outline: &[Polyline],
    charge: &mut dyn FnMut(usize) -> Result<(), Error>,
) -> Result<(), Error> {
    screen(halftone, outline, charge).map(drop)
}

/// What a halftone makes of a shape, once its work is charged.
enum Plan {
    /// The tone is the number 0.
    Solid,
    /// No dot touches the shape.
    Bare,
    /// The cells to make dots in.
    Cells(Box<Cells>),
}

/// The rows of cells a screen has near a shape.
enum Rows {
    /// The first lattice point, and how many there are across and down, in the frame of the rows.
    Straight {
        first: (f64, f64),
        across: usize,
        down: usize,
    },
    /// The rings.
    Rings(RoundRows),
    /// The arms of the spiral.
    Spiral(RoundRows),
    /// The cells out along `rays` rays at the widest.
    Rays { rows: RoundRows, rays: f64 },
}

/// The polygon of a dot made for an area and a size (their bits): a number's dots are all one.
type Made = (u64, u64, Vec<(f64, f64)>);

/// One cell of a screen: where it is, which way its row runs, and its length along the row and its width across.
struct Cell {
    center: Point,
    /// The sine and cosine of the direction of the row.
    turn: (f64, f64),
    along: f64,
    across: f64,
}

/// The cells whose dots can reach a shape, and what a dot is.
struct Cells {
    /// The outline's edges in a grid, so that a dot is placed against the edges near it.
    outline: Outline,
    bounds: (f64, f64, f64, f64),
    layout: Layout,
    center: Point,
    rows: Rows,
    dot: Dot,
    /// How far a dot reaches from its cell's center, in cells of the size of the cell.
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
    if level.is_some_and(|tone| tone >= 1.0) {
        return Ok(Plan::Bare);
    }
    // an outline of no edge (lone points) fills nothing
    let (Some(bounds), Ok(edges)) = (bounds(outline), Outline::from_polylines(outline, "halftone")) else {
        return Ok(Plan::Bare);
    };
    let pitch = halftone.pitch();
    let dot = Dot::new(halftone.shape(), halftone.aspect())?;
    // a cell of a grid or a wave is a pitch across; a wedge of the others may be wider
    let size = pitch
        * if matches!(halftone.screen(), Arrangement::Grid | Arrangement::Wave) {
            1.0
        } else {
            WIDEST_CELL
        };
    let corners_of_dot = dot.corners(size);
    let reach = dot.reach();
    let reach_px = reach * size;
    let screen = halftone.screen();
    let family = screen.family();
    let layout = Layout {
        family,
        spacing: pitch,
        // a layout that has no direction is laid at 0
        angle: if family.reads("angle") { halftone.angle() } else { 0.0 },
        wave: Some((halftone.wavelength(), halftone.amplitude())),
    };
    let rect = Rect {
        x: bounds.0,
        y: bounds.1,
        width: bounds.2 - bounds.0,
        height: bounds.3 - bounds.1,
    };
    let center = halftone.center().unwrap_or_else(|| middle(rect));
    let (rows, cells) = rows(&layout, rect, center, reach_px)?;
    // a cell is a lattice point or a wedge: the work here is its corners made, the edges near it are charged as weighed
    let work = if cells.is_finite() {
        (cells * corners_of_dot as f64) as usize
    } else {
        usize::MAX
    };
    charge(work)?;
    if let Some(field) = halftone.tone().field() {
        field.charge_reads("pitch", "halftone", cells)?;
    }
    Ok(Plan::Cells(Box::new(Cells {
        outline: edges,
        bounds,
        layout,
        center,
        rows,
        dot,
        reach,
    })))
}

/// The rows of cells of `layout` that can reach the box `rect` with a dot that reaches `reach_px`, and an upper bound of
/// how many cells there are (not a number if the work is not).
///
/// # Errors
/// [`Error::Parameter`] naming `center` if it is too far from the box for a pitch to move there.
fn rows(layout: &Layout, rect: Rect, center: Point, reach_px: f64) -> Result<(Rows, f64), Error> {
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
            .map_or(0.0, |(_, height)| height.abs() * pitch);
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
        return Ok((
            Rows::Straight {
                first: (first_i, first_j),
                across: across as usize,
                down: down as usize,
            },
            cells,
        ));
    }
    // a dot is no smaller than a pitch's step at the coordinates in play, or its cells cannot be told apart
    // rays as bullterrier-image has them: a ring's worth of cells half way to the nearest side of the shape
    let rays = ring_cells(rect.width.min(rect.height) / 2.0 / pitch);
    let round = layout.round_rows(center, rect, reach_px, pitch, rays)?;
    Ok(match layout.family {
        Family::Rings => (Rows::Rings(round), round.work),
        Family::Spiral => (Rows::Spiral(round), round.work),
        _ => (
            Rows::Rays {
                rows: round,
                rays: round.outermost,
            },
            round.work,
        ),
    })
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
                            center: layout.straight(i, j),
                            turn: (sin, cos),
                            along: pitch,
                            across: pitch,
                        });
                    }
                }
            }
            Rows::Rings(rows) => {
                for k in rows.indices() {
                    let (radius, count) = (layout.ring_radius(k), ring_cells(k + 0.5));
                    for at in 0..count as usize {
                        let angle = TAU * (at as f64 + 0.5) / count;
                        each(Self::wedge(
                            polar(self.center, radius, angle),
                            angle,
                            radius * TAU / count,
                            pitch,
                            true,
                        ));
                    }
                }
            }
            Rows::Spiral(rows) => {
                for k in rows.indices() {
                    let count = ring_cells(k + 1.0);
                    for at in 0..count as usize {
                        let turns = (at as f64 + 0.5) / count;
                        let radius = layout.spiral_radius(TAU * (k + 0.5 + turns));
                        let angle = TAU * turns + layout.turn();
                        each(Self::wedge(
                            polar(self.center, radius, angle),
                            angle,
                            radius * TAU / count,
                            pitch,
                            true,
                        ));
                    }
                }
            }
            Rows::Rays { rows, rays } => {
                for m in rows.indices() {
                    let radius = layout.ring_radius(m);
                    // wedge `at` takes the turns from `at / wedges` to the next, the last whatever is left of the turn
                    let wedges = ray_wedges(rays, m + 0.5);
                    for at in 0..wedges.ceil() as usize {
                        let (start, end) = (at as f64 / wedges, ((at + 1) as f64 / wedges).min(1.0));
                        let angle = layout.turn() + TAU * f64::midpoint(start, end);
                        each(Self::wedge(
                            polar(self.center, radius, angle),
                            angle,
                            pitch,
                            radius * TAU * (end - start),
                            false,
                        ));
                    }
                }
            }
        }
    }

    /// The cell at `center` of a round layout whose row runs round (`tangent`, a quarter turn from `angle`) or out along
    /// `angle`, `along` long and `across` wide.
    fn wedge(center: Point, angle: f64, along: f64, across: f64, tangent: bool) -> Cell {
        let heading = if tangent { angle + PI / 2.0 } else { angle };
        let (sin, cos) = crate::math::sin_cos(heading);
        Cell {
            center,
            turn: (sin, cos),
            along,
            across,
        }
    }

    /// The dots of the cells that touch the shape, each charged to `charge` the edges it is placed against.
    ///
    /// # Errors
    /// What `charge` returns.
    fn dots(
        &self,
        halftone: &HalftoneStyle,
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<Dots, Error> {
        let tone = halftone.tone();
        let (left, top, right, bottom) = self.bounds;
        let mut dots = Dots {
            inside: Vec::new(),
            crossing: Vec::new(),
        };
        // the polygon of the last area and size: a number's dots are all one
        let mut last: Option<Made> = None;
        let mut failed = None;
        self.each_cell(|cell| {
            if failed.is_some() {
                return;
            }
            let size = cell.along.max(cell.across);
            let margin = self.reach * size + 1.0;
            let at = cell.center;
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
            let target = match self.place(at, radius, charge) {
                Ok(Place::Outside) => return,
                Ok(Place::Inside) => &mut dots.inside,
                Ok(Place::Crossing) => &mut dots.crossing,
                Err(error) => {
                    failed = Some(error);
                    return;
                }
            };
            target.push(polygon);
        });
        failed.map_or(Ok(dots), Err)
    }

    /// Where a dot of this `radius` about `center` lies against the shape: crossing if an edge comes nearer than the
    /// radius, else inside or outside as the nonzero rule fills the center.
    ///
    /// # Errors
    /// What `charge` returns.
    fn place(
        &self,
        center: Point,
        radius: f64,
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<Place, Error> {
        if self.outline.reaches(center, radius, charge)? {
            return Ok(Place::Crossing);
        }
        charge(self.outline.contains_cost(center))?;
        Ok(if self.outline.contains(center) {
            Place::Inside
        } else {
            Place::Outside
        })
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
