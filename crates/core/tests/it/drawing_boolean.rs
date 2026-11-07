//! The booleans as steps of a `Drawing` (`union`, `subtract`, `intersect`, `exclude`, `resolve`) through the public API.
//! Expected values are areas worked out by hand (the shoelace sum in `common`), the other drawing is a `Drawing` or a
//! `Path`, and the document round trips through JSON.

use crate::common::{Outcome, format_refusal, net_area, parameter_refusal, rings};
use bullterrier_vector::{
    Color, Drawing, Error, Inputs, Param, Path, Shape, Size, Stroke, Values, drawing_schema, verbs,
};

/// The design tolerance of the hand cases: nothing in them is curved.
const TOLERANCE: f64 = 0.01;

fn rect(x: f64, y: f64, width: f64, height: f64) -> Result<Path, Error> {
    Path::rect(x, y, width, height, 0.0)
}

fn no_values() -> Values {
    Values::new()
}

/// The area every shape of `shapes` fills.
fn area(shapes: &[Shape]) -> Result<f64, Box<dyn std::error::Error>> {
    let mut total = 0.0;
    for shape in shapes {
        total += net_area(&rings(shape.path())?);
    }
    Ok(total)
}

/// The area the drawing fills, its shapes added.
fn filled(drawing: &Drawing) -> Result<f64, Box<dyn std::error::Error>> {
    area(&drawing.shapes(&no_values())?)
}

type Step = fn(&Drawing, Drawing, f64) -> Drawing;

/// The four steps that take another drawing, in the order the tables list them.
const STEPS: [(&str, Step); 4] = [
    ("union", |d, other, tolerance| d.union(other, tolerance)),
    ("subtract", |d, other, tolerance| d.subtract(other, tolerance)),
    ("intersect", |d, other, tolerance| d.intersect(other, tolerance)),
    ("exclude", |d, other, tolerance| d.exclude(other, tolerance)),
];

// Hand areas --------------------------------------------------------------------------------------------------------

#[test]
fn each_step_fills_the_region_its_set_rule_says_whether_other_is_a_drawing_or_a_path() -> Outcome {
    // 4 x 4 squares offset by 2: the overlap is 2 x 2
    let subject = Drawing::from(rect(0.0, 0.0, 4.0, 4.0)?);
    let other = rect(2.0, 2.0, 4.0, 4.0)?;
    for ((name, step), expected) in STEPS.iter().zip([28.0, 12.0, 4.0, 24.0]) {
        let by_path = step(&subject, Drawing::from(other.clone()), TOLERANCE);
        let by_drawing = step(&subject, Drawing::new([other.clone()]).translate(0.0, 0.0), TOLERANCE);
        assert_eq!(filled(&by_path)?, expected, "{name}");
        assert_eq!(filled(&by_drawing)?, expected, "{name}: other as a longer drawing");
    }
    Ok(())
}

#[test]
fn a_path_is_accepted_as_other_directly() -> Outcome {
    let subject = Drawing::from(rect(0.0, 0.0, 4.0, 4.0)?);
    let other = rect(2.0, 2.0, 4.0, 4.0)?;
    assert_eq!(filled(&subject.union(other.clone(), TOLERANCE))?, 28.0);
    assert_eq!(filled(&subject.intersect(other, TOLERANCE))?, 4.0);
    Ok(())
}

#[test]
fn resolve_removes_the_overlaps_of_each_shape() -> Outcome {
    // two same-way squares in one shape fill their union; the area counted by the shoelace sum would be 32
    let overlapping = Path::from_svg_path("M0 0 L4 0 L4 4 L0 4 Z M2 2 L6 2 L6 6 L2 6 Z")?;
    let resolved = Drawing::from(overlapping).resolve(TOLERANCE);
    assert_eq!(filled(&resolved)?, 28.0);
    // an hourglass drawn as one outline crossing itself is two triangles of 4
    let bow = Path::from_svg_path("M0 0 L4 4 L4 0 L0 4 Z")?;
    assert_eq!(filled(&Drawing::from(bow).resolve(TOLERANCE))?, 8.0);
    Ok(())
}

