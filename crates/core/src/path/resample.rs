//! Resampling: a path replaced by straight segments whose corners are a fixed distance apart along it.

use super::walk::Run;
use super::{Path, Segment, SubPath};
use crate::Error;
use crate::geometry::coordinate;
use crate::limits::MAX_SEGMENTS;

impl Path {
    /// The path as straight segments with corners `spacing` design pixels apart along it. Each sub-path is measured
    /// as [`Path::length`] measures it, from its own start (a closed one from its written start), with corners at 0,
    /// `spacing`, 2 `spacing`, ... along it, every one on the path. An open sub-path keeps its end, so its last gap
    /// is at most `spacing`; a closed one stays closed and its last gap wraps to the start, so a `spacing` past its
    /// perimeter leaves its start alone. A sub-path with no length stays a lone point; an empty path stays empty.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `spacing` if it is not a number above 0 and at most 1e100, or if the corners it
    /// would make, over all the sub-paths, are more than `MAX_SEGMENTS` (10,000,000); nothing is allocated for them.
    pub fn resample(&self, spacing: f64) -> Result<Self, Error> {
        self.resample_charged(spacing, &mut |count| {
            if count > MAX_SEGMENTS {
                Err(Error::parameter(
                    "spacing",
                    format!("MAX_SEGMENTS {MAX_SEGMENTS} passed"),
                ))
            } else {
                Ok(())
            }
        })
    }

    /// As [`Path::resample`], calling `charge` once with the number of corners the result would have, before any
    /// is made; its error stops the verb.
    pub(crate) fn resample_charged(
        &self,
        spacing: f64,
        charge: &mut dyn FnMut(usize) -> Result<(), Error>,
    ) -> Result<Self, Error> {
        if spacing <= 0.0 {
            return Err(Error::parameter(
                "spacing",
                format!("must be a number above 0, got {spacing}"),
            ));
        }
        let spacing = coordinate("spacing", spacing)?;
        let runs: Vec<Run<'_>> = self
            .subpaths
            .iter()
            // a closed sub-path is walked from its written start, so that resampling keeps it
            .map(|subpath| Run::new(subpath, 0.0, Some(subpath.start)))
            .collect();
        // the most corners of each run: those at 0, s, 2s, ... below its length, and the end
        let wanted: f64 = runs.iter().map(|run| (run.length / spacing).ceil() + 1.0).sum();
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
            charge(wanted as usize)?;
        }
        Ok(Self {
            subpaths: runs.iter().map(|run| resampled(run, spacing)).collect(),
        })
    }
}

/// The run as straight segments `spacing` apart.
fn resampled(run: &Run<'_>, spacing: f64) -> SubPath {
    let subpath = run.subpath;
    if run.length <= 0.0 {
        return SubPath::open(subpath.start);
    }
    let mut corners = Vec::new();
    let mut index = 0.0_f64;
    while index * spacing < run.length {
        corners.push(run.at(index * spacing));
        index += 1.0;
    }
    if !subpath.closed {
        corners.push(run.at(run.length));
    }
    let mut corners = corners.into_iter();
    let start = corners.next().unwrap_or(subpath.start);
    let resampled = SubPath {
        start,
        segments: corners.map(Segment::Line).collect(),
        closed: false,
    };
    if subpath.closed { resampled.closed() } else { resampled }
}
