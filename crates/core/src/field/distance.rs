//! `Field`: a number at every point.

use super::outline::Outline;
use super::segment_budget;
use crate::Error;
use crate::geometry::{Point, coordinate_point};
use crate::key::{Key, Keyed};
use crate::path::{Path, Polyline};
use std::sync::Arc;

/// What a field is made of, as a document writes it.
#[derive(Debug, Clone)]
pub(crate) enum FieldKind {
    Constant(f64),
    Ramp { start: Point, end: Point },
    Distance { path: Path, normalized: bool },
}

/// A number at every point: a constant, a ramp along a line, or the distance to a path's outline. A value, built once
/// and cheap to clone; `at` reads it. Tones for [`Hatch`](crate::Hatch) come from one.
#[derive(Debug, Clone)]
pub struct Field {
    kind: FieldKind,
    /// The distance field's outlines; none for the other kinds.
    parts: Option<Arc<Parts>>,
}

impl Field {
    /// The same number everywhere.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `value` if it is NaN or infinite.
    pub fn constant(value: f64) -> Result<Self, Error> {
        let value = crate::geometry::finite("value", value)?;
        Ok(Self {
            kind: FieldKind::Constant(value),
            parts: None,
        })
    }

    /// 0 at `start`, 1 at `end`, linear along the line between and held beyond: the value of a point is the share of
    /// the line its projection onto the line makes.
    ///
    /// # Errors
    /// [`Error::Parameter`] if a point is not a coordinate or the two are equal.
    pub fn ramp(start: impl Into<Point>, end: impl Into<Point>) -> Result<Self, Error> {
        let (start, end) = (coordinate_point("start", start)?, coordinate_point("end", end)?);
        if start == end {
            return Err(Error::parameter("end", "must differ from start"));
        }
        Ok(Self {
            kind: FieldKind::Ramp { start, end },
            parts: None,
        })
    }

    /// The distance to the outline of `path` in design pixels, negative inside (by the nonzero rule). With
    /// `normalized`, a letter-shaped value instead: inside each part (an outline with its holes) 0 on the outer edge
    /// rising to 1 on the holes' edges (`d / (d + distance to the holes)`), or, for a part with no hole, to 1 at its
    /// deepest point (`d` over the part's largest inscribed radius, estimated to about a percent); 0 outside and
    /// inside a hole. Where parts overlap, the distance is to the nearest edge of any: resolve overlaps first.
    ///
    /// # Errors
    /// [`Error::Geometry`] if the path has no edge; [`Error::Parameter`] past `MAX_SEGMENTS` edges.
    pub fn distance(path: &Path, normalized: bool) -> Result<Self, Error> {
        let polylines = path.flattened(&mut segment_budget("path"))?;
        let whole = Outline::from_polylines(&polylines, "distance")?;
        let parts = Parts::new(whole, &polylines, normalized)?;
        Ok(Self {
            kind: FieldKind::Distance {
                path: path.clone(),
                normalized,
            },
            parts: Some(Arc::new(parts)),
        })
    }

    /// The field's number at `point`.
    #[must_use]
    pub fn at(&self, point: impl Into<Point>) -> f64 {
        let point = point.into();
        match (&self.kind, &self.parts) {
            (FieldKind::Constant(value), _) => *value,
            (FieldKind::Ramp { start, end }, _) => {
                let (dx, dy) = (end.x - start.x, end.y - start.y);
                (((point.x - start.x) * dx + (point.y - start.y) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0)
            }
            (FieldKind::Distance { normalized, .. }, Some(parts)) => parts.at(point, *normalized),
            (FieldKind::Distance { .. }, None) => 0.0,
        }
    }

    pub(crate) fn kind(&self) -> &FieldKind {
        &self.kind
    }
}

impl Keyed for Field {
    fn write_key(&self, key: &mut Key) {
        match &self.kind {
            FieldKind::Constant(value) => {
                key.word(1);
                key.number(*value);
            }
            FieldKind::Ramp { start, end } => {
                key.word(2);
                key.of(start);
                key.of(end);
            }
            FieldKind::Distance { path, normalized } => {
                key.word(3);
                key.word(u64::from(*normalized));
                key.of(path);
            }
        }
    }
}

/// An outline with its holes: one letter shape.
#[derive(Debug)]
struct Part {
    extent: (f64, f64, f64, f64),
    region: Outline,
    outer: Outline,
    holes: Option<Outline>,
    /// The largest distance to the outer edge inside, for a part with no hole.
    inradius: f64,
}

#[derive(Debug)]
struct Parts {
    whole: Outline,
    parts: Vec<Part>,
}

impl Parts {
    fn new(whole: Outline, polylines: &[Polyline], normalized: bool) -> Result<Self, Error> {
        let parts = if normalized { split(polylines)? } else { Vec::new() };
        Ok(Self { whole, parts })
    }

