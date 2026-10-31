//! `Error` through the public API: each kind names where it happened and says what is wrong.

use bullterrier_vector::Error;

#[test]
fn a_parameter_error_prints_the_parameter_then_the_message() {
    let error = Error::Parameter {
        parameter: String::from("step 3 (rotate).angle"),
        message: String::from("must be finite, got NaN"),
    };
    assert_eq!(error.to_string(), "step 3 (rotate).angle: must be finite, got NaN");
}

#[test]
fn a_geometry_error_prints_the_verb_then_the_message() {
    let error = Error::Geometry {
        verb: String::from("bounds"),
        message: String::from("the path is empty"),
    };
    assert_eq!(error.to_string(), "bounds: the path is empty");
}

#[test]
fn the_two_kinds_are_different_errors_even_with_the_same_text() {
    let parameter = Error::Parameter {
        parameter: String::from("at"),
        message: String::from("outside 0..1"),
    };
    let geometry = Error::Geometry {
        verb: String::from("at"),
        message: String::from("outside 0..1"),
    };
    assert_ne!(parameter, geometry);
    assert_eq!(parameter.clone(), parameter);
}
