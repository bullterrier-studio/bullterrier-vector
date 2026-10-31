//! What the integration tests share: the outcome type, readers of the errors the library returns, and the helpers
//! that spread proptest sizes over orders of magnitude. Each test file uses some of them.
#![allow(dead_code, reason = "each test file uses a different part")]

use bullterrier_vector::Error;
use proptest::prelude::*;

pub(crate) type Outcome = Result<(), Box<dyn std::error::Error>>;

/// The parameter and message of the `Error::Parameter` an operation returned; an error if it succeeded or failed
/// otherwise.
pub(crate) fn parameter_refusal<T>(outcome: Result<T, Error>) -> Result<(String, String), Box<dyn std::error::Error>> {
    match outcome {
        Err(Error::Parameter { parameter, message }) => Ok((parameter, message)),
        Err(other) => Err(format!("expected a parameter error, got {other}").into()),
        Ok(_) => Err("expected a parameter error, but it succeeded".into()),
    }
}

/// The verb and message of the `Error::Geometry` an operation returned; an error if it succeeded or failed otherwise.
pub(crate) fn geometry_refusal<T>(outcome: Result<T, Error>) -> Result<(String, String), Box<dyn std::error::Error>> {
    match outcome {
        Err(Error::Geometry { verb, message }) => Ok((verb, message)),
        Err(other) => Err(format!("expected a geometry error, got {other}").into()),
        Ok(_) => Err("expected a geometry error, but it succeeded".into()),
    }
}

pub(crate) fn distance(from: (f64, f64), to: (f64, f64)) -> f64 {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    (dx * dx + dy * dy).sqrt()
}

/// Numbers spread evenly over the orders of magnitude from `10^low` to `10^high`: sizes are log-uniform.
pub(crate) fn log_uniform(low: f64, high: f64) -> impl Strategy<Value = f64> {
    (low..high).prop_map(|exponent| libm::pow(10.0, exponent))
}
