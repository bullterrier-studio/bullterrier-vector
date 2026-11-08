//! `offset` and `outline` as steps of a `Drawing` through the public API. Expected values are bounds and areas worked
//! out by hand (the shoelace sum in `common`); a document round trips through JSON. The geometry itself is tested on
//! `Path` (`path_offset`), and the steps against it in `node_contract`.

use crate::common::fonts::fraunces;
use crate::common::{Outcome, format_refusal, net_area, parameter_refusal, rings};
use bullterrier_vector::{
    Cap, Color, Drawing, Error, Inputs, Join, Param, Path, Shape, Size, Stroke, Values, by_index, drawing_schema, text,
    verbs,
};
use std::f64::consts::PI;

const TOLERANCE: f64 = 0.01;
const LIMIT: f64 = 4.0;

fn square(x: f64, size: f64) -> Result<Path, Error> {
    Path::rect(x, 0.0, size, size, 0.0)
}

fn drawn(drawing: &Drawing) -> Result<Vec<Shape>, Error> {
    drawing.shapes(&Values::new())
}

/// The area every shape fills.
fn area(shape: &Shape) -> Result<f64, Box<dyn std::error::Error>> {
    Ok(net_area(&rings(shape.path())?))
}

/// `(left, top, right, bottom)` of the shape.
fn extent(shape: &Shape) -> Result<(f64, f64, f64, f64), Error> {
    let r = shape.path().bounds()?;
    Ok((r.x, r.y, r.x + r.width, r.y + r.height))
}

fn assert_near(actual: f64, wanted: f64, slack: f64, what: &str) {
    assert!(
        (actual - wanted).abs() <= slack,
        "{what}: {actual} is not {wanted} within {slack}"
    );
}

fn assert_extent(shape: &Shape, wanted: [f64; 4], what: &str) -> Outcome {
    let found = extent(shape)?;
    for (got, want) in [found.0, found.1, found.2, found.3].into_iter().zip(wanted) {
        assert_near(got, want, 1e-9, what);
    }
    Ok(())
}

// What the steps do ----------------------------------------------------------------------------------------------------

#[test]
fn offset_moves_the_edge_by_the_distance_and_the_join_shapes_the_corners() -> Outcome {
    // a 10 square grown by 2: the sides add 4 x 10 x 2 = 80, and each of the 4 corners adds its own piece
    let drawing = Drawing::from(square(0.0, 10.0)?);
    for (join, corner, name) in [
        (Join::Miter, 4.0, "miter: the 2 x 2 square"),
        (Join::Bevel, 2.0, "bevel: the triangle half of it"),
        (Join::Round, PI, "round: a quarter of the disc of radius 2"),
    ] {
        let grown = drawn(&drawing.offset(2.0, join, LIMIT, TOLERANCE))?;
        assert_eq!(grown.len(), 1, "{name}");
        assert_extent(&grown[0], [-2.0, -2.0, 12.0, 12.0], name)?;
        assert_near(area(&grown[0])?, 100.0 + 80.0 + 4.0 * corner, 0.1, name);
    }
    Ok(())
}

#[test]
fn a_negative_distance_shrinks_and_a_shape_shrunk_past_nothing_stays_in_the_list_with_an_empty_path() -> Outcome {
    let drawing = Drawing::new([square(0.0, 10.0)?, square(20.0, 4.0)?]);
    let shrunk = drawn(&drawing.offset(-3.0, Join::Miter, LIMIT, TOLERANCE))?;
    assert_eq!(shrunk.len(), 2);
    assert_extent(&shrunk[0], [3.0, 3.0, 7.0, 7.0], "10 square less 3 each side")?;
    assert_eq!(
        shrunk[1].path().to_svg_path(),
        "",
        "a 4 square less 3 each side is nothing"
    );
    Ok(())
}

