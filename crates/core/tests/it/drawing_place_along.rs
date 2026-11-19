//! `Drawing::place_along` through the public API: copies of the drawing at even spacing along a path, each turned to the
//! way the path runs, not bent. Written from the usage first (beads on a ring, an arrow every 40 pixels, a word
//! marching round a circle); expected places are by hand (a straight line, a circle's quarter points) and are
//! checked against `Path::point_at` and `Path::tangent_at`, which share no code with the step.

use crate::common::fonts::fraunces;
use crate::common::{Outcome, distance, geometry_refusal, parameter_refusal};
use bullterrier_vector::{Color, Drawing, Inputs, Param, Path, PlaceAlong, Shape, Spacing, Text, Values, text};

fn line(length: f64) -> Result<Path, Box<dyn std::error::Error>> {
    Ok(Path::line((0.0, 0.0), (length, 0.0))?)
}

/// A 2 by 2 square about the origin: a stamp whose middle is where it sits.
fn bead() -> Result<Drawing, Box<dyn std::error::Error>> {
    Ok(Drawing::from(Path::rect(-1.0, -1.0, 2.0, 2.0, 0.0)?))
}

fn center(path: &Path) -> Result<(f64, f64), Box<dyn std::error::Error>> {
    let bounds = path.bounds()?;
    Ok((bounds.x + bounds.width / 2.0, bounds.y + bounds.height / 2.0))
}

fn centers(shapes: &[Shape]) -> Result<Vec<(f64, f64)>, Box<dyn std::error::Error>> {
    shapes.iter().map(|shape| center(shape.path())).collect()
}

fn placed(
    stamp: &Drawing,
    guide: &Path,
    spacing: Spacing,
    options: PlaceAlong,
) -> Result<Vec<Shape>, Box<dyn std::error::Error>> {
    Ok(stamp.place_along(guide, spacing, options).shapes(&Values::new())?)
}

fn near(found: &[(f64, f64)], expected: &[(f64, f64)], tolerance: f64) -> Outcome {
    if found.len() == expected.len() && found.iter().zip(expected).all(|(a, b)| distance(*a, *b) <= tolerance) {
        Ok(())
    } else {
        Err(format!("{found:?} is not {expected:?}").into())
    }
}

// Positions, by hand -----------------------------------------------------------------------------------------------

#[test]
fn a_count_on_an_open_path_includes_both_ends() -> Outcome {
    let found = placed(&bead()?, &line(100.0)?, Spacing::Count(5), PlaceAlong::new())?;
    near(
        &centers(&found)?,
        &[(0.0, 0.0), (25.0, 0.0), (50.0, 0.0), (75.0, 0.0), (100.0, 0.0)],
        1e-9,
    )?;
    // one copy sits at the start
    let one = placed(&bead()?, &line(100.0)?, Spacing::Count(1), PlaceAlong::new())?;
    near(&centers(&one)?, &[(0.0, 0.0)], 1e-9)
}

#[test]
fn a_spacing_on_an_open_path_places_at_0_s_2s_up_to_the_end() -> Outcome {
    let (guide, stamp) = (line(100.0)?, bead()?);
    let every = |px| placed(&stamp, &guide, Spacing::Every(px), PlaceAlong::new());
    near(
        &centers(&every(30.0)?)?,
        &[(0.0, 0.0), (30.0, 0.0), (60.0, 0.0), (90.0, 0.0)],
        1e-9,
    )?;
    // the end is there only where it falls on a multiple
    assert_eq!(every(25.0)?.len(), 5);
    near(&centers(&every(25.0)?)?[4..], &[(100.0, 0.0)], 1e-9)?;
    // a spacing past the whole path leaves the copy at the start
    near(&centers(&every(500.0)?)?, &[(0.0, 0.0)], 1e-9)
}

