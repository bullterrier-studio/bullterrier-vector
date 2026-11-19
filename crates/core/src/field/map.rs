//! `Field::map`: a grayscale map laid over a box, read with bilinear sampling, and `Field::to_map`: a field written
//! out as the same kind of map.

#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "pixel indexes are clamped into a map of at most MAX_MAP_PIXELS pixels before they are used"
)]

use super::Field;
use super::contours::cells_along;
use crate::Error;
use crate::geometry::{Point, Rect, coordinate, finite};
use sha2::{Digest, Sha256};

/// The most pixels a map holds, read or written.
// constant: 2^24 bytes is 16 MiB, a 4096 by 4096 picture; a larger photo is scaled down by the caller, and the
// distance field written out costs a nearest-edge query a pixel (2 microseconds measured on a headline: 40 s at the cap)
pub(crate) const MAX_MAP_PIXELS: usize = 1 << 24;

/// A map's bytes and where it lies.
#[derive(Debug)]
pub(crate) struct MapData {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) bytes: Vec<u8>,
    pub(crate) frame: Rect,
    /// What a saved document calls the map, if it was given a name.
    pub(crate) name: Option<String>,
    /// The SHA-256 of the bytes: how a document names them and how two maps are told apart.
    pub(crate) sha256: [u8; 32],
}

impl MapData {
    /// The SHA-256 of the bytes as 64 lower-case hexadecimal digits: the name a document gives the input.
    pub(crate) fn sha256_hex(&self) -> String {
        crate::key::hex(&self.sha256)
    }

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
pub(crate) fn frame_of(frame: Option<Rect>, width: usize, height: usize) -> Result<Rect, Error> {
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
                format!(
                    "{name} must be above 0 and at most {:e}, got {value:e}",
                    crate::geometry::MAX_COORDINATE
                ),
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
            format!("must be from 1, as must `height`, got {width} by {height}"),
        ));
    }
    if width.checked_mul(height).is_none_or(|pixels| pixels > MAX_MAP_PIXELS) {
        return Err(Error::parameter(
            "width",
            format!("times `height` must be at most {MAX_MAP_PIXELS} pixels, got {width} by {height}"),
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
    /// [`Error::Parameter`] naming `width` unless both sizes are from 1 with at most 2^24 pixels; naming `data` unless
    /// it holds `width * height` bytes; naming `box` unless it is finite, within ±1e100 and has a width and a height
    /// above 0; naming `name` unless it is letters, digits and `_` starting with a letter.
    ///
    /// `name` is what a saved document calls the map; whoever loads the document supplies the bytes under that name.
    /// Without one it is `map1`, `map2`... in the order the document meets them.
    pub fn map(
        width: usize,
        height: usize,
        data: &[u8],
        frame: Option<Rect>,
        name: Option<&str>,
    ) -> Result<Self, Error> {
        size_of(width, height)?;
        if let Some(name) = name
            && !crate::param::is_name(name)
        {
            return Err(Error::parameter(
                "name",
                format!("must be letters, digits and _, starting with a letter, got {name:?}"),
            ));
        }
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
            name: name.map(str::to_owned),
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
    /// [`Error::Parameter`] naming `width` unless both sizes are from 1 with at most 2^24 pixels; naming `box` as for
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
        self.charge_reads("width", "to_map", (width * height) as f64)?;
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

/// A field read at the centres of a grid of equal cells over a frame, the readings scaled so that the lowest is 0 and the
/// highest 1 (a map read at its own pixels keeps its tones, 0 to 1, unscaled): what `trace_levels` traces.
pub(crate) struct Raster {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) frame: Rect,
    /// `height` rows of `width` readings, the top row first.
    pub(crate) readings: Vec<f64>,
    /// The reading scaled to 0 and the size of the range scaled to 1.
    low: f64,
    spread: f64,
}

impl Raster {
    /// `level` on the scale of the readings.
    pub(crate) fn level(&self, level: f64) -> f64 {
        (level - self.low) / self.spread
    }
}

impl Field {
    /// This field read at the centres of the cells of `frame` cut into equal cells no wider than `cell` (design pixels).
    /// Left out, they are a map's own box and one map pixel, and the readings its bytes; a field that is not a map has
    /// neither and refuses them.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `box` or `cell` when left out for a field that is not a map, `box` as for
    /// [`Field::map`], `cell` if the grid would pass `MAX_MAP_PIXELS` cells or the reads `MAX_READ_WORK`.
    pub(crate) fn raster(&self, frame: Option<Rect>, cell: Option<f64>, verb: &str) -> Result<Raster, Error> {
        let map = self.map_data();
        let own = |name: &str| {
            Error::parameter(
                name,
                format!(
                    "is needed: {verb} reads a field that is not a Field.map, which has no box or pixels of its own"
                ),
            )
        };
        let frame = frame_of(
            Some(frame.or(map.map(|map| map.frame)).ok_or_else(|| own("box"))?),
            1,
            1,
        )?;
        let (cell_x, cell_y) = match (cell, map) {
            (Some(cell), _) => (cell, cell),
            (None, Some(map)) => (map.frame.width / map.width as f64, map.frame.height / map.height as f64),
            (None, None) => return Err(own("cell")),
        };
        let (columns, rows) = (cells_along(frame.width, cell_x), cells_along(frame.height, cell_y));
        if columns * rows > MAX_MAP_PIXELS as f64 {
            return Err(Error::parameter(
                "cell",
                format!("is too small for a box this large: its grid would pass {MAX_MAP_PIXELS} cells"),
            ));
        }
        let (width, height) = (columns as usize, rows as usize);
        if let Some(map) = map
            && map.frame == frame
            && (map.width, map.height) == (width, height)
        {
            let readings = map.bytes.iter().map(|&byte| f64::from(byte) / 255.0).collect();
            return Ok(Raster {
                width,
                height,
                frame,
                readings,
                low: 0.0,
                spread: 1.0,
            });
        }
        self.charge_reads("cell", verb, columns * rows)?;
        let mut readings = Vec::with_capacity(width * height);
        for row in 0..height {
            let y = frame.y + (row as f64 + 0.5) * frame.height / rows;
            for column in 0..width {
                readings.push(self.at(Point::new(frame.x + (column as f64 + 0.5) * frame.width / columns, y)));
            }
        }
        let low = readings.iter().copied().fold(f64::INFINITY, f64::min);
        let high = readings.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let spread = if high > low { high - low } else { 1.0 };
        for reading in &mut readings {
            *reading = (*reading - low) / spread;
        }
        Ok(Raster {
            width,
            height,
            frame,
            readings,
            low,
            spread,
        })
    }
}
