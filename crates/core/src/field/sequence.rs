//! Helpers for a sequence of maps the caller holds (the frames of a video), so a trace of each one is steady from
//! frame to frame: `Field::smooth_maps` averages neighbouring maps. The library keeps no sequence of its own: maps in,
//! maps out.

#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "a map's pixel counts are at most 2^24 and a byte is rounded from a mean of bytes"
)]

use super::map::{MAX_MAP_PIXELS, MapData};
use super::{Field, MAX_READ_WORK};
use crate::Error;

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
    /// adding to 1 (the ends are not padded), each byte rounded to the nearest. `sigma` 0, or so small that no neighbour has a
    /// weight above 0, gives the maps back. A static
    /// sequence is unchanged. The maps come back unnamed, in order.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `maps` unless every one is a [`Field::map`] of the size and box of the first (and
    /// together at most 2^28 pixels), naming `sigma` unless it is finite and from 0, and naming `sigma` when the work for
    /// one output frame (the maps it reaches times pixels) passes 2e8 reads: a clip costs its frame count times that.
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
        // each output frame reads its window of maps once: the work grows with the frame count, a frame's share is bounded
        if ((2 * reach + 1).min(count) as f64) * (pixels as f64) > MAX_READ_WORK {
            return Err(Error::parameter("sigma", "reaches too many maps for this many pixels"));
        }
        // the weight of a neighbour `d` away; d / sigma overflows to infinity for a tiny sigma, so its weight is 0, never NaN
        let weight = |d: usize| {
            if d == 0 {
                1.0
            } else {
                libm::exp(-0.5 * (d as f64 / sigma) * (d as f64 / sigma))
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
}
