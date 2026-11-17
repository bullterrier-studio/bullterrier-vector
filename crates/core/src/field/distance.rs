//! `Field`: a number at every point.

use super::map::MapData;
use super::outline::Outline;
use super::{Shapes, once, read, segment_budget};
use crate::geometry::{Point, Rect, coordinate_point};
use crate::key::{Key, Keyed, key_of};
use crate::path::Polyline;
use crate::{Drawing, Error, Expr, Param, Values};
use std::fmt;
use std::sync::Arc;

/// What a field is made of, as a document writes it.
#[derive(Debug, Clone)]
pub(crate) enum FieldKind {
    Constant(Expr),
    Ramp { start: Point, end: Point },
    Distance { drawing: Drawing, normalized: bool },
    Map(Arc<MapData>),
}

/// What reads a field at a point: the made field, with its numbers worked out and its drawings drawn.
#[derive(Debug, Clone)]
enum Bound {
    Constant(f64),
    Ramp { start: Point, end: Point },
    Distance { parts: Arc<Parts>, normalized: bool },
    Map(Arc<MapData>),
}

/// A number at every point: a constant, a ramp along a line, the distance to a drawing's outline, or a grayscale
/// map. A value, built once and cheap to clone; `at` reads it, with any Param at its default. Tones for
/// [`Hatch`](crate::Hatch) come from one, and a `hatch` step reads it again with the values it draws with.
#[derive(Debug, Clone)]
pub struct Field {
    kind: FieldKind,
    bound: Bound,
}

impl Field {
    /// The same number everywhere: a number, or an expression of Params (read once, with the values of the draw).
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `value` if it is NaN or infinite, or a per-item number.
    pub fn constant(value: impl Into<Expr>) -> Result<Self, Error> {
        Self::made(&FieldKind::Constant(once("value", &value.into())?))
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
        Self::made(&FieldKind::Ramp { start, end })
    }

    /// The distance to the outline of `drawing` in design pixels, negative inside (by the nonzero rule): the
    /// outlines of all its shapes together, drawn with the Params at their defaults for [`Field::at`] and with the
    /// values of the draw in a `hatch` step. A `Path` goes where a drawing is asked for. With `normalized`, a
    /// letter-shaped value instead: inside each part (an outline with its holes) 0 on the outer edge rising to 1 on the
    /// holes' edges (`d / (d + distance to the holes)`), or, for a part with no hole, to 1 at its deepest point (`d`
    /// over the part's largest inscribed radius, estimated to about a percent); 0 outside and inside a hole. Where
    /// parts overlap, the distance is to the nearest edge of any: resolve overlaps first.
    ///
    /// # Errors
    /// The drawing's own error if it cannot be drawn; [`Error::Geometry`] if it has no edge; [`Error::Parameter`] past
    /// `MAX_SEGMENTS` edges.
    pub fn distance(drawing: impl Into<Drawing>, normalized: bool) -> Result<Self, Error> {
        Self::made(&FieldKind::Distance {
            drawing: drawing.into(),
            normalized,
        })
    }

    /// The field `kind` says, with its Params at their defaults.
    fn made(kind: &FieldKind) -> Result<Self, Error> {
        Self::build(kind, &Values::new(), &mut Shapes::Defaults)
    }

    /// The field `kind` says with the Params at `env` and the drawings from `shapes`.
    fn build(kind: &FieldKind, env: &Values, shapes: &mut Shapes<'_>) -> Result<Self, Error> {
        let bound = match kind {
            FieldKind::Constant(value) => Bound::Constant(read("value", value, env)?),
            FieldKind::Ramp { start, end } => Bound::Ramp {
                start: *start,
                end: *end,
            },
            FieldKind::Distance { drawing, normalized } => {
                let path = shapes.path_of(drawing)?;
                let polylines = path.flattened(&mut segment_budget("drawing"))?;
                let whole = Outline::from_polylines(&polylines, "distance")?;
                Bound::Distance {
                    parts: Arc::new(Parts::new(whole, &polylines, *normalized)?),
                    normalized: *normalized,
                }
            }
            FieldKind::Map(map) => Bound::Map(Arc::clone(map)),
        };
        Ok(Self {
            kind: kind.clone(),
            bound,
        })
    }

