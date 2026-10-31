//! Drawing a `Drawing`: its steps evaluated, in order, to a list of shapes.

use super::Drawing;
use crate::{Error, Shape, Values};

impl Drawing {
    /// The shapes the drawing evaluates to, in paint order (the bottom first), in design pixels, with the Params at
    /// the `values` given and at their defaults where none is given. The default paint is resolved: an unpainted
    /// closed outline has a black fill, an unpainted open line a black stroke.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming the Param if `values` gives one the drawing does not read.
    pub fn shapes(&self, values: &Values) -> Result<Vec<Shape>, Error> {
        if let Some(name) = values.names().next() {
            return Err(Error::parameter(
                name,
                "the drawing reads no Params, so there is no value to give it",
            ));
        }
        Ok(vec![Shape::from_path(self.step.path.clone(), 0).painted()])
    }
}
