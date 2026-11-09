//! A gradient in the mesh `Drawing::to_mesh` fills: refused, naming the argument and the shape, where a knockout (which
//! ignores its paint) and a colour next to it are not.

use crate::common::mesh::{mesh_of, same_triangles};
use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Color, Drawing, Error, Gradient, Path, Stroke};

const RED: Color = Color::hex(0xff_00_00);
const BLUE: Color = Color::hex(0x00_00_ff);

fn rect() -> Result<Path, Error> {
    Path::rect(10.0, 10.0, 30.0, 20.0, 0.0)
}

fn fade() -> Result<Gradient, Error> {
    Gradient::linear((0.0, 0.0), (60.0, 0.0), [(0.0, RED), (1.0, BLUE)])
}

#[test]
fn a_gradient_fill_or_stroke_is_refused_naming_the_argument_and_the_shape() -> Outcome {
    let rows = [
        ("fill", Drawing::from(rect()?).fill(fade()?), "fill"),
        ("stroke", Drawing::from(rect()?).stroke(fade()?), "stroke"),
        (
            "a stroke on a colour fill",
            Drawing::from(rect()?).fill(RED).stroke(Stroke::new(fade()?).width(3.0)),
            "stroke",
        ),
        (
            "the second shape",
            Drawing::new([Drawing::from(rect()?).fill(RED), Drawing::from(rect()?).fill(fade()?)]),
            "fill",
        ),
    ];
    for (case, drawing, argument) in rows {
        let (parameter, message) = parameter_refusal(mesh_of(&drawing, 60.0, 60.0))?;
        assert_eq!(parameter, argument, "{case}");
        assert!(
            message.contains("gradient") && message.contains("to_rgba"),
            "{case}: {message}"
        );
    }
    let (_, message) = parameter_refusal(mesh_of(
        &Drawing::new([Drawing::from(rect()?).fill(RED), Drawing::from(rect()?).fill(fade()?)]),
        60.0,
        60.0,
    ))?;
    assert!(message.contains("shape 1"), "{message}");
    Ok(())
}

#[test]
fn a_shape_that_knocks_out_cuts_the_same_with_a_gradient_as_with_a_colour() -> Outcome {
    let paper = || -> Result<Drawing, Error> { Ok(Drawing::from(Path::rect(0.0, 0.0, 60.0, 60.0, 0.0)?).fill(RED)) };
    let stroke = |paint: Stroke| paint.width(3.0);
    let with_gradient = Drawing::new([
        paper()?,
        Drawing::from(rect()?)
            .fill(fade()?)
            .stroke(stroke(Stroke::new(fade()?)))
            .knockout(),
    ]);
    let with_colour = Drawing::new([
        paper()?,
        Drawing::from(rect()?)
            .fill(BLUE)
            .stroke(stroke(Stroke::new(BLUE)))
            .knockout(),
    ]);
    same_triangles(
        &mesh_of(&with_gradient, 60.0, 60.0)?,
        &mesh_of(&with_colour, 60.0, 60.0)?,
    )?;
    Ok(())
}

#[test]
fn a_gradient_on_a_shape_with_no_path_draws_nothing_and_is_not_refused() -> Outcome {
    let drawing = Drawing::from(Path::new()).fill(fade()?);
    assert_eq!(mesh_of(&drawing, 60.0, 60.0)?.indices(), []);
    Ok(())
}
