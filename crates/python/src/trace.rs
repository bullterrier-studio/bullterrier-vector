//! The tidying numbers of `Drawing.trace_levels` and `trace_labels`.

use crate::expr::Num;
use bullterrier_vector as vector;

/// The tidying numbers of a trace as the caller gave them; what is left out keeps the default.
pub(crate) fn trace_of(
    smooth: Option<Num>,
    tolerance: Option<Num>,
    open: Option<Num>,
    close: Option<Num>,
    min_area: Option<Num>,
    min_hole: Option<Num>,
) -> vector::Trace {
    let mut trace = vector::Trace::new();
    if let Some(Num(value)) = smooth {
        trace = trace.smooth(value);
    }
    if let Some(Num(value)) = tolerance {
        trace = trace.tolerance(value);
    }
    if let Some(Num(value)) = open {
        trace = trace.open(value);
    }
    if let Some(Num(value)) = close {
        trace = trace.close(value);
    }
    if let Some(Num(value)) = min_area {
        trace = trace.min_area(value);
    }
    if let Some(Num(value)) = min_hole {
        trace = trace.min_hole(value);
    }
    trace
}