#[test]
fn a_closed_path_has_no_copy_at_its_end_and_wraps_with_start() -> Outcome {
    let ring = Path::circle((0.0, 0.0), 100.0)?;
    let quarter = [(0.0, -100.0), (100.0, 0.0), (0.0, 100.0), (-100.0, 0.0)];
    let found = placed(&bead()?, &ring, Spacing::Count(4), PlaceAlong::new())?;
    // by hand: the circle starts at its top and runs clockwise; the 0.01 flattening is within 0.05 of the circle
    near(&centers(&found)?, &quarter, 0.05)?;
    // a spacing of a quarter of the circumference gives the same four (the circumference as the path measures it)
    let every = ring.length() / 4.0;
    let by_spacing = placed(&bead()?, &ring, Spacing::Every(every), PlaceAlong::new())?;
    near(&centers(&by_spacing)?, &quarter, 0.05)?;
    // start moves them on, and a whole turn is where they began
    let turned = placed(&bead()?, &ring, Spacing::Count(4), PlaceAlong::new().start(0.25))?;
    near(
        &centers(&turned)?,
        &[(100.0, 0.0), (0.0, 100.0), (-100.0, 0.0), (0.0, -100.0)],
        0.05,
    )?;
    let again = placed(&bead()?, &ring, Spacing::Count(4), PlaceAlong::new().start(1.25))?;
    near(&centers(&again)?, &centers(&turned)?, 1e-6)
}

#[test]
fn a_spacing_on_a_closed_path_makes_whole_gaps_no_closer_than_the_spacing() -> Outcome {
    let ring = Path::circle((0.0, 0.0), 100.0)?;
    // 2 pi r = 628.3: 100 fits six times, so six copies and the last gap is the long one
    let found = placed(&bead()?, &ring, Spacing::Every(100.0), PlaceAlong::new())?;
    assert_eq!(found.len(), 6);
    // a spacing past the circumference leaves one copy
    assert_eq!(
        placed(&bead()?, &ring, Spacing::Every(1000.0), PlaceAlong::new())?.len(),
        1
    );
    Ok(())
}

#[test]
fn start_past_the_ends_of_an_open_path_goes_on_straight() -> Outcome {
    let found = placed(&bead()?, &line(100.0)?, Spacing::Count(2), PlaceAlong::new().start(0.5))?;
    near(&centers(&found)?, &[(50.0, 0.0), (150.0, 0.0)], 1e-9)?;
    let before = placed(
        &bead()?,
        &line(100.0)?,
        Spacing::Count(2),
        PlaceAlong::new().start(-0.5),
    )?;
    near(&centers(&before)?, &[(-50.0, 0.0), (50.0, 0.0)], 1e-9)
}

#[test]
fn the_copies_sit_on_the_points_the_path_gives_for_those_positions() -> Outcome {
    // a path with a curve and a corner: the oracle is Path::point_at, which the step shares no code with
    let wave = Path::from_svg_path("M0 0 C 40 -60 80 60 120 0 L 200 40")?;
    let found = placed(&bead()?, &wave, Spacing::Count(7), PlaceAlong::new())?;
    let expected: Vec<(f64, f64)> = (0..7)
        .map(|i| {
            let point = wave.point_at(f64::from(i) / 6.0, None)?;
            Ok((point.x, point.y))
        })
        .collect::<Result<_, bullterrier_vector::Error>>()?;
    near(&centers(&found)?, &expected, 1e-6)
}

// Direction ------------------------------------------------------------------------------------------------------

#[test]
fn a_copy_turns_so_its_x_axis_runs_along_the_path_and_turn_false_keeps_it() -> Outcome {
    // a bar 20 wide and 2 high, standing along x; the path runs down the screen: the bar must lie along y
    let bar = Drawing::from(Path::rect(-10.0, -1.0, 20.0, 2.0, 0.0)?);
    let down = Path::line((5.0, 0.0), (5.0, 90.0))?;
    for shape in placed(&bar, &down, Spacing::Count(3), PlaceAlong::new())? {
        let bounds = shape.path().bounds()?;
        assert!(
            (bounds.width - 2.0).abs() < 1e-9 && (bounds.height - 20.0).abs() < 1e-9,
            "{bounds:?}"
        );
    }
    for shape in placed(&bar, &down, Spacing::Count(3), PlaceAlong::new().turn(false))? {
        let bounds = shape.path().bounds()?;
        assert!(
            (bounds.width - 20.0).abs() < 1e-9 && (bounds.height - 2.0).abs() < 1e-9,
            "{bounds:?}"
        );
    }
    Ok(())
}

