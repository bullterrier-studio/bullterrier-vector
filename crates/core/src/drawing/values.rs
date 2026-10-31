//! `with_values`: fixing Params for the steps before it.

use super::Drawing;
use crate::Values;
use crate::node::{Given, values};

impl Drawing {
    /// This drawing with the Params named in `values` fixed to them, for the steps before this one and every drawing
    /// those steps take; steps added after it read the Params as before. The fixed Params leave
    /// [`parameters`](Drawing::parameters) (a step after that reads one again brings it back) but still count for the
    /// rule that two Params of one name are equal. A cyclic Param's value is stored wrapped.
    ///
    /// A name that is not a free Param of the drawing (unknown, or already fixed), or a value its Param does not
    /// accept, leaves the drawing with an [`error`](Drawing::error) naming `step 3 (with_values).name`.
    #[must_use]
    pub fn with_values(&self, values: &Values) -> Self {
        self.then(&values::WithValues, vec![("values", Given::Values(values.clone()))])
    }
}
