//! What can go wrong when the core runs.

use std::fmt;

/// Why the core refused a request. The bindings pass every value on unchecked, so the core owns the range checks and
/// every language gets the same error.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// A parameter is outside what the core accepts.
    Parameter {
        /// The parameter's name, as the caller knows it; a drawing prefixes the place (`step 3 (rotate).angle`).
        parameter: String,
        /// What is wrong: the accepted range and the value given.
        message: String,
    },
    /// The geometry cannot do what was asked, such as the bounds of an empty path.
    Geometry {
        /// The verb that failed (`bounds`, `point_at`), as the caller knows it.
        verb: String,
        /// What is wrong with the geometry.
        message: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parameter {
                parameter: name,
                message,
            }
            | Self::Geometry { verb: name, message } => {
                write!(formatter, "{name}: {message}")
            }
        }
    }
}

impl std::error::Error for Error {}