#[test]
fn on_a_circle_every_copy_is_turned_to_the_tangent_there() -> Outcome {
    // the bar's right end (its +x) is the point at 10 along x from the middle; after turning, that end minus the
    // middle must point where tangent_at points
    let bar = Drawing::from(Path::rect(-10.0, -1.0, 20.0, 2.0, 0.0)?);
    let ring = Path::circle((0.0, 0.0), 100.0)?;
    let found = placed(&bar, &ring, Spacing::Count(12), PlaceAlong::new())?;
    for (index, shape) in (0_u32..).zip(&found) {
        let tangent = ring.tangent_at(f64::from(index) / 12.0, None)?;
        // a bar along the tangent: the bounds' long diagonal is the bar's own; compare the points of the outline that
        // the turn sends furthest along the tangent
        let (middle_x, middle_y) = center(shape.path())?;
        let data = crate::common::commands(&shape.path().to_svg_path())?;
        let end = bullterrier_vector::Point::new(data[0].1[0], data[0].1[1]); // first corner: (-10, -1) turned
        // (-10, -1) turned by the tangent (tx, ty): (-10 tx + ty, -10 ty - tx)
        let expected = (
            middle_x - 10.0 * tangent.x + tangent.y,
            middle_y - 10.0 * tangent.y - tangent.x,
        );
        // the circle's flattening makes the walk's edge direction differ from the exact tangent by under a degree
        assert!(
            distance((end.x, end.y), expected) < 0.35,
            "copy {index}: {end:?} vs {expected:?}"
        );
    }
    Ok(())
}

// Stamps of several shapes ------------------------------------------------------------------------------------------

fn arrow() -> Result<Drawing, Box<dyn std::error::Error>> {
    Ok(Drawing::new([
        Path::rect(0.0, -1.0, 30.0, 2.0, 0.0)?,
        Path::rect(30.0, -5.0, 10.0, 10.0, 0.0)?,
    ]))
}

#[test]
fn a_stamp_of_two_shapes_keeps_its_layout_and_sits_by_the_middle_of_its_bounds() -> Outcome {
    // the arrow's bounds are 0..40 by -5..5: its middle (20, 0) goes to each position
    let flat = placed(&arrow()?, &line(120.0)?, Spacing::Every(40.0), PlaceAlong::new())?;
    assert_eq!(flat.len(), 8, "four copies of two shapes");
    for (copy, pair) in flat.chunks(2).enumerate() {
        let (shaft, head) = (center(pair[0].path())?, center(pair[1].path())?);
        let at = 40.0 * f64::from(u32::try_from(copy)?);
        // the shaft's middle was at (15, 0), 5 left of the stamp's middle; the head's (35, 0), 15 right of it
        assert!(distance(shaft, (at - 5.0, 0.0)) < 1e-9, "copy {copy}: {shaft:?}");
        assert!(distance(head, (at + 15.0, 0.0)) < 1e-9, "copy {copy}: {head:?}");
    }
    // along a path running down the screen the layout turns with it: the head is below the shaft
    let down = Path::line((0.0, 0.0), (0.0, 80.0))?;
    let turned = placed(&arrow()?, &down, Spacing::Count(2), PlaceAlong::new())?;
    for (copy, pair) in turned.chunks(2).enumerate() {
        let (shaft, head) = (center(pair[0].path())?, center(pair[1].path())?);
        let at = 80.0 * f64::from(u32::try_from(copy)?);
        assert!(distance(shaft, (0.0, at - 5.0)) < 1e-9, "copy {copy}: {shaft:?}");
        assert!(distance(head, (0.0, at + 15.0)) < 1e-9, "copy {copy}: {head:?}");
    }
    Ok(())
}

// Attributes -----------------------------------------------------------------------------------------------------------

#[test]
fn the_copies_keep_style_and_attributes_and_number_themselves() -> Outcome {
    let stamp = arrow()?.fill(Color::hex(0xff_00_00));
    let found = placed(&stamp, &line(100.0)?, Spacing::Count(3), PlaceAlong::new())?;
    let before = stamp.shapes(&Values::new())?;
    for (index, shape) in found.iter().enumerate() {
        assert_eq!(shape.style(), before[index % 2].style());
        assert_eq!(shape.attributes().copy(), Some(u32::try_from(index / 2)?));
        assert_eq!(shape.index(), index);
        // a many-shape stamp keeps the order its shapes carry: 0 and 1 over the two
        assert_eq!(shape.attributes().order(), before[index % 2].attributes().order());
    }
    Ok(())
}

