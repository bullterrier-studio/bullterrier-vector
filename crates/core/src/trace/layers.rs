//! `trace_levels` and `trace_labels`: the closed outlines of the regions of a map where it is at or above a level, or
//! where it is one label, found by marching squares over the map's pixel centres.
//!
//! The lattice of readings is the map's pixels padded with rings of the nearest edge value, and one outer ring of a
//! value far below every level. A shape that reaches the map's border therefore goes on past it and closes outside the
//! frame, and no edge of any shape lies along the border: a viewer that crops at the frame sees none. Closing and
//! opening shrink an edge by their radius, so the padding is as wide as the largest of them (and the curve tolerance)
//! plus two pixels.

#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "the lattice has at most MAX_NODES nodes, checked before any index is made"
)]

use super::Plan;
use super::tidy::tidy;
use crate::Error;
use crate::field::{Grid, MAX_NODES, MAX_READ_WORK, MAX_SMOOTH_WORK, MapData, blur, trace_level};
use crate::geometry::Rect;
use crate::path::Path;

/// The most levels one call traces.
// constant: each level is a pass over the whole lattice, and no picture reads more than a few dozen tones
pub(crate) const MAX_LEVELS: usize = 256;

/// The reading of the outermost ring of nodes: the lines close just inside it.
// constant: far below any level a map's 0 to 1 tones are asked for, so the line between the ring before it and this one
// lies at the ring before it, whatever that reading is
const OUTSIDE: f64 = -1e9;

/// Levels are compared with readings of 0 to 1, so one outside the range is the same as the nearest end of it.
// constant: a map's tones lie from 0 to 1; 2 is above every blurred reading
const LEVEL_RANGE: (f64, f64) = (0.0, 2.0);

/// The lattice of a map: its size, the padding and the passes it will be traced in.
struct Lattice {
    width: usize,
    height: usize,
    pad: usize,
    frame: Rect,
    pixel: (f64, f64),
}

impl Lattice {
    fn new(map: &MapData, plan: &Plan, passes: usize, passes_name: &str) -> Result<Self, Error> {
        let pixel = (map.frame.width / map.width as f64, map.frame.height / map.height as f64);
        let margin = plan.open.max(plan.close) + plan.tolerance;
        let (wide, name) = if plan.open >= plan.close {
            (plan.open, "open")
        } else {
            (plan.close, "close")
        };
        let rings = 2.0 + (margin / pixel.0.min(pixel.1)).ceil();
        let (nodes_wide, nodes_high) = (map.width as f64 + 2.0 * rings, map.height as f64 + 2.0 * rings);
        if nodes_wide * nodes_high > MAX_NODES {
            return Err(Error::parameter(
                if wide > 0.0 { name } else { "map" },
                format!("the map with its padding of {rings} pixels would pass {MAX_NODES} nodes"),
            ));
        }
        if nodes_wide * nodes_high * passes as f64 > MAX_READ_WORK {
            return Err(Error::parameter(
                passes_name,
                format!(
                    "{passes} passes over {} nodes would pass {MAX_READ_WORK} steps",
                    nodes_wide * nodes_high
                ),
            ));
        }
        if plan.smooth > 0.0 {
            let taps = |s: f64| 2.0 * (3.0 * s).ceil() + 1.0;
            let work = (map.width * map.height) as f64
                * (taps(plan.smooth / pixel.0) + taps(plan.smooth / pixel.1))
                * passes.max(1) as f64;
            if work > MAX_SMOOTH_WORK {
                return Err(Error::parameter(
                    "smooth",
                    format!("is too wide for a map this large: the blur would pass {MAX_SMOOTH_WORK} steps"),
                ));
            }
        }
        Ok(Self {
            width: map.width,
            height: map.height,
            pad: rings as usize,
            frame: map.frame,
            pixel,
        })
    }

