//! `Path::rows`: ridgelines. Horizontal rows across a region, each lifted by a tone, with the parts a nearer row hides
//! removed, so a map of tone reads as a landscape of lines.

#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "the counts of rows and samples are checked against MAX_SAMPLES before they are used"
)]

use super::within::cut;
use crate::field::{Field, Outline, once, read, segment_budget};
use crate::geometry::{MAX_COORDINATE, Point};
use crate::path::{Path, Region};
use crate::{Error, Expr, Values};

/// How far apart the points of a row are, as a share of the spacing between rows.
// constant: half a spacing follows a tone that varies over about a spacing without visible corners, and puts a
// point every few pixels for the hidden-line test to compare against the rows in front
const STEP: f64 = 0.5;

/// The most points all the rows of one call may have.
// constant: 5 million points are 80 MB of coordinates and about a second of reads; a poster at 600 dpi still fits
const MAX_SAMPLES: f64 = 5_000_000.0;

/// The most edges the region scans of one call may visit.
// constant: a billion visits of a few instructions is a few seconds, as for `hatch`
const MAX_VISITS: f64 = 1e9;

/// What to draw: the spacing between rows, the tone that lifts them and by how much.
#[derive(Debug, Clone)]
pub struct Rows {
    spacing: Expr,
    tone: Field,
    amplitude: Expr,
}

impl Rows {
    /// Rows `spacing` design pixels apart, the first half a spacing below the top of the region. A row is lifted
    /// (towards the top of the picture) by `amplitude` times `tone` at the point of the row it lifts, read as it is: a
    /// map is 0 to 1, and a negative amplitude pushes a row down. Each row hides what lies behind it, below its curve
    /// and farther from the viewer, that is higher up the picture.
    ///
    /// Both numbers are numbers or expressions of Params, read once.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `spacing` or `amplitude` if it is a per-item number.
    pub fn new(spacing: impl Into<Expr>, tone: Field, amplitude: impl Into<Expr>) -> Result<Self, Error> {
        Ok(Self {
            spacing: once("spacing", &spacing.into())?,
            tone,
            amplitude: once("amplitude", &amplitude.into())?,
        })
    }

    pub(crate) fn spacing_of(&self) -> &Expr {
        &self.spacing
    }

    pub(crate) fn amplitude_of(&self) -> &Expr {
        &self.amplitude
    }

    pub(crate) fn tone_of(&self) -> &Field {
        &self.tone
    }
}

impl Path {
    /// Ridgelines inside the region this path fills (nonzero rule): rows `spacing` apart, lifted by `amplitude` times
    /// `tone` (see [`Rows::new`]), each cut where a nearer row's silhouette passes over it. Rows run left to right, from
    /// the top of the picture down, in pieces where the region or a nearer row interrupts them. A row starts where its
    /// base is inside the region and is drawn only where it is still inside once lifted; the part lifted out of the
    /// region is cut away and hides nothing, so the rows behind it show again. The tone is read at the
    /// point of the row before it is lifted. A Param is read at its default here; a `rows` step reads it with the values
    /// it draws with. An empty path gives an empty path.
    ///
    /// Where the region's edge is not at a point of the row's lattice, the hidden-line test reads the nearer rows
    /// between two lattice points by interpolation, so a row can end a fraction of a step early there.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `spacing` if it is not above 0 and a coordinate, if the region would need more than
    /// 5 million points for it or more than a billion edge visits to scan it, or if a step of it would not move at the
    /// region's coordinates; naming `amplitude` if it is not a number within ±1e100; naming `spacing` past
    /// `MAX_SEGMENTS` points.
    pub fn rows(&self, rows: &Rows) -> Result<Path, Error> {
        let values = Values::new();
        let spacing = read("spacing", &rows.spacing, &values)?;
        let amplitude = read("amplitude", &rows.amplitude, &values)?;
        self.rows_charged(spacing, amplitude, &rows.tone, &mut segment_budget("spacing"))
    }