#[test]
fn a_distance_for_each_shape_gives_every_shape_its_own_edge() -> Outcome {
    let drawing = Drawing::new([square(0.0, 10.0)?, square(30.0, 10.0)?, square(60.0, 10.0)?]);
    let grown = drawn(&drawing.offset(by_index(1.0, 3.0), Join::Miter, LIMIT, TOLERANCE))?;
    assert_extent(&grown[0], [-1.0, -1.0, 11.0, 11.0], "first, 1")?;
    assert_extent(&grown[1], [28.0, -2.0, 42.0, 12.0], "middle, 2")?;
    assert_extent(&grown[2], [57.0, -3.0, 73.0, 13.0], "last, 3")?;
    let widths = drawn(&drawing.outline(by_index(2.0, 6.0), Join::Miter, Cap::Butt, LIMIT, TOLERANCE))?;
    // the outline of a closed square reaches half its width out
    assert_extent(&widths[0], [-1.0, -1.0, 11.0, 11.0], "first, 2 wide")?;
    assert_extent(&widths[1], [28.0, -2.0, 42.0, 12.0], "middle, 4 wide")?;
    assert_extent(&widths[2], [57.0, -3.0, 73.0, 13.0], "last, 6 wide")?;
    Ok(())
}

#[test]
fn a_distance_that_reads_a_param_follows_the_value() -> Outcome {
    let grow = Param::new("grow", 0.0, -4.0, 4.0)?;
    let drawing = Drawing::from(square(0.0, 10.0)?).offset(&grow, Join::Miter, LIMIT, TOLERANCE);
    assert_eq!(drawing.parameters()?, vec![grow.clone()]);
    for (value, wanted) in [
        (0.0, [0.0, 0.0, 10.0, 10.0]),
        (3.0, [-3.0, -3.0, 13.0, 13.0]),
        (-4.0, [4.0, 4.0, 6.0, 6.0]),
    ] {
        let shapes = drawing.shapes(&Values::new().set("grow", value))?;
        assert_extent(&shapes[0], wanted, &format!("grow {value}"))?;
    }
    Ok(())
}

#[test]
fn outline_paints_the_region_a_stroke_covers_and_its_caps_and_joins_choose_the_ends_and_corners() -> Outcome {
    let line = Drawing::from(Path::from_svg_path("M0 0 L10 0")?);
    // a 4 wide stroke along 10: the body is 40, a round end adds a half disc of radius 2, a square end a 2 x 4 box
    for (cap, area_wanted, name) in [
        (Cap::Butt, 40.0, "butt"),
        (Cap::Square, 56.0, "square"),
        (Cap::Round, 40.0 + 4.0 * PI, "round"),
    ] {
        let outlined = drawn(&line.outline(4.0, Join::Round, cap, LIMIT, TOLERANCE))?;
        assert_near(area(&outlined[0])?, area_wanted, 0.1, name);
    }
    // two segments meeting at a right angle, 4 wide: the bodies (10 x 4 each) share a 2 x 2 box, and the outside corner
    // adds a 2 x 2 box, half of it, or a quarter disc of radius 2
    let bent = Drawing::from(Path::from_svg_path("M0 0 L10 0 L10 10")?);
    for (join, corner, name) in [
        (Join::Miter, 4.0, "miter"),
        (Join::Bevel, 2.0, "bevel"),
        (Join::Round, PI, "round"),
    ] {
        let outlined = drawn(&bent.outline(4.0, join, Cap::Butt, LIMIT, TOLERANCE))?;
        assert_near(area(&outlined[0])?, 76.0 + corner, 0.1, name);
    }
    Ok(())
}

#[test]
fn each_step_keeps_the_style_attributes_and_order_of_every_shape() -> Outcome {
    let red = Color::hex(0xff_00_00);
    let blue = Color::hex(0x00_00_ff);
    let first = Drawing::from(square(0.0, 10.0)?)
        .fill(red)
        .stroke(Stroke::new(blue).width(2.0))
        .opacity(0.5);
    let second = Drawing::from(square(30.0, 10.0)?).fill(blue);
    let subject = Drawing::new([first, second]);
    let before = drawn(&subject)?;
    let steps = [
        ("offset", subject.offset(1.0, Join::Round, LIMIT, TOLERANCE)),
        (
            "outline",
            subject.outline(1.0, Join::Round, Cap::Round, LIMIT, TOLERANCE),
        ),
    ];
    for (name, step) in steps {
        let after = drawn(&step)?;
        assert_eq!(after.len(), 2, "{name}");
        for (index, (shape, original)) in after.iter().zip(&before).enumerate() {
            assert_eq!(shape.style(), original.style(), "{name}: style of shape {index}");
            assert_eq!(
                shape.attributes(),
                original.attributes(),
                "{name}: attributes of shape {index}"
            );
        }
        // the shapes stay where they were, in order
        assert!(extent(&after[0])?.2 < extent(&after[1])?.0, "{name}: order");
    }
    Ok(())
}