    /// This field again with the Params at `env` and the drawings it reads from `shapes`.
    pub(crate) fn bind(&self, env: &Values, shapes: &mut Shapes<'_>) -> Result<Self, Error> {
        Self::build(&self.kind, env, shapes)
    }

    pub(crate) fn of_map(map: MapData) -> Self {
        let map = Arc::new(map);
        Self {
            kind: FieldKind::Map(Arc::clone(&map)),
            bound: Bound::Map(map),
        }
    }

    /// The drawings this field reads, in order.
    pub(crate) fn drawings(&self) -> Vec<&Drawing> {
        match &self.kind {
            FieldKind::Distance { drawing, .. } => vec![drawing],
            _ => Vec::new(),
        }
    }

    /// Adds the Params this field reads to `into`.
    pub(crate) fn collect_params(&self, into: &mut Vec<Param>) {
        if let FieldKind::Constant(value) = &self.kind {
            value.collect_params(into);
        }
    }

    /// The field's number at `point`.
    #[must_use]
    pub fn at(&self, point: impl Into<Point>) -> f64 {
        let point = point.into();
        match &self.bound {
            Bound::Constant(value) => *value,
            Bound::Ramp { start, end } => {
                let (dx, dy) = (end.x - start.x, end.y - start.y);
                (((point.x - start.x) * dx + (point.y - start.y) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0)
            }
            Bound::Distance { parts, normalized } => parts.at(point, *normalized),
            Bound::Map(map) => map.sample(point),
        }
    }

    pub(crate) fn kind(&self) -> &FieldKind {
        &self.kind
    }
}

impl Keyed for Field {
    /// A drawing is told apart by the call that takes it (the cache and the graph), so the key holds none: a drawing's
    /// own key would hold the keys of the drawings its steps take, once for each way to reach them.
    fn write_key(&self, key: &mut Key) {
        match &self.kind {
            FieldKind::Constant(value) => {
                key.word(1);
                key.of(value);
            }
            FieldKind::Ramp { start, end } => {
                key.word(2);
                key.of(start);
                key.of(end);
            }
            FieldKind::Distance { normalized, .. } => {
                key.word(3);
                key.word(u64::from(*normalized));
            }
            FieldKind::Map(map) => {
                key.word(4);
                key.count(map.width);
                key.count(map.height);
                key.number(map.frame.x);
                key.number(map.frame.y);
                key.number(map.frame.width);
                key.number(map.frame.height);
                for chunk in map.sha256.chunks(8) {
                    key.word(chunk.iter().fold(0, |word, byte| word << 8 | u64::from(*byte)));
                }
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

impl PartialEq for Field {
    /// The same call: equal numbers, and drawings that are the same ([`Drawing::is_same_as`]).
    fn eq(&self, other: &Self) -> bool {
        key_of(self) == key_of(other)
            && self
                .drawings()
                .iter()
                .zip(other.drawings())
                .all(|(a, b)| a.is_same_as(b))
    }
}

/// The field as the call that makes it reads in the Python package: `Field.ramp((0, 0), (200, 0))`.
impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            FieldKind::Constant(value) => write!(f, "{value}"),
            FieldKind::Ramp { start, end } => {
                write!(f, "Field.ramp(({}, {}), ({}, {}))", start.x, start.y, end.x, end.y)
            }
            FieldKind::Map(map) => {
                let Rect { x, y, width, height } = map.frame;
                write!(
                    f,
                    "Field.map({}, {}, <{} bytes>, box=({x}, {y}, {width}, {height}))",
                    map.width,
                    map.height,
                    map.bytes.len()
                )
            }
            FieldKind::Distance { drawing, normalized } => {
                match drawing.lone_path() {
                    Some(path) => write!(f, "Field.distance(Path.from_svg_path('{}')", path.to_svg_path())?,
                    None => write!(f, "Field.distance({drawing:?}")?,
                }
                write!(f, "{})", if *normalized { ", normalized=True" } else { "" })
            }
        }
    }
}
