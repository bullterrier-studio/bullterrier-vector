//! Lines made from a tone: the families of V15c (`rows`, `bands`, `stipple`) and the clip that keeps lines inside a
//! region (`Path::within`).

mod bands;
mod layout;
mod rows;
mod stipple;
mod within;

pub use bands::Bands;
pub(crate) use bands::Plan;
pub(crate) use layout::Family;
pub use rows::Rows;
pub(crate) use stipple::Plan as StipplePlan;
pub use stipple::Stipple;
