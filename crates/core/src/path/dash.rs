//! Dashing: a path cut into the pieces a pattern of "on" and "off" lengths leaves.

use super::walk::Run;
use super::{Path, SubPath};
use crate::Error;
use crate::geometry::{Point, coordinate, coordinate_point, finite};
use crate::limits::{MAX_DASH_PATTERN, MAX_SEGMENTS};

impl Path {
    /// The pieces of the path a dash `pattern` leaves: `[on, off, on, off, ...]`, lengths in design pixels. A pattern
    /// of odd length is repeated to make an even one, as SVG's `stroke-dasharray` does (`[a]` is `[a, a]`, `[a, b, c]`
    /// is `[a, b, c, a, b, c]`). Each sub-path starts the pattern again at its own start (a closed one at its topmost
    /// point, or the point nearest `near`, as for [`Path::slice`]), and the pattern runs on until the sub-path ends.
    ///
    /// `phase` is in cycles of the whole (even) pattern and wraps: it moves the dashes forward along the path, so at
    /// `phase` 0.25 the first dash starts a quarter of a cycle from the start, and `phase` 1 is `phase` 0. Negative
    /// phases move them back.
    ///
    /// The pieces are open sub-paths whose curves stay curves, cut where the pattern falls, as [`Path::slice`] cuts.
    /// The one exception is a closed sub-path that lies wholly inside one dash: it stays closed. A closed sub-path's
    /// last dash, when it runs to its end, and its first, when it starts at its start, are two pieces, not one
    /// (a known limit). A sub-path with no length stays a lone point; a sub-path every dash misses leaves nothing, so
    /// a path wholly in the gaps is empty; an empty path stays empty.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `pattern` if it is empty, has more than 1,000 entries, or an entry is not a number
    /// above 0 and at most 1e100; naming `phase` if it is NaN or infinite; naming `near` if it is beyond ±1e100; or
    /// naming `pattern` if a bound on the pieces, over all the sub-paths (cycles crossed + 1, times
    /// dashes per cycle), is more than `MAX_SEGMENTS` (10,000,000), which is checked before any is made.
    pub fn dash(&self, pattern: &[f64], phase: f64, near: Option<Point>) -> Result<Self, Error> {
        self.dash_charged(pattern, phase, near, &mut |count| {
            if count > MAX_SEGMENTS {
                Err(Error::parameter(
                    "pattern",
                    format!("MAX_SEGMENTS {MAX_SEGMENTS} passed"),
                ))
            } else {
                Ok(())
            }
        })
    }

    /// As [`Path::dash`], calling `charge` once with the most segments the result and its walk may need, before any
    /// piece is made; its error stops the verb.
    pub(crate) fn dash_charged(
        &self,
        pattern: &[f64],
        phase: f64,
        near: Option<Point>,
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<Self, Error> {
        if pattern.is_empty() || pattern.len() > MAX_DASH_PATTERN {
            return Err(Error::parameter(
                "pattern",
                format!("must have from 1 to {MAX_DASH_PATTERN} lengths, got {}", pattern.len()),
            ));
        }
        for &length in pattern {
            coordinate("pattern", length)?;
            if length <= 0.0 {
                return Err(Error::parameter(
                    "pattern",
                    format!("every length must be above 0, got {length}"),
                ));
            }
        }
        let phase = finite("phase", phase)?;
        let near = near.map(|near| coordinate_point("near", near)).transpose()?;
        let pattern = Pattern::new(pattern, phase);
        let runs: Vec<Run<'_>> = self
            .subpaths
            .iter()
            .map(|subpath| Run::new(subpath, 0.0, near))
            .collect();
        // the most pieces of each run: the dashes of the cycles it crosses (one more for the part cycle at each end)
        #[expect(clippy::cast_precision_loss, reason = "at most 2,000")]
        let dashes = (pattern.entries() / 2) as f64;
        let wanted: f64 = runs
            .iter()
            .map(|run| ((run.length / pattern.cycle).ceil() + 1.0) * dashes + 1.0)
            .sum();
        #[expect(clippy::cast_precision_loss, reason = "10 million is exact in f64")]
        let past = wanted > MAX_SEGMENTS as f64;
        if past {
            // the exact count is not needed to refuse it; any count past the limit is refused by the charge
            charge(MAX_SEGMENTS + 1)?;
        } else {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "0 to MAX_SEGMENTS"
            )]
            charge(wanted as usize + self.weight())?;
        }
        let mut subpaths = Vec::new();
        for run in &runs {
            pattern.cut(run, &mut subpaths);
        }
        Ok(Self { subpaths })
    }
}

/// A dash pattern made even, with the distance into its cycle at which a path starts.
struct Pattern<'a> {
    lengths: &'a [f64],
    /// The entries of the even pattern: those of `lengths`, twice if there is an odd number.
    entries: usize,
    /// The length of one cycle of the even pattern.
    cycle: f64,
    /// Where in the cycle the path's start falls.
    start: f64,
}

impl<'a> Pattern<'a> {
    fn new(lengths: &'a [f64], phase: f64) -> Self {
        let entries = lengths.len() * (1 + lengths.len() % 2);
        let repeats = if lengths.len() % 2 == 1 { 2.0 } else { 1.0 };
        let cycle = lengths.iter().sum::<f64>() * repeats;
        // dashes move forward with the phase, so the start of the path is the earlier in the cycle
        let wrapped = (-phase).rem_euclid(1.0) * cycle;
        // rem_euclid can round up to a whole cycle
        let start = if wrapped < cycle { wrapped } else { 0.0 };
        Self {
            lengths,
            entries,
            cycle,
            start,
        }
    }

    fn entries(&self) -> usize {
        self.entries
    }

    fn length(&self, entry: usize) -> f64 {
        self.lengths[entry % self.lengths.len()]
    }

    /// The pieces of `run` the pattern leaves, pushed in order.
    fn cut(&self, run: &Run<'_>, into: &mut Vec<SubPath>) {
        if run.length <= 0.0 {
            into.push(SubPath::open(run.subpath.start));
            return;
        }
        // the entry the path starts in, and what is left of it
        let (mut entry, mut left) = (0, self.start);
        while entry + 1 < self.entries && left >= self.length(entry) {
            left -= self.length(entry);
            entry += 1;
        }
        let mut remaining = self.length(entry) - left;
        let mut at = 0.0_f64;
        while at < run.length {
            let end = (at + remaining).min(run.length);
            // even entries are dashes, odd ones gaps
            if entry % 2 == 0 && end > at {
                let piece = run.piece(at, end);
                into.push(if at <= 0.0 && end >= run.length && run.subpath.closed {
                    piece.closed()
                } else {
                    piece
                });
            }
            at += remaining;
            entry = (entry + 1) % self.entries;
            remaining = self.length(entry);
        }
    }
}
