//! What a dot of a halftone is: the polygon of a [`Mark`] of a given area in a unit cell. The cell is the square from
//! -0.5 to 0.5 about the dot's center, `x` along the row, `y` across it and down; a polygon runs clockwise on the screen
//! (`y` down), and its own shoelace area is the area asked for, to the last bit of the arithmetic: the corners are
//! solved for it, not left to the circle they stand for.

#![expect(
    clippy::cast_precision_loss,
    reason = "the counts of corners are whole numbers far below 2^53"
)]

use super::halftone::Mark;
use crate::math::sin_cos;
use crate::path::FLATNESS;
use crate::{Error, Path};
use std::f64::consts::{PI, TAU};

/// How many equal steps of angle the polygon of a euclidean dot has.
// constant: 128 steps keep a dot of a 40 pixel radius within 0.012 px of its curve, and 128 is a multiple of 8, so the
// diagonals of the cell are corners and a dot that fills the cell has straight sides
const EUCLIDEAN_STEPS: usize = 128;

/// How many covers the table of a euclidean dot holds: one for each 1/128 of the cell.
// constant: the area between two covers is under 0.8 % of the cell, where a straight blend of two outlines is within a
// part in 10^4 of the one between, and the area is then made exact by a scale
const EUCLIDEAN_COVERS: usize = 128;

/// A mark made ready to give the polygon of a dot of any area.
pub(crate) struct Dot {
    kind: Kind,
}

enum Kind {
    Round,
    Square,
    Ellipse {
        aspect: f64,
    },
    Diamond,
    Euclidean(Box<Table>),
    /// The outline at area 1 about its centroid, clockwise on the screen.
    Outline(Vec<(f64, f64)>),
}

impl Dot {
    /// The dot of `mark`, `aspect` as long as wide if it is an ellipse.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `shape` if the caller's outline is not one closed outline with an area.
    pub(crate) fn new(mark: &Mark, aspect: f64) -> Result<Self, Error> {
        let kind = match mark {
            Mark::Dot => Kind::Round,
            Mark::Square => Kind::Square,
            Mark::Ellipse => Kind::Ellipse { aspect },
            Mark::Diamond => Kind::Diamond,
            Mark::Euclidean => Kind::Euclidean(Box::new(Table::new())),
            Mark::Path(path) => Kind::Outline(outline(path)?),
        };
        Ok(Self { kind })
    }

    /// The most corners a dot has in a cell `size` design pixels across: what a work charge counts for each dot.
    pub(crate) fn corners(&self, size: f64) -> usize {
        match &self.kind {
            Kind::Round => corners_for(size * (1.0 / PI).sqrt()),
            Kind::Ellipse { aspect } => multiple_of_four(corners_for(size * (aspect / PI).sqrt())),
            Kind::Square => 4,
            Kind::Diamond => 8,
            Kind::Euclidean(_) => EUCLIDEAN_STEPS,
            Kind::Outline(points) => points.len(),
        }
    }

    /// How far from its center a dot can reach, in cells: a little over the circumradius of the dot that covers the
    /// whole cell. Worked out, never made, so that a mark that is absurdly long is refused by the work it would take.
    pub(crate) fn reach(&self) -> f64 {
        match &self.kind {
            // a polygon of 8 corners or more is within 6 % of the circle it stands for, and of 3 corners within 25 %
            Kind::Round => 0.71,
            Kind::Ellipse { aspect } => 1.06 * (aspect / PI).sqrt(),
            Kind::Square | Kind::Diamond | Kind::Euclidean(_) => 0.7072,
            Kind::Outline(points) => 1.0001 * points.iter().map(|(x, y)| (x * x + y * y).sqrt()).fold(0.0, f64::max),
        }
    }