#[test]
fn a_chain_of_steps_works_on_text_and_keeps_a_shape_for_each_glyph() -> Outcome {
    let word = text("Hi", &fraunces()?);
    let glyphs = drawn(&word)?;
    assert_eq!(glyphs.len(), 2);
    let outlined =
        word.offset(1.0, Join::Round, LIMIT, TOLERANCE)
            .outline(0.5, Join::Round, Cap::Round, LIMIT, TOLERANCE);
    let shapes = drawn(&outlined)?;
    assert_eq!(shapes.len(), 2);
    for (shape, glyph) in shapes.iter().zip(&glyphs) {
        assert_eq!(shape.attributes(), glyph.attributes());
        // the glyph grown by 1 and stroked 0.5 wide reaches 1.25 past the glyph on each side
        let (grown, bare) = (extent(shape)?, extent(glyph)?);
        assert_near(grown.0, bare.0 - 1.25, 0.05, "left");
        assert_near(grown.2, bare.2 + 1.25, 0.05, "right");
    }
    Ok(())
}

// Refusals -----------------------------------------------------------------------------------------------------------

#[test]
fn a_number_the_library_refuses_leaves_the_step_with_an_error_naming_it() -> Outcome {
    let base = Drawing::from(square(0.0, 10.0)?);
    for (name, drawing, argument, value) in [
        (
            "miter_limit",
            base.offset(1.0, Join::Miter, 0.5, TOLERANCE),
            "miter_limit",
            "0.5",
        ),
        ("tolerance", base.offset(1.0, Join::Miter, LIMIT, 0.0), "tolerance", "0"),
        (
            "distance",
            base.offset(f64::NAN, Join::Miter, LIMIT, TOLERANCE),
            "distance",
            "NaN",
        ),
        (
            "width zero",
            base.outline(0.0, Join::Miter, Cap::Butt, LIMIT, TOLERANCE),
            "width",
            "0",
        ),
        (
            "width negative",
            base.outline(-2.0, Join::Miter, Cap::Butt, LIMIT, TOLERANCE),
            "width",
            "-2",
        ),
        (
            "outline limit",
            base.outline(2.0, Join::Miter, Cap::Butt, 0.5, TOLERANCE),
            "miter_limit",
            "0.5",
        ),
    ] {
        let (place, message) = parameter_refusal(Err::<(), _>(drawing.error().cloned().ok_or("no error held")?))?;
        let step = if name.starts_with("width") || name == "outline limit" {
            "outline"
        } else {
            "offset"
        };
        assert_eq!(place, format!("step 1 ({step}).{argument}"), "{name}");
        assert!(message.contains(value), "{name}: {message}");
    }
    Ok(())
}

#[test]
fn a_width_read_from_a_param_is_refused_when_drawn_naming_the_step_and_the_shape() -> Outcome {
    let width = Param::new("width", 2.0, -5.0, 5.0)?;
    let drawing = Drawing::new([square(0.0, 10.0)?, square(30.0, 10.0)?])
        .translate(0.0, 0.0)
        .outline(&width, Join::Round, Cap::Round, LIMIT, TOLERANCE);
    assert_eq!(drawing.error(), None);
    let (place, message) = parameter_refusal(drawing.shapes(&Values::new().set("width", 0.0)))?;
    assert_eq!(place, "step 2 (outline).width");
    assert!(message.contains('0'), "{message}");
    Ok(())
}