    /// As [`Path::rows`] with the numbers worked out, calling `charge` with the points made before they are kept.
    pub(crate) fn rows_charged(
        &self,
        spacing: f64,
        amplitude: f64,
        tone: &Field,
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<Path, Error> {
        if !(spacing.is_finite() && spacing > 0.0 && spacing <= MAX_COORDINATE) {
            return Err(Error::parameter("spacing", format!("must be above 0, got {spacing}")));
        }
        if amplitude.abs() > MAX_COORDINATE {
            return Err(Error::parameter(
                "amplitude",
                format!("must be within ±{MAX_COORDINATE}, got {amplitude}"),
            ));
        }
        if !self.may_fill() {
            return Ok(Path::new());
        }
        let region = Region::of(self, charge)?;
        let Some((top, bottom)) = region.extent() else {
            return Ok(Path::new());
        };
        // the same flattening the region was charged for
        let Ok(outline) = Outline::from_polylines(&self.flattened(&mut |_| Ok(()))?, "rows") else {
            return Ok(Path::new());
        };
        let mut cut_visits = 0.0;
        let mut work = |edges: usize| {
            cut_visits += edges as f64;
            if cut_visits > MAX_VISITS {
                Err(Error::parameter(
                    "spacing",
                    format!(
                        "is too small for a region this detailed: cutting to it would pass {MAX_VISITS} edge visits"
                    ),
                ))
            } else {
                Ok(())
            }
        };
        let (left, right) = self_x_extent(self)?;
        let step = STEP * spacing;
        let reach = left.abs().max(right.abs()).max(top.abs()).max(bottom.abs());
        if reach + step <= reach {
            return Err(Error::parameter(
                "spacing",
                format!("is too small for coordinates near {reach}: a step of it would not move"),
            ));
        }
        let rows = if bottom - top >= spacing / 2.0 {
            ((bottom - top - spacing / 2.0) / spacing).floor() + 1.0
        } else {
            0.0
        };
        let columns = ((right - left) / step).ceil() + 1.0;
        // the lattice of columns is made whatever the rows, so it is bounded on its own
        if rows.max(1.0) * columns > MAX_SAMPLES {
            return Err(Error::parameter(
                "spacing",
                format!("is too small for a region this large: it would need over {MAX_SAMPLES} points"),
            ));
        }
        tone.charge_reads("spacing", "rows", rows * columns * FOOTPRINT.len() as f64)?;
        let mut visits = 0.0;
        let lattice: Vec<f64> = (0..columns as usize)
            .map(|i| (left + i as f64 * step).min(right))
            .collect();
        let mut horizon = vec![f64::INFINITY; lattice.len()];
        let mut drawn: Vec<Vec<Path>> = Vec::new();
        for k in (0..rows as usize).rev() {
            let base = top + spacing / 2.0 + k as f64 * spacing;
            let spans = region.spans_at(base, |edges| {
                visits += edges as f64;
                if visits > MAX_VISITS {
                    Err(Error::parameter(
                        "spacing",
                        format!(
                            "is too small for a region this detailed: scanning it would pass {MAX_VISITS} edge visits"
                        ),
                    ))
                } else {
                    Ok(())
                }
            })?;
            let mut row = Vec::new();
            for (from, to) in spans {
                let (first, last) = (
                    lattice.partition_point(|x| *x <= from),
                    lattice.partition_point(|x| *x < to),
                );
                let xs = std::iter::once(from)
                    .chain(lattice[first..last].iter().copied())
                    .chain(std::iter::once(to));
                let points: Vec<(Point, f64)> = xs
                    .map(|x| {
                        let lifted = Point::new(x, base - amplitude * footprint(tone, x, base, step, spacing));
                        (lifted, cover(&lattice, &horizon, x) - lifted.y)
                    })
                    .collect();
                for run in visible(&points) {
                    for (piece, _) in cut(&run, false, &outline, &mut work)? {
                        charge(piece.len())?;
                        row.push(Path::polyline(piece, false)?);
                    }
                }
                // a row runs left to right, so each of its parts inside the region is a stretch of x
                let lifted: Vec<Point> = points.iter().map(|(point, _)| *point).collect();
                let inside: Vec<(f64, f64)> = cut(&lifted, false, &outline, &mut work)?
                    .iter()
                    .map(|(piece, _)| (piece[0].x, piece[piece.len() - 1].x))
                    .collect();
                for (point, _) in &points[1..points.len() - 1] {
                    if inside.iter().any(|(from, to)| (*from..=*to).contains(&point.x)) {
                        let at = lattice.partition_point(|x| *x < point.x);
                        horizon[at] = horizon[at].min(point.y);
                    }
                }
            }
            drawn.push(row);
        }
        Ok(Path::joined(drawn.iter().rev().flatten()))
    }
}

/// Where a point reads its tone, as shares of the step along the row and of the spacing across it, and how much each
/// reading weighs: a tent over the point's share of the rows.
// constant: three readings each way, half a step and half a spacing apart, flatten any detail of a period of a step
// along or a spacing across (too fine for the rows to draw) to its mean, and leave a slope as it is
const FOOTPRINT: [(f64, f64, f64); 9] = [
    (-0.5, -0.5, 1.0),
    (0.0, -0.5, 2.0),
    (0.5, -0.5, 1.0),
    (-0.5, 0.0, 2.0),
    (0.0, 0.0, 4.0),
    (0.5, 0.0, 2.0),
    (-0.5, 0.5, 1.0),
    (0.0, 0.5, 2.0),
    (0.5, 0.5, 1.0),
];

/// The tone at `(x, y)` read over the footprint of a point of the rows, `step` wide and `spacing` tall.
fn footprint(tone: &Field, x: f64, y: f64, step: f64, spacing: f64) -> f64 {
    FOOTPRINT
        .iter()
        .map(|(along, across, weight)| weight * tone.at(Point::new(x + along * step, y + across * spacing)))
        .sum::<f64>()
        / 16.0
}

/// The left and right of the path's bounds.
fn self_x_extent(path: &Path) -> Result<(f64, f64), Error> {
    let bounds = path.bounds()?;
    Ok((bounds.x, bounds.x + bounds.width))
}

/// The height of the nearest cover at `x`: the horizon at a lattice point, between two of them the interpolation, and
/// infinite (no cover) where either is.
fn cover(lattice: &[f64], horizon: &[f64], x: f64) -> f64 {
    let at = lattice.partition_point(|point| *point < x);
    if at < lattice.len() && lattice[at].to_bits() == x.to_bits() {
        return horizon[at];
    }
    if at == 0 || at >= lattice.len() {
        return f64::INFINITY;
    }
    let (a, b) = (horizon[at - 1], horizon[at]);
    if !(a.is_finite() && b.is_finite()) {
        return f64::INFINITY;
    }
    let t = (x - lattice[at - 1]) / (lattice[at] - lattice[at - 1]);
    a + (b - a) * t
}

/// The runs of `points` (each with how far it lies below its cover: above 0 is seen), cut where the sign changes.
fn visible(points: &[(Point, f64)]) -> Vec<Vec<Point>> {
    let mut runs: Vec<Vec<Point>> = Vec::new();
    let mut current: Vec<Point> = Vec::new();
    let mut before: Option<(Point, f64)> = None;
    for &(point, depth) in points {
        match before {
            None => {
                if depth > 0.0 {
                    current.push(point);
                }
            }
            Some((last, last_depth)) => match (last_depth > 0.0, depth > 0.0) {
                (true, true) => current.push(point),
                (true, false) => {
                    if last_depth.is_finite() {
                        let t = last_depth / (last_depth - depth);
                        current.push(Point::new(
                            last.x + (point.x - last.x) * t,
                            last.y + (point.y - last.y) * t,
                        ));
                    }
                    runs.push(std::mem::take(&mut current));
                }
                (false, true) => {
                    if depth.is_finite() {
                        let t = -last_depth / (depth - last_depth);
                        current = vec![Point::new(
                            last.x + (point.x - last.x) * t,
                            last.y + (point.y - last.y) * t,
                        )];
                    }
                    current.push(point);
                }
                (false, false) => {}
            },
        }
        before = Some((point, depth));
    }
    if !current.is_empty() {
        runs.push(current);
    }
    runs.retain(|run| run.len() > 1);
    runs
}
