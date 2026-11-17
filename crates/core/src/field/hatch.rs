//! `Hatch`: evenly spaced streamlines (Jobard and Lefer, 1997) through a [`Flow`] inside the region a path fills.
//!
//! A line is traced from a seed in both directions, stepping along the flow, and stops where it leaves the region,
//! enters what is avoided, or comes closer than half the spacing to a sample of another line. The next seeds stand one
//! spacing to either side of every sample, so lines settle one spacing apart where the flow is straight and split
//! or end where it spreads or gathers. A lattice of one narrowest spacing finds a start for every part the region has.

#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "the lattice count is at most the 4 million cells checked before it is used"
)]

use super::outline::Outline;
use super::seeds::{Sample, Samples};
use super::{Field, Flow, length, segment_budget};
use crate::geometry::Point;
use crate::path::Path;
use crate::{Error, Expr, Values};
use std::collections::VecDeque;

/// How far a line steps, as a share of the spacing where it is.
// constant: a quarter spacing keeps a line smooth to the eye at any spacing and makes a sample every few pixels,
// which the half-spacing floor below needs to be measured against
const STEP: f64 = 0.25;

/// Where a line stops short of another: this share of the spacing at the point.
// constant: Jobard and Lefer's d_test; half the spacing leaves room to seed between and stops lines from merging
const TEST: f64 = 0.5;

/// How clear of every sample a seed must stand, as a share of the spacing: a seed one spacing from its line passes,
/// rounding apart.
// constant: one percent under 1, so that a seed placed exactly one spacing away is not refused by rounding
const CLEAR: f64 = 0.99;

/// A line ignores its own samples within this many spacings along itself, so that a bend does not stop it.
// constant: two spacings is the length of a turn tight enough that the line would meet its own neighbour
const OWN_GRACE: f64 = 2.0;

/// A line closes on its start only after this many spacings, so that it cannot close at once.
// constant: a loop shorter than four spacings is a bend, not a loop
const CLOSE_AFTER: f64 = 4.0;

/// A line shorter than this many spacings is dropped.
// constant: a stub shorter than a spacing is a fragment of an edge, not a line
const SHORTEST: f64 = 1.0;

/// The most cells the lattice and the grid of samples may have.
// constant: 4 million cells of the grid are 16 MB of heads; a shape that needs more would draw more lines than
// `MAX_SEGMENTS` allows anyway
const MAX_CELLS: f64 = 4_000_000.0;

/// The most cell visits the lattice may cost: each of its points looks at the cells within the widest spacing, at most
/// the whole grid.
// constant: a billion visits of a few instructions is a few seconds; a pair of spacings far apart over a large shape
// would cost minutes
const MAX_VISITS: f64 = 1e9;

/// What to draw: the spacing, a tone that varies it, and the flow the lines follow.
#[derive(Debug, Clone)]
pub struct Hatch {
    at_zero: f64,
    at_one: f64,
    tone: Option<Field>,
    flow: Flow,
    below: Option<Expr>,
}

impl Hatch {
    /// Lines `spacing` pixels apart, along a flow of 45 degrees.
    #[must_use]
    pub fn new(spacing: f64) -> Self {
        Self {
            at_zero: spacing,
            at_one: spacing,
            tone: None,
            flow: Flow::fixed(45.0),
            below: None,
        }
    }

    /// Lines whose spacing is `at_tone_0` where `tone` is 0 and `at_tone_1` where it is 1, linear between and held
    /// beyond, along a flow of 45 degrees.
    #[must_use]
    pub fn between(at_tone_0: f64, at_tone_1: f64, tone: Field) -> Self {
        Self {
            at_zero: at_tone_0,
            at_one: at_tone_1,
            tone: Some(tone),
            flow: Flow::fixed(45.0),
            below: None,
        }
    }

    /// The hatch a document's values make: `spacing` is `(at tone 0, at tone 1)`, which differ only where there is a
    /// `tone` to read.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `tone` if the two spacings differ and there is no tone.
    pub(crate) fn checked(spacing: (f64, f64), tone: Option<Field>) -> Result<Self, Error> {
        match tone {
            Some(tone) => Ok(Self::between(spacing.0, spacing.1, tone)),
            None if spacing.0.to_bits() == spacing.1.to_bits() => Ok(Self::new(spacing.0)),
            None => Err(Error::parameter("tone", "is needed when `spacing` is a pair")),
        }
    }

    /// The lines follow `flow`.
    #[must_use]
    pub fn flow(mut self, flow: Flow) -> Self {
        self.flow = flow;
        self
    }

    /// Lines only where the tone is at most `level`: a line stops where the tone passes it (within a step, a quarter
    /// of the spacing there) and no line starts above it. Needs a tone; with tone 0 dark, a hatch pass for the
    /// darker half of a picture is `below(0.5)`, and passes at other angles and levels over each other crosshatch.
    #[must_use]
    pub fn below(mut self, level: impl Into<Expr>) -> Self {
        self.below = Some(level.into());
        self
    }

    pub(crate) fn below_of(&self) -> Option<&Expr> {
        self.below.as_ref()
    }

