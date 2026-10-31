//! `Drawing::to_rgba` through the public API: the size it takes and refuses, the image it returns for nothing drawn,
//! and where the view puts the design frame. Expected values are written by hand or worked out in closed form, never
//! read from the code under test.

mod common;

use bullterrier_vector::{Color, Drawing, Path, Size, Values};
use common::pixels::{pixel_size_refusals, render};
use common::{Outcome, parameter_refusal};

fn empty() -> Drawing {
    Drawing::new(Vec::<Drawing>::new())
}

fn rgba(drawing: &Drawing, width: f64, height: f64) -> Result<Vec<u8>, bullterrier_vector::Error> {
    drawing.to_rgba(Size::px(width, height), &Values::new())
}

#[test]
fn nothing_drawn_is_transparent_black_of_the_stated_length() -> Outcome {
    for (width, height) in [(1, 1), (7, 5), (5, 7)] {
        let image = render(&empty(), width, height)?;
        assert_eq!(image.bytes.len(), width * height * 4, "{width} by {height}");
        assert!(image.bytes.iter().all(|&byte| byte == 0), "{width} by {height}");
    }
    Ok(())
}

#[test]
fn a_size_outside_whole_pixels_from_1_to_65000_is_refused_naming_the_side() -> Outcome {
    for (case, width, height, side, shown) in pixel_size_refusals() {
        let (parameter, message) = parameter_refusal(rgba(&empty(), width, height))?;
        assert_eq!(parameter, side, "{case}");
        assert!(message.contains(shown), "{case}: {message}");
    }
    Ok(())
}

#[test]
fn the_largest_sides_are_accepted_with_a_shape_over_the_whole_canvas() -> Outcome {
    // the rasteriser panics past about 65,400 when something is drawn at the far edge: the limit keeps clear of it
    let everything = Drawing::from(Path::rect(-10.0, -10.0, 70_000.0, 70_000.0, 0.0)?).fill(Color::parse("#336699")?);
    for (width, height) in [(65_000, 1), (1, 65_000)] {
        let image = render(&everything, width, height)?;
        assert_eq!(image.bytes.len(), width * height * 4);
        assert_eq!(image.pixel(width - 1, height - 1), [0x33, 0x66, 0x99, 255]);
        assert_eq!(image.pixel(0, 0), [0x33, 0x66, 0x99, 255]);
    }
    Ok(())
}

#[test]
fn a_bad_size_is_refused_before_the_drawing_is_evaluated() -> Outcome {
    let broken = Drawing::from(Path::circle((0.0, 0.0), 5.0)?).opacity(2.0);
    assert!(broken.error().is_some());
    let (parameter, _) = parameter_refusal(rgba(&broken, 0.0, 10.0))?;
    assert_eq!(parameter, "width");
    Ok(())
}

#[test]
fn rows_run_from_the_top_and_the_design_frame_is_fitted_and_centred_as_in_svg() -> Outcome {
    // a 100 by 50 frame in a 200 by 200 output is doubled and centred down: the frame fills rows 50..150 of 200. Its
    // red rectangle covers the frame's top left quarter: columns 0..100 and rows 50..100 (the SVG table in svg_view)
    let quarter = Drawing::from(Path::rect(0.0, 0.0, 50.0, 25.0, 0.0)?)
        .fill(Color::parse("#ff0000")?)
        .with_size(100.0, 50.0);
    let image = render(&quarter, 200, 200)?;
    for (x, y, alpha) in [
        (0, 49, 0),
        (0, 50, 255),
        (99, 99, 255),
        (100, 99, 0),
        (99, 100, 0),
        (150, 120, 0),
        (0, 150, 0),
    ] {
        assert_eq!(image.alpha(x, y), alpha, "column {x} row {y}");
    }
    assert_eq!(image.pixel(10, 60), [255, 0, 0, 255]);
    Ok(())
}
