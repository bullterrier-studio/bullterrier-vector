//! The scenes `Drawing::to_rgba` must neither fail on nor hand the rasteriser: shapes off the canvas, huge shapes,
//! shapes with nothing to draw, and coordinates past what the rasteriser computes exactly (its arithmetic is f32).

mod common;

use bullterrier_vector::{Cap, Color, Drawing, Join, Path, Size, Stroke, Values};
use common::pixels::render;
use common::{Outcome, parameter_refusal};
use proptest::prelude::*;

fn red() -> Result<Color, bullterrier_vector::Error> {
    Color::parse("#ff0000")
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> Result<Drawing, bullterrier_vector::Error> {
    Ok(Drawing::from(Path::rect(x, y, width, height, 0.0)?).fill(red()?))
}

#[test]
fn shapes_entirely_off_the_canvas_draw_nothing() -> Outcome {
    for (x, y) in [
        (500.0, 500.0),
        (-600.0, 2.0),
        (2.0, -600.0),
        (10.0, 0.0),
        (0.0, 10.0),
        (-4.0, 0.0),
    ] {
        let image = render(&rect(x, y, 4.0, 4.0)?, 10, 10)?;
        assert!(image.bytes.iter().all(|&byte| byte == 0), "rectangle at ({x}, {y})");
    }
    Ok(())
}

#[test]
fn a_rectangle_a_million_pixels_across_is_a_background() -> Outcome {
    // 1e6 pixels is far past the canvas and far inside what the rasteriser computes exactly
    let image = render(&rect(-500_000.0, -500_000.0, 1e6, 1e6)?, 30, 20)?;
    assert!(image.all_pixels_are([255, 0, 0, 255]));
    Ok(())
}

#[test]
fn a_shape_with_no_outline_draws_nothing() -> Outcome {
    let stroke = Stroke::new(Color::parse("#000000")?).width(6.0).cap(Cap::Round);
    for (case, path) in [
        ("an empty path", Path::new()),
        ("a lone move", Path::start((5.0, 5.0))?),
    ] {
        let image = render(&Drawing::from(path).fill(red()?).stroke(stroke.clone()), 10, 10)?;
        assert!(image.bytes.iter().all(|&byte| byte == 0), "{case}");
    }
    Ok(())
}

#[test]
fn a_coordinate_past_ten_million_device_pixels_is_refused_naming_the_shape() -> Outcome {
    // (case, the shape that is too far, the design frame the drawing has, the output side); it is drawn second, over a
    // small rectangle, and it is the one named
    let line = || Path::line((0.0, 0.0), (1.0, 1.0));
    let rows = [
        ("past on the right", rect(0.0, 0.0, 1.0e7 + 1.0, 4.0)?, None, 10.0),
        ("past on the left", rect(-1.0e7 - 1.0, 0.0, 4.0, 4.0)?, None, 10.0),
        ("past above", rect(0.0, -1.0e7 - 1.0, 4.0, 4.0)?, None, 10.0),
        (
            "a coordinate the view scales past it: 2e5 in a frame of 1 drawn 100 across",
            rect(0.0, 0.0, 2.0e5, 1.0)?,
            Some((1.0, 1.0)),
            100.0,
        ),
        (
            "a stroke the view scales past it",
            Drawing::from(line()?)
                .fill(Color::NONE)
                .stroke(Stroke::new(red()?).width(2.0e5)),
            Some((1.0, 1.0)),
            100.0,
        ),
    ];
    for (case, far, frame, side) in rows {
        let mut drawing = Drawing::new([rect(0.0, 0.0, 2.0, 2.0)?, far]);
        if let Some((width, height)) = frame {
            drawing = drawing.with_size(width, height);
        }
        let (parameter, message) = parameter_refusal(drawing.to_rgba(Size::px(side, side), &Values::new()))?;
        assert_eq!(parameter, "shape 1", "{case}");
        assert!(message.contains("10000000"), "{case}: {message}");
    }
    Ok(())
}

#[test]
fn coordinates_up_to_ten_million_are_drawn() -> Outcome {
    // the bound itself is allowed; a rectangle from -1e7 to 1e7 covers a small canvas
    let image = render(&rect(-1.0e7, -1.0e7, 2.0e7, 2.0e7)?, 8, 8)?;
    assert!(image.all_pixels_are([255, 0, 0, 255]));
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Whatever lies within the bound is drawn without the rasteriser failing.
    #[test]
    fn no_shape_within_the_bound_stops_the_rasteriser(
        points in prop::collection::vec((-1.0e7f64..1.0e7, -1.0e7f64..1.0e7), 2..6),
        width in prop_oneof![0.001f64..1.0, 1.0f64..1.0e7],
        join in 0usize..3,
        cap in 0usize..3,
        closed in any::<bool>(),
    ) {
        let path = Path::polyline(points, closed).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let stroke = Stroke::new(red().map_err(|e| TestCaseError::fail(e.to_string()))?)
            .width(width)
            .join([Join::Round, Join::Miter, Join::Bevel][join])
            .cap([Cap::Butt, Cap::Round, Cap::Square][cap]);
        let drawing = Drawing::from(path).fill(Color::parse("#00ff00").map_err(|e| TestCaseError::fail(e.to_string()))?).stroke(stroke);
        let image = render(&drawing, 16, 16).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(image.bytes.len(), 16 * 16 * 4);
    }
}

#[test]
fn flattening_more_than_ten_million_edges_in_a_draw_is_refused_naming_the_shape_and_the_counter() -> Outcome {
    // a curve whose control points are a billion pixels away is halved to the depth cap, 65,536 edges; a path of 160 of
    // them is over the 10,000,000 that one draw may flatten, and the shape is named when the counter passes it
    let mut path = Path::start((0.0, 0.0))?;
    for index in 0..160 {
        let end = if index % 2 == 0 { (100.0, 0.0) } else { (0.0, 0.0) };
        path = path.curve_to((1.0e9, 1.0e9), (-1.0e9, 1.0e9), end)?;
    }
    let drawing = Drawing::new([rect(0.0, 0.0, 2.0, 2.0)?, Drawing::from(path).fill(red()?)]);
    let (parameter, message) = parameter_refusal(drawing.to_rgba(Size::px(10.0, 10.0), &Values::new()))?;
    assert_eq!(parameter, "shape 1");
    assert!(message.contains("MAX_SEGMENTS 10000000"), "{message}");
    Ok(())
}

#[test]
fn the_outline_of_one_shapes_stroke_is_refused_past_max_segments() -> Outcome {
    // 1,000 points that turn back on themselves, joined round at the widest stroke the bound allows: each join is an arc
    // of about 25,000 edges, 25 million in all, though the path itself is 1,000 edges; the shape is named. (The outlines of
    // different shapes are not added up: thousands of stroked circles are millions of edges and are drawn)
    let zigzag: Vec<(f64, f64)> = (0..1000u32)
        .map(|index| (500.0 + f64::from(index % 2) * 3.0, 500.0 + f64::from(index) * 0.001))
        .collect();
    let stroke = Stroke::new(red()?).width(1.0e7).join(Join::Round).cap(Cap::Butt);
    let drawing = Drawing::from(Path::polyline(zigzag, false)?)
        .fill(Color::NONE)
        .stroke(stroke);
    let (parameter, message) = parameter_refusal(drawing.to_rgba(Size::px(1000.0, 1000.0), &Values::new()))?;
    assert_eq!(parameter, "shape 0");
    assert!(message.contains("MAX_SEGMENTS 10000000"), "{message}");
    Ok(())
}
