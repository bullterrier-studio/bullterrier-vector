//! The geometry steps of a `Drawing` (`translate`, `rotate`, `scale`) through the public API: each equals the `Path`
//! verb applied to the shape's path, takes a number, a Param or an expression, and reports a number it refuses at the
//! call (`error()`) or when drawn, naming the step. Expected values are written by hand; `node_contract.rs` compares
//! each step with the `Path` verb.

mod common;

use bullterrier_vector::{Deg, Drawing, Error, Expr, Param, Path, Point, Rotate, ScaleBy, Values};
use common::{Outcome, parameter_refusal};

const SHAPE: &str = "M1 2 L10 4.25 Q11 5 12 6.5 C1 2 3 4 5 6 Z";

/// The path data of the only shape `drawing` draws with `values`.
fn data(drawing: &Drawing, values: &Values) -> Result<String, Box<dyn std::error::Error>> {
    let shapes = drawing.shapes(values)?;
    match shapes.as_slice() {
        [only] => Ok(only.path().to_svg_path()),
        other => Err(format!("expected one shape, got {}", other.len()).into()),
    }
}

/// A Param of a wide range, so that it can carry a number that leaves what a coordinate holds.
fn wide(name: &str, default: f64) -> Result<Param, Error> {
    Param::new(name, default, -1e200, 1e200)
}

/// The error a drawing holds, or an error if it holds none.
fn held(drawing: &Drawing) -> Result<Error, Box<dyn std::error::Error>> {
    drawing
        .error()
        .cloned()
        .ok_or_else(|| "the drawing holds no error".into())
}

// Translate ---------------------------------------------------------------------------------------------------------

#[test]
fn translating_a_drawing_adds_to_every_point_like_the_path_does() -> Outcome {
    let drawing = Drawing::from(Path::from_svg_path(SHAPE)?).translate(3.5, -7.25);
    assert_eq!(drawing.error(), None);
    assert_eq!(
        data(&drawing, &Values::new())?,
        "M4.5 -5.25 L13.5 -3 Q14.5 -2.25 15.5 -0.75 C4.5 -5.25 6.5 -3.25 8.5 -1.25 Z"
    );
    Ok(())
}

#[test]
fn translating_by_a_param_reads_its_value_when_drawn_and_its_default_otherwise() -> Outcome {
    let slide = Param::new("slide", 4.0, -100.0, 100.0)?;
    let drawing = Drawing::from(Path::line((0.0, 0.0), (2.0, 1.0))?).translate(&slide, 0.5);
    assert_eq!(data(&drawing, &Values::new())?, "M4 0.5 L6 1.5");
    assert_eq!(data(&drawing, &Values::new().set("slide", -10.0))?, "M-10 0.5 L-8 1.5");
    // the Param is read by the drawing, in the order it was first used
    let read = drawing.parameters()?;
    assert_eq!(read, vec![slide]);
    Ok(())
}

#[test]
fn translating_by_an_expression_works_it_out_for_the_values_given() -> Outcome {
    let p = Param::new("p", 0.0, -100.0, 100.0)?;
    let q = Param::new("q", 0.0, -100.0, 100.0)?;
    let drawing = Drawing::from(Path::line((0.0, 0.0), (2.0, 1.0))?).translate(2.0 * &p + 1.0, &q / 4.0);
    let values = Values::new().set("p", 3.0).set("q", -10.0);
    // dx = 2 * 3 + 1 = 7, dy = -10 / 4 = -2.5
    assert_eq!(data(&drawing, &values)?, "M7 -2.5 L9 -1.5");
    // the Params come in the order the arguments read them
    assert_eq!(drawing.parameters()?, vec![p, q]);
    Ok(())
}

#[test]
fn a_list_gives_the_shape_its_entry() -> Outcome {
    let slide = Param::new("slide", 0.0, -100.0, 100.0)?;
    let drawing = Drawing::from(Path::line((0.0, 0.0), (2.0, 1.0))?).translate(&slide, 0.0);
    assert_eq!(data(&drawing, &Values::new().set("slide", vec![7.5]))?, "M7.5 0 L9.5 1");
    Ok(())
}

