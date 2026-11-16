//! Morphing: one outline carried to another. [`Path::morph`] flattens both paths, pairs their contours (by role and by
//! where they are), aligns each pair's start, and moves every point of one to the point of the other that stands at
//! the same share of the way round. A contour with no partner shrinks to, or grows from, a single point; a hole with no
//! partner is subtracted from the rest, never wound (a reversed contour leaving the moving body would paint outside it).

use super::curve::lerp_number;
use super::polyline::Polyline;
use super::warp::segment_budget;
use super::{Combine, FLATNESS, Path, Segment, SubPath};
use crate::Error;
use crate::geometry::{Point, finite};
use std::collections::BTreeSet;

/// The points of each contour at equal steps of its length that decide where a closed contour starts.
// constant: 128 samples show the shape of an outline (a letter's features are wider than a 128th of its perimeter) and
// the start they pick is as good as a finer search's: over every ordered pair of the closed contours of the glyphs of
// "ABRSgq&@" in the bundled Bricolage Grotesque and of a circle (380 pairs), the cost of the pick (summed squared
// distance of 1024 samples) is at most 1.0005 times the best of the 1024 starts (measured once); the cost of a search
// grows as the square of the samples
const ALIGN_SAMPLES: u32 = 128;

impl Path {
    /// The outline `fraction` of the way from this path to `target`: with `fraction` 0 it fills what this path fills, with
    /// 1 what `target` fills, and any finite number between or beyond is the straight lerp of the two (outside 0 to 1 it
    /// extrapolates, so an overshooting ease leaves the hull of the two shapes). The result is made of straight lines;
    /// each closed sub-path stays closed and each open one stays open.
    ///
    /// Both paths are flattened as for drawing. A closed contour is an outer or a hole by the sign of its area (a
    /// contour with no area is an outer), after the whole path is reversed once if its largest contour runs the other
    /// way (never one contour alone, which would turn a hole into a fill). Closed contours pair with closed ones of the
    /// same role, open with open, nearest first: by the distance between the contours' centres (the mean of their points),
    /// each taken in the box of its own path, scaled to 0 to 1; ties go to the lower indices, so morphing back from the
    /// other end pairs alike. A closed pair starts where its two outlines lie nearest (128 equal steps of length compared
    /// by summed squared distance; the first start wins a tie); an open pair is joined end to end or end to start,
    /// whichever is nearer. The points of a pair are the vertices of both outlines, each placed at its share of the
    /// length, so at 0 and 1 the result is the flattening of this path and of `target`. A contour with no length is its
    /// point.
    ///
    /// A contour with no partner, because the other path has fewer of its kind, shrinks to (or grows from) one point:
    /// the point in the other path's box that has the same place there as the contour's centre has in its own box (the
    /// contour's own centre if the other path is empty). Past 1 (a growing one: before 0) it stays that point, never
    /// coming back through it, where a matched contour goes on along its line. A hole without a partner is not wound:
    /// it is subtracted, and an outer inside it stays, as the nonzero rule fills it. When there is one, every closed
    /// contour of the result goes through the boolean engine (as in [`Path::subtract`]): the
    /// closed part is canonical, overlapping outers are merged and a closed contour of fewer than 3 points is dropped;
    /// open contours are kept as they are. Without one the result is the lerp as it stands. An empty path on one side
    /// makes every contour of the other unpartnered. Sub-paths come out grouped by role, in the order they were paired,
    /// not in the order they were written.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `fraction` if it is NaN or infinite, if a point of the result would be beyond
    /// ±1e100, or if the points of the flattening, the pairs to rank and the points of the result, over all the
    /// contours, are more than `MAX_SEGMENTS` (10,000,000), counted before any is made (a subtraction counts its own edge pairs
    /// and vertices against the same total).
    pub fn morph(&self, target: &Self, fraction: f64) -> Result<Self, Error> {
        self.morph_charged(target, fraction, &mut segment_budget("fraction"))
    }

