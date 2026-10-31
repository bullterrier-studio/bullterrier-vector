//! The pure-Rust core of bullterrier-vector, a typography-first vector graphics scripting library. The same inputs and
//! seed give the same bytes on every target, native or WebAssembly; the core reads no files, no clock and no
//! environment. For now it holds the geometry value [`Path`]: build it, write it as SVG path data, measure it, cut
//! it, test points against it.

#![forbid(unsafe_code)]

mod error;
mod geometry;
mod math;
mod path;

pub use error::Error;
pub use geometry::{Point, Rect};
pub use path::Path;