#[test]
fn a_list_of_the_wrong_length_is_refused_naming_the_param_the_step_and_both_counts() -> Outcome {
    let slide = Param::new("slide", 0.0, -100.0, 100.0)?;
    let drawing = Drawing::from(Path::line((0.0, 0.0), (2.0, 1.0))?).translate(&slide, 0.0);
    let (name, message) = parameter_refusal(drawing.shapes(&Values::new().set("slide", vec![1.0, 2.0, 3.0])))?;
    assert_eq!(name, "step 1 (translate).dx");
    for text in ["slide", "3", "1 shape"] {
        assert!(message.contains(text), "{text:?} not in {message}");
    }
    Ok(())
}

#[test]
fn a_constant_the_step_refuses_is_refused_when_the_call_is_made_naming_the_step_and_the_value() -> Outcome {
    let line = Drawing::from(Path::line((0.0, 0.0), (2.0, 1.0))?);
    // (case, the drawing after the call, the argument named, a text the message holds)
    let rows = [
        ("NaN", line.translate(f64::NAN, 0.0), "step 1 (translate).dx", "NaN"),
        (
            "infinity",
            line.translate(0.0, f64::INFINITY),
            "step 1 (translate).dy",
            "inf",
        ),
        (
            "too large",
            line.translate(2e100, 0.0),
            "step 1 (translate).dx",
            "2000000",
        ),
        (
            "too small",
            line.translate(0.0, -1.5e100),
            "step 1 (translate).dy",
            "-1500000",
        ),
        (
            "overflow in an expression",
            line.translate(Expr::from(1e300) * 1e300, 0.0),
            "step 1 (translate).dx",
            "(1000000",
        ),
    ];
    for (case, drawing, argument, text) in rows {
        let (name, message) = parameter_refusal(Err::<(), _>(held(&drawing)?))?;
        assert!(name == argument && message.contains(text), "{case}: {name}: {message}");
        // the same error comes back when it is drawn and when the Params are asked for
        assert_eq!(drawing.shapes(&Values::new()).err(), drawing.error().cloned(), "{case}");
        assert_eq!(drawing.parameters().err(), drawing.error().cloned(), "{case}");
    }
    // the bounds themselves are accepted
    assert_eq!(line.translate(1e100, -1e100).error(), None);
    Ok(())
}

#[test]
fn a_constant_division_by_zero_is_refused_at_the_call_naming_the_expression() -> Outcome {
    let line = Drawing::from(Path::line((0.0, 0.0), (2.0, 1.0))?);
    let drawing = line.translate(Expr::from(1.0) / 0.0, 0.0);
    let (name, message) = parameter_refusal(Err::<(), _>(held(&drawing)?))?;
    assert_eq!(name, "step 1 (translate).dx");
    assert!(message.contains("(1 / 0)"), "{message}");
    Ok(())
}

#[test]
fn an_expression_that_reads_a_param_is_checked_when_drawn_not_when_called() -> Outcome {
    let p = Param::new("p", 1.0, -10.0, 10.0)?;
    let line = Drawing::from(Path::line((0.0, 0.0), (2.0, 1.0))?);
    // this divides by zero only for the Param's value 0
    let drawing = line.translate(1.0 / &p, 0.0);
    assert_eq!(drawing.error(), None);
    assert_eq!(data(&drawing, &Values::new())?, "M1 0 L3 1");
    let (name, message) = parameter_refusal(drawing.shapes(&Values::new().set("p", 0.0)))?;
    assert_eq!(name, "step 1 (translate).dx");
    assert!(message.contains("(1 / p)") && message.contains("zero"), "{message}");
    Ok(())
}

#[test]
fn a_number_that_leaves_the_coordinates_when_drawn_is_refused_naming_the_step_the_shape_and_the_value() -> Outcome {
    let far = wide("far", 0.0)?;
    let drawing = Drawing::from(Path::line((0.0, 0.0), (2.0, 1.0))?).translate(0.0, &far);
    let (name, message) = parameter_refusal(drawing.shapes(&Values::new().set("far", 1e150)))?;
    assert_eq!(name, "step 1 (translate).dy");
    assert!(
        message.starts_with("shape 0: ") && message.contains(&format!("{}", 1e150)),
        "{message}"
    );
    Ok(())
}