    /// As [`Path::morph`], calling `charge` with the edges of each curve flattened, the pairs of contours it ranks and
    /// the points of each contour of the result, all before the lists are made; its error stops the verb.
    pub(crate) fn morph_charged(
        &self,
        target: &Self,
        fraction: f64,
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<Self, Error> {
        let fraction = finite("fraction", fraction)?;
        let (from, to) = (Side::new(self, charge)?, Side::new(target, charge)?);
        let plans = plan(&from, &to, charge)?;
        for plan in &plans {
            charge(plan.points())?;
        }
        let (mut body, mut cuts) = (Vec::new(), Vec::new());
        for plan in &plans {
            let moved = plan.sub_path(fraction);
            if plan.is_unmatched_hole() {
                cuts.extend(moved);
            } else {
                body.extend(moved);
            }
        }
        let body = Self { subpaths: body }.within_limit("fraction")?;
        if cuts.is_empty() {
            return Ok(body);
        }
        let cuts = Self { subpaths: cuts }.within_limit("fraction")?;
        // the engine takes closed outlines only; an open sub-path would be filled as if closed
        let (closed, open): (Vec<SubPath>, Vec<SubPath>) = body.subpaths.into_iter().partition(|sub| sub.closed);
        // the cuts wind the other way, so under the nonzero rule they take away what the closed part fills and leave an
        // outer inside a cut standing, as the input itself fills it; the intersection keeps them from painting outside
        let body = Self { subpaths: closed };
        let whole = Self {
            subpaths: body.subpaths.iter().chain(&cuts.subpaths).cloned().collect(),
        };
        let region = whole.combined(&body, Combine::Intersect, FLATNESS, charge)?;
        Ok(Self {
            subpaths: region.subpaths.into_iter().chain(open).collect(),
        })
    }
}

/// What a contour is for pairing: closed ones pair by sign of area, open ones with open ones.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Role {
    Outer,
    Hole,
    Open,
}

/// One flattened sub-path with where each vertex lies along it.
struct Contour {
    points: Vec<Point>,
    /// The share of the length (0 to 1, by the straight edges) at which each vertex lies: the first at 0, and an open
    /// contour's last at 1. A contour with no length is its first point alone, at 0.
    at: Vec<f64>,
    closed: bool,
    drawn: bool,
    /// The signed area of a closed contour: positive when it runs clockwise on screen (`y` down). 0 for an open one.
    area: f64,
}

impl Contour {
    fn new(mut points: Vec<Point>, closed: bool, drawn: bool) -> Self {
        let mut at = Vec::with_capacity(points.len());
        let mut run = 0.0;
        for (index, &point) in points.iter().enumerate() {
            at.push(run);
            let next = points.get(index + 1).or(if closed { points.first() } else { None });
            if let Some(&next) = next {
                run += libm::hypot(next.x - point.x, next.y - point.y);
            }
        }
        let area = match points.first() {
            Some(&origin) if closed => {
                points
                    .iter()
                    .zip(points.iter().cycle().skip(1))
                    .map(|(from, to)| (from.x - origin.x) * (to.y - origin.y) - (to.x - origin.x) * (from.y - origin.y))
                    .sum::<f64>()
                    / 2.0
            }
            _ => 0.0,
        };
        if run > 0.0 {
            for share in &mut at {
                *share /= run;
            }
        } else {
            points.truncate(1);
            at = vec![0.0];
        }
        Self {
            points,
            at,
            closed,
            drawn,
            area,
        }
    }

    fn of(polyline: Polyline) -> Self {
        Self::new(polyline.points, polyline.closed, polyline.drawn)
    }

    /// The same outline run the other way, from its last point.
    fn reversed(self) -> Self {
        let mut points = self.points;
        points.reverse();
        Self::new(points, self.closed, self.drawn)
    }

    fn role(&self) -> Role {
        if !self.closed {
            Role::Open
        } else if self.area < 0.0 {
            Role::Hole
        } else {
            Role::Outer
        }
    }