#[test]
fn a_hole_stays_a_hole() -> Outcome {
    let ring = Drawing::from(rect(0.0, 0.0, 10.0, 10.0)?).subtract(rect(3.0, 3.0, 4.0, 4.0)?, TOLERANCE);
    let shapes = ring.shapes(&no_values())?;
    assert_eq!(shapes.len(), 1);
    assert_eq!(rings(shapes[0].path())?.len(), 2, "the outline and the hole");
    assert_eq!(area(&shapes)?, 84.0);
    Ok(())
}

// Other with several shapes, or none ---------------------------------------------------------------------------------

#[test]
fn other_with_several_shapes_is_the_union_of_them_all_taken_once() -> Outcome {
    // 10 x 10; B is two overlapping 4 x 4 squares that cover 6 x 4 = 24 together, both inside A
    let subject = Drawing::from(rect(0.0, 0.0, 10.0, 10.0)?);
    let other = Drawing::new([rect(2.0, 2.0, 4.0, 4.0)?, rect(4.0, 2.0, 4.0, 4.0)?]);
    // a sequence of the single-shape booleans would give 84 for exclude (the overlap put back), not 76
    for ((name, step), expected) in STEPS.iter().zip([100.0, 76.0, 24.0, 76.0]) {
        assert_eq!(filled(&step(&subject, other.clone(), TOLERANCE))?, expected, "{name}");
    }
    Ok(())
}

#[test]
fn the_shapes_of_other_fill_by_themselves_whichever_way_each_is_drawn() -> Outcome {
    // the second square runs the other way round: joined as they are the overlap would wind 0 and be a hole
    let clockwise = Path::from_svg_path("M2 2 L6 2 L6 6 L2 6 Z")?;
    let counter = Path::from_svg_path("M4 2 L4 6 L8 6 L8 2 Z")?;
    let subject = Drawing::from(rect(0.0, 0.0, 10.0, 10.0)?);
    let other = Drawing::new([clockwise, counter]);
    assert_eq!(filled(&subject.intersect(other, TOLERANCE))?, 24.0);
    Ok(())
}

#[test]
fn other_with_no_shapes_leaves_the_region_and_intersect_empties_it() -> Outcome {
    let nothing = || Drawing::new(Vec::<Drawing>::new());
    assert_eq!(
        nothing().shapes(&no_values())?.len(),
        0,
        "the empty drawing has no shapes"
    );
    let subject = Drawing::from(rect(0.0, 0.0, 4.0, 4.0)?);
    for ((name, step), expected) in STEPS.iter().zip([16.0, 16.0, 0.0, 16.0]) {
        let shapes = step(&subject, nothing(), TOLERANCE).shapes(&no_values())?;
        assert_eq!(shapes.len(), 1, "{name}: the shape stays");
        assert_eq!(area(&shapes)?, expected, "{name}");
    }
    let emptied = subject.intersect(nothing(), TOLERANCE).shapes(&no_values())?;
    assert_eq!(emptied[0].path().to_svg_path(), "", "intersect gives an empty path");
    Ok(())
}

#[test]
fn a_path_with_nothing_in_it_is_an_empty_other() -> Outcome {
    let subject = Drawing::from(rect(0.0, 0.0, 4.0, 4.0)?);
    assert_eq!(filled(&subject.union(Path::new(), TOLERANCE))?, 16.0);
    assert_eq!(filled(&subject.intersect(Path::new(), TOLERANCE))?, 0.0);
    Ok(())
}

// Every shape, and what it keeps -------------------------------------------------------------------------------------

#[test]
fn every_shape_is_combined_alone_and_keeps_its_style_and_attributes() -> Outcome {
    let red = Color::hex(0xff_00_00);
    let blue = Color::hex(0x00_00_ff);
    // the first square meets the other, the second does not; each has its own paint
    let first = Drawing::from(rect(0.0, 0.0, 4.0, 4.0)?)
        .fill(red)
        .stroke(Stroke::new(blue).width(2.0))
        .opacity(0.5);
    let second = Drawing::from(rect(20.0, 0.0, 4.0, 4.0)?).fill(blue);
    let subject = Drawing::new([first, second]);
    let before = subject.shapes(&no_values())?;
    for ((name, step), expected) in STEPS.iter().zip([[28.0, 32.0], [12.0, 16.0], [4.0, 0.0], [24.0, 32.0]]) {
        let after = step(&subject, Drawing::from(rect(2.0, 2.0, 4.0, 4.0)?), TOLERANCE).shapes(&no_values())?;
        assert_eq!(after.len(), 2, "{name}");
        for (index, (shape, original)) in after.iter().zip(&before).enumerate() {
            assert_eq!(shape.style(), original.style(), "{name}: style of shape {index}");
            assert_eq!(
                shape.attributes(),
                original.attributes(),
                "{name}: attributes of shape {index}"
            );
            assert_eq!(
                area(std::slice::from_ref(shape))?,
                expected[index],
                "{name}: area of shape {index}"
            );
        }
    }
    let resolved = subject.resolve(TOLERANCE).shapes(&no_values())?;
    for (shape, original) in resolved.iter().zip(&before) {
        assert_eq!(
            (shape.style(), shape.attributes()),
            (original.style(), original.attributes())
        );
    }
    Ok(())
}

