//! The pure-Rust core of bullterrier-vector, a typography-first vector graphics scripting library. The same inputs and
//! seed give the same bytes on every target, native or WebAssembly; the core reads no files, no clock and no
//! environment. For now it holds the geometry value [`Path`] (build it, write it as SVG path data, measure it, cut it,
//! move it, test points against it) and the start of drawings: a [`Drawing`] of a path draws [`Shape`]s, numbers a
//! drawing reads can be a [`Param`] or an [`Expr`] on Params, and [`Values`] gives them their values.

#![forbid(unsafe_code)]

mod drawing;
mod error;
mod expr;
mod geometry;
mod json;
mod key;
mod limits;
mod math;
mod node;
mod param;
mod path;
mod registry;
mod shape;
mod values;

pub use drawing::{Deg, Drawing, Pivot, Rotate, ScaleBy, Select, Stats, Stroke, Wanted};
pub use error::Error;
pub use expr::Expr;
pub use geometry::{Point, Rect, Scale};
pub use node::{Arg, Domain, Kind, Literal, Reads, Role, Verb};
pub use param::{Param, Unit};
pub use path::Path;
pub use registry::verbs;
pub use shape::{Attributes, Blend, Cap, Color, Join, Shape, StrokeStyle, Style};
pub use values::{Value, Values};