    /// The mean of the points.
    fn centre(&self) -> Point {
        #[expect(clippy::cast_precision_loss, reason = "a count of at most 10 million is exact")]
        let count = self.points.len() as f64;
        let (x, y) = self.points.iter().fold((0.0, 0.0), |(x, y), p| (x + p.x, y + p.y));
        Point::new(x / count, y / count)
    }

    /// The point `share` of the way along (0 to 1; an open contour's end at 1, a closed one's start at 0 and 1).
    fn point_at(&self, share: f64) -> Point {
        let Some((&first, _)) = self.points.split_first().filter(|(_, rest)| !rest.is_empty()) else {
            return self.points.first().copied().unwrap_or(Point::new(0.0, 0.0));
        };
        let index = self.at.partition_point(|&at| at <= share).saturating_sub(1);
        let from = self.points.get(index).copied().unwrap_or(first);
        let next = match self.points.get(index + 1) {
            Some(&next) => next,
            None if self.closed => first,
            None => return from,
        };
        let (begin, end) = (
            self.at.get(index).copied().unwrap_or(0.0),
            self.at.get(index + 1).copied().unwrap_or(1.0),
        );
        let along = if end > begin {
            (share - begin) / (end - begin)
        } else {
            0.0
        };
        Point::new(lerp_number(from.x, next.x, along), lerp_number(from.y, next.y, along))
    }

    /// `count` points at equal steps of `1 / steps` of the length, from the start.
    fn samples(&self, count: u32, steps: u32) -> Vec<Point> {
        (0..count)
            .map(|index| self.point_at(f64::from(index) / f64::from(steps)))
            .collect()
    }

    fn is_point(&self) -> bool {
        self.points.len() == 1
    }
}

/// The sum of the squared distances between the points of two lists, paired in order.
fn cost<'a>(first: &[Point], second: impl Iterator<Item = &'a Point>) -> f64 {
    first
        .iter()
        .zip(second)
        .map(|(a, b)| (a.x - b.x) * (a.x - b.x) + (a.y - b.y) * (a.y - b.y))
        .sum()
}

/// A share wrapped into 0 up to 1.
fn wrapped(share: f64) -> f64 {
    let share = share.rem_euclid(1.0);
    // a share a hair below 0 wraps to exactly 1
    if share >= 1.0 { 0.0 } else { share }
}

/// How far round `to` its start lies from the start of `from`, the share of its length at which the two outlines lie
/// nearest each other (0 if two are equally near, and for a contour with no length).
fn closed_start(from: &Contour, to: &Contour) -> f64 {
    if from.is_point() || to.is_point() {
        return 0.0;
    }
    let (first, second) = (
        from.samples(ALIGN_SAMPLES, ALIGN_SAMPLES),
        to.samples(ALIGN_SAMPLES, ALIGN_SAMPLES),
    );
    let mut best = (f64::INFINITY, 0);
    for shift in 0..ALIGN_SAMPLES {
        let nearness = cost(&first, second.iter().cycle().skip(shift as usize));
        if nearness < best.0 {
            best = (nearness, shift);
        }
    }
    f64::from(best.1) / f64::from(ALIGN_SAMPLES)
}

/// Whether an open `to` lies nearer `from` run the other way round.
fn is_nearer_reversed(from: &Contour, to: &Contour) -> bool {
    if from.is_point() || to.is_point() {
        return false;
    }
    let (first, second) = (
        from.samples(ALIGN_SAMPLES, ALIGN_SAMPLES - 1),
        to.samples(ALIGN_SAMPLES, ALIGN_SAMPLES - 1),
    );
    cost(&first, second.iter().rev()) < cost(&first, second.iter())
}