#[test]
fn a_step_is_charged_for_the_edges_it_flattens_and_refused_past_ten_million_segments() -> Outcome {
    // 45 circles of radius 1e6 at a tolerance of 1e-6 are 262,144 edges each: 11,796,480 in all
    let circles = (0..45)
        .map(|number| Ok(Path::circle((f64::from(number), 0.0), 1.0e6)?.to_svg_path()))
        .collect::<Result<Vec<_>, Error>>()?;
    let heavy = Drawing::from(Path::from_svg_path(&circles.join(" "))?);
    let steps = [
        ("offset", heavy.offset(1.0, Join::Round, LIMIT, 1.0e-6)),
        ("outline", heavy.outline(1.0, Join::Round, Cap::Round, LIMIT, 1.0e-6)),
    ];
    for (name, drawing) in steps {
        let (place, message) = parameter_refusal(drawing.shapes(&Values::new()))?;
        assert!(
            place.contains(&format!("step 1 ({name})")) && message.contains("MAX_SEGMENTS"),
            "{name}: {place}: {message}"
        );
    }
    Ok(())
}

// JSON ---------------------------------------------------------------------------------------------------------------

#[test]
fn the_steps_write_their_calls_leave_the_defaults_out_and_read_back_the_same_drawing() -> Outcome {
    let drawing = Drawing::from(square(0.0, 10.0)?)
        .offset(2.0, Join::Round, LIMIT, TOLERANCE)
        .offset(by_index(1.0, 3.0), Join::Miter, 2.0, 0.05)
        .outline(3.0, Join::Round, Cap::Round, LIMIT, TOLERANCE)
        .outline(1.5, Join::Bevel, Cap::Square, 6.0, 0.25);
    let text = drawing.to_json()?;
    for expected in [
        r#"{"offset":{"distance":2}}"#,
        r#"{"offset":{"distance":{"by_index":{"first":1,"last":3}},"join":"miter","miter_limit":2,"tolerance":0.05}}"#,
        r#"{"outline":{"width":3}}"#,
        r#"{"outline":{"cap":"square","join":"bevel","miter_limit":6,"tolerance":0.25,"width":1.5}}"#,
    ] {
        assert!(text.contains(expected), "{expected} not in {text}");
    }
    let again = Drawing::from_json(&text, &Inputs::new())?;
    assert_eq!(again.to_json()?, text);
    assert_eq!(again.shapes(&Values::new())?, drawing.shapes(&Values::new())?);
    let size = Size::px(60.0, 60.0);
    assert_eq!(
        again.to_svg(size, &Values::new())?,
        drawing.to_svg(size, &Values::new())?
    );
    Ok(())
}

#[test]
fn a_document_with_the_steps_loads_and_draws_them() -> Outcome {
    let text =
        r#"{"version":1,"root":[{"path":"M0 0 L10 0 L10 10 L0 10 Z"},{"offset":{"distance":2,"join":"miter"}}]}"#;
    let shapes = drawn(&Drawing::from_json(text, &Inputs::new())?)?;
    assert_extent(&shapes[0], [-2.0, -2.0, 12.0, 12.0], "offset")?;
    let text = r#"{"version":1,"root":[{"path":"M0 0 L10 0"},{"outline":{"width":4,"cap":"butt"}}]}"#;
    let shapes = drawn(&Drawing::from_json(text, &Inputs::new())?)?;
    assert_extent(&shapes[0], [0.0, -2.0, 10.0, 2.0], "outline")?;
    Ok(())
}