// A Param in other ---------------------------------------------------------------------------------------------------

#[test]
fn other_that_reads_a_param_changes_the_result_with_the_value_and_is_not_drawn_again_for_the_same_one() -> Outcome {
    let shift = Param::new("shift", 0.0, -10.0, 10.0)?;
    let subject = Drawing::from(rect(0.0, 0.0, 4.0, 4.0)?);
    let moving = Drawing::from(rect(0.0, 0.0, 4.0, 4.0)?).translate(&shift, 0.0);
    let drawing = subject.intersect(moving, TOLERANCE);
    assert_eq!(drawing.parameters()?, vec![shift.clone()]);
    // the overlap of two 4 x 4 squares shifted by `shift` is (4 - shift) x 4
    for (value, expected) in [(0.0, 16.0), (1.0, 12.0), (3.0, 4.0), (5.0, 0.0), (1.0, 12.0)] {
        let values = Values::new().set("shift", value);
        assert_eq!(area(&drawing.shapes(&values)?)?, expected, "shift {value}");
    }
    drawing.shapes(&Values::new().set("shift", 1.0))?;
    drawing.shapes(&Values::new().set("shift", 1.0))?;
    assert_eq!(drawing.stats().computed, 0, "the same values draw nothing again");
    Ok(())
}

#[test]
fn two_others_on_one_subject_are_two_results_even_though_they_share_a_cache() -> Outcome {
    let subject = Drawing::from(rect(0.0, 0.0, 4.0, 4.0)?);
    let touching = subject.union(rect(2.0, 2.0, 4.0, 4.0)?, TOLERANCE);
    let apart = subject.union(rect(10.0, 10.0, 4.0, 4.0)?, TOLERANCE);
    assert_eq!(filled(&touching)?, 28.0);
    assert_eq!(filled(&apart)?, 32.0);
    assert_eq!(filled(&touching)?, 28.0);
    Ok(())
}

#[test]
fn the_tolerance_sets_how_close_the_edges_of_a_curve_are_to_it() -> Outcome {
    // a circle of radius 40 inside a square: its edges stray from the curve by the tolerance, and the four cubics
    // that make the circle by up to 0.011 (0.027% of the radius) themselves
    let circle = Path::circle((50.0, 50.0), 40.0)?;
    let (whole, circumference) = (5_026.548_245_743_669_f64, 251.327_412_287_183_45_f64);
    let square = Drawing::from(rect(0.0, 0.0, 100.0, 100.0)?);
    let mut corners = Vec::new();
    for tolerance in [0.01, 5.0] {
        let shapes = square.intersect(circle.clone(), tolerance).shapes(&no_values())?;
        let missed = whole - area(&shapes)?;
        assert!(
            missed.abs() <= (tolerance + 0.011) * circumference,
            "tolerance {tolerance}: missed {missed}"
        );
        corners.push(rings(shapes[0].path())?[0].len());
    }
    assert!(
        corners[0] > 4 * corners[1],
        "{corners:?}: a finer tolerance has more corners"
    );
    Ok(())
}

// JSON ---------------------------------------------------------------------------------------------------------------

#[test]
fn a_document_with_a_union_loads_and_draws_the_region() -> Outcome {
    let text = r#"{"version":1,"root":[{"path":"M0 0 L4 0 L4 4 L0 4 Z"},{"union":{"other":[{"path":"M2 2 L6 2 L6 6 L2 6 Z"}]}}]}"#;
    let drawing = Drawing::from_json(text, &Inputs::new())?;
    assert_eq!(filled(&drawing)?, 28.0);
    Ok(())
}