#[test]
fn a_one_shape_stamp_gets_the_order_of_its_copy_so_by_order_varies_per_copy() -> Outcome {
    let found = placed(&bead()?, &line(100.0)?, Spacing::Count(5), PlaceAlong::new())?;
    let orders: Vec<Option<f64>> = found.iter().map(|shape| shape.attributes().order()).collect();
    assert_eq!(orders, [Some(0.0), Some(0.25), Some(0.5), Some(0.75), Some(1.0)]);
    Ok(())
}

#[test]
fn the_origin_moves_with_the_copy_and_a_shared_origin_is_the_anchor() -> Outcome {
    // one glyph cut to its first third: `slice` leaves the origin where it was, so it is off the middle of what is left
    let glyph = text("H", Text::new(&fraunces()?).size(100.0)).slice(0.0, 0.3);
    let flat = glyph.shapes(&Values::new())?;
    let origin = flat[0].attributes().origin().ok_or("a glyph has an origin")?;
    let middle = center(flat[0].path())?;
    assert!(
        distance((origin.x, origin.y), middle) > 5.0,
        "the test needs an origin off the middle"
    );
    let found = placed(
        &glyph,
        &Path::line((10.0, 20.0), (210.0, 20.0))?,
        Spacing::Count(3),
        PlaceAlong::new(),
    )?;
    for (copy, shape) in found.iter().enumerate() {
        let at = (10.0 + 100.0 * f64::from(u32::try_from(copy)?), 20.0);
        // the origin is what is on the path point, and the glyph keeps where it was from its origin
        let moved = shape.attributes().origin().ok_or("the origin stays")?;
        assert!(distance((moved.x, moved.y), at) < 1e-9, "copy {copy}: {moved:?}");
        let there = center(shape.path())?;
        let from = (middle.0 - origin.x, middle.1 - origin.y);
        assert!(
            distance((there.0 - moved.x, there.1 - moved.y), from) < 1e-9,
            "copy {copy}"
        );
    }
    Ok(())
}

// Params -----------------------------------------------------------------------------------------------------------------

#[test]
fn start_is_a_param_that_moves_every_copy_and_the_copies_stay_the_same_number() -> Outcome {
    let march = Param::new("march", 0.0, 0.0, 1.0)?;
    let ring = Path::circle((0.0, 0.0), 100.0)?;
    let drawing = bead()?.place_along(&ring, Spacing::Count(4), PlaceAlong::new().start(&march));
    let at = |value: f64| -> Result<Vec<(f64, f64)>, Box<dyn std::error::Error>> {
        centers(&drawing.shapes(&Values::new().set("march", value))?)
    };
    near(
        &at(0.0)?,
        &[(0.0, -100.0), (100.0, 0.0), (0.0, 100.0), (-100.0, 0.0)],
        0.05,
    )?;
    near(
        &at(0.25)?,
        &[(100.0, 0.0), (0.0, 100.0), (-100.0, 0.0), (0.0, -100.0)],
        0.05,
    )?;
    near(&at(1.0)?, &at(0.0)?, 1e-6)
}

#[test]
fn start_takes_one_number_not_a_list_per_shape() -> Outcome {
    let march = Param::new("march", 0.0, 0.0, 1.0)?;
    let drawing = bead()?.place_along(&line(10.0)?, Spacing::Count(2), PlaceAlong::new().start(&march));
    let (place, _) = parameter_refusal(drawing.shapes(&Values::new().set("march", vec![0.0, 0.5])))?;
    assert!(place.contains("start"), "{place}");
    Ok(())
}

// The document ---------------------------------------------------------------------------------------------------------

