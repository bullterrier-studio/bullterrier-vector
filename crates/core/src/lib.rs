//! The core of bullterrier-vector: a deterministic vector scene graph and its renderers, in pure Rust. The same inputs
//! and seed give the same bytes on every target, native or WebAssembly; the core reads no files, no clock and no
//! environment. For now it holds only the walking skeleton: [`polygon_svg`] draws a regular polygon as SVG.

#![forbid(unsafe_code)]

mod error;
mod math;
mod polygon;

pub use error::Error;
pub use polygon::polygon_svg;
