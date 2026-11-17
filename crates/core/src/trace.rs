//! Tracing a map: the outlines of a grayscale map at chosen levels, of a map of colour labels, and, exactly, of a map of
//! square cells (a dithered image). Vector links no raster library: the map is a [`Field::map`](crate::Field::map).

mod cells;
mod layers;
mod options;
mod tidy;

pub(crate) use cells::trace as trace_cells;
pub(crate) use layers::{MAX_LEVELS, labels as trace_labels, levels as trace_levels};
pub(crate) use options::Plan;
pub use options::Trace;