/// The point pairs of two contours of one role: every vertex of either, with the point of the other at the same share
/// of the way round (`to` entered `start` of its length late), in order from the start of `from`.
fn matched(from: &Contour, to: &Contour) -> Vec<(Point, Point)> {
    let reversed;
    let (to, start) = if from.closed {
        (to, closed_start(from, to))
    } else if is_nearer_reversed(from, to) {
        reversed = Contour::new(to.points.iter().rev().copied().collect(), false, to.drawn);
        (&reversed, 0.0)
    } else {
        (to, 0.0)
    };
    // an open pair has no start to turn: its ends are at 0 and 1
    let turned = |share: f64| if from.closed { wrapped(share) } else { share };
    // (share along `from`, share along `to`) of every vertex of either
    let mut shares: Vec<(f64, f64)> = Vec::with_capacity(from.at.len() + to.at.len());
    shares.extend(from.at.iter().map(|&share| (share, turned(share + start))));
    shares.extend(to.at.iter().map(|&share| (turned(share - start), share)));
    shares.sort_by(|a, b| a.0.total_cmp(&b.0));
    shares.dedup_by(|later, earlier| later.0.total_cmp(&earlier.0).is_eq());
    shares
        .into_iter()
        .map(|(along, over)| (from.point_at(along), to.point_at(over)))
        .collect()
}

/// A path's contours, and the box they fill together.
struct Side {
    contours: Vec<Contour>,
    /// The least and the greatest of each coordinate; none when there is no point.
    bounds: Option<(Point, Point)>,
}

impl Side {
    /// `path` flattened, whole-reversed if its largest contour runs the wrong way for an outer.
    fn new(path: &Path, charge: &mut dyn FnMut(usize) -> Result<(), Error>) -> Result<Self, Error> {
        let mut contours: Vec<Contour> = path.flattened(charge)?.into_iter().map(Contour::of).collect();
        let largest = contours
            .iter()
            .filter(|contour| contour.closed)
            .fold(0.0_f64, |most, contour| {
                if contour.area.abs() > most.abs() {
                    contour.area
                } else {
                    most
                }
            });
        if largest < 0.0 {
            contours = contours
                .into_iter()
                .map(|contour| if contour.closed { contour.reversed() } else { contour })
                .collect();
        }
        let mut points = contours.iter().flat_map(|contour| &contour.points);
        let bounds = points.next().map(|&first| {
            points.fold((first, first), |(low, high), p| {
                (
                    Point::new(low.x.min(p.x), low.y.min(p.y)),
                    Point::new(high.x.max(p.x), high.y.max(p.y)),
                )
            })
        });
        Ok(Self { contours, bounds })
    }

    /// Where the centre of `contour` lies in this path's box, from 0 to 1 each way (a box with no width counts as 1).
    fn place(&self, contour: &Contour) -> (f64, f64) {
        let Some((low, high)) = self.bounds else {
            return (0.0, 0.0);
        };
        let (centre, span) = (contour.centre(), |size: f64| if size > 0.0 { size } else { 1.0 });
        (
            (centre.x - low.x) / span(high.x - low.x),
            (centre.y - low.y) / span(high.y - low.y),
        )
    }

    /// The point at `place` of this path's box (the contour's own centre for a path with no points).
    fn at(&self, place: (f64, f64), own: Point) -> Point {
        self.bounds.map_or(own, |(low, high)| {
            Point::new(low.x + place.0 * (high.x - low.x), low.y + place.1 * (high.y - low.y))
        })
    }
}

