//! What can go wrong when the core runs.

use crate::node::Place;
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

impl Error {
    /// A [`Error::Parameter`] naming `parameter`.
    pub(crate) fn parameter(parameter: &str, message: impl Into<String>) -> Self {
        Self::Parameter {
            parameter: parameter.to_owned(),
            message: message.into(),
        }
    }

    /// An [`Error::Geometry`] from `verb`.
    pub(crate) fn geometry(verb: &str, message: impl Into<String>) -> Self {
        Self::Geometry {
            verb: verb.to_owned(),
            message: message.into(),
        }
    }
}

impl Error {
    /// This error placed at a step: the error came from an argument or a `Path` method, so a parameter's name
    /// becomes `step 3 (rotate).angle` and a geometry error's verb the place, and the message starts with
    /// `shape 7: ` when it came from the shape at that index.
    #[must_use]
    pub(crate) fn at_argument(self, place: &Place, shape: Option<usize>) -> Self {
        let prefix = |message: String| match shape {
            Some(index) => format!("shape {index}: {message}"),
            None => message,
        };
        match self {
            Self::Parameter { parameter, message } => Self::Parameter {
                parameter: format!("{place}.{parameter}"),
                message: prefix(message),
            },
            Self::Geometry { message, .. } => Self::Geometry {
                verb: place.to_string(),
                message: prefix(message),
            },
        }
    }

    /// This error, already placed, found inside `prefix` (`step 2 (stack) / item 1`): joined with ` / `.
    #[must_use]
    pub(crate) fn inside(self, prefix: &str) -> Self {
        match self {
            Self::Parameter { parameter, message } => Self::Parameter {
                parameter: format!("{prefix} / {parameter}"),
                message,
            },
            Self::Geometry { verb, message } => Self::Geometry {
                verb: format!("{prefix} / {verb}"),
                message,
            },
        }
    }
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
