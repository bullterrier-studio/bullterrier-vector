//! `Drawing::bounds` through the public API: the union of the bounds of the shapes a drawing draws. Expected values
//! are worked out by hand.

use crate::common::{Outcome, geometry_refusal};
use bullterrier_vector::{Deg, Drawing, Param, Path, Rect, Values};

fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
    Rect { x, y, width, height }
}

#[test]
fn the_bounds_of_a_stack_are_the_union_of_its_shapes() -> Outcome {
    let stack = Drawing::new([
        Path::rect(10.0, 20.0, 5.0, 5.0, 0.0)?,
        Path::rect(-4.0, 0.0, 6.0, 8.0, 0.0)?,
    ]);
    // x from -4 to 15, y from 0 to 25
    assert_eq!(stack.bounds(&Values::new())?, rect(-4.0, 0.0, 19.0, 25.0));
    Ok(())
}

#[test]
fn a_rotated_and_translated_shape_is_measured_where_it_lands() -> Outcome {
    // a 10 by 4 rectangle turned 90 degrees about its centre (5, 2) is 4 wide and 10 high, then moved by (100, 0)
    let drawing = Drawing::from(Path::rect(0.0, 0.0, 10.0, 4.0, 0.0)?)
        .rotate(Deg(90.0))
        .translate(100.0, 0.0);
    let found = drawing.bounds(&Values::new())?;
    for (got, want) in [
        (found.x, 103.0),
        (found.y, -3.0),
        (found.width, 4.0),
        (found.height, 10.0),
    ] {
        assert!((got - want).abs() < 1e-9, "{found:?}");
    }
    Ok(())
}

#[test]
fn bounds_follow_the_values_a_param_is_given() -> Outcome {
    let slide = Param::new("slide", 0.0, -100.0, 100.0)?;
    let drawing = Drawing::from(Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)?).translate(&slide, 0.0);
    assert_eq!(drawing.bounds(&Values::new())?, rect(0.0, 0.0, 10.0, 10.0));
    assert_eq!(
        drawing.bounds(&Values::new().set("slide", 30.0))?,
        rect(30.0, 0.0, 10.0, 10.0)
    );
    Ok(())
}

#[test]
fn a_drawing_with_no_shapes_has_no_bounds() -> Outcome {
    let (verb, _) = geometry_refusal(Drawing::new(Vec::<Path>::new()).bounds(&Values::new()))?;
    assert_eq!(verb, "bounds");
    Ok(())
}
