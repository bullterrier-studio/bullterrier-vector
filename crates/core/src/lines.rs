//! Lines made from a tone: the families of V15c (`rows`, `bands`, `stipple`) and the clip that keeps lines inside a
//! region (`Path::within`).

mod bands;
mod layout;
mod rows;
mod stipple;
mod within;

pub use bands::Bands;
pub(crate) use bands::Plan;
pub use layout::Family;
pub(crate) use layout::{
    DEFAULT_AMPLITUDE, DEFAULT_WAVELENGTH, Layout, RoundRows, corners, lattice, middle, polar, ray_wedges, ring_cells,
};
pub use rows::Rows;
pub(crate) use stipple::Plan as StipplePlan;
pub use stipple::Stipple;
