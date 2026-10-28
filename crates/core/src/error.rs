//! What can go wrong when the core runs.

use std::fmt;

/// Why the core refused a request. The bindings pass every value on unchecked, so the core owns the range checks and
/// every language gets the same error.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// A parameter is outside what the core accepts.
    Parameter {
        /// The parameter's name, as the caller knows it.
        parameter: &'static str,
        /// What is wrong: the accepted range and the value given.
        message: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parameter { parameter, message } => write!(formatter, "{parameter}: {message}"),
        }
    }
}

impl std::error::Error for Error {}
