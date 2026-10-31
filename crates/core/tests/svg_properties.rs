//! Properties of `Drawing::to_svg` over random sized drawings: the path data reads back as the shape's path moved by
//! the view, and the same drawing at twice the output size has every number exactly twice as large.

#![expect(
    clippy::float_cmp,
    reason = "the properties are about exact numbers: powers of two lose nothing"
)]

mod common;

use bullterrier_vector::{Color, Drawing, Path, Size, Stroke, Values};
use common::{Command, build, commands, log_uniform, step};
use proptest::prelude::*;

type Failure = Box<dyn std::error::Error>;

/// What the SVG says: the path data and stroke width of each element, and the width and height of the root.
struct Document {
    elements: Vec<(String, f64)>,
    sides: [f64; 2],
}

fn fail(error: impl std::fmt::Display) -> TestCaseError {
    TestCaseError::fail(error.to_string())
}

/// What one random drawing is made of: for each shape its start, its steps and the width of its stroke, and the
/// design frame.
#[derive(Debug, Clone)]
struct Scene {
    shapes: Vec<((f64, f64), Vec<common::Step>, f64)>,
    frame: (f64, f64),
}

fn scenes() -> impl Strategy<Value = Scene> {
    let point = (-1000.0f64..1000.0, -1000.0f64..1000.0);
    let shape = (
        point.clone(),
        prop::collection::vec(step(point), 0..8),
        log_uniform(-1.0, 2.0),
    );
    (
        prop::collection::vec(shape, 1..4),
        (log_uniform(0.0, 3.5), log_uniform(0.0, 3.5)),
    )
        .prop_map(|(shapes, frame)| Scene { shapes, frame })
}

/// The drawing of the scene: every shape an unfilled outline with a black stroke of its own width.
fn drawing(scene: &Scene) -> Result<Drawing, Failure> {
    let mut items = Vec::new();
    for (start, steps, width) in &scene.shapes {
        let black = Color::hex(0);
        items.push(
            Drawing::from(build(*start, steps)?)
                .fill(Color::NONE)
                .stroke(Stroke::new(black).width(*width)),
        );
    }
    Ok(Drawing::new(items).with_size(scene.frame.0, scene.frame.1))
}

/// The path data and the stroke width of every element of the SVG, and the numbers of the root.
fn read(svg: &str) -> Result<Document, Failure> {
    let document = roxmltree::Document::parse(svg)?;
    let root = document.root_element();
    let side =
        |name: &str| -> Result<f64, Failure> { Ok(root.attribute(name).ok_or("a root without a size")?.parse()?) };
    let sides = [side("width")?, side("height")?];
    let mut elements = Vec::new();
    for element in root.children().filter(roxmltree::Node::is_element) {
        let data = element.attribute("d").ok_or("no path data")?.to_owned();
        let width = element.attribute("stroke-width").ok_or("no stroke width")?.parse()?;
        elements.push((data, width));
    }
    Ok(Document { elements, sides })
}

proptest! {
    // The data of each element is the path of the shape, scaled by the fit of the frame in the output and centred, to
    // within rounding.
    #[test]
    fn each_element_reads_back_as_its_shape_moved_by_the_view(
        scene in scenes(),
        output in (1u32..=4000, 1u32..=4000),
    ) {
        let drawing = drawing(&scene).map_err(fail)?;
        let (width, height) = (f64::from(output.0), f64::from(output.1));
        let svg = drawing.to_svg(Size::px(width, height), &Values::new()).map_err(fail)?;
        let elements = read(&svg).map_err(fail)?.elements;
        let shapes: Vec<_> = drawing
            .shapes(&Values::new())
            .map_err(fail)?
            .into_iter()
            .filter(|shape| *shape.path() != Path::default())
            .collect();
        prop_assert_eq!(elements.len(), shapes.len());
        let (frame_width, frame_height) = scene.frame;
        let scale = (width / frame_width).min(height / frame_height);
        let (left, top) = ((width - frame_width * scale) / 2.0, (height - frame_height * scale) / 2.0);
        for ((data, stroke_width), shape) in elements.iter().zip(&shapes) {
            let moved = shape
                .path()
                .map_points(|point| Ok(bullterrier_vector::Point::new(point.x * scale + left, point.y * scale + top)))
                .map_err(fail)?;
            // the same commands and, number by number, the same values to 1e-9 of their size: the writer centres the
            // frame without the rounding residue this formula leaves on the axis that fits exactly
            let (written, expected) = (commands(data).map_err(fail)?, commands(&moved.to_svg_path()).map_err(fail)?);
            prop_assert_eq!(written.len(), expected.len());
            for ((letter, numbers), (letter_expected, numbers_expected)) in written.iter().zip(&expected) {
                prop_assert_eq!(letter, letter_expected);
                prop_assert_eq!(numbers.len(), numbers_expected.len());
                for (number, number_expected) in numbers.iter().zip(numbers_expected) {
                    prop_assert!(
                        (number - number_expected).abs() <= 1e-9 * (1.0 + number_expected.abs()),
                        "{} is not {}",
                        number,
                        number_expected
                    );
                }
            }
            let drawn = shape.style().stroke().map(bullterrier_vector::StrokeStyle::width);
            prop_assert_eq!(Some(*stroke_width), drawn.map(|design| design * scale));
        }
    }

    // At twice the output size every number of the document is exactly twice as large (the miter limit is a ratio and
    // stays): scaling by a power of two is exact, so no rounding differs.
    #[test]
    fn at_twice_the_output_size_every_number_is_exactly_twice(
        scene in scenes(),
        output in (1u32..=4000, 1u32..=4000),
    ) {
        let drawing = drawing(&scene).map_err(fail)?;
        let (width, height) = (f64::from(output.0), f64::from(output.1));
        let once = drawing.to_svg(Size::px(width, height), &Values::new()).map_err(fail)?;
        let twice = drawing.to_svg(Size::px(2.0 * width, 2.0 * height), &Values::new()).map_err(fail)?;
        let (once, twice) = (read(&once).map_err(fail)?, read(&twice).map_err(fail)?);
        prop_assert_eq!(twice.sides, once.sides.map(|side| 2.0 * side));
        prop_assert_eq!(once.elements.len(), twice.elements.len());
        for ((data, stroke_width), (data_2x, stroke_width_2x)) in once.elements.iter().zip(&twice.elements) {
            prop_assert_eq!(*stroke_width_2x, 2.0 * stroke_width);
            let (before, after): (Vec<Command>, Vec<Command>) =
                (commands(data).map_err(fail)?, commands(data_2x).map_err(fail)?);
            prop_assert_eq!(before.len(), after.len());
            for ((letter, numbers), (letter_2x, numbers_2x)) in before.iter().zip(&after) {
                prop_assert_eq!(letter, letter_2x);
                prop_assert_eq!(numbers_2x, &numbers.iter().map(|number| 2.0 * number).collect::<Vec<_>>());
            }
        }
    }
}
