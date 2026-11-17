//! `Field::map`: a grayscale map laid over a box, read with bilinear sampling, and `Field::to_map`: a field written
//! out as the same kind of map.

#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "pixel indexes are clamped into a map of at most MAX_MAP_PIXELS pixels before they are used"
)]

use super::Field;
use crate::Error;
use crate::geometry::{Point, Rect, coordinate, finite};
use sha2::{Digest, Sha256};

/// The most pixels a map holds, read or written.
// constant: 2^26 bytes is 64 MiB, a 8192 by 8192 picture; a larger photo is scaled down by the caller, and the
// distance field written out costs a nearest-edge query a pixel
pub(crate) const MAX_MAP_PIXELS: usize = 1 << 26;

/// A map's bytes and where it lies.
#[derive(Debug)]
pub(crate) struct MapData {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) bytes: Vec<u8>,
    pub(crate) frame: Rect,
    /// The SHA-256 of the bytes: how a document names them and how two maps are told apart.
    pub(crate) sha256: [u8; 32],
}

impl MapData {
    /// The map's value, 0 to 1, at `point`: bilinear between pixel centres, the nearest edge value beyond the frame.
    pub(crate) fn sample(&self, point: Point) -> f64 {
        let across = |p: f64, origin: f64, size: f64, count: usize| {
            let at = (p - origin) / size * count as f64 - 0.5;
            at.clamp(0.0, (count - 1) as f64)
        };
        let u = across(point.x, self.frame.x, self.frame.width, self.width);
        let v = across(point.y, self.frame.y, self.frame.height, self.height);
        let (i, j) = (u.floor() as usize, v.floor() as usize);
        let (i1, j1) = ((i + 1).min(self.width - 1), (j + 1).min(self.height - 1));
        let (fu, fv) = (u - i as f64, v - j as f64);
        let at = |x: usize, y: usize| f64::from(self.bytes[y * self.width + x]);
        let top = at(i, j) * (1.0 - fu) + at(i1, j) * fu;
        let bottom = at(i, j1) * (1.0 - fu) + at(i1, j1) * fu;
        (top * (1.0 - fv) + bottom * fv) / 255.0
    }
}

/// `frame`, or the one pixel a map pixel from the origin when none is given, checked.
fn frame_of(frame: Option<Rect>, width: usize, height: usize) -> Result<Rect, Error> {
    let frame = frame.unwrap_or(Rect {
        x: 0.0,
        y: 0.0,
        width: width as f64,
        height: height as f64,
    });
    for (name, value) in [("x", frame.x), ("y", frame.y)] {
        coordinate(&format!("box.{name}"), finite("box", value)?)?;
    }
    for (name, value) in [("width", frame.width), ("height", frame.height)] {
        if !(value.is_finite() && value > 0.0 && value <= crate::geometry::MAX_COORDINATE) {
            return Err(Error::parameter(
                "box",
                format!("must have a width and a height above 0, got {name} {value}"),
            ));
        }
    }
    Ok(frame)
}

/// `width` and `height` as sizes of a map: whole numbers from 1 with at most `MAX_MAP_PIXELS` pixels.
fn size_of(width: usize, height: usize) -> Result<(), Error> {
    if width == 0 || height == 0 {
        return Err(Error::parameter(
            "width",
            format!("and `height` must be from 1, got {width} by {height}"),
        ));
    }
    if width.checked_mul(height).is_none_or(|pixels| pixels > MAX_MAP_PIXELS) {
        return Err(Error::parameter(
            "width",
            format!("by `height` would pass {MAX_MAP_PIXELS} pixels, got {width} by {height}"),
        ));
    }
    Ok(())
}

impl Field {
    /// An 8-bit grayscale map as a field: `data` holds `height` rows of `width` bytes, the top row first, 0 black and
    /// 255 white, so the field reads 0 to 1. The map lies over `frame` (default: one design pixel a map pixel, from
    /// the origin); a pixel's value stands at its centre, the field is bilinear between centres and holds the
    /// nearest edge value beyond the frame.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `width` unless both sizes are from 1 with at most 2^26 pixels; naming `data` unless
    /// it holds `width * height` bytes; naming `box` unless it is finite, within ±1e100 and has a width and a height
    /// above 0.
    pub fn map(width: usize, height: usize, data: &[u8], frame: Option<Rect>) -> Result<Self, Error> {
        size_of(width, height)?;
        if data.len() != width * height {
            return Err(Error::parameter(
                "data",
                format!(
                    "must hold width * height = {} bytes, got {}",
                    width * height,
                    data.len()
                ),
            ));
        }
        let frame = frame_of(frame, width, height)?;
        Ok(Self::of_map(MapData {
            width,
            height,
            bytes: data.to_vec(),
            frame,
            sha256: Sha256::digest(data).into(),
        }))
    }

    /// This field written out as an 8-bit grayscale map of `width` by `height` pixels over `frame` (default: one
    /// design pixel a map pixel, from the origin), the top row first: each pixel is the field at its centre, with
    /// `low` written as 0 and `high` as 255, linear between and held beyond, rounded to the nearest level. It is the
    /// input form of [`Field::map`], so a map read back over the same frame gives the field at the pixel centres within
    /// half a level.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `width` unless both sizes are from 1 with at most 2^26 pixels; naming `box` as for
    /// [`Field::map`]; naming `high` unless `low` and `high` are finite and `high` is above `low`.
    pub fn to_map(
        &self,
        width: usize,
        height: usize,
        frame: Option<Rect>,
        low: f64,
        high: f64,
    ) -> Result<Vec<u8>, Error> {
        size_of(width, height)?;
        let frame = frame_of(frame, width, height)?;
        if !(low.is_finite() && high.is_finite() && high > low) {
            return Err(Error::parameter(
                "high",
                format!("must be above `low` and both finite, got {low} and {high}"),
            ));
        }
        let mut bytes = Vec::with_capacity(width * height);
        for row in 0..height {
            let y = frame.y + (row as f64 + 0.5) * frame.height / height as f64;
            for column in 0..width {
                let x = frame.x + (column as f64 + 0.5) * frame.width / width as f64;
                let level = ((self.at(Point::new(x, y)) - low) / (high - low)).clamp(0.0, 1.0) * 255.0;
                bytes.push(level.round() as u8);
            }
        }
        Ok(bytes)
    }
}