    /// The corners of a dot of `area` (above 0, at most 1) of the cell, in a cell `size` design pixels across (which
    /// decides how many corners a curve is given).
    pub(crate) fn polygon(&self, area: f64, size: f64) -> Vec<(f64, f64)> {
        match &self.kind {
            Kind::Round => {
                let corners = corners_for(size * (area / PI).sqrt());
                let radius = (2.0 * area / (corners as f64 * sin_cos(TAU / corners as f64).0)).sqrt();
                ellipse(corners, radius, radius)
            }
            Kind::Ellipse { aspect } => {
                let corners = multiple_of_four(corners_for(size * (area * aspect / PI).sqrt()));
                let product = 2.0 * area / (corners as f64 * sin_cos(TAU / corners as f64).0);
                let long = (product * aspect).sqrt();
                ellipse(corners, long, long / aspect)
            }
            Kind::Square => {
                let half = area.sqrt() / 2.0;
                vec![(-half, -half), (half, -half), (half, half), (-half, half)]
            }
            Kind::Diamond => diamond(area),
            Kind::Euclidean(table) => table.polygon(area),
            Kind::Outline(points) => {
                let scale = area.sqrt();
                points.iter().map(|(x, y)| (x * scale, y * scale)).collect()
            }
        }
    }
}

/// The corners a circle of `radius` design pixels needs to be within [`FLATNESS`], at least a triangle.
fn corners_for(radius: f64) -> usize {
    let corners = (TAU / (8.0 * FLATNESS / radius).sqrt()).ceil().max(3.0);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a whole number from 3; a radius that is not a number gives 3, and a huge one is refused by the caller's work charge before it is made"
    )]
    {
        if corners.is_nan() { 3 } else { corners.min(1e9) as usize }
    }
}

fn multiple_of_four(corners: usize) -> usize {
    corners.max(8).div_ceil(4) * 4
}

/// The `corners` of the ellipse of semi-axes `long` and `short`, from the end of the long axis, clockwise on the screen.
fn ellipse(corners: usize, long: f64, short: f64) -> Vec<(f64, f64)> {
    (0..corners)
        .map(|corner| {
            let (sin, cos) = sin_cos(TAU * corner as f64 / corners as f64);
            (long * cos, short * sin)
        })
        .collect()
}

/// A diamond |x| + |y| < l of the area `area`; past half the cell its corners are cut off by the cell, as the print screen
/// has them: the area is then 1 - 2 (1 - l)^2.
fn diamond(area: f64) -> Vec<(f64, f64)> {
    if area <= 0.5 {
        let reach = (area / 2.0).sqrt();
        return vec![(0.0, -reach), (reach, 0.0), (0.0, reach), (-reach, 0.0)];
    }
    // the diamond |x| + |y| < l meets a side of the cell at c = l - 1/2, with l = 1 - sqrt((1 - area) / 2)
    let near = 0.5 - ((1.0 - area) / 2.0).sqrt();
    if near >= 0.5 - 1e-12 {
        return vec![(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)];
    }
    vec![
        (-near, -0.5),
        (near, -0.5),
        (0.5, -near),
        (0.5, near),
        (near, 0.5),
        (-near, 0.5),
        (-0.5, near),
        (-0.5, -near),
    ]
}

/// The outline of `path` at area 1 about its centroid, clockwise on the screen.
fn outline(path: &Path) -> Result<Vec<(f64, f64)>, Error> {
    let polylines = path.flattened(&mut crate::field::segment_budget("shape"))?;
    let [only] = polylines.as_slice() else {
        return Err(Error::parameter(
            "shape",
            format!("must be one closed outline, this has {} sub-paths", polylines.len()),
        ));
    };
    if !only.closed || only.points.len() < 3 {
        return Err(Error::parameter(
            "shape",
            "must be a closed outline of at least 3 points",
        ));
    }
    let (mut area, mut x, mut y) = (0.0, 0.0, 0.0);
    // measured from the first point, so that far from the origin the products do not swamp the area
    let origin = only.points[0];
    for (at, from) in only.points.iter().enumerate() {
        let to = only.points[(at + 1) % only.points.len()];
        let (x0, y0, x1, y1) = (from.x - origin.x, from.y - origin.y, to.x - origin.x, to.y - origin.y);
        let cross = x0 * y1 - x1 * y0;
        area += cross;
        x += (x0 + x1) * cross;
        y += (y0 + y1) * cross;
    }
    let span = only
        .points
        .iter()
        .map(|p| (p.x - origin.x).abs().max((p.y - origin.y).abs()))
        .fold(0.0, f64::max);
    if !(area.is_finite() && area.abs() > 1e-9 * span * span) {
        return Err(Error::parameter("shape", "must enclose an area"));
    }
    let (center_x, center_y) = (x / (3.0 * area), y / (3.0 * area));
    let scale = 1.0 / (area.abs() / 2.0).sqrt();
    let mut points: Vec<(f64, f64)> = only
        .points
        .iter()
        .map(|p| ((p.x - origin.x - center_x) * scale, (p.y - origin.y - center_y) * scale))
        .collect();
    // the shoelace sum is positive when the outline runs clockwise on the screen (`y` down)
    if area < 0.0 {
        points.reverse();
    }
    Ok(points)
}