#[test]
fn the_step_saves_and_loads_the_same_drawing() -> Outcome {
    let march = Param::new("march", 0.0, 0.0, 1.0)?;
    let ring = Path::circle((50.0, 50.0), 40.0)?;
    for (spacing, key) in [
        (Spacing::Count(12), r#""count":12"#),
        (Spacing::Every(30.0), r#""spacing":30"#),
    ] {
        let drawing = bead()?.place_along(&ring, spacing, PlaceAlong::new().start(&march).turn(false));
        let json = drawing.to_json()?;
        assert!(json.contains(key) && json.contains(r#""turn":"none""#), "{json}");
        let again = Drawing::from_json(&json, &Inputs::new())?;
        assert_eq!(again.to_json()?, json);
        let values = Values::new().set("march", 0.3);
        assert_eq!(again.shapes(&values)?, drawing.shapes(&values)?);
    }
    // the defaults are left out
    let json = bead()?
        .place_along(&ring, Spacing::Count(3), PlaceAlong::new())
        .to_json()?;
    assert!(
        !json.contains("turn") && !json.contains("start") && !json.contains("near"),
        "{json}"
    );
    Ok(())
}

#[test]
fn a_document_with_both_or_neither_of_count_and_spacing_is_refused_naming_them() -> Outcome {
    let square = Drawing::from(Path::rect(0.0, 0.0, 4.0, 4.0, 0.0)?).to_json()?;
    let with = |arguments: &str| {
        square.replace(
            "}],",
            &format!(r#"}},{{"place_along":{{"path":"M0 0 L9 0",{arguments}}}}}],"#),
        )
    };
    for arguments in [r#""count":3,"spacing":4"#, r#""start":0"#, r#""turn":"along""#] {
        let text = with(arguments);
        let error = Drawing::from_json(&text, &Inputs::new())
            .err()
            .map(|error| error.to_string());
        let message = error.ok_or_else(|| format!("{arguments} was accepted: {text}"))?;
        assert!(message.contains("count") && message.contains("spacing"), "{message}");
    }
    Ok(())
}

// Refusals and limits --------------------------------------------------------------------------------------------------------

#[test]
fn a_count_or_spacing_that_cannot_place_names_the_argument() -> Outcome {
    let guide = line(10.0)?;
    let stamp = bead()?;
    for (spacing, argument) in [
        (Spacing::Count(0), "count"),
        (Spacing::Every(0.0), "spacing"),
        (Spacing::Every(-1.0), "spacing"),
        (Spacing::Every(f64::NAN), "spacing"),
        (Spacing::Every(f64::INFINITY), "spacing"),
        (Spacing::Every(1.000_000_1e100), "spacing"),
    ] {
        let drawing = stamp.place_along(&guide, spacing, PlaceAlong::new());
        let error = drawing.error().map(ToString::to_string).ok_or("no error at the call")?;
        assert!(error.contains(argument) && error.contains("place_along"), "{error}");
    }
    Ok(())
}

#[test]
fn an_empty_path_or_one_without_length_is_a_geometry_error() -> Outcome {
    let stamp = bead()?;
    for guide in [Path::new(), Path::start((1.0, 1.0))?] {
        let drawing = stamp.place_along(&guide, Spacing::Count(2), PlaceAlong::new());
        let (verb, _) = geometry_refusal(drawing.shapes(&Values::new()))?;
        assert!(verb.contains("place_along"), "{verb}");
    }
    Ok(())
}

#[test]
fn too_many_copies_are_refused_before_any_is_made() -> Outcome {
    let guide = Path::line((0.0, 0.0), (1.0e6, 0.0))?;
    for drawing in [
        bead()?.place_along(&guide, Spacing::Every(1.0e-3), PlaceAlong::new()),
        bead()?.place_along(&guide, Spacing::Count(1_000_000_000), PlaceAlong::new()),
        // few copies, but of a stamp that is many shapes: copies x shapes is what is charged
        Drawing::new(
            (0..1000)
                .map(|_| Path::rect(0.0, 0.0, 1.0, 1.0, 0.0))
                .collect::<Result<Vec<_>, _>>()?,
        )
        .place_along(&guide, Spacing::Count(1001), PlaceAlong::new()),
    ] {
        let (place, message) = parameter_refusal(drawing.shapes(&Values::new()))?;
        assert!(
            place.contains("place_along") && message.contains("MAX_"),
            "{place}: {message}"
        );
    }
    Ok(())
}

#[test]
fn a_huge_count_of_an_empty_drawing_returns_at_once() -> Outcome {
    let guide = Path::line((0.0, 0.0), (1.0e6, 0.0))?;
    let empty = Drawing::new(Vec::<Path>::new());
    for spacing in [Spacing::Count(usize::MAX), Spacing::Every(1.0e-9)] {
        // refused or an empty result: either is fine, hanging is not
        if let Ok(shapes) = empty
            .place_along(&guide, spacing, PlaceAlong::new())
            .shapes(&Values::new())
        {
            assert_eq!(shapes.len(), 0);
        }
    }
    Ok(())
}