    pub(crate) fn spacings(&self) -> (f64, f64) {
        (self.at_zero, self.at_one)
    }

    pub(crate) fn tone(&self) -> Option<&Field> {
        self.tone.as_ref()
    }

    pub(crate) fn flow_of(&self) -> &Flow {
        &self.flow
    }
}

impl Path {
    /// Evenly spaced lines along `hatch`'s flow inside the region this path fills (nonzero rule), never entering the
    /// region `avoid` fills. The result is open lines, and closed ones where a line comes round to its own start, in the
    /// order they were made; no two lines come within half a spacing of each other. An empty path gives an empty path.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `spacing` if a spacing is not above 0, or if the shape is so large next to the
    /// spacing that it would take over 4 million cells, or if the spacings are so far apart for the shape's size that
    /// placing the lines would take over a billion cell visits (a pair 1 to 1000 over a 400 by 400 shape); naming `spacing` past `MAX_SEGMENTS` points.
    pub fn hatch(&self, hatch: &Hatch, avoid: Option<&Path>) -> Result<Path, Error> {
        let avoid: Vec<&Path> = avoid.into_iter().collect();
        self.hatch_charged(hatch, &avoid, &mut segment_budget("spacing"))
    }

    /// As [`Path::hatch`], calling `charge` with the points made, before they are kept; its error stops the verb.
    pub(crate) fn hatch_charged(
        &self,
        hatch: &Hatch,
        avoid: &[&Path],
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<Path, Error> {
        let (near, far) = hatch.spacings();
        for spacing in [near, far] {
            if !(spacing.is_finite() && spacing > 0.0 && spacing <= crate::geometry::MAX_COORDINATE) {
                return Err(Error::parameter("spacing", format!("must be above 0, got {spacing}")));
            }
        }
        // a Param is read at its default here; the step reads it at the values it draws with
        let below = hatch
            .below
            .as_ref()
            .map(|level| {
                let level = level
                    .value(&Values::new())
                    .map_err(|error| Error::parameter("below", error.to_string()))?;
                if !level.is_finite() {
                    return Err(Error::parameter(
                        "below",
                        format!("must be a finite number, got {level}"),
                    ));
                }
                if hatch.tone.is_none() {
                    return Err(Error::parameter("below", "needs a `tone` to read"));
                }
                Ok(level)
            })
            .transpose()?;
        let Ok(region) = Outline::of(self, "hatch", charge) else {
            return Ok(Path::new());
        };
        let blocks = avoid
            .iter()
            .filter_map(|path| Outline::of(path, "hatch", charge).ok())
            .collect::<Vec<_>>();
        let (left, top, right, bottom) = region.extent();
        let narrowest = near.min(far);
        if Samples::cells(right - left, bottom - top, narrowest) > MAX_CELLS {
            return Err(Error::parameter(
                "spacing",
                format!("is too small for a shape this large: its grid would pass {MAX_CELLS} cells"),
            ));
        }
        // a step smaller than the gap between coordinates here would not move a line, or a lattice point
        let reach = left.abs().max(right.abs()).max(top.abs()).max(bottom.abs());
        if reach + STEP * narrowest <= reach {
            return Err(Error::parameter(
                "spacing",
                format!("is too small for coordinates near {reach}: a step of it would not move"),
            ));
        }
        let widest = near.max(far);
        let cells = Samples::cells(right - left, bottom - top, narrowest);
        if cells * cells.min((widest / narrowest) * (widest / narrowest)) > MAX_VISITS {
            return Err(Error::parameter(
                "spacing",
                format!(
                    "the widest and narrowest spacing are too far apart for a shape this large: {narrowest} to {widest}"
                ),
            ));
        }
        let mut run = Run {
            hatch,
            region,
            blocks,
            below,
            samples: Samples::new(left, top, right - left, bottom - top, narrowest),
            lines: Vec::new(),
        };
        run.fill((left, top, right, bottom), narrowest, charge)?;
        let paths = run
            .lines
            .iter()
            .filter(|line| line.kept)
            .map(|line| Path::polyline(line.points.iter().copied(), line.closed))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Path::joined(paths.iter()))
    }
}

/// One line.
struct Traced {
    points: Vec<Point>,
    closed: bool,
    kept: bool,
}

struct Run<'a> {
    hatch: &'a Hatch,
    region: Outline,
    blocks: Vec<Outline>,
    below: Option<f64>,
    samples: Samples,
    lines: Vec<Traced>,
}

