//! Helpers for a sequence of maps the caller holds (the frames of a video), so a trace of each one is steady from
//! frame to frame: `Field::smooth_maps` averages neighbouring maps, `Field::persist_holes` fills a hole the
//! neighbouring maps do not have. The library keeps no sequence of its own: maps in, maps out.

#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "a map's pixel counts are at most 2^24 and a byte is rounded from a mean of bytes"
)]

use super::map::{MAX_MAP_PIXELS, MapData};
use super::{Field, MAX_READ_WORK};
use crate::Error;

/// The most levels `persist_holes` reads, as for a trace.
const MAX_LEVELS: usize = 256;
/// The most bytes of maps held at once (one mask a map): 2^28.
const MAX_HELD: usize = 1 << 28;

/// The maps of `maps` with one size and one frame, or the refusal naming `maps`.
fn maps_of(maps: &[Field]) -> Result<Vec<&MapData>, Error> {
    let mut data = Vec::with_capacity(maps.len());
    for (i, field) in maps.iter().enumerate() {
        let map = field
            .map_data()
            .ok_or_else(|| Error::parameter("maps", format!("map {i} is not a Field.map")))?;
        if let Some(first) = data.first().copied() {
            let (a, b): (&MapData, &MapData) = (first, map);
            if (a.width, a.height) != (b.width, b.height) || a.frame != b.frame {
                return Err(Error::parameter(
                    "maps",
                    format!(
                        "map {i} must have the size and box of map 0 ({} by {}), got {} by {} or another box",
                        a.width, a.height, b.width, b.height
                    ),
                ));
            }
        }
        data.push(map);
    }
    if data
        .first()
        .is_some_and(|m| m.width * m.height * data.len() > MAX_HELD.min(MAX_MAP_PIXELS * 16))
    {
        return Err(Error::parameter("maps", "hold too many pixels in all (at most 2^28)"));
    }
    Ok(data)
}

/// A map the size and box of `like`, with `bytes`.
fn made(like: &MapData, bytes: Vec<u8>) -> Field {
    use sha2::{Digest, Sha256};
    Field::of_map(MapData {
        width: like.width,
        height: like.height,
        sha256: Sha256::digest(&bytes).into(),
        bytes,
        frame: like.frame,
        name: None,
    })
}

/// The frames of `0..count` within `reach` of `at`.
fn window(at: usize, reach: usize, count: usize) -> std::ops::Range<usize> {
    at.saturating_sub(reach)..at.saturating_add(reach).saturating_add(1).min(count)
}

impl Field {
    /// Each map of `maps` (the frames of a sequence, every one a [`Field::map`] of one size and box) as the Gaussian
    /// weighted mean of itself and its neighbours, `sigma` in maps: frame `i` takes frame `j` with weight
    /// `exp(-(i - j)^2 / (2 sigma^2))` for `|i - j|` up to `ceil(3 sigma)`, the weights of the frames that exist
    /// adding to 1 (the ends are not padded), each byte rounded to the nearest. `sigma` 0 gives the maps back. A static
    /// sequence is unchanged. The maps come back unnamed, in order.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `maps` unless every one is a [`Field::map`] of the size and box of the first (and
    /// together at most 2^28 pixels), naming `sigma` unless it is finite and from 0, and naming `sigma` when the work
    /// (maps times frames reached times pixels) passes 2e8 reads.
    pub fn smooth_maps(maps: &[Self], sigma: f64) -> Result<Vec<Self>, Error> {
        if !(sigma.is_finite() && sigma >= 0.0) {
            return Err(Error::parameter(
                "sigma",
                format!("must be finite and from 0, got {sigma}"),
            ));
        }
        let data = maps_of(maps)?;
        let Some(first) = data.first().copied() else {
            return Ok(Vec::new());
        };
        let count = data.len();
        let reach = (sigma * 3.0).ceil().min(count as f64) as usize;
        let pixels = first.width * first.height;
        if (count as f64) * ((2 * reach + 1).min(count) as f64) * (pixels as f64) > MAX_READ_WORK {
            return Err(Error::parameter("sigma", "reaches too many maps for this many pixels"));
        }
        let weight = |d: usize| {
            if sigma == 0.0 {
                f64::from(d == 0)
            } else {
                libm::exp(-((d * d) as f64) / (2.0 * sigma * sigma))
            }
        };
        let mut out = Vec::with_capacity(count);
        let mut sum = vec![0.0_f64; pixels];
        for at in 0..count {
            sum.fill(0.0);
            let mut total = 0.0;
            for j in window(at, reach, count) {
                let w = weight(at.abs_diff(j));
                total += w;
                for (s, &b) in sum.iter_mut().zip(&data[j].bytes) {
                    *s += w * f64::from(b);
                }
            }
            out.push(made(first, sum.iter().map(|s| (s / total).round() as u8).collect()));
        }
        Ok(out)
    }

