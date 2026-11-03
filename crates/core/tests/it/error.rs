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

#[test]
fn a_format_error_prints_the_pointer_then_the_message() {
    let error = Error::Format {
        pointer: String::from("/root/2/rotate/angle"),
        message: String::from("must be a number"),
    };
    assert_eq!(error.to_string(), "/root/2/rotate/angle: must be a number");
}

#[test]
fn a_format_error_at_the_document_names_the_document() {
    let error = Error::Format {
        pointer: String::new(),
        message: String::from("not JSON"),
    };
    assert_eq!(error.to_string(), "document: not JSON");
}

#[test]
fn a_format_error_is_a_different_kind_from_a_parameter_error() {
    let format = Error::Format {
        pointer: String::from("at"),
        message: String::from("outside 0..1"),
    };
    let parameter = Error::Parameter {
        parameter: String::from("at"),
        message: String::from("outside 0..1"),
    };
    assert_ne!(format, parameter);
}