#[test]
fn the_steps_write_other_as_calls_and_leave_the_default_tolerance_out_and_read_back_the_same() -> Outcome {
    let shift = Param::new("shift", 1.0, -10.0, 10.0)?;
    let other = Drawing::from(rect(2.0, 2.0, 4.0, 4.0)?).translate(&shift, 0.0);
    let drawing = Drawing::from(rect(0.0, 0.0, 4.0, 4.0)?)
        .union(other.clone(), TOLERANCE)
        .subtract(rect(1.0, 1.0, 1.0, 1.0)?, 0.05)
        .resolve(0.25);
    let text = drawing.to_json()?;
    for expected in [
        r#"{"union":{"other":[{"path":"M2 2 L6 2 L6 6 L2 6 Z"},{"translate":{"dx":{"param":"shift"},"dy":0}}]}}"#,
        r#"{"subtract":{"other":[{"path":"M1 1 L2 1 L2 2 L1 2 Z"}],"tolerance":0.05}}"#,
        r#"{"resolve":{"tolerance":0.25}}"#,
    ] {
        assert!(text.contains(expected), "{expected} not in {text}");
    }
    let again = Drawing::from_json(&text, &Inputs::new())?;
    assert_eq!(again.to_json()?, text);
    for value in [1.0, -3.0] {
        let values = Values::new().set("shift", value);
        assert_eq!(again.shapes(&values)?, drawing.shapes(&values)?, "shapes at {value}");
        let size = Size::px(40.0, 40.0);
        assert_eq!(
            again.to_svg(size, &values)?,
            drawing.to_svg(size, &values)?,
            "svg at {value}"
        );
    }
    Ok(())
}

