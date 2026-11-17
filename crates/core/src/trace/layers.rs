//! `trace_levels` and `trace_labels`: the closed outlines of the regions of a map where it is at or above a level, or
//! where it is one label, found by marching squares over the map's pixel centres.
//!
//! The lattice of readings is the map's pixels padded with rings of the nearest edge value, and one outer ring of a
//! value far below every level. A shape that reaches the map's border therefore goes on past it and closes outside the
//! frame, and no edge of any shape lies along the border: a viewer that crops at the frame sees none. Closing and
//! opening shrink an edge by their radius, so the padding is the larger of them plus the curve tolerance, in map pixels
//! (at least one), plus two pixels.
//!
//! The labels of a map tile the lattice: the commonest label is the base and takes what the others leave, each other
//! label is its traced shape minus the labels after it. The shapes are disjoint and cover the whole padded rectangle.

#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "the lattice has at most MAX_LATTICE_NODES nodes, checked before any index is made"
)]

use super::Plan;
use super::tidy::tidy;
use crate::Error;
use crate::field::{Grid, MAX_READ_WORK, MAX_SMOOTH_WORK, MapData, blur, trace_level};
use crate::geometry::Rect;
use crate::path::{Combine, Path};

/// The most levels one call traces.
// constant: each level is a pass over the whole lattice, and no picture reads more than a few dozen tones
pub(crate) const MAX_LEVELS: usize = 256;

/// The most nodes a padded lattice has.
// constant: a node costs 8 bytes in the grid and 8 in the map's readings, so 2^25 nodes is half a gigabyte at most, and
// a 4096 by 4096 map (the largest a `Field.map` holds, 2^24 pixels) with a few hundred pixels of padding fits
const MAX_LATTICE_NODES: f64 = 33_554_432.0;

/// How closely the booleans that cut the labels apart follow a curve, in design pixels.
// constant: a hundredth of a design pixel is below anything that shows, and is the tidying's own
const CUT_TOLERANCE: f64 = 0.01;

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
        let rings = 2.0 + (margin / pixel.0.min(pixel.1)).ceil();
        let (nodes_wide, nodes_high) = (map.width as f64 + 2.0 * rings, map.height as f64 + 2.0 * rings);
        if nodes_wide * nodes_high > MAX_LATTICE_NODES {
            // the map alone fits (a `Field.map` has at most 2^24 pixels), so the padding is too wide: name its widest part
            let (name, _) = [
                ("open", plan.open),
                ("close", plan.close),
                ("tolerance", plan.tolerance),
            ]
            .into_iter()
            .fold(("map", 0.0), |most, one| if one.1 > most.1 { one } else { most });
            return Err(Error::parameter(
                name,
                format!("the map with its padding of {rings} pixels would pass {MAX_LATTICE_NODES} nodes"),
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

    /// The rectangle the tiled labels cover: the rings of padding where a shape can lie, and half a pixel more.
    fn cover(&self) -> Rect {
        let (pad, wide, high) = (self.pad as f64, self.width as f64, self.height as f64);
        Rect {
            x: self.frame.x + (1.0 - pad) * self.pixel.0,
            y: self.frame.y + (1.0 - pad) * self.pixel.1,
            width: (wide + 2.0 * pad - 2.0) * self.pixel.0,
            height: (high + 2.0 * pad - 2.0) * self.pixel.1,
        }
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

/// One outline for each label (byte) the map holds, tiling the map: the commonest label first (the lowest byte of a tie),
/// the others in order of the byte. Each is the region where the map is that byte, tidied, and the labels never overlap
/// or leave a gap: the commonest label is whatever the others leave of the padded rectangle, and every other label is
/// its tidied region without the labels after it.
pub(crate) fn labels(
    map: &MapData,
    max_colors: usize,
    plan: &Plan,
    charge: &mut dyn FnMut(usize) -> Result<(), Error>,
) -> Result<Vec<(u8, Path)>, Error> {
    let mut counts = [0_usize; 256];
    for &byte in &map.bytes {
        counts[usize::from(byte)] += 1;
    }
    let mut present: Vec<u8> = (0..=255_u8).filter(|&label| counts[usize::from(label)] > 0).collect();
    if present.len() > max_colors {
        return Err(Error::parameter(
            "max_colors",
            format!("the map has {} labels, more than {max_colors}", present.len()),
        ));
    }
    let Some(&base) = present
        .iter()
        .max_by_key(|&&label| (counts[usize::from(label)], std::cmp::Reverse(label)))
    else {
        return Ok(Vec::new());
    };
    present.retain(|&label| label != base);
    // the base is never traced: it is what the others leave
    let lattice = Lattice::new(map, plan, present.len(), "max_colors")?;
    let mut traced = Vec::with_capacity(present.len());
    for label in present {
        let readings = map
            .bytes
            .iter()
            .map(|&byte| f64::from(u8::from(byte == label)))
            .collect();
        let grid = lattice.grid(readings, plan.smooth);
        traced.push((label, layer(&grid, 0.5, plan, charge)?));
    }
    tile(lattice.cover(), base, traced, charge)
}

/// The base and the traced labels cut so that none overlaps another and together they fill `cover`: the base is `cover`
/// without all of them, each other label is its path without those after it. A label left with nothing is dropped.
fn tile(
    cover: Rect,
    base: u8,
    traced: Vec<(u8, Path)>,
    charge: &mut dyn FnMut(usize) -> Result<(), Error>,
) -> Result<Vec<(u8, Path)>, Error> {
    let count = traced.len() + 1;
    // the cuts are charged to the draw; running out is the number of labels' doing
    let mut charge = |edges| {
        charge(edges).map_err(|_| {
            Error::parameter(
                "max_colors",
                format!(
                    "cutting {count} labels apart would pass the limit on segments: use fewer colours or a smaller map"
                ),
            )
        })
    };
    // fitted apart, a hole can poke out of its outline by the tolerance, which would count against the labels beside it
    let traced = traced
        .into_iter()
        .map(|(label, path)| Ok((label, path.resolved(CUT_TOLERANCE, &mut charge)?)))
        .collect::<Result<Vec<_>, Error>>()?;
    let whole = Path::rect(cover.x, cover.y, cover.width, cover.height, 0.0)?;
    // the outlines of labels may overlap a little, and under the nonzero rule their joined outlines fill their union
    let all = Path::joined(traced.iter().map(|(_, path)| path));
    let mut tiles = vec![(
        base,
        whole.combined(&all, Combine::Subtract, CUT_TOLERANCE, &mut charge)?,
    )];
    for (at, (label, path)) in traced.iter().enumerate() {
        let later = Path::joined(traced[at + 1..].iter().map(|(_, path)| path));
        let cut = path.combined(&later, Combine::Subtract, CUT_TOLERANCE, &mut charge)?;
        if !cut.has_nothing() {
            tiles.push((*label, cut));
        }
    }
    Ok(tiles)
}
