//! Where `Drawing::to_svg` puts the design frame in the output: fit and centre, one decision for every output. The
//! expected coordinates are worked by hand for each row.

mod common;

use bullterrier_vector::{Drawing, Path, Size, Values};
use common::Outcome;

/// A three-point line: the origin, a point inside any frame below, and one above and left of the frame.
fn line() -> Result<Path, bullterrier_vector::Error> {
    Path::polyline([(0.0, 0.0), (10.0, 20.0), (-10.0, -5.0)], false)
}

/// The path data of the only element `drawing` is written as at `width` by `height`.
fn data(drawing: &Drawing, width: f64, height: f64) -> Result<String, Box<dyn std::error::Error>> {
    let text = drawing.to_svg(Size::px(width, height), &Values::new())?;
    let document = roxmltree::Document::parse(&text)?;
    let mut paths = document.descendants().filter(|node| node.has_tag_name("path"));
    let path = paths.next().ok_or("no path element")?;
    if paths.next().is_some() {
        return Err("more than one path element".into());
    }
    Ok(path.attribute("d").ok_or("no d attribute")?.to_owned())
}

#[test]
fn the_design_frame_is_scaled_to_fit_the_output_and_centred() -> Outcome {
    // (rule, design frame, output, the path data: the points (0,0), (10,20) and (-10,-5) moved by hand)
    let rows = [
        (
            "same size: untouched",
            Some((400.0, 400.0)),
            (400.0, 400.0),
            "M0 0 L10 20 L-10 -5",
        ),
        (
            "wider output: scale 1, centred across",
            Some((400.0, 400.0)),
            (800.0, 400.0),
            "M200 0 L210 20 L190 -5",
        ),
        (
            "taller output: scale 1, centred down",
            Some((400.0, 400.0)),
            (400.0, 800.0),
            "M0 200 L10 220 L-10 195",
        ),
        (
            "wide frame in a square: scale 2, centred down",
            Some((100.0, 50.0)),
            (200.0, 200.0),
            "M0 50 L20 90 L-20 40",
        ),
        (
            "tall frame in a square: scale 2, centred across",
            Some((50.0, 100.0)),
            (200.0, 200.0),
            "M50 0 L70 40 L30 -10",
        ),
        (
            "smaller output: halved",
            Some((100.0, 100.0)),
            (50.0, 50.0),
            "M0 0 L5 10 L-5 -2.5",
        ),
        (
            "height decides: scale 0.5",
            Some((200.0, 100.0)),
            (100.0, 100.0),
            "M0 25 L5 35 L-5 22.5",
        ),
        (
            "width decides: scale 1",
            Some((50.0, 100.0)),
            (100.0, 100.0),
            "M25 0 L35 20 L15 -5",
        ),
        (
            "tall frame in a wide output: scale 2",
            Some((100.0, 200.0)),
            (400.0, 400.0),
            "M100 0 L120 40 L80 -10",
        ),
        (
            "no frame: one output pixel to the design pixel",
            None,
            (300.0, 200.0),
            "M0 0 L10 20 L-10 -5",
        ),
    ];
    for (rule, frame, (width, height), expected) in rows {
        let drawing = match frame {
            Some((frame_width, frame_height)) => Drawing::from(line()?).with_size(frame_width, frame_height),
            None => Drawing::from(line()?),
        };
        assert_eq!(data(&drawing, width, height)?, expected, "{rule}");
    }
    Ok(())
}