impl Run<'_> {
    /// The spacing at `at`.
    fn spacing(&self, at: Point) -> f64 {
        let (zero, one) = self.hatch.spacings();
        match self.hatch.tone() {
            Some(tone) => zero + (one - zero) * tone.at(at).clamp(0.0, 1.0),
            None => zero,
        }
    }

    fn usable(&self, at: Point) -> bool {
        self.region.contains(at)
            && !self.blocks.iter().any(|block| block.contains(at))
            && match (self.below, self.hatch.tone()) {
                (Some(level), Some(tone)) => tone.at(at) <= level,
                _ => true,
            }
    }

    /// Whether a line may start at `at`: usable and clear of every kept sample by `gap`, the spacing of what it is
    /// seeded from.
    fn free(&self, at: Point, gap: f64) -> bool {
        self.usable(at)
            && !self
                .samples
                .any_within(at, CLEAR * gap, |sample| self.lines[sample.line as usize].kept)
    }

    fn fill(
        &mut self,
        (left, top, right, bottom): (f64, f64, f64, f64),
        lattice: f64,
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let mut queue: VecDeque<usize> = VecDeque::new();
        // by index, so that a point never depends on the sum of the ones before; the count is the lattice's cells
        let count = |extent: f64| (extent / lattice).ceil() as usize;
        for row in 0..count(bottom - top) {
            let y = top + lattice / 2.0 + row as f64 * lattice;
            for column in 0..count(right - left) {
                let start = Point::new(left + lattice / 2.0 + column as f64 * lattice, y);
                if self.free(start, self.spacing(start))
                    && let Some(line) = self.trace(start, charge)?
                {
                    queue.push_back(line);
                    while let Some(done) = queue.pop_front() {
                        self.seed_from(done, &mut queue, charge)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Starts a line on each side of every sample of line `done`, where there is room.
    fn seed_from(
        &mut self,
        done: usize,
        queue: &mut VecDeque<usize>,
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let points = self.lines[done].points.clone();
        for at in points {
            let (dx, dy) = self.hatch.flow_of().direction(at);
            let gap = self.spacing(at);
            for side in [1.0, -1.0] {
                let seed = Point::new(at.x - dy * gap * side, at.y + dx * gap * side);
                if self.free(seed, gap)
                    && let Some(line) = self.trace(seed, charge)?
                {
                    queue.push_back(line);
                }
            }
        }
        Ok(())
    }

    /// The line through `seed`, kept unless it is a stub; its index.
    fn trace(
        &mut self,
        seed: Point,
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<Option<usize>, Error> {
        let id = self.lines.len();
        self.lines.push(Traced {
            points: Vec::new(),
            closed: false,
            kept: true,
        });
        charge(1)?;
        self.samples.add(Sample {
            at: seed,
            line: u32::try_from(id).unwrap_or(u32::MAX),
            along: 0.0,
        });
        let heading = self.hatch.flow_of().direction(seed);
        let (forward, closed) = self.walk(id, seed, heading, 1.0, charge)?;
        let backward = if closed {
            Vec::new()
        } else {
            self.walk(id, seed, (-heading.0, -heading.1), -1.0, charge)?.0
        };
        let mut points: Vec<Point> = backward.into_iter().rev().collect();
        points.push(seed);
        points.extend(forward);
        let length: f64 = points
            .windows(2)
            .map(|pair| length(pair[1].x - pair[0].x, pair[1].y - pair[0].y))
            .sum();
        let kept = closed || (points.len() > 1 && length >= SHORTEST * self.spacing(seed));
        let line = &mut self.lines[id];
        line.closed = closed;
        line.kept = kept;
        line.points = points;
        Ok(self.lines[id].kept.then_some(id))
    }

    /// Steps from `from` along the flow, `sign` 1 forwards and -1 backwards along the line, until it must stop; the
    /// points made, and whether it came round to `from` (only forwards).
    fn walk(
        &mut self,
        id: usize,
        from: Point,
        heading: (f64, f64),
        sign: f64,
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<(Vec<Point>, bool), Error> {
        let flow = self.hatch.flow_of();
        let (mut points, mut at, mut heading, mut along) = (Vec::new(), from, heading, 0.0);
        loop {
            let step = STEP * self.spacing(at);
            // the midpoint rule: one look half a step ahead, for a path that bends the way the flow does
            let first = aligned(flow.direction(at), heading);
            let middle = Point::new(at.x + first.0 * step / 2.0, at.y + first.1 * step / 2.0);
            let turn = aligned(flow.direction(middle), first);
            let next = Point::new(at.x + turn.0 * step, at.y + turn.1 * step);
            let next_along = along + sign * step;
            if !self.usable(next) {
                return Ok((points, false));
            }
            let gap = self.spacing(next);
            if sign > 0.0 && next_along > CLOSE_AFTER * gap && length(next.x - from.x, next.y - from.y) < TEST * gap {
                return Ok((points, true));
            }
            let own = u32::try_from(id).unwrap_or(u32::MAX);
            let blocked = self.samples.any_within(next, TEST * gap, |sample| {
                self.lines[sample.line as usize].kept
                    && !(sample.line == own && (sample.along - next_along).abs() < OWN_GRACE * gap)
            });
            if blocked {
                return Ok((points, false));
            }
            charge(1)?;
            self.samples.add(Sample {
                at: next,
                line: own,
                along: next_along,
            });
            points.push(next);
            (at, heading, along) = (next, turn, next_along);
        }
    }
}

/// `direction` with the sign that agrees with `heading`: a flow has no sign, a line keeps one way.
fn aligned(direction: (f64, f64), heading: (f64, f64)) -> (f64, f64) {
    if direction.0 * heading.0 + direction.1 * heading.1 < 0.0 {
        (-direction.0, -direction.1)
    } else {
        direction
    }
}