/// The outlines of the euclidean dot at a ladder of covers, each as the distance from the center at equally spaced
/// angles: the level sets of the cosine spot function `(cos 2 pi x + cos 2 pi y) / 2` inside the cell. The level 0 is the
/// diamond; above it the sets are round, below it the cell less four round holes at its corners.
struct Table {
    /// The area of each outline, rising.
    areas: Vec<f64>,
    /// The distance of each outline's corners from the center.
    radii: Vec<Vec<f64>>,
    /// The sine and cosine of each corner's angle.
    angles: Vec<(f64, f64)>,
}

impl Table {
    fn new() -> Self {
        let steps = EUCLIDEAN_STEPS;
        let angles: Vec<(f64, f64)> = (0..steps).map(|at| sin_cos(TAU * at as f64 / steps as f64)).collect();
        let octant = steps / 8;
        let mut areas = Vec::with_capacity(EUCLIDEAN_COVERS + 1);
        let mut radii = Vec::with_capacity(EUCLIDEAN_COVERS + 1);
        for cover in 0..=EUCLIDEAN_COVERS {
            let level = 1.0 - 2.0 * cover as f64 / EUCLIDEAN_COVERS as f64;
            // the first octant, and the rest by the symmetry of the function and of the cell
            let first: Vec<f64> = (0..=octant).map(|at| reach_of_level(angles[at], level)).collect();
            let ring: Vec<f64> = (0..steps)
                .map(|at| {
                    let within = at % (steps / 4);
                    first[if within <= octant { within } else { steps / 4 - within }]
                })
                .collect();
            areas.push(area_of(&ring, steps));
            radii.push(ring);
        }
        Self { areas, radii, angles }
    }

    /// The outline of `area` of the cell: the two of the ladder it lies between, blended, then scaled to the exact area.
    fn polygon(&self, area: f64) -> Vec<(f64, f64)> {
        let upper = self
            .areas
            .partition_point(|have| *have < area)
            .clamp(1, self.areas.len() - 1);
        let (low, high) = (&self.radii[upper - 1], &self.radii[upper]);
        let share = ((area - self.areas[upper - 1]) / (self.areas[upper] - self.areas[upper - 1])).clamp(0.0, 1.0);
        let blended: Vec<f64> = low.iter().zip(high).map(|(a, b)| a + (b - a) * share).collect();
        let scale = (area / area_of(&blended, blended.len())).sqrt();
        blended
            .iter()
            .zip(&self.angles)
            .map(|(radius, (sin, cos))| (radius * scale * cos, radius * scale * sin))
            .collect()
    }
}

/// The area of the outline with these `radii` at `steps` equal angles.
fn area_of(radii: &[f64], steps: usize) -> f64 {
    let sine = sin_cos(TAU / steps as f64).0;
    0.5 * sine * (0..steps).map(|at| radii[at] * radii[(at + 1) % steps]).sum::<f64>()
}

/// The distance from the center, along the direction `(cos, sin)` in the first octant, at which the spot function falls
/// to `level`, held to the cell: the cell's side is 0.5 / cos away.
fn reach_of_level((sin, cos): (f64, f64), level: f64) -> f64 {
    let side = 0.5 / cos;
    if level >= 1.0 {
        return 0.0;
    }
    let spot = |at: f64| f64::midpoint(sin_cos(TAU * at * cos).1, sin_cos(TAU * at * sin).1);
    if spot(side) >= level {
        return side;
    }
    let (mut low, mut high) = (0.0, side);
    for _ in 0..50 {
        let middle = f64::midpoint(low, high);
        if spot(middle) > level {
            low = middle;
        } else {
            high = middle;
        }
    }
    f64::midpoint(low, high)
}