    /// The marching grid of `readings` (one per pixel, blurred by `smooth` first), padded.
    fn grid(&self, mut readings: Vec<f64>, smooth: f64) -> Grid {
        if smooth > 0.0 {
            blur(
                &mut readings,
                self.width,
                self.height,
                (smooth / self.pixel.0, smooth / self.pixel.1),
            );
        }
        let (wide, high) = (self.width + 2 * self.pad, self.height + 2 * self.pad);
        let mut values = Vec::with_capacity(wide * high);
        for j in 0..high {
            for i in 0..wide {
                let outside = i == 0 || j == 0 || i + 1 == wide || j + 1 == high;
                values.push(if outside {
                    OUTSIDE
                } else {
                    let x = i.saturating_sub(self.pad).min(self.width - 1);
                    let y = j.saturating_sub(self.pad).min(self.height - 1);
                    readings[y * self.width + x]
                });
            }
        }
        let origin = |at: f64, size: f64| at + (0.5 - self.pad as f64) * size;
        Grid {
            columns: wide - 1,
            rows: high - 1,
            frame: Rect {
                x: origin(self.frame.x, self.pixel.0),
                y: origin(self.frame.y, self.pixel.1),
                width: (wide - 1) as f64 * self.pixel.0,
                height: (high - 1) as f64 * self.pixel.1,
            },
            values,
        }
    }
}

/// The outline of one layer of `grid` at `level`, tidied.
fn layer(
    grid: &Grid,
    level: f64,
    plan: &Plan,
    charge: &mut dyn FnMut(usize) -> Result<(), Error>,
) -> Result<Path, Error> {
    let mut lines = Vec::new();
    trace_level(
        grid,
        level.clamp(LEVEL_RANGE.0, LEVEL_RANGE.1),
        0.0,
        &mut |count| charge(count),
        &mut lines,
    )?;
    tidy(&Path::joined(lines.iter()), plan, charge)
}

/// One outline for each of `levels` that the map reaches: the region where the map is at or above the level.
pub(crate) fn levels(
    map: &MapData,
    levels: &[f64],
    plan: &Plan,
    charge: &mut dyn FnMut(usize) -> Result<(), Error>,
) -> Result<Vec<(f64, Path)>, Error> {
    if levels.len() > MAX_LEVELS {
        return Err(Error::parameter(
            "levels",
            format!("at most {MAX_LEVELS} levels, got {}", levels.len()),
        ));
    }
    if let Some(bad) = levels.iter().find(|level| !level.is_finite()) {
        return Err(Error::parameter("levels", format!("must be finite numbers, got {bad}")));
    }
    let lattice = Lattice::new(map, plan, levels.len(), "levels")?;
    let readings: Vec<f64> = map.bytes.iter().map(|&byte| f64::from(byte) / 255.0).collect();
    let grid = lattice.grid(readings, plan.smooth);
    let mut found = Vec::new();
    for &level in levels {
        let path = layer(&grid, level, plan, charge)?;
        if !path.has_nothing() {
            found.push((level, path));
        }
    }
    Ok(found)
}

/// One outline for each label (byte) the map holds, the region where the map is that byte.
pub(crate) fn labels(
    map: &MapData,
    max_colors: usize,
    plan: &Plan,
    charge: &mut dyn FnMut(usize) -> Result<(), Error>,
) -> Result<Vec<(u8, Path)>, Error> {
    let mut present = [false; 256];
    for &byte in &map.bytes {
        present[usize::from(byte)] = true;
    }
    let count = present.iter().filter(|&&p| p).count();
    if count > max_colors {
        return Err(Error::parameter(
            "max_colors",
            format!("the map has {count} labels, more than {max_colors}"),
        ));
    }
    let lattice = Lattice::new(map, plan, count, "max_colors")?;
    let mut found = Vec::new();
    for label in (0..=255_u8).filter(|&label| present[usize::from(label)]) {
        let readings = map
            .bytes
            .iter()
            .map(|&byte| f64::from(u8::from(byte == label)))
            .collect();
        let grid = lattice.grid(readings, plan.smooth);
        let path = layer(&grid, 0.5, plan, charge)?;
        if !path.has_nothing() {
            found.push((label, path));
        }
    }
    Ok(found)
}