#[test]
fn a_value_outside_the_range_of_a_param_is_refused_naming_the_step_and_the_param() -> Outcome {
    let slide = Param::new("slide", 0.0, -100.0, 100.0)?;
    let drawing = Drawing::from(Path::line((0.0, 0.0), (2.0, 1.0))?).translate(&slide, 0.0);
    let (name, message) = parameter_refusal(drawing.shapes(&Values::new().set("slide", 500.0)))?;
    assert_eq!(name, "step 1 (translate).dx");
    assert!(message.contains("slide") && message.contains("500"), "{message}");
    Ok(())
}

#[test]
fn an_expression_deeper_than_the_limit_is_refused_at_the_call_even_with_a_param_in_it() -> Outcome {
    let p = Param::new("p", 0.0, -1.0, 1.0)?;
    let line = Drawing::from(Path::line((0.0, 0.0), (2.0, 1.0))?);
    let mut deep = Expr::from(&p);
    for _ in 0..127 {
        deep = deep + 1.0;
    }
    // 128 levels are accepted, 129 are not
    assert_eq!(line.translate(deep.clone(), 0.0).error(), None);
    let drawing = line.translate(deep + 1.0, 0.0);
    let (name, message) = parameter_refusal(Err::<(), _>(held(&drawing)?))?;
    assert_eq!(name, "step 1 (translate).dx");
    assert!(message.contains("129 levels") && message.contains("128"), "{message}");
    Ok(())
}

// Chains ------------------------------------------------------------------------------------------------------------

#[test]
fn an_error_in_a_later_step_names_that_step() -> Outcome {
    let slide = Param::new("slide", 0.0, -100.0, 100.0)?;
    let line = Drawing::from(Path::line((0.0, 0.0), (2.0, 1.0))?);
    // refused at the call, in step 2
    let at_call = line.translate(1.0, 1.0).translate(f64::NAN, 0.0);
    assert_eq!(
        held(&at_call)?.to_string().split(':').next(),
        Some("step 2 (translate).dx")
    );
    // refused when drawn, in step 3
    let when_drawn = line.translate(1.0, 1.0).translate(2.0, 2.0).translate(0.0, &slide);
    let (name, _) = parameter_refusal(when_drawn.shapes(&Values::new().set("slide", 1e101)))?;
    assert_eq!(name, "step 3 (translate).dy");
    Ok(())
}

#[test]
fn steps_apply_in_the_order_they_were_called() -> Outcome {
    let line = Drawing::from(Path::line((0.0, 0.0), (2.0, 1.0))?);
    let moved = line.translate(1.0, 2.0).translate(10.0, 20.0);
    assert_eq!(data(&moved, &Values::new())?, "M11 22 L13 23");
    // the drawing it was made from is unchanged
    assert_eq!(data(&line, &Values::new())?, "M0 0 L2 1");
    Ok(())
}

#[test]
fn a_chain_keeps_its_first_error_and_steps_after_it_change_nothing() -> Outcome {
    let line = Drawing::from(Path::line((0.0, 0.0), (2.0, 1.0))?);
    let first = line.translate(f64::NAN, 0.0);
    let then_another = first.translate(3e100, 0.0).translate(1.0, 1.0);
    assert_eq!(then_another.error(), first.error());
    assert_eq!(
        held(&then_another)?.to_string().split(':').next(),
        Some("step 1 (translate).dx")
    );
    assert_eq!(then_another.shapes(&Values::new()).err(), first.error().cloned());
    assert_eq!(then_another.parameters().err(), first.error().cloned());
    // a good drawing holds no error, and the branch that failed did not touch it
    assert_eq!(line.error(), None);
    Ok(())
}

#[test]
fn a_value_for_a_name_the_drawing_does_not_read_is_refused_listing_what_it_reads() -> Outcome {
    let slide = Param::new("slide", 0.0, -100.0, 100.0)?;
    let drawing = Drawing::from(Path::line((0.0, 0.0), (2.0, 1.0))?).translate(&slide, 0.0);
    let (name, message) = parameter_refusal(drawing.shapes(&Values::new().set("slyde", 1.0)))?;
    assert!(name == "slyde" && message.contains("slide"), "{name}: {message}");
    // a drawing that reads nothing says so
    let still = Drawing::from(Path::line((0.0, 0.0), (2.0, 1.0))?).translate(1.0, 1.0);
    let (name, message) = parameter_refusal(still.shapes(&Values::new().set("slide", 1.0)))?;
    assert!(name == "slide" && message.contains("no Param"), "{name}: {message}");
    Ok(())
}

