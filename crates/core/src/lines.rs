//! Lines made from a tone: the families of V15c (`rows`, `bands`, `stipple`) and the clip that keeps lines inside a
//! region (`Path::within`).

mod bands;
mod rows;
mod within;

pub use bands::Bands;
pub(crate) use bands::{Family, Plan};
pub use rows::Rows;