/// What becomes of a contour on the way.
enum Plan<'a> {
    /// Carried to the contour of the other path.
    Matched(&'a Contour, &'a Contour),
    /// Shrinks to a point.
    Leaving(&'a Contour, Point),
    /// Grows from a point.
    Arriving(&'a Contour, Point),
}

impl Plan<'_> {
    /// The most points the result contour has.
    fn points(&self) -> usize {
        match self {
            Self::Matched(from, to) => from.points.len() + to.points.len(),
            Self::Leaving(contour, _) | Self::Arriving(contour, _) => contour.points.len(),
        }
    }

    fn is_unmatched_hole(&self) -> bool {
        match self {
            Self::Matched(..) => false,
            Self::Leaving(contour, _) | Self::Arriving(contour, _) => contour.role() == Role::Hole,
        }
    }

    /// The contour's points at the start and at the end.
    fn pairs(&self) -> Vec<(Point, Point)> {
        match *self {
            Self::Matched(from, to) => matched(from, to),
            Self::Leaving(contour, point) => contour.points.iter().map(|&p| (p, point)).collect(),
            Self::Arriving(contour, point) => contour.points.iter().map(|&p| (point, p)).collect(),
        }
    }

    /// The sub-path `fraction` of the way.
    fn sub_path(&self, fraction: f64) -> Option<SubPath> {
        let (closed, drawn) = match *self {
            Self::Matched(from, to) => (from.closed, from.drawn || to.drawn),
            Self::Leaving(contour, _) | Self::Arriving(contour, _) => (contour.closed, contour.drawn),
        };
        // a contour with no partner is its own size times a share that stops at 0: it never comes back through its point
        let fraction = match self {
            Self::Matched(..) => fraction,
            Self::Leaving(..) => fraction.min(1.0),
            Self::Arriving(..) => fraction.max(0.0),
        };
        let points: Vec<Point> = self
            .pairs()
            .into_iter()
            .map(|(a, b)| Point::new(lerp_number(a.x, b.x, fraction), lerp_number(a.y, b.y, fraction)))
            .collect();
        let (&start, rest) = points.split_first()?;
        let mut segments: Vec<Segment> = rest.iter().map(|&point| Segment::Line(point)).collect();
        if drawn && segments.is_empty() {
            // a line to its own start still marks a place
            segments.push(Segment::Line(start));
        }
        let sub = SubPath {
            start,
            segments,
            closed: false,
        };
        Some(if closed { sub.closed() } else { sub })
    }
}

/// What becomes of every contour of both sides, charging the pairs it ranks before it ranks them.
fn plan<'a>(
    from: &'a Side,
    to: &'a Side,
    charge: &mut dyn FnMut(usize) -> Result<(), Error>,
) -> Result<Vec<Plan<'a>>, Error> {
    let mut plans = Vec::new();
    for role in [Role::Outer, Role::Hole, Role::Open] {
        let entries = |side: &'a Side| -> Vec<(&'a Contour, (f64, f64))> {
            (side.contours.iter())
                .filter(|contour| contour.role() == role)
                .map(|contour| (contour, side.place(contour)))
                .collect()
        };
        let (here, there) = (entries(from), entries(to));
        charge(here.len().saturating_mul(there.len()))?;
        // nearest first; equal distances go to the lower indices
        let mut candidates: Vec<(f64, usize, usize)> = Vec::with_capacity(here.len() * there.len());
        for (i, &(_, a)) in here.iter().enumerate() {
            for (j, &(_, b)) in there.iter().enumerate() {
                candidates.push((libm::hypot(a.0 - b.0, a.1 - b.1), i, j));
            }
        }
        candidates.sort_by(|x, y| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)).then(x.2.cmp(&y.2)));
        let (mut used_from, mut used_to) = (BTreeSet::new(), BTreeSet::new());
        let mut pairs = Vec::new();
        for &(_, i, j) in &candidates {
            if !used_from.contains(&i) && !used_to.contains(&j) {
                used_from.insert(i);
                used_to.insert(j);
                pairs.push((i, j));
            }
        }
        pairs.sort_unstable();
        for (i, j) in pairs {
            if let (Some(&(a, _)), Some(&(b, _))) = (here.get(i), there.get(j)) {
                plans.push(Plan::Matched(a, b));
            }
        }
        for (i, &(contour, place)) in here.iter().enumerate() {
            if !used_from.contains(&i) {
                plans.push(Plan::Leaving(contour, to.at(place, contour.centre())));
            }
        }
        for (j, &(contour, place)) in there.iter().enumerate() {
            if !used_to.contains(&j) {
                plans.push(Plan::Arriving(contour, from.at(place, contour.centre())));
            }
        }
    }
    Ok(plans)
}