#[test]
fn two_different_params_with_one_name_are_refused_at_the_call() -> Outcome {
    let narrow = Param::new("slide", 0.0, -1.0, 1.0)?;
    let wide = Param::new("slide", 0.0, -1.0, 2.0)?;
    let line = Drawing::from(Path::line((0.0, 0.0), (2.0, 1.0))?);
    let drawing = line.translate(&narrow, 0.0).translate(&wide, 0.0);
    let (name, message) = parameter_refusal(Err::<(), _>(held(&drawing)?))?;
    assert!(name.contains("slide") && message.contains("max"), "{name}: {message}");
    // the same Param twice is one
    let twice = line.translate(&narrow, &narrow);
    assert_eq!(twice.error(), None);
    assert_eq!(twice.parameters()?, vec![narrow]);
    Ok(())
}

// Rotate ------------------------------------------------------------------------------------------------------------

/// A square, 10 by 10, with its corner at the origin.
fn square() -> Result<Path, Error> {
    Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)
}

#[test]
fn rotating_turns_clockwise_about_the_given_point_or_the_middle_of_the_bounds() -> Outcome {
    let none = Values::new();
    // a quarter turn about the corner: (x, y) goes to (-y, x), the y axis pointing down
    let about_corner = Drawing::from(square()?).rotate(Rotate::new(Deg(90.0)).about((0.0, 0.0)));
    assert_eq!(data(&about_corner, &none)?, "M0 0 L0 10 L-10 10 L-10 0 Z");
    // about nothing given, the square turns about its middle (5, 5) and lands on itself, its start corner moved on
    let about_middle = Drawing::from(square()?).rotate(Deg(90.0));
    assert_eq!(data(&about_middle, &none)?, "M10 0 L10 10 L0 10 L0 0 Z");
    // a point can be a Point
    let about_point = Drawing::from(square()?).rotate(Rotate::new(Deg(90.0)).about(Point::new(0.0, 0.0)));
    assert_eq!(data(&about_point, &none)?, "M0 0 L0 10 L-10 10 L-10 0 Z");
    Ok(())
}

#[test]
fn an_angle_of_any_size_wraps_and_multiples_of_ninety_are_exact() -> Outcome {
    let none = Values::new();
    let base = Drawing::from(square()?);
    let turned = |angle: f64| data(&base.rotate(Deg(angle)), &none);
    let quarter = turned(90.0)?;
    for angle in [450.0, -270.0, 90.0 + 360.0 * 1000.0, 90.0 - 720.0] {
        assert_eq!(turned(angle)?, quarter, "{angle}");
    }
    assert_eq!(turned(360.0)?, turned(0.0)?);
    assert_eq!(turned(0.0)?, "M0 0 L10 0 L10 10 L0 10 Z");
    assert_eq!(turned(180.0)?, "M10 10 L0 10 L0 0 L10 0 Z");
    Ok(())
}

#[test]
fn rotating_by_a_param_or_an_expression_reads_it_when_drawn() -> Outcome {
    let spin = Param::new("spin", 0.0, 0.0, 360.0)?;
    let drawing = Drawing::from(square()?).rotate(Rotate::new(Deg(2.0 * &spin)).about((0.0, 0.0)));
    assert_eq!(data(&drawing, &Values::new())?, "M0 0 L10 0 L10 10 L0 10 Z");
    assert_eq!(
        data(&drawing, &Values::new().set("spin", 45.0))?,
        "M0 0 L0 10 L-10 10 L-10 0 Z"
    );
    // a point whose coordinates read Params
    let pivot = Param::new("pivot", 0.0, -100.0, 100.0)?;
    let about_param =
        Drawing::from(Path::line((0.0, 0.0), (10.0, 0.0))?).rotate(Rotate::new(Deg(180.0)).about((&pivot, 2.0)));
    assert_eq!(data(&about_param, &Values::new().set("pivot", 4.0))?, "M8 4 L-2 4");
    assert_eq!(drawing.parameters()?, vec![spin]);
    Ok(())
}

