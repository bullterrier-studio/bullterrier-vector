//! The readings of a distance leaf over a grid of points, kept by whoever draws, so that a later read of the same
//! outline over the same grid is not searched again.

use std::sync::Arc;

/// A place that remembers readings of a distance leaf by content: the key names the outline, the flag and the grid of
/// points read, so equal keys hold equal readings. Remembering is invisible to results (a reading is the number the
/// leaf gives at that point, kept as it was made) and bounded by its owner, which says when readings nobody used go.
pub(crate) trait Grids: std::fmt::Debug {
    /// The readings held under `key`, or, if none are, what `fill` makes, which is then held.
    fn grid(&self, key: Vec<u64>, fill: &mut dyn FnMut() -> Vec<f64>) -> Arc<Vec<f64>>;
}
