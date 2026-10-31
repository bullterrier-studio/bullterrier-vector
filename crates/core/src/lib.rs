//! The pure-Rust core of bullterrier-vector, a typography-first vector graphics scripting library. The same inputs and
//! seed give the same bytes on every target, native or WebAssembly; the core reads no files, no clock and no
//! environment. For now it holds the geometry value [`Path`]: build it, write it as SVG path data, measure it, cut it,
//! move it, test points against it.

#![forbid(unsafe_code)]

mod error;
mod geometry;
mod math;
mod node;
mod param;
mod path;
mod registry;
mod values;

pub use error::Error;
pub use geometry::{Point, Rect, Scale};
pub use node::{Arg, Domain, Kind, Literal, Reads, Role, Verb};
pub use param::{Param, Unit};
pub use path::Path;
pub use registry::verbs;
pub use values::{Value, Values};