#[test]
fn an_angle_or_a_pivot_the_step_refuses_is_refused_naming_the_argument() -> Outcome {
    let line = Drawing::from(Path::line((0.0, 0.0), (2.0, 1.0))?);
    // (case, drawing, argument)
    let rows = [
        ("NaN angle", line.rotate(Deg(f64::NAN)), "step 1 (rotate).angle"),
        (
            "infinite angle",
            line.rotate(Deg(f64::INFINITY)),
            "step 1 (rotate).angle",
        ),
        (
            "pivot beyond the coordinates",
            line.rotate(Rotate::new(Deg(1.0)).about((2e100, 0.0))),
            "step 1 (rotate).about",
        ),
        (
            "NaN pivot",
            line.rotate(Rotate::new(Deg(1.0)).about((0.0, f64::NAN))),
            "step 1 (rotate).about",
        ),
    ];
    for (case, drawing, argument) in rows {
        let (name, message) = parameter_refusal(Err::<(), _>(held(&drawing)?))?;
        assert_eq!(name, argument, "{case}: {message}");
    }
    Ok(())
}

// Scale -------------------------------------------------------------------------------------------------------------

#[test]
fn scaling_by_a_number_or_a_pair_about_a_point_or_the_middle() -> Outcome {
    let none = Values::new();
    let base = Drawing::from(square()?);
    let about_origin = |by: ScaleBy| base.scale(by.about((0.0, 0.0)));
    assert_eq!(
        data(&about_origin(ScaleBy::new((2.0, 3.0))), &none)?,
        "M0 0 L20 0 L20 30 L0 30 Z"
    );
    assert_eq!(
        data(&about_origin(ScaleBy::new(2.0)), &none)?,
        "M0 0 L20 0 L20 20 L0 20 Z"
    );
    // a negative factor mirrors
    assert_eq!(
        data(&about_origin(ScaleBy::new(-1.0)), &none)?,
        "M0 0 L-10 0 L-10 -10 L0 -10 Z"
    );
    assert_eq!(
        data(&about_origin(ScaleBy::new((1.0, -0.5))), &none)?,
        "M0 0 L10 0 L10 -5 L0 -5 Z"
    );
    // with no pivot, the middle of the bounds (5, 5) stays where it is
    let about_middle = Drawing::from(square()?).scale(2.0);
    assert_eq!(data(&about_middle, &none)?, "M-5 -5 L15 -5 L15 15 L-5 15 Z");
    Ok(())
}

#[test]
fn a_factor_of_zero_is_refused_at_the_call_and_when_drawn() -> Outcome {
    let line = Drawing::from(Path::line((0.0, 0.0), (2.0, 1.0))?);
    for (case, drawing) in [
        ("number", line.scale(0.0)),
        ("first of a pair", line.scale((0.0, 2.0))),
        ("second of a pair", line.scale((2.0, 0.0))),
        ("negative zero", line.scale(-0.0)),
        ("worked out", line.scale(Expr::from(2.0) - 2.0)),
    ] {
        let (name, message) = parameter_refusal(Err::<(), _>(held(&drawing)?))?;
        assert!(
            name == "step 1 (scale).by" && message.contains('0'),
            "{case}: {name}: {message}"
        );
    }
    let factor = Param::new("factor", 1.0, -10.0, 10.0)?;
    let drawing = Drawing::from(Path::line((0.0, 0.0), (2.0, 1.0))?).scale((2.0, &factor));
    assert_eq!(drawing.error(), None);
    let (name, message) = parameter_refusal(drawing.shapes(&Values::new().set("factor", 0.0)))?;
    assert!(
        name == "step 1 (scale).by" && message.starts_with("shape 0: "),
        "{name}: {message}"
    );
    // any other value, a small one included, is accepted
    assert_eq!(data(&drawing, &Values::new().set("factor", 0.5))?, "M-1 0.25 L3 0.75");
    Ok(())
}

#[test]
fn a_pivot_that_reads_a_param_works_like_a_number() -> Outcome {
    let cx = Param::new("cx", 0.0, -100.0, 100.0)?;
    let drawing = Drawing::from(Path::line((0.0, 0.0), (2.0, 0.0))?).scale(ScaleBy::new(3.0).about((&cx, 0.0)));
    assert_eq!(data(&drawing, &Values::new().set("cx", 1.0))?, "M-2 0 L4 0");
    Ok(())
}