    fn at(&self, point: Point, normalized: bool) -> f64 {
        if !normalized {
            let distance = self.whole.nearest(point).distance;
            return if self.whole.contains(point) {
                -distance
            } else {
                distance
            };
        }
        let mut best = 0.0_f64;
        for part in &self.parts {
            let (l, t, r, b) = part.extent;
            if point.x < l || point.x > r || point.y < t || point.y > b || !part.region.contains(point) {
                continue;
            }
            let outer = part.outer.nearest(point).distance;
            let value = match &part.holes {
                Some(holes) => {
                    let total = outer + holes.nearest(point).distance;
                    if total > 0.0 { outer / total } else { 0.0 }
                }
                None => (outer / part.inradius).min(1.0),
            };
            best = best.max(value);
        }
        best
    }
}

/// The closed outlines of `polylines` grouped into parts: an outline nested an even number of times is an outer, an
/// odd number a hole of the smallest outer that holds it.
fn split(polylines: &[Polyline]) -> Result<Vec<Part>, Error> {
    let shapes: Vec<&Polyline> = polylines
        .iter()
        .filter(|line| line.drawn && line.points.len() > 2)
        .collect();
    let holders: Vec<Vec<usize>> = shapes
        .iter()
        .enumerate()
        .map(|(index, line)| {
            (0..shapes.len())
                .filter(|&other| other != index && winding(&shapes[other].points, line.points[0]) != 0)
                .collect()
        })
        .collect();
    let area = |index: usize| shoelace(&shapes[index].points).abs();
    let mut parts = Vec::new();
    for (index, holding) in holders.iter().enumerate() {
        if holding.len() % 2 == 1 {
            continue;
        }
        let mut members = vec![shapes[index].clone()];
        let mut holes = Vec::new();
        for (hole, holding_hole) in holders.iter().enumerate() {
            let owner = holding_hole
                .iter()
                .copied()
                .min_by(|&a, &b| area(a).total_cmp(&area(b)));
            if holding_hole.len() % 2 == 1 && owner == Some(index) {
                members.push(shapes[hole].clone());
                holes.push(shapes[hole].clone());
            }
        }
        let outer = Outline::from_polylines(std::slice::from_ref(shapes[index]), "distance")?;
        let region = Outline::from_polylines(&members, "distance")?;
        let holes = if holes.is_empty() {
            None
        } else {
            Some(Outline::from_polylines(&holes, "distance")?)
        };
        let inradius = if holes.is_none() {
            inradius(&region, &outer)
        } else {
            1.0
        };
        parts.push(Part {
            extent: region.extent(),
            region,
            outer,
            holes,
            inradius,
        });
    }
    Ok(parts)
}

/// The largest distance from an interior point to the outer edge: the best of a grid of probes, then three rounds of
/// looking round the best one at half the step.
fn inradius(region: &Outline, outer: &Outline) -> f64 {
    // constant: 48 probes a side find the deepest part of a letter-sized outline; the rounds below tighten the answer
    // to about a percent of the part's size
    const PROBES: u32 = 48;
    // constant: each round halves the step, so three rounds leave the deepest point within a 1/8 of a probe step
    const ROUNDS: u32 = 3;
    let (left, top, right, bottom) = region.extent();
    let (step_x, step_y) = ((right - left) / f64::from(PROBES), (bottom - top) / f64::from(PROBES));
    let depth = |p: Point| {
        if region.contains(p) {
            outer.nearest(p).distance
        } else {
            0.0
        }
    };
    let (mut best, mut at) = (0.0_f64, Point::new(left.midpoint(right), top.midpoint(bottom)));
    for i in 0..=PROBES {
        for j in 0..=PROBES {
            let p = Point::new(left + step_x * f64::from(i), top + step_y * f64::from(j));
            let d = depth(p);
            if d > best {
                (best, at) = (d, p);
            }
        }
    }
    let (mut sx, mut sy) = (step_x, step_y);
    for _ in 0..ROUNDS {
        let centre = at;
        for dx in [-1.0, 0.0, 1.0] {
            for dy in [-1.0, 0.0, 1.0] {
                let p = Point::new(centre.x + dx * sx, centre.y + dy * sy);
                let d = depth(p);
                if d > best {
                    (best, at) = (d, p);
                }
            }
        }
        sx /= 2.0;
        sy /= 2.0;
    }
    best.max(f64::MIN_POSITIVE)
}

fn shoelace(points: &[Point]) -> f64 {
    let mut sum = 0.0;
    for (index, a) in points.iter().enumerate() {
        let b = points[(index + 1) % points.len()];
        sum += a.x * b.y - b.x * a.y;
    }
    sum / 2.0
}

fn winding(corners: &[Point], point: Point) -> i32 {
    let mut turns = 0;
    for (index, from) in corners.iter().enumerate() {
        let to = corners[(index + 1) % corners.len()];
        let side = (to.x - from.x) * (point.y - from.y) - (point.x - from.x) * (to.y - from.y);
        if from.y <= point.y {
            if to.y > point.y && side > 0.0 {
                turns += 1;
            }
        } else if to.y <= point.y && side < 0.0 {
            turns -= 1;
        }
    }
    turns
}