#[test]
fn a_document_that_gives_a_step_a_wrong_argument_is_refused_at_the_value() -> Outcome {
    let call = |step: &str, call: &str| {
        format!(r#"{{"version":1,"root":[{{"path":"M0 0 L4 0 L4 4 L0 4 Z"}},{{"{step}":{call}}}]}}"#)
    };
    for (what, step, call_text, pointer, message) in [
        ("distance missing", "offset", r"{}", "/root/1/offset", "distance"),
        (
            "join not a word",
            "offset",
            r#"{"distance":1,"join":"sharp"}"#,
            "/root/1/offset/join",
            "round",
        ),
        (
            "join a number",
            "offset",
            r#"{"distance":1,"join":1}"#,
            "/root/1/offset/join",
            "string",
        ),
        (
            "cap on an offset",
            "offset",
            r#"{"distance":1,"cap":"butt"}"#,
            "/root/1/offset/cap",
            "not an argument",
        ),
        (
            "miter_limit below 1",
            "offset",
            r#"{"distance":1,"miter_limit":0.5}"#,
            "/root/1/offset/miter_limit",
            "at least",
        ),
        (
            "miter_limit a Param",
            "offset",
            r#"{"distance":1,"miter_limit":{"param":"m"}}"#,
            "/root/1/offset/miter_limit",
            "number",
        ),
        (
            "tolerance zero",
            "offset",
            r#"{"distance":1,"tolerance":0}"#,
            "/root/1/offset/tolerance",
            "above 0",
        ),
        ("width missing", "outline", r"{}", "/root/1/outline", "width"),
        (
            "width zero",
            "outline",
            r#"{"width":0}"#,
            "/root/1/outline/width",
            "above 0",
        ),
        (
            "cap not a word",
            "outline",
            r#"{"width":1,"cap":"flat"}"#,
            "/root/1/outline/cap",
            "butt",
        ),
        (
            "join not a word",
            "outline",
            r#"{"width":1,"join":"sharp"}"#,
            "/root/1/outline/join",
            "round",
        ),
    ] {
        let (at, reason) = format_refusal(Drawing::from_json(&call(step, call_text), &Inputs::new()))?;
        assert_eq!(at, pointer, "{what}: {reason}");
        assert!(reason.contains(message), "{what}: {reason}");
    }
    Ok(())
}

#[test]
fn the_schema_and_the_registry_describe_both_steps() {
    let registered: Vec<_> = verbs().map(|verb| verb.name).collect();
    assert!(registered.contains(&"offset") && registered.contains(&"outline"));
    let schema = drawing_schema();
    for needle in [r#""offset""#, r#""outline""#, r#""miter_limit""#, r#""butt""#] {
        assert!(schema.contains(needle), "{needle} not in the schema");
    }
}

#[test]
fn the_miter_limit_clips_a_sharp_point_in_both_steps() -> Outcome {
    // the 30 degree apex of this triangle at (20, 0) is a miter of 1 / sin(15 degrees) = 3.86 half-widths long, so a
    // limit of 2 cuts it flat where a limit of 10 keeps the point
    let thin = Drawing::from(Path::from_svg_path("M0 0 L20 0 L0 5.36 Z")?);
    let right = |drawing: &Drawing| -> Result<f64, Box<dyn std::error::Error>> { Ok(extent(&drawn(drawing)?[0])?.2) };
    let kept = right(&thin.offset(1.0, Join::Miter, 10.0, TOLERANCE))?;
    let clipped = right(&thin.offset(1.0, Join::Miter, 2.0, TOLERANCE))?;
    assert!(kept > clipped + 1.0, "offset: {kept} against {clipped}");
    let kept = right(&thin.outline(2.0, Join::Miter, Cap::Butt, 10.0, TOLERANCE))?;
    let clipped = right(&thin.outline(2.0, Join::Miter, Cap::Butt, 2.0, TOLERANCE))?;
    assert!(kept > clipped + 1.0, "outline: {kept} against {clipped}");
    Ok(())
}

#[test]
fn the_tolerance_sets_how_many_straight_edges_a_curve_and_a_round_join_become() -> Outcome {
    let disc = Drawing::from(Path::circle((0.0, 0.0), 50.0)?);
    let edges = |drawing: &Drawing| -> Result<usize, Box<dyn std::error::Error>> {
        Ok(drawn(drawing)?[0].path().to_svg_path().matches('L').count())
    };
    let (fine, coarse) = (
        edges(&disc.offset(5.0, Join::Round, LIMIT, 0.01))?,
        edges(&disc.offset(5.0, Join::Round, LIMIT, 1.0))?,
    );
    assert!(fine > 2 * coarse, "offset: {fine} against {coarse}");
    let (fine, coarse) = (
        edges(&disc.outline(5.0, Join::Round, Cap::Round, LIMIT, 0.01))?,
        edges(&disc.outline(5.0, Join::Round, Cap::Round, LIMIT, 1.0))?,
    );
    assert!(fine > 2 * coarse, "outline: {fine} against {coarse}");
    Ok(())
}
