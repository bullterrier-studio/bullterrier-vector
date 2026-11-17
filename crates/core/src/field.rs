//! Fields: a number at every point ([`Field`]) and a direction without sign at every point (Flow), the inputs of
//! hatched lines. Each is a plain value built once from literal numbers and paths, read by `at`, and saved in a
//! document by its parts.

mod contours;
mod distance;
mod flow;
mod hatch;
mod map;
mod outline;
mod seeds;

pub use distance::Field;
pub(crate) use distance::FieldKind;
pub use flow::Flow;
pub(crate) use flow::FlowKind;
pub use hatch::Hatch;

use crate::Error;
use crate::limits::MAX_SEGMENTS;

/// A count of the edges a field or a hatch makes that fails naming `parameter` past `MAX_SEGMENTS`.
pub(crate) fn segment_budget(parameter: &'static str) -> impl FnMut(usize) -> Result<(), Error> {
    let mut used = 0_usize;
    move |count| {
        used = used.saturating_add(count);
        if used > MAX_SEGMENTS {
            Err(Error::parameter(
                parameter,
                format!("MAX_SEGMENTS {MAX_SEGMENTS} passed"),
            ))
        } else {
            Ok(())
        }
    }
}

/// The length of the vector `(x, y)`: `sqrt` is exact on every target, where `hypot` is the platform's.
pub(crate) fn length(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}
