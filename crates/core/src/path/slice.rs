//! Cutting a path along its length and turning it round.

use super::{Path, SubPath};
use crate::Error;
use crate::geometry::{Point, fraction};

impl Path {
    /// The piece of the outline between the positions `start` and `end` (fractions 0 to 1 of the length, as for
    /// [`Path::point_at`]); `slice(0, progress)` draws the outline on. Curves stay curves, cut where the positions
    /// fall. A closed sub-path that lies wholly inside the piece stays closed, so the whole of a closed outline is
    /// closed again; any other piece is open. Sub-paths the positions pass between are left out, and a sub-path
    /// with no length is left out too. A piece with `start` equal to `end` is one lone start at that point. `near`
    /// is as for [`Path::point_at`].
    ///
    /// # Errors
    /// [`Error::Parameter`] if `start` or `end` is outside 0 to 1 or NaN, if `start` is after `end` (to travel the
    /// other way, [`Path::reverse`] the result), or if `near` is NaN, infinite or beyond ±1e100; [`Error::Geometry`] if the path is
    /// empty.
    pub fn slice(&self, start: f64, end: f64, near: Option<Point>) -> Result<Self, Error> {
        let start = fraction("start", start)?;
        let end = fraction("end", end)?;
        if start > end {
            return Err(Error::parameter(
                "start",
                format!(
                    "must not be after end: got start {start} and end {end} (reverse the path to run it backwards)"
                ),
            ));
        }
        let walk = self.nonempty_walk("slice", near)?;
        let (from, to) = (start * walk.length, end * walk.length);
        let mut subpaths = Vec::new();
        for run in &walk.runs {
            let run_end = run.offset + run.length;
            if from >= run_end {
                continue;
            }
            let lo = if from <= run.offset { 0.0 } else { from - run.offset };
            let hi = if to >= run_end { run.length } else { to - run.offset };
            if hi <= lo {
                continue;
            }
            let piece = run.piece(lo, hi);
            let whole = lo <= 0.0 && hi >= run.length;
            subpaths.push(if whole && run.subpath.closed {
                piece.closed()
            } else {
                piece
            });
        }
        if subpaths.is_empty() {
            // no length between the positions: the point they share
            let point = walk
                .spot(from)
                .map_or(walk.runs[0].subpath.start, |spot| walk.point(spot));
            subpaths.push(SubPath::open(point));
        }
        Ok(Self { subpaths })
    }

    /// The same outline travelled the other way: the sub-paths in the opposite order, each one reversed. A closed
    /// sub-path keeps its written start and runs back round from it. Reversing twice gives this path again, except
    /// that a closed sub-path loses the straight segments of no length that begin it. A position `t` along this path
    /// is the position `1 - t` along the reversed one, except exactly on a boundary between sub-paths, which belongs
    /// to the later one in either direction.
    #[must_use]
    pub fn reverse(&self) -> Self {
        Self {
            subpaths: self.subpaths.iter().rev().map(SubPath::reversed).collect(),
        }
    }
}

impl SubPath {
    fn reversed(&self) -> Self {
        let mut curves = self.curves();
        if self.closed && curves.last().is_some_and(|closing| closing.start() == closing.end()) {
            curves.pop();
        }
        // the last curve ends where the first of the reversed ones starts: at the written start of a closed ring
        let start = curves.last().map_or(self.start, |last| last.end());
        let reversed = Self {
            start,
            segments: curves
                .into_iter()
                .rev()
                .map(|curve| curve.reversed().segment())
                .collect(),
            closed: false,
        };
        if self.closed { reversed.closed() } else { reversed }
    }
}