#[test]
fn a_drawing_used_as_the_subject_and_inside_other_is_one_shared_drawing_in_the_document() -> Outcome {
    let square = Drawing::from(rect(0.0, 0.0, 4.0, 4.0)?);
    let drawing = square.exclude(square.translate(2.0, 2.0), TOLERANCE);
    let text = drawing.to_json()?;
    assert!(text.contains(r#""drawings":{"d1":"#), "{text}");
    assert!(text.contains(r#"{"drawing":"d1"}"#), "{text}");
    let again = Drawing::from_json(&text, &Inputs::new())?;
    assert_eq!(again.to_json()?, text);
    assert_eq!(filled(&again)?, 24.0);
    Ok(())
}

#[test]
fn a_document_that_gives_other_or_tolerance_wrongly_is_refused_at_the_value() -> Outcome {
    let call =
        |call: &str| format!(r#"{{"version":1,"root":[{{"path":"M0 0 L4 0 L4 4 L0 4 Z"}},{{"union":{call}}}]}}"#);
    for (what, call_text, pointer, message) in [
        ("other missing", r"{}", "/root/1/union", "other"),
        (
            "other not an array",
            r#"{"other":"M0 0"}"#,
            "/root/1/union/other",
            "array",
        ),
        ("other with no call", r#"{"other":[]}"#, "/root/1/union/other", "source"),
        (
            "other starts with a step",
            r##"{"other":[{"fill":"#000000"}]}"##,
            "/root/1/union/other/0/fill",
            "source",
        ),
        (
            "tolerance zero",
            r#"{"other":[{"path":"M0 0"}],"tolerance":0}"#,
            "/root/1/union/tolerance",
            "above 0",
        ),
        (
            "tolerance an expression",
            r#"{"other":[{"path":"M0 0"}],"tolerance":{"param":"t"}}"#,
            "/root/1/union/tolerance",
            "number",
        ),
        (
            "tolerance beyond the bound",
            r#"{"other":[{"path":"M0 0"}],"tolerance":1e101}"#,
            "/root/1/union/tolerance",
            "at most",
        ),
        (
            "an argument it lacks",
            r#"{"other":[{"path":"M0 0"}],"mode":1}"#,
            "/root/1/union/mode",
            "not an argument",
        ),
    ] {
        let (at, reason) = format_refusal(Drawing::from_json(&call(call_text), &Inputs::new()))?;
        assert_eq!(at, pointer, "{what}: {reason}");
        assert!(reason.contains(message), "{what}: {reason}");
    }
    Ok(())
}

#[test]
fn the_schema_takes_other_as_a_drawing_and_a_tolerance_above_zero() {
    let schema = drawing_schema();
    for verb in ["union", "subtract", "intersect", "exclude", "resolve"] {
        assert!(
            verbs().any(|registered| registered.name == verb),
            "{verb} is not registered"
        );
        assert!(schema.contains(&format!("\"{verb}\":")), "{verb} is not in the schema");
    }
    assert!(schema.contains(r##""other":{"$ref":"#/$defs/drawing"}"##), "{schema}");
    assert!(
        schema.contains(r#""tolerance":{"default":0.01,"exclusiveMinimum":0"#),
        "{schema}"
    );
}

// Errors -------------------------------------------------------------------------------------------------------------

#[test]
fn an_other_that_holds_an_error_leaves_the_step_with_it_named_by_item() -> Outcome {
    let broken = Drawing::from(rect(0.0, 0.0, 1.0, 1.0)?).translate(f64::NAN, 0.0);
    assert!(broken.error().is_some());
    let drawing = Drawing::from(rect(0.0, 0.0, 4.0, 4.0)?).union(broken, TOLERANCE);
    let (name, _) = parameter_refusal(Err::<(), _>(drawing.error().cloned().ok_or("no error held")?))?;
    assert!(name.contains("step 1 (union) / item 1"), "{name}");
    Ok(())
}

#[test]
fn a_tolerance_the_library_refuses_leaves_the_step_with_an_error_naming_it() -> Outcome {
    for tolerance in [0.0, -1.0, f64::NAN, f64::INFINITY, 1e101] {
        let drawing = Drawing::from(rect(0.0, 0.0, 4.0, 4.0)?).resolve(tolerance);
        let (name, message) = parameter_refusal(Err::<(), _>(drawing.error().cloned().ok_or("no error held")?))?;
        assert_eq!(name, "step 1 (resolve).tolerance", "{tolerance}");
        assert!(message.contains(&format!("{tolerance}")), "{tolerance}: {message}");
    }
    Ok(())
}

// The work -----------------------------------------------------------------------------------------------------------

#[test]
fn a_step_is_charged_for_the_edges_it_flattens_before_the_engine_runs_and_refused_past_ten_million_segments() -> Outcome
{
    // a circle of radius 1e6 at a tolerance of 1e-6 is cut to the most edges a curve gets, 262,144; 45 of them in one
    // other pass 10,000,000 while they are flattened, before the engine sees a single edge
    let circles = (0..45)
        .map(|number| Ok(Path::circle((f64::from(number), 0.0), 1.0e6)?.to_svg_path()))
        .collect::<Result<Vec<_>, Error>>()?;
    let heavy = Path::from_svg_path(&circles.join(" "))?;
    let drawing = Drawing::from(rect(0.0, 0.0, 4.0, 4.0)?).intersect(heavy, 1.0e-6);
    let (name, message) = parameter_refusal(drawing.shapes(&no_values()))?;
    assert!(
        name.contains("step 1 (intersect)") && message.contains("MAX_SEGMENTS"),
        "{name}: {message}"
    );
    Ok(())
}

/// A closed polygon through `count` points scattered over a 1000 square by a fixed xorshift: every edge is long and
/// crosses about a fifth of the others.
fn scribble(count: usize) -> Result<Path, Error> {
    let mut state = 0x1234_5678_9abc_def1_u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        #[expect(clippy::cast_precision_loss, reason = "the top 53 bits")]
        let share = (state >> 11) as f64 / (1_u64 << 53) as f64;
        share * 1000.0
    };
    Path::polyline((0..count).map(|_| (next(), next())).collect::<Vec<_>>(), true)
}

#[test]
fn a_step_is_refused_when_the_edges_cross_so_often_that_the_engine_would_exhaust_memory() -> Outcome {
    // 16,000 edges are far below the ten million segments a draw may flatten, yet they cross about 30 million times:
    // resolving them takes minutes and more than 4 GB, which a charge on the edges alone cannot see
    let drawing = Drawing::from(scribble(16_000)?).resolve(TOLERANCE);
    let (name, message) = parameter_refusal(drawing.shapes(&no_values()))?;
    assert!(
        name.contains("step 1 (resolve)") && message.contains("MAX_SEGMENTS"),
        "{name}: {message}"
    );
    // known good: 400 edges cross about 18,000 times and resolve
    let drawing = Drawing::from(scribble(400)?).resolve(TOLERANCE);
    assert_eq!(drawing.shapes(&no_values())?.len(), 1);
    Ok(())
}