    /// Each map of `maps` with the holes the neighbouring maps do not have filled in. At each of `levels` (numbers
    /// from 0 to 1, as a level trace reads them) a hole of a map is a part of it below the level that does not reach
    /// the map's edge (what meets the edge goes on past it and is no hole). A hole of map `i` is kept when it overlaps
    /// a hole at the same level in at least `at_least` of the maps within `reach` of `i`, counting `i` itself (so
    /// `at_least` 2 is "one neighbour has it too"); the number is cut to the frames the window holds, so the ends of
    /// the sequence and a sequence of one keep what a static one keeps. A hole that is not kept is raised to the
    /// lowest tone round it, which is at or above the level, so it stays filled after the trace's own blur. A hole
    /// that persists at a level also persists at every lower one, so nesting holds and the levels are independent. A
    /// static sequence is unchanged. Smooth the maps first ([`Field::smooth_maps`]) and then call this on the result.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `maps` as for [`Field::smooth_maps`], `levels` unless there are at most 256 and each
    /// is finite (or when the work, levels times maps times window times pixels, passes 2e8), and `at_least` below 1.
    pub fn persist_holes(maps: &[Self], levels: &[f64], reach: usize, at_least: usize) -> Result<Vec<Self>, Error> {
        if at_least == 0 {
            return Err(Error::parameter("at_least", "must be from 1, 1 being no check at all"));
        }
        if levels.len() > MAX_LEVELS || levels.iter().any(|level| !level.is_finite()) {
            return Err(Error::parameter(
                "levels",
                format!("must be at most {MAX_LEVELS} finite numbers"),
            ));
        }
        let data = maps_of(maps)?;
        let Some(first) = data.first().copied() else {
            return Ok(Vec::new());
        };
        let count = data.len();
        let reach = reach.min(count);
        let pixels = first.width * first.height;
        let work = (levels.len() as f64) * (count as f64) * ((2 * reach + 2) as f64) * (pixels as f64);
        if work > MAX_READ_WORK {
            return Err(Error::parameter(
                "levels",
                "with this many maps and pixels would read too much",
            ));
        }
        let mut out: Vec<Vec<u8>> = data.iter().map(|m| m.bytes.clone()).collect();
        let mut scratch = Scratch::new(first.width, first.height);
        for &level in levels {
            let below: [bool; 256] = std::array::from_fn(|b| (b as f64) / 255.0 < level);
            let holes: Vec<Vec<bool>> = data.iter().map(|m| scratch.holes(&m.bytes, &below)).collect();
            for at in 0..count {
                let span = window(at, reach, count);
                let need = at_least.min(span.len());
                scratch.fill_unkept(&data[at].bytes, &holes, at, span, need, &mut out[at]);
            }
        }
        Ok(data.iter().zip(out).map(|(like, bytes)| made(like, bytes)).collect())
    }
}

/// The 4-neighbours of pixel `p` in a map `width` by `height`.
fn around(width: usize, height: usize, p: usize) -> impl Iterator<Item = usize> + use<> {
    let (x, y) = (p % width, p / width);
    [
        (x > 0).then(|| p - 1),
        (x + 1 < width).then_some(p + 1),
        (y > 0).then(|| p - width),
        (y + 1 < height).then_some(p + width),
    ]
    .into_iter()
    .flatten()
}

/// Buffers reused for every map: the labels of the holes of one map and the walk's worklist.
struct Scratch {
    width: usize,
    height: usize,
    label: Vec<u32>,
    work: Vec<usize>,
}

impl Scratch {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            label: vec![0; width * height],
            work: Vec::new(),
        }
    }

    /// The pixels of `bytes` that are below the level and cannot be reached from the edge through pixels below it.
    fn holes(&mut self, bytes: &[u8], below: &[bool; 256]) -> Vec<bool> {
        let mut outside = vec![false; bytes.len()];
        self.work.clear();
        for p in 0..bytes.len() {
            let (x, y) = (p % self.width, p / self.width);
            let edge = x == 0 || y == 0 || x + 1 == self.width || y + 1 == self.height;
            if edge && below[usize::from(bytes[p])] {
                outside[p] = true;
                self.work.push(p);
            }
        }
        while let Some(p) = self.work.pop() {
            for q in around(self.width, self.height, p) {
                if !outside[q] && below[usize::from(bytes[q])] {
                    outside[q] = true;
                    self.work.push(q);
                }
            }
        }
        bytes
            .iter()
            .zip(&outside)
            .map(|(&b, &out)| below[usize::from(b)] && !out)
            .collect()
    }

    /// Raises into `out` every hole of frame `at` that fewer than `need` frames of `span` (itself included) share.
    fn fill_unkept(
        &mut self,
        bytes: &[u8],
        holes: &[Vec<bool>],
        at: usize,
        span: std::ops::Range<usize>,
        need: usize,
        out: &mut [u8],
    ) {
        let own = &holes[at];
        self.label.fill(0);
        let mut components = 0_u32;
        for start in 0..own.len() {
            if own[start] && self.label[start] == 0 {
                components += 1;
                self.label[start] = components;
                self.work.push(start);
                while let Some(p) = self.work.pop() {
                    for q in around(self.width, self.height, p) {
                        if own[q] && self.label[q] == 0 {
                            self.label[q] = components;
                            self.work.push(q);
                        }
                    }
                }
            }
        }
        if components == 0 {
            return;
        }
        let mut shared = vec![1_usize; components as usize + 1];
        let mut stamp = vec![usize::MAX; components as usize + 1];
        for j in span.filter(|&j| j != at) {
            for (p, &l) in self.label.iter().enumerate() {
                if l != 0 && holes[j][p] && stamp[l as usize] != j {
                    stamp[l as usize] = j;
                    shared[l as usize] += 1;
                }
            }
        }
        // the lowest tone on the ring of each hole that is dropped
        let mut ring = vec![u8::MAX; components as usize + 1];
        for p in 0..own.len() {
            let l = self.label[p] as usize;
            if l != 0 && shared[l] < need {
                for q in around(self.width, self.height, p) {
                    if self.label[q] as usize != l {
                        ring[l] = ring[l].min(bytes[q]);
                    }
                }
            }
        }
        for (p, byte) in out.iter_mut().enumerate() {
            let l = self.label[p] as usize;
            if l != 0 && shared[l] < need {
                *byte = (*byte).max(ring[l]);
            }
        }
    }
}
