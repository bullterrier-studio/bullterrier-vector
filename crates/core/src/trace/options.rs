//! `Trace`: how the outlines of a traced layer are tidied.

use crate::{Expr, Rect};

/// How a traced layer is read and tidied, in design pixels. The default reads a map over its own frame and pixels, blurs
/// nothing, opens and closes nothing, drops nothing and fits curves within half a pixel. The tidying numbers are numbers or
/// expressions of Params, read once.
#[derive(Debug, Clone)]
pub struct Trace {
    pub(crate) smooth: Expr,
    pub(crate) tolerance: Expr,
    pub(crate) open: Expr,
    pub(crate) close: Expr,
    pub(crate) min_area: Expr,
    pub(crate) min_hole: Expr,
    pub(crate) frame: Option<Rect>,
    pub(crate) cell: Option<f64>,
}

impl Default for Trace {
    fn default() -> Self {
        Self {
            smooth: Expr::from(0.0),
            tolerance: Expr::from(0.5),
            open: Expr::from(0.0),
            close: Expr::from(0.0),
            min_area: Expr::from(0.0),
            min_hole: Expr::from(0.0),
            frame: None,
            cell: None,
        }
    }
}

impl Trace {
    /// The defaults: see [`Trace`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Blur the map with a Gaussian of standard deviation `sigma` design pixels before tracing (0: not at all). Grain
    /// makes tiny loops and a jagged edge; a sigma of a pixel or two removes them.
    #[must_use]
    pub fn smooth(mut self, sigma: impl Into<Expr>) -> Self {
        self.smooth = sigma.into();
        self
    }

    /// Fit curves through each traced edge so that every point of the fit is within `tolerance` design pixels of the
    /// traced polyline (0: keep the polyline). Default 0.5.
    #[must_use]
    pub fn tolerance(mut self, tolerance: impl Into<Expr>) -> Self {
        self.tolerance = tolerance.into();
        self
    }

    /// Open each layer by `radius` design pixels: shrink then grow, which removes what is narrower than twice `radius`
    /// and rounds the outside corners of what stays.
    #[must_use]
    pub fn open(mut self, radius: impl Into<Expr>) -> Self {
        self.open = radius.into();
        self
    }

    /// Close each layer by `radius` design pixels: grow then shrink, which fills what is narrower than twice `radius`
    /// (gaps and thin cracks) and rounds the inside corners. Done before [`Trace::open`].
    #[must_use]
    pub fn close(mut self, radius: impl Into<Expr>) -> Self {
        self.close = radius.into();
        self
    }

    /// Drop the parts of each layer whose outline encloses less than `area` square design pixels.
    #[must_use]
    pub fn min_area(mut self, area: impl Into<Expr>) -> Self {
        self.min_area = area.into();
        self
    }

    /// Read the field over `frame` (design pixels) instead of a map's own frame: needed for a field that is not a map.
    /// [`Drawing::trace_levels`](crate::Drawing::trace_levels) only; `trace_labels` refuses it.
    #[must_use]
    pub fn frame(mut self, frame: Rect) -> Self {
        self.frame = Some(frame);
        self
    }

    /// Read the field at the centers of cells no wider than `cell` design pixels instead of a map's own pixels: needed
    /// for a field that is not a map. [`Drawing::trace_levels`](crate::Drawing::trace_levels) only; `trace_labels`
    /// refuses it.
    #[must_use]
    pub fn cell(mut self, cell: f64) -> Self {
        self.cell = Some(cell);
        self
    }

    /// Fill the holes of each layer that enclose less than `area` square design pixels.
    #[must_use]
    pub fn min_hole(mut self, area: impl Into<Expr>) -> Self {
        self.min_hole = area.into();
        self
    }
}

/// The numbers of a [`Trace`] for one draw.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Plan {
    pub(crate) smooth: f64,
    pub(crate) tolerance: f64,
    pub(crate) open: f64,
    pub(crate) close: f64,
    pub(crate) min_area: f64,
    pub(crate) min_hole: f64,
}
