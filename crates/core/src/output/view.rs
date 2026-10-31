//! Where the design frame lies in the output: the one decision every output (SVG now, pixels and mesh later) shares.

use crate::geometry::{Point, coordinate};
use crate::{Error, Path};

/// The design frame fitted into an output: scaled by the largest factor that keeps all of it inside, and centred. A
/// drawing without a frame is at scale 1 from the top left. Content outside the frame is not clipped; it lands where
/// the same scale and offset put it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct View {
    scale: f64,
    x: f64,
    y: f64,
}

impl View {
    /// The view of a design frame `design` (width, height) in an output `output`, both sides above 0.
    pub(crate) fn fit(design: Option<(f64, f64)>, output: (f64, f64)) -> Self {
        let Some((design_width, design_height)) = design else {
            return Self {
                scale: 1.0,
                x: 0.0,
                y: 0.0,
            };
        };
        let scale = (output.0 / design_width).min(output.1 / design_height);
        Self {
            scale,
            x: (output.0 - design_width * scale) / 2.0,
            y: (output.1 - design_height * scale) / 2.0,
        }
    }

    /// A length in design pixels, in output pixels.
    ///
    /// # Errors
    /// [`Error::Parameter`] if the length would be beyond ±1e100, as a coordinate may not be.
    pub(crate) fn length(&self, design: f64) -> Result<f64, Error> {
        coordinate("length", design * self.scale)
    }

    /// `path` in output pixels.
    ///
    /// # Errors
    /// [`Error::Parameter`] if a coordinate would be beyond ±1e100.
    pub(crate) fn path(&self, path: &Path) -> Result<Path, Error> {
        path.scale(self.scale, Some(Point::new(0.0, 0.0)))?
            .translate(self.x, self.y)
    }
}
