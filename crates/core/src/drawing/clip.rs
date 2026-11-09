//! The steps that cut shapes: `clip` and `mask`; and the same on a `Path`.

use super::Drawing;
use crate::Path;
use crate::node::{Given, clip};

impl Drawing {
    /// This drawing with every shape, and its shadow, cut to the region `region` (a `Drawing`, [`Path`] or
    /// [`Shape`](crate::Shape)) fills: all its shapes united once, whatever the way round their outlines run. The
    /// region is in the design frame, as a gradient is. Each shape is cut on its own, as `opacity` applies to it, not as
    /// a group of the drawing: shapes that overlap show through each other. A region that fills nothing leaves nothing
    /// drawn. A second clip cuts to both; a knockout is not cut.
    ///
    /// A `region` that holds an error leaves the drawing with an [`error`](Drawing::error) naming the step; the work
    /// of uniting its shapes is charged to `MAX_SEGMENTS` as a boolean step is.
    #[must_use]
    pub fn clip(&self, region: impl Into<Self>) -> Self {
        self.then(&clip::Clip, vec![("region", Given::Drawing(region.into()))])
    }

    /// This drawing with every shape, and its shadow, faded by the luminance of `source` (a `Drawing`, [`Path`] or
    /// [`Shape`](crate::Shape)) as it is painted, gradients, blends and opacity included: 0.2126 red + 0.7152 green +
    /// 0.0722 blue of the sRGB values, times the alpha. White keeps the shape, black and what `source` leaves bare hide
    /// it. The shape's opacity is multiplied by it, per pixel. Each shape is masked on its own, not as a group. A shape
    /// takes one mask, and the shapes of `source` take none (the drawing then has an [`error`](Drawing::error) naming
    /// `source`); a knockout is not masked. Drawing a mask costs the pixels of the canvas once for each mask, charged to
    /// `MAX_FILTER_PIXELS`; a mesh refuses a mask.
    #[must_use]
    pub fn mask(&self, source: impl Into<Self>) -> Self {
        self.then(&clip::Mask, vec![("source", Given::Drawing(source.into()))])
    }
}

impl Path {
    /// A [`Drawing`] of this path clipped to `region`.
    #[must_use]
    pub fn clip(&self, region: impl Into<Drawing>) -> Drawing {
        Drawing::from(self.clone()).clip(region)
    }

    /// A [`Drawing`] of this path masked by `source`.
    #[must_use]
    pub fn mask(&self, source: impl Into<Drawing>) -> Drawing {
        Drawing::from(self.clone()).mask(source)
    }
}
