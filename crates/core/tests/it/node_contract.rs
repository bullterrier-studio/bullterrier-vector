//! The contract every registered verb keeps, checked over a table with one row per verb. The rows are enrolled exactly:
//! `verbs()` and the table name the same verbs, so a verb without a row, or a row without a verb, fails by name. Every
//! other check runs over the rows, so a verb added later is checked as soon as it has its row. Ranges, sizes and
//! hand results are written out here, not read from the library.
//!
//! 1. It draws the same shapes every time, and an equal drawing built apart draws them too.
//! 2. It accepts the boundaries of the ranges its arguments declare and refuses what is past them, at the call and
//!    when drawn, naming the step, the argument and the value.
//! 3. It makes the same paths and styles at any design size.
//! 4. A geometry verb equals the `Path` method of the same name, for any path, and gives a result written by hand.

use crate::common::fonts::fraunces;
use crate::common::{Outcome, build, log_uniform, parameter_refusal, step};
use bullterrier_vector::{
    Along, Arch, Blend, Bulge, Cap, Color, Deg, Drawing, Envelope, Error, Expr, Gradient, Halftone, Join, Kind, Param,
    Path, PlaceAlong, Point, Reads, Rect, Role, Rotate, Scale, ScaleBy, Select, Shadow, Shape, Spacing, Stroke, Text,
    Twist, Values, text, verbs,
};
use proptest::prelude::*;
use std::collections::BTreeSet;

/// A drawing made of a verb with one argument set from an expression, or the reason the row has no such argument.
type With = fn(&str, Expr) -> Result<Drawing, Error>;

/// One case: its name, the drawing that applies the verb, and what the `Path` method returns.
type Case = (&'static str, Drawing, Result<Path, Error>);

/// The cases of a geometry verb on a path: the drawing that applies the verb to the path with the numbers given, and
/// what the `Path` method of the same name returns for them.
type Cases = fn(&Path, f64, f64) -> Result<Vec<(&'static str, Drawing, Result<Path, Error>)>, Error>;

struct Row {
    /// The verb, as `verbs()` names it.
    verb: &'static str,
    /// A drawing that uses the verb, built through the typed API.
    sample: fn() -> Result<Drawing, Error>,
    /// The drawing that uses the verb with one number argument set, on a drawing of a square at the origin, so that
    /// the verb is the one step after the source.
    with: Option<With>,
    /// `(argument, value, accepted)`: the boundaries of each argument's range, both sides.
    ranges: &'static [(&'static str, f64, bool)],
    /// For a geometry verb: the verb against the `Path` method, and results written by hand.
    geometry: Option<Geometry>,
}

/// A path, the verb applied to a drawing of it, and the path data that must result.
type ByHand = (&'static str, fn(Drawing) -> Result<Drawing, Error>, &'static str);

struct Geometry {
    cases: Cases,
    by_hand: &'static [ByHand],
}

const SQUARE: &str = "M0 0 L10 0 L10 10 L0 10 Z";

fn square() -> Result<Drawing, Error> {
    Ok(Drawing::from(Path::from_svg_path(SQUARE)?))
}

fn unknown(verb: &str, argument: &str) -> Error {
    Error::Parameter {
        parameter: argument.to_owned(),
        message: format!("the row of {verb} has no such argument"),
    }
}

/// The guide the `along` rows bend along: a closed circle, so that every start has a place.
fn ring() -> Result<Path, Error> {
    Path::circle((0.0, 0.0), 50.0)
}

/// The top curve of the `envelope` rows: a slanted line.
fn slope() -> Result<Path, Error> {
    Path::line((0.0, 0.0), (100.0, 20.0))
}

/// A gradient across the square: the paint the `fill` and `stroke` rows are made with.
fn fade() -> Result<Gradient, Error> {
    Gradient::linear((0.0, 0.0), (10.0, 0.0), [(0.0, blue()), (1.0, Color::hex(0xff_ff_00))])
}

fn blue() -> Color {
    Color::hex(0x2a_5b_ff)
}

/// What the boolean rows combine the square with: a square of the same size, 5 to the right and down.
fn offset_square() -> Result<Path, Error> {
    Path::rect(5.0, 5.0, 10.0, 10.0, 0.0)
}

/// The number of a literal argument, which takes no Param: the row cannot set it from anything else. Read from the
/// text, since a number that is not finite has no `constant()`.
fn literal(argument: &str, value: &Expr) -> Result<f64, Error> {
    value.to_string().parse().map_err(|_| Error::Parameter {
        parameter: argument.to_owned(),
        message: String::from("is a literal and takes no Param"),
    })
}

/// The edges of a tolerance: the largest a path accepts, and the smallest that is still above 0.
const TOLERANCE_RANGE: [(&str, f64, bool); 8] = [
    ("tolerance", 0.01, true),
    ("tolerance", 1e100, true),
    ("tolerance", 1e-300, true),
    ("tolerance", 0.0, false),
    ("tolerance", -1.0, false),
    ("tolerance", 1.000_000_1e100, false),
    ("tolerance", f64::NAN, false),
    ("tolerance", f64::INFINITY, false),
];

/// The cases of a boolean verb on a path: the other operand a path or a drawing, a circle moved by the numbers, and a
/// tolerance from them.
fn boolean_cases(
    path: &Path,
    a: f64,
    b: f64,
    step: fn(&Drawing, Drawing, f64) -> Drawing,
    method: fn(&Path, &Path, f64) -> Result<Path, Error>,
) -> Result<Vec<Case>, Error> {
    let other = Path::circle((a.abs() % 1000.0 - 500.0, b.abs() % 1000.0 - 500.0), 300.0)?;
    let tolerance = 0.05 + a.abs() % 1.0;
    let subject = Drawing::from(path.clone());
    Ok(vec![
        (
            "other a path",
            step(&subject, Drawing::from(other.clone()), tolerance),
            method(path, &other, tolerance),
        ),
        (
            "other a longer drawing",
            step(&subject, Drawing::from(other.clone()).translate(0.0, 0.0), tolerance),
            method(path, &other, tolerance),
        ),
    ])
}

#[expect(clippy::too_many_lines, reason = "a table: one row per verb")]
fn rows() -> Vec<Row> {
    vec![
        Row {
            verb: "path",
            sample: || square(),
            with: None,
            ranges: &[],
            geometry: None,
        },
        Row {
            verb: "shape",
            sample: || {
                let shapes = square()?.fill(blue()).shapes(&Values::new())?;
                Ok(Drawing::from(shapes[0].clone()))
            },
            with: None,
            ranges: &[],
            geometry: None,
        },
        Row {
            verb: "stack",
            sample: || Ok(Drawing::new([square()?, square()?.translate(20.0, 0.0)])),
            with: None,
            ranges: &[],
            geometry: None,
        },
        Row {
            verb: "text",
            sample: || Ok(text("Hi", &fraunces()?)),
            with: Some(|argument, value| {
                let options = Text::new(&fraunces()?);
                match argument {
                    "size" => Ok(text("Hi", options.size(value))),
                    "tracking" => Ok(text("Hi", options.tracking(value))),
                    "leading" => Ok(text("Hi", options.leading(value))),
                    "width" => Ok(text("Hi", options.width(value))),
                    _ => Err(unknown("text", argument)),
                }
            }),
            ranges: &[
                ("size", 1.0, true),
                ("size", 100_000.0, true),
                ("size", 0.0, false),
                ("size", -1.0, false),
                ("size", f64::NAN, false),
                ("size", f64::INFINITY, false),
                ("tracking", 0.0, true),
                ("tracking", -250.0, true),
                ("tracking", 1000.0, true),
                ("tracking", f64::NAN, false),
                ("tracking", f64::NEG_INFINITY, false),
                ("leading", 0.0, true),
                ("leading", -600.0, true),
                ("leading", 5000.0, true),
                ("leading", f64::NAN, false),
                ("leading", f64::INFINITY, false),
                ("width", 1.0, true),
                ("width", 100_000.0, true),
                ("width", 0.0, false),
                ("width", -1.0, false),
                ("width", f64::NAN, false),
                ("width", f64::INFINITY, false),
            ],
            geometry: None,
        },
        Row {
            verb: "translate",
            sample: || Ok(square()?.translate(3.0, 4.0)),
            with: Some(|argument, value| match argument {
                "dx" => Ok(square()?.translate(value, 0.0)),
                "dy" => Ok(square()?.translate(0.0, value)),
                _ => Err(unknown("translate", argument)),
            }),
            ranges: &[
                ("dx", 0.0, true),
                ("dx", 1e100, true),
                ("dx", -1e100, true),
                ("dx", 2e100, false),
                ("dx", -2e100, false),
                ("dx", f64::NAN, false),
                ("dx", f64::INFINITY, false),
                ("dy", 0.0, true),
                ("dy", 1e100, true),
                ("dy", -1e100, true),
                ("dy", 2e100, false),
                ("dy", -2e100, false),
                ("dy", f64::NAN, false),
                ("dy", f64::NEG_INFINITY, false),
            ],
            geometry: Some(Geometry {
                cases: |path, a, b| {
                    Ok(vec![(
                        "by two numbers",
                        Drawing::from(path.clone()).translate(a, b),
                        path.translate(a, b),
                    )])
                },
                by_hand: &[(
                    "M1 2 L10 4.25 Z",
                    |d| Ok(d.translate(3.5, -7.25)),
                    "M4.5 -5.25 L13.5 -3 Z",
                )],
            }),
        },
        Row {
            verb: "rotate",
            sample: || Ok(square()?.rotate(Deg(30.0))),
            with: Some(|argument, value| match argument {
                "angle" => Ok(square()?.rotate(Deg(value))),
                _ => Err(unknown("rotate", argument)),
            }),
            ranges: &[
                ("angle", 0.0, true),
                ("angle", -720.5, true),
                ("angle", 1e300, true),
                ("angle", f64::NAN, false),
                ("angle", f64::INFINITY, false),
            ],
            geometry: Some(Geometry {
                cases: |path, a, b| {
                    let about = Point::new(b, -a);
                    Ok(vec![
                        (
                            "about the middle",
                            Drawing::from(path.clone()).rotate(Deg(a)),
                            path.rotate(a, None),
                        ),
                        (
                            "about a point",
                            Drawing::from(path.clone()).rotate(Rotate::new(Deg(a)).about(about)),
                            path.rotate(a, Some(about)),
                        ),
                    ])
                },
                // a quarter turn clockwise on the screen about the origin: (10, 0) goes to (0, 10)
                by_hand: &[(
                    SQUARE,
                    |d| Ok(d.rotate(Rotate::new(Deg(90.0)).about((0.0, 0.0)))),
                    "M0 0 L0 10 L-10 10 L-10 0 Z",
                )],
            }),
        },
        Row {
            verb: "scale",
            sample: || Ok(square()?.scale(2.0)),
            with: Some(|argument, value| match argument {
                "by" => Ok(square()?.scale(value)),
                _ => Err(unknown("scale", argument)),
            }),
            ranges: &[
                ("by", 2.0, true),
                ("by", -2.0, true),
                ("by", 1e-300, true),
                ("by", 0.0, false),
                ("by", f64::NAN, false),
                ("by", f64::INFINITY, false),
            ],
            geometry: Some(Geometry {
                cases: |path, a, b| {
                    let about = Point::new(b, -a);
                    Ok(vec![
                        (
                            "by a number",
                            Drawing::from(path.clone()).scale(a),
                            path.scale(Scale::from(a), None),
                        ),
                        (
                            "by a pair about a point",
                            Drawing::from(path.clone()).scale(ScaleBy::new((a, -b.abs() - 1.0)).about(about)),
                            path.scale((a, -b.abs() - 1.0), Some(about)),
                        ),
                    ])
                },
                by_hand: &[(
                    "M1 2 L10 4 Z",
                    |d| Ok(d.scale(ScaleBy::new((2.0, -0.5)).about((0.0, 0.0)))),
                    "M2 -1 L20 -2 Z",
                )],
            }),
        },
        Row {
            verb: "along",
            sample: || Ok(square()?.along(&ring()?, Along::new().start(0.1))),
            with: Some(|argument, value| match argument {
                "start" => Ok(square()?.along(&ring()?, Along::new().start(value))),
                _ => Err(unknown("along", argument)),
            }),
            ranges: &[
                ("start", 0.0, true),
                ("start", -2.5, true),
                ("start", 1e6, true),
                ("start", f64::NAN, false),
                ("start", f64::INFINITY, false),
                ("start", f64::NEG_INFINITY, false),
            ],
            geometry: Some(Geometry {
                cases: |path, a, b| {
                    let near = Point::new(b, -a);
                    let guide = ring()?;
                    Ok(vec![
                        (
                            "from the top",
                            Drawing::from(path.clone()).along(&guide, Along::new().start(a)),
                            path.along(&guide, a, None),
                        ),
                        (
                            "from the point nearest another",
                            Drawing::from(path.clone()).along(&guide, Along::new().start(a).near(near)),
                            path.along(&guide, a, Some(near)),
                        ),
                    ])
                },
                // the square 12.5 along a guide that runs east along y = 20 from x = 10: moved by (22.5, 20)
                by_hand: &[(
                    SQUARE,
                    |d| Ok(d.along(&Path::line((10.0, 20.0), (110.0, 20.0))?, Along::new().start(0.125))),
                    "M22.5 20 L32.5 20 L32.5 30 L22.5 30 Z",
                )],
            }),
        },
        Row {
            verb: "slice",
            sample: || Ok(square()?.slice(0.25, 0.75)),
            with: Some(|argument, value| match argument {
                "start" => Ok(square()?.slice(value, 1.0)),
                "end" => Ok(square()?.slice(0.0, value)),
                _ => Err(unknown("slice", argument)),
            }),
            ranges: &[
                ("start", 0.0, true),
                ("start", 1.0, true),
                ("start", 1.000_000_1, false),
                ("start", -1e-9, false),
                ("start", f64::NAN, false),
                ("end", 0.0, true),
                ("end", 1.0, true),
                ("end", 1.000_000_1, false),
                ("end", -1e-9, false),
                ("end", f64::NAN, false),
                ("end", f64::INFINITY, false),
            ],
            geometry: Some(Geometry {
                cases: |path, a, b| {
                    // two positions in 0..1 from the two numbers, the smaller first
                    let (start, end) = (
                        a.abs().fract().min(b.abs().fract()),
                        a.abs().fract().max(b.abs().fract()),
                    );
                    let near = Point::new(b, -a);
                    Ok(vec![
                        (
                            "from the top",
                            Drawing::from(path.clone()).slice(start, end),
                            path.slice(start, end, None),
                        ),
                        (
                            "from the point nearest another",
                            Drawing::from(path.clone()).slice_near(start, end, near),
                            path.slice(start, end, Some(near)),
                        ),
                    ])
                },
                // the middle half of a line 10 long
                by_hand: &[("M0 0 L10 0", |d| Ok(d.slice(0.25, 0.75)), "M2.5 0 L7.5 0")],
            }),
        },
        Row {
            verb: "resample",
            sample: || Ok(square()?.resample(4.0)),
            with: Some(|argument, value| match argument {
                "spacing" => Ok(square()?.resample(value)),
                _ => Err(unknown("resample", argument)),
            }),
            ranges: &[
                ("spacing", 4.0, true),
                ("spacing", 1e-3, true),
                ("spacing", 1e100, true),
                ("spacing", 0.0, false),
                ("spacing", -1.0, false),
                ("spacing", 1.000_000_1e100, false),
                ("spacing", f64::NAN, false),
                ("spacing", f64::INFINITY, false),
            ],
            geometry: Some(Geometry {
                cases: |path, a, _| {
                    let spacing = 1.0 + a.abs() % 100.0;
                    Ok(vec![(
                        "a spacing",
                        Drawing::from(path.clone()).resample(spacing),
                        path.resample(spacing),
                    )])
                },
                // a line 10 long, a corner every 4 and the end
                by_hand: &[("M0 0 L10 0", |d| Ok(d.resample(4.0)), "M0 0 L4 0 L8 0 L10 0")],
            }),
        },
        Row {
            verb: "simplify",
            sample: || Ok(square()?.simplify(0.5)),
            with: Some(|argument, value| match argument {
                "tolerance" => Ok(square()?.simplify(value)),
                _ => Err(unknown("simplify", argument)),
            }),
            ranges: &[
                ("tolerance", 0.5, true),
                ("tolerance", 1e-3, true),
                ("tolerance", 1e100, true),
                ("tolerance", 0.0, false),
                ("tolerance", -1.0, false),
                ("tolerance", 1.000_000_1e100, false),
                ("tolerance", f64::NAN, false),
                ("tolerance", f64::INFINITY, false),
            ],
            geometry: Some(Geometry {
                cases: |path, a, _| {
                    let tolerance = 0.1 + a.abs() % 10.0;
                    Ok(vec![(
                        "a tolerance",
                        Drawing::from(path.clone()).simplify(tolerance),
                        path.simplify(tolerance),
                    )])
                },
                // three points on a line, within the tolerance of their chord
                by_hand: &[("M0 0 L5 0 L10 0", |d| Ok(d.simplify(0.5)), "M0 0 L10 0")],
            }),
        },
        Row {
            verb: "reverse",
            sample: || Ok(square()?.reverse()),
            with: None,
            ranges: &[],
            geometry: Some(Geometry {
                cases: |path, _, _| {
                    Ok(vec![(
                        "no argument",
                        Drawing::from(path.clone()).reverse(),
                        Ok(path.reverse()),
                    )])
                },
                by_hand: &[("M0 0 L10 0 L10 5", |d| Ok(d.reverse()), "M10 5 L10 0 L0 0")],
            }),
        },
        Row {
            verb: "dash",
            sample: || Ok(square()?.dash([4.0, 2.0], 0.25)),
            with: Some(|argument, value| match argument {
                "pattern" => Ok(square()?.dash([value], 0.0)),
                "phase" => Ok(square()?.dash([4.0, 2.0], value)),
                _ => Err(unknown("dash", argument)),
            }),
            ranges: &[
                ("pattern", 4.0, true),
                ("pattern", 1e100, true),
                ("pattern", 0.0, false),
                ("pattern", -1.0, false),
                ("pattern", 1.000_000_1e100, false),
                ("pattern", f64::NAN, false),
                ("pattern", f64::INFINITY, false),
                ("phase", 0.25, true),
                ("phase", -3.5, true),
                ("phase", f64::NAN, false),
                ("phase", f64::INFINITY, false),
            ],
            geometry: Some(Geometry {
                cases: |path, a, b| {
                    let pattern = [1.0 + a.abs() % 50.0, 1.0 + b.abs() % 50.0];
                    let phase = (a + b) % 3.0;
                    Ok(vec![(
                        "a pattern and a phase",
                        Drawing::from(path.clone()).dash(pattern, phase),
                        path.dash(&pattern, phase, None),
                    )])
                },
                // a line 16 long, dashes 4 long every 8
                by_hand: &[("M0 0 L16 0", |d| Ok(d.dash([4.0, 4.0], 0.0)), "M0 0 L4 0 M8 0 L12 0")],
            }),
        },
        Row {
            verb: "place_along",
            sample: || Ok(square()?.place_along(&ring()?, Spacing::Count(4), PlaceAlong::new().start(0.1))),
            with: Some(|argument, value| match argument {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "a row's whole number"
                )]
                "count" => Ok(square()?.place_along(
                    &ring()?,
                    Spacing::Count(literal(argument, &value)? as usize),
                    PlaceAlong::new(),
                )),
                "spacing" => {
                    Ok(square()?.place_along(&ring()?, Spacing::Every(literal(argument, &value)?), PlaceAlong::new()))
                }
                "start" => Ok(square()?.place_along(&ring()?, Spacing::Count(3), PlaceAlong::new().start(value))),
                _ => Err(unknown("place_along", argument)),
            }),
            ranges: &[
                ("count", 1.0, true),
                ("count", 12.0, true),
                ("count", 0.0, false),
                ("spacing", 40.0, true),
                ("spacing", 1e100, true),
                ("spacing", 0.0, false),
                ("spacing", -1.0, false),
                ("spacing", 1.000_000_1e100, false),
                ("spacing", f64::NAN, false),
                ("spacing", f64::INFINITY, false),
                ("start", 0.0, true),
                ("start", -2.5, true),
                ("start", f64::NAN, false),
                ("start", f64::INFINITY, false),
            ],
            // copies of a drawing are not a `Path` method: no Path result to equal
            geometry: None,
        },
        Row {
            verb: "wave",
            sample: || Ok(square()?.wave(8.0, 120.0, Deg(30.0), 0.25)),
            with: Some(|argument, value| match argument {
                "amplitude" => Ok(square()?.wave(value, 120.0, Deg(0.0), 0.0)),
                "wavelength" => Ok(square()?.wave(8.0, value, Deg(0.0), 0.0)),
                "angle" => Ok(square()?.wave(8.0, 120.0, Deg(value), 0.0)),
                "phase" => Ok(square()?.wave(8.0, 120.0, Deg(0.0), value)),
                _ => Err(unknown("wave", argument)),
            }),
            ranges: &[
                ("amplitude", 8.0, true),
                ("amplitude", -3.0, true),
                ("amplitude", 1e5, true),
                ("amplitude", f64::NAN, false),
                ("amplitude", f64::INFINITY, false),
                ("wavelength", 120.0, true),
                ("wavelength", 0.5, true),
                ("wavelength", 1e100, true),
                ("wavelength", 0.0, false),
                ("wavelength", -1.0, false),
                ("wavelength", 1.000_000_1e100, false),
                ("wavelength", f64::NAN, false),
                ("wavelength", f64::INFINITY, false),
                ("angle", 0.0, true),
                ("angle", -720.5, true),
                ("angle", 1e300, true),
                ("angle", f64::NAN, false),
                ("angle", f64::INFINITY, false),
                ("phase", 0.25, true),
                ("phase", -3.5, true),
                ("phase", 1e300, true),
                ("phase", f64::NAN, false),
                ("phase", f64::INFINITY, false),
            ],
            geometry: Some(Geometry {
                cases: |path, a, b| {
                    let (amplitude, wavelength) = (a % 20.0, 10.0 + b.abs() % 100.0);
                    Ok(vec![(
                        "an amplitude, a wavelength, an angle and a phase",
                        Drawing::from(path.clone()).wave(amplitude, wavelength, Deg(a), b),
                        path.wave(amplitude, wavelength, a, b),
                    )])
                },
                // a line 10 long waved sideways to a travel straight down: it is all at distance 0, so a phase of a
                // quarter cycle moves it 2 along x and the one piece (wavelength 240, step 10) stays one
                by_hand: &[("M0 0 L10 0", |d| Ok(d.wave(2.0, 240.0, Deg(90.0), 0.25)), "M2 0 L12 0")],
            }),
        },
        Row {
            verb: "wobble",
            sample: || Ok(square()?.wobble(4.0, 80.0, 0.25, 3.0)),
            with: Some(|argument, value| match argument {
                "amplitude" => Ok(square()?.wobble(value, 80.0, 0.0, 0.0)),
                "wavelength" => Ok(square()?.wobble(4.0, value, 0.0, 0.0)),
                "phase" => Ok(square()?.wobble(4.0, 80.0, value, 0.0)),
                "seed" => Ok(square()?.wobble(4.0, 80.0, 0.0, value)),
                _ => Err(unknown("wobble", argument)),
            }),
            ranges: &[
                ("amplitude", 4.0, true),
                ("amplitude", -3.0, true),
                ("amplitude", f64::NAN, false),
                ("amplitude", f64::INFINITY, false),
                ("wavelength", 80.0, true),
                ("wavelength", 0.5, true),
                ("wavelength", 1e100, true),
                ("wavelength", 0.0, false),
                ("wavelength", -1.0, false),
                ("wavelength", 1.000_000_1e100, false),
                ("wavelength", f64::NAN, false),
                ("wavelength", f64::INFINITY, false),
                ("phase", 0.25, true),
                ("phase", -3.5, true),
                ("phase", 1e300, true),
                ("phase", f64::NAN, false),
                ("phase", f64::INFINITY, false),
                ("seed", 7.0, true),
                ("seed", -7.0, true),
                ("seed", f64::NAN, false),
                ("seed", f64::INFINITY, false),
            ],
            geometry: Some(Geometry {
                cases: |path, a, b| {
                    let (amplitude, wavelength) = (a % 20.0, 10.0 + b.abs() % 100.0);
                    Ok(vec![(
                        "an amplitude, a wavelength, a phase and a seed",
                        Drawing::from(path.clone()).wobble(amplitude, wavelength, b, 5.0),
                        path.wobble(amplitude, wavelength, b, 5.0),
                    )])
                },
                by_hand: &[],
            }),
        },
        Row {
            verb: "twist",
            sample: || Ok(square()?.twist(Twist::new(Deg(90.0)).radius(8.0).about((5.0, 5.0)))),
            with: Some(|argument, value| match argument {
                "angle" => Ok(square()?.twist(Deg(value))),
                "radius" => Ok(square()?.twist(Twist::new(Deg(90.0)).radius(value))),
                _ => Err(unknown("twist", argument)),
            }),
            ranges: &[
                ("angle", 90.0, true),
                ("angle", -720.5, true),
                ("angle", 3600.0, true),
                ("angle", f64::NAN, false),
                ("angle", f64::INFINITY, false),
                ("radius", 7.0, true),
                ("radius", 0.5, true),
                ("radius", 1e100, true),
                ("radius", 0.0, false),
                ("radius", -1.0, false),
                ("radius", 1.000_000_1e100, false),
                ("radius", f64::NAN, false),
                ("radius", f64::INFINITY, false),
            ],
            geometry: Some(Geometry {
                cases: |path, a, b| {
                    let angle = a % 720.0;
                    Ok(vec![
                        (
                            "an angle about the middle of the bounds",
                            Drawing::from(path.clone()).twist(Deg(angle)),
                            path.twist(angle, None, None),
                        ),
                        (
                            "an angle, a radius and a point",
                            Drawing::from(path.clone()).twist(
                                Twist::new(Deg(angle))
                                    .radius(10.0 + b.abs() % 90.0)
                                    .about((a % 50.0, b % 50.0)),
                            ),
                            path.twist(angle, Some(10.0 + b.abs() % 90.0), Some(Point::new(a % 50.0, b % 50.0))),
                        ),
                    ])
                },
                // (3, 4) is 5 from the point, half of a radius of 10, where the falloff is a half: 180 degrees turn 90,
                // a quarter turn clockwise on the screen takes (x, y) to (-y, x)
                by_hand: &[(
                    "M3 4",
                    |d| Ok(d.twist(Twist::new(Deg(180.0)).radius(10.0).about((0.0, 0.0)))),
                    "M-4 3",
                )],
            }),
        },
        Row {
            verb: "bulge",
            sample: || Ok(square()?.bulge(Bulge::new(0.8).radius(8.0).about((5.0, 5.0)))),
            with: Some(|argument, value| match argument {
                "amount" => Ok(square()?.bulge(value)),
                "radius" => Ok(square()?.bulge(Bulge::new(0.5).radius(value))),
                _ => Err(unknown("bulge", argument)),
            }),
            ranges: &[
                ("amount", 0.5, true),
                ("amount", -0.99, true),
                ("amount", 1.45, true),
                ("amount", -1.0, false),
                ("amount", 16.0 / 11.0, false),
                ("amount", f64::NAN, false),
                ("amount", f64::INFINITY, false),
                ("radius", 7.0, true),
                ("radius", 0.5, true),
                ("radius", 1e100, true),
                ("radius", 0.0, false),
                ("radius", -1.0, false),
                ("radius", 1.000_000_1e100, false),
                ("radius", f64::NAN, false),
                ("radius", f64::INFINITY, false),
            ],
            geometry: Some(Geometry {
                cases: |path, a, b| {
                    let amount = -0.9 + a.abs() % 2.3;
                    Ok(vec![
                        (
                            "an amount about the middle of the bounds",
                            Drawing::from(path.clone()).bulge(amount),
                            path.bulge(amount, None, None),
                        ),
                        (
                            "an amount, a radius and a point",
                            Drawing::from(path.clone()).bulge(
                                Bulge::new(amount)
                                    .radius(10.0 + b.abs() % 90.0)
                                    .about((a % 50.0, b % 50.0)),
                            ),
                            path.bulge(
                                amount,
                                Some(10.0 + b.abs() % 90.0),
                                Some(Point::new(a % 50.0, b % 50.0)),
                            ),
                        ),
                    ])
                },
                // (3, 4) is 5 from the point, half of a radius of 10, where the falloff is a half: it goes out to 1.5 times
                // as far along its ray
                by_hand: &[(
                    "M3 4",
                    |d| Ok(d.bulge(Bulge::new(1.0).radius(10.0).about((0.0, 0.0)))),
                    "M4.5 6",
                )],
            }),
        },
        Row {
            verb: "envelope",
            sample: || Ok(square()?.envelope(&slope()?, Envelope::new().amount(0.8))),
            with: Some(|argument, value| match argument {
                "amount" => Ok(square()?.envelope(&slope()?, Envelope::new().amount(value))),
                _ => Err(unknown("envelope", argument)),
            }),
            ranges: &[
                ("amount", 1.0, true),
                ("amount", -3.0, true),
                ("amount", 0.0, true),
                ("amount", f64::NAN, false),
                ("amount", f64::INFINITY, false),
            ],
            geometry: Some(Geometry {
                cases: |path, a, b| {
                    let amount = a % 3.0;
                    let frame = Rect {
                        x: a % 50.0,
                        y: b % 50.0,
                        width: 10.0 + b.abs() % 90.0,
                        height: 10.0 + a.abs() % 90.0,
                    };
                    Ok(vec![
                        (
                            "an amount, the bounds as the frame",
                            Drawing::from(path.clone()).envelope(&slope()?, Envelope::new().amount(amount)),
                            path.envelope(&slope()?, None, None, amount),
                        ),
                        (
                            "a bottom, a frame and an amount",
                            Drawing::from(path.clone())
                                .envelope(&slope()?, Envelope::new().bottom(&ring()?).frame(frame).amount(amount)),
                            path.envelope(&slope()?, Some(&ring()?), Some(frame), amount),
                        ),
                    ])
                },
                // the top runs to (10, 2): the point (5, 5) of the frame (0, 0, 10, 10) is half way across and half way
                // down, between (5, 1) and the straight bottom (5, 10)
                by_hand: &[(
                    "M5 5",
                    |d| {
                        Ok(d.envelope(
                            &Path::line((0.0, 0.0), (10.0, 2.0))?,
                            Envelope::new().frame(Rect {
                                x: 0.0,
                                y: 0.0,
                                width: 10.0,
                                height: 10.0,
                            }),
                        ))
                    },
                    "M5 5.5",
                )],
            }),
        },
        Row {
            verb: "arch",
            sample: || Ok(square()?.arch(0.3)),
            with: Some(|argument, value| match argument {
                "bend" => Ok(square()?.arch(value)),
                _ => Err(unknown("arch", argument)),
            }),
            ranges: &[
                ("bend", 0.3, true),
                ("bend", -2.0, true),
                ("bend", f64::NAN, false),
                ("bend", f64::NEG_INFINITY, false),
            ],
            geometry: Some(Geometry {
                cases: |path, a, b| {
                    let bend = a % 2.0;
                    let frame = Rect {
                        x: a % 50.0,
                        y: b % 50.0,
                        width: 10.0 + b.abs() % 90.0,
                        height: 10.0 + a.abs() % 90.0,
                    };
                    Ok(vec![
                        (
                            "a bend, the bounds as the frame",
                            Drawing::from(path.clone()).arch(bend),
                            path.arch(bend, None),
                        ),
                        (
                            "a bend and a frame",
                            Drawing::from(path.clone()).arch(Arch::new(bend).frame(frame)),
                            path.arch(bend, Some(frame)),
                        ),
                    ])
                },
                // the middle of the top of the frame (0, 0, 10, 10) rises by the bend times the width: 0.2 * 10 = 2
                by_hand: &[(
                    "M5 0",
                    |d| {
                        Ok(d.arch(Arch::new(0.2).frame(Rect {
                            x: 0.0,
                            y: 0.0,
                            width: 10.0,
                            height: 10.0,
                        })))
                    },
                    "M5 -2",
                )],
            }),
        },
        Row {
            verb: "fill",
            sample: || Ok(square()?.fill(fade()?)),
            with: None,
            ranges: &[],
            geometry: None,
        },
        Row {
            verb: "stroke",
            sample: || Ok(square()?.stroke(Stroke::new(fade()?).width(3.0))),
            with: Some(|argument, value| match argument {
                "width" => Ok(square()?.stroke(Stroke::new(fade()?).width(value))),
                "miter_limit" => Ok(square()?.stroke(Stroke::new(fade()?).miter_limit(value))),
                _ => Err(unknown("stroke", argument)),
            }),
            ranges: &[
                ("width", 2.0, true),
                ("width", 1e-9, true),
                ("width", 0.0, false),
                ("width", -1.0, false),
                ("width", f64::NAN, false),
                ("miter_limit", 1.0, true),
                ("miter_limit", 1e6, true),
                ("miter_limit", 0.99, false),
                ("miter_limit", f64::NAN, false),
                ("miter_limit", f64::INFINITY, false),
            ],
            geometry: None,
        },
        Row {
            verb: "opacity",
            sample: || Ok(square()?.opacity(0.5)),
            with: Some(|argument, value| match argument {
                "alpha" => Ok(square()?.opacity(value)),
                _ => Err(unknown("opacity", argument)),
            }),
            ranges: &[
                ("alpha", 0.0, true),
                ("alpha", 0.5, true),
                ("alpha", 1.0, true),
                ("alpha", 1.000_000_1, false),
                ("alpha", -1e-9, false),
                ("alpha", f64::NAN, false),
                ("alpha", f64::INFINITY, false),
            ],
            geometry: None,
        },
        Row {
            verb: "blend",
            sample: || Ok(square()?.blend(Blend::ColorDodge)),
            with: None,
            ranges: &[],
            geometry: None,
        },
        Row {
            verb: "shadow",
            sample: || Ok(square()?.shadow(Shadow::new().dy(4.0).blur(6.0))),
            with: Some(|argument, value| match argument {
                "dx" => Ok(square()?.shadow(Shadow::new().dx(value))),
                "dy" => Ok(square()?.shadow(Shadow::new().dy(value))),
                "blur" => Ok(square()?.shadow(Shadow::new().blur(value))),
                _ => Err(unknown("shadow", argument)),
            }),
            ranges: &[
                ("dx", -1e300, true),
                ("dx", f64::NAN, false),
                ("dx", f64::INFINITY, false),
                ("dy", 1e300, true),
                ("dy", f64::NEG_INFINITY, false),
                ("blur", 0.0, true),
                ("blur", 16.0, true),
                ("blur", -1e-9, false),
                ("blur", f64::NAN, false),
                ("blur", f64::INFINITY, false),
            ],
            geometry: None,
        },
        Row {
            verb: "halftone",
            sample: || Ok(square()?.halftone(Halftone::new(Color::hex(0), 0.5).pitch(6.0))),
            with: Some(|argument, value| {
                let dots = Halftone::new(Color::hex(0), 0.5);
                match argument {
                    "tone" => Ok(square()?.halftone(Halftone::new(Color::hex(0), value))),
                    "pitch" => Ok(square()?.halftone(dots.pitch(value))),
                    "angle" => Ok(square()?.halftone(dots.angle(Deg(value)))),
                    _ => Err(unknown("halftone", argument)),
                }
            }),
            ranges: &[
                ("tone", 0.0, true),
                ("tone", 1.0, true),
                ("tone", -1e-9, false),
                ("tone", 1.000_000_001, false),
                ("tone", f64::NAN, false),
                ("pitch", 1e300, true),
                ("pitch", 0.0, false),
                ("pitch", -1.0, false),
                ("pitch", f64::INFINITY, false),
                ("pitch", f64::NAN, false),
                ("angle", -1e300, true),
                ("angle", f64::NAN, false),
                ("angle", f64::INFINITY, false),
            ],
            geometry: None,
        },
        Row {
            verb: "knockout",
            sample: || Ok(square()?.knockout()),
            with: None,
            ranges: &[],
            geometry: None,
        },
        Row {
            verb: "clip",
            sample: || Ok(square()?.clip(offset_square()?)),
            with: None,
            ranges: &[],
            geometry: None,
        },
        Row {
            verb: "mask",
            sample: || Ok(square()?.mask(Drawing::from(offset_square()?).fill(Color::hex(0x80_80_80)))),
            with: None,
            ranges: &[],
            geometry: None,
        },
        Row {
            verb: "split",
            sample: || Ok(Drawing::from(Path::from_svg_path("M0 0 L4 0 L4 4 Z M10 0 L14 0 L14 4 Z")?).split()),
            with: None,
            ranges: &[],
            geometry: None,
        },
        Row {
            verb: "merge",
            sample: || Ok(Drawing::new([square()?, square()?.translate(20.0, 0.0)]).merge_by("order")),
            with: None,
            ranges: &[],
            geometry: None,
        },
        Row {
            verb: "union",
            sample: || Ok(square()?.union(offset_square()?, 0.01)),
            with: Some(|argument, value| match argument {
                "tolerance" => Ok(square()?.union(offset_square()?, literal(argument, &value)?)),
                _ => Err(unknown("union", argument)),
            }),
            ranges: &TOLERANCE_RANGE,
            geometry: Some(Geometry {
                cases: |path, a, b| boolean_cases(path, a, b, Drawing::union, Path::union),
                by_hand: &[(
                    SQUARE,
                    |d| Ok(d.union(offset_square()?, 0.01)),
                    "M0 0 L10 0 L10 5 L15 5 L15 15 L5 15 L5 10 L0 10 Z",
                )],
            }),
        },
        Row {
            verb: "subtract",
            sample: || Ok(square()?.subtract(offset_square()?, 0.01)),
            with: Some(|argument, value| match argument {
                "tolerance" => Ok(square()?.subtract(offset_square()?, literal(argument, &value)?)),
                _ => Err(unknown("subtract", argument)),
            }),
            ranges: &TOLERANCE_RANGE,
            geometry: Some(Geometry {
                cases: |path, a, b| boolean_cases(path, a, b, Drawing::subtract, Path::subtract),
                by_hand: &[(
                    SQUARE,
                    |d| Ok(d.subtract(offset_square()?, 0.01)),
                    "M0 0 L10 0 L10 5 L5 5 L5 10 L0 10 Z",
                )],
            }),
        },
        Row {
            verb: "intersect",
            sample: || Ok(square()?.intersect(offset_square()?, 0.01)),
            with: Some(|argument, value| match argument {
                "tolerance" => Ok(square()?.intersect(offset_square()?, literal(argument, &value)?)),
                _ => Err(unknown("intersect", argument)),
            }),
            ranges: &TOLERANCE_RANGE,
            geometry: Some(Geometry {
                cases: |path, a, b| boolean_cases(path, a, b, Drawing::intersect, Path::intersect),
                by_hand: &[(
                    SQUARE,
                    |d| Ok(d.intersect(offset_square()?, 0.01)),
                    "M5 5 L10 5 L10 10 L5 10 Z",
                )],
            }),
        },
        Row {
            verb: "exclude",
            sample: || Ok(square()?.exclude(offset_square()?, 0.01)),
            with: Some(|argument, value| match argument {
                "tolerance" => Ok(square()?.exclude(offset_square()?, literal(argument, &value)?)),
                _ => Err(unknown("exclude", argument)),
            }),
            ranges: &TOLERANCE_RANGE,
            geometry: Some(Geometry {
                cases: |path, a, b| boolean_cases(path, a, b, Drawing::exclude, Path::exclude),
                // the shape keeps one path with both pieces: the L of the square, then the L of the other
                by_hand: &[(
                    SQUARE,
                    |d| Ok(d.exclude(offset_square()?, 0.01)),
                    "M0 0 L10 0 L10 5 L5 5 L5 10 L0 10 Z M10 5 L15 5 L15 15 L5 15 L5 10 L10 10 Z",
                )],
            }),
        },
        Row {
            verb: "resolve",
            sample: || {
                Ok(Drawing::from(Path::from_svg_path(
                    "M0 0 L10 0 L10 10 L0 10 Z M5 5 L15 5 L15 15 L5 15 Z",
                )?)
                .resolve(0.01))
            },
            with: Some(|argument, value| match argument {
                "tolerance" => Ok(square()?.resolve(literal(argument, &value)?)),
                _ => Err(unknown("resolve", argument)),
            }),
            ranges: &TOLERANCE_RANGE,
            geometry: Some(Geometry {
                cases: |path, a, _| {
                    let tolerance = 0.05 + a.abs() % 1.0;
                    Ok(vec![(
                        "a tolerance",
                        Drawing::from(path.clone()).resolve(tolerance),
                        path.resolve(tolerance),
                    )])
                },
                by_hand: &[(
                    "M0 0 L10 0 L10 10 L0 10 Z M5 5 L15 5 L15 15 L5 15 Z",
                    |d| Ok(d.resolve(0.01)),
                    "M0 0 L10 0 L10 5 L15 5 L15 15 L5 15 L5 10 L0 10 Z",
                )],
            }),
        },
        Row {
            verb: "offset",
            sample: || Ok(square()?.offset(2.0, Join::Miter, 4.0, 0.01)),
            with: Some(|argument, value| match argument {
                "distance" => Ok(square()?.offset(value, Join::Round, 4.0, 0.01)),
                "miter_limit" => Ok(square()?.offset(1.0, Join::Miter, literal(argument, &value)?, 0.01)),
                "tolerance" => Ok(square()?.offset(1.0, Join::Round, 4.0, literal(argument, &value)?)),
                _ => Err(unknown("offset", argument)),
            }),
            ranges: &[
                ("distance", 2.0, true),
                ("distance", 0.0, true),
                ("distance", -2.0, true),
                ("distance", 1e90, true),
                ("distance", 1.000_000_1e100, false),
                ("distance", f64::NAN, false),
                ("distance", f64::INFINITY, false),
                ("miter_limit", 1.0, true),
                ("miter_limit", 1e6, true),
                ("miter_limit", 0.99, false),
                ("miter_limit", f64::NAN, false),
                ("miter_limit", f64::INFINITY, false),
                ("tolerance", 0.01, true),
                ("tolerance", 1e100, true),
                ("tolerance", 1e-300, true),
                ("tolerance", 0.0, false),
                ("tolerance", -1.0, false),
                ("tolerance", 1.000_000_1e100, false),
                ("tolerance", f64::NAN, false),
                ("tolerance", f64::INFINITY, false),
            ],
            geometry: Some(Geometry {
                cases: |path, a, b| {
                    let (a, tolerance) = (a % 300.0, 0.05 + b.abs() % 1.0);
                    let subject = Drawing::from(path.clone());
                    Ok([(Join::Round, "round"), (Join::Miter, "miter"), (Join::Bevel, "bevel")]
                        .into_iter()
                        .map(|(join, name)| {
                            (
                                name,
                                subject.offset(a, join, 3.0, tolerance),
                                path.offset(a, join, 3.0, tolerance),
                            )
                        })
                        .collect())
                },
                by_hand: &[
                    (
                        SQUARE,
                        |d| Ok(d.offset(2.0, Join::Miter, 4.0, 0.01)),
                        "M-2 -2 L12 -2 L12 12 L-2 12 Z",
                    ),
                    (
                        SQUARE,
                        |d| Ok(d.offset(-2.0, Join::Round, 4.0, 0.01)),
                        "M2 2 L8 2 L8 8 L2 8 Z",
                    ),
                ],
            }),
        },
        Row {
            verb: "outline",
            sample: || Ok(square()?.outline(2.0, Join::Miter, Cap::Butt, 4.0, 0.01)),
            with: Some(|argument, value| match argument {
                "width" => Ok(square()?.outline(value, Join::Round, Cap::Round, 4.0, 0.01)),
                "miter_limit" => Ok(square()?.outline(1.0, Join::Miter, Cap::Butt, literal(argument, &value)?, 0.01)),
                "tolerance" => Ok(square()?.outline(1.0, Join::Round, Cap::Round, 4.0, literal(argument, &value)?)),
                _ => Err(unknown("outline", argument)),
            }),
            ranges: &[
                ("width", 2.0, true),
                ("width", 1e-9, true),
                ("width", 1e90, true),
                ("width", 0.0, false),
                ("width", -1.0, false),
                ("width", 1.000_000_1e100, false),
                ("width", f64::NAN, false),
                ("width", f64::INFINITY, false),
                ("miter_limit", 1.0, true),
                ("miter_limit", 1e6, true),
                ("miter_limit", 0.99, false),
                ("miter_limit", f64::NAN, false),
                ("miter_limit", f64::INFINITY, false),
                ("tolerance", 0.01, true),
                ("tolerance", 1e100, true),
                ("tolerance", 1e-300, true),
                ("tolerance", 0.0, false),
                ("tolerance", -1.0, false),
                ("tolerance", 1.000_000_1e100, false),
                ("tolerance", f64::NAN, false),
                ("tolerance", f64::INFINITY, false),
            ],
            geometry: Some(Geometry {
                cases: |path, a, b| {
                    let (a, tolerance) = (a.abs() % 300.0, 0.05 + b.abs() % 1.0);
                    let subject = Drawing::from(path.clone());
                    Ok([(Cap::Butt, "butt"), (Cap::Round, "round"), (Cap::Square, "square")]
                        .into_iter()
                        .map(|(cap, name)| {
                            (
                                name,
                                subject.outline(a, Join::Miter, cap, 3.0, tolerance),
                                path.outline(a, Join::Miter, cap, 3.0, tolerance),
                            )
                        })
                        .collect())
                },
                // a stroke 2 wide round the square: a ring, the outer outline clockwise, the inner one counter
                by_hand: &[(
                    SQUARE,
                    |d| Ok(d.outline(2.0, Join::Miter, Cap::Butt, 4.0, 0.01)),
                    "M-1 -1 L11 -1 L11 11 L-1 11 Z M1 1 L1 9 L9 9 L9 1 Z",
                )],
            }),
        },
        Row {
            verb: "only",
            sample: || {
                Ok(Drawing::new([square()?, square()?.translate(20.0, 0.0)]).only(Select::new().attribute("index", 1)))
            },
            with: None,
            ranges: &[],
            geometry: None,
        },
        Row {
            verb: "without",
            sample: || {
                Ok(Drawing::new([square()?, square()?.translate(20.0, 0.0)])
                    .without(Select::new().attribute("order", 0.0)))
            },
            with: None,
            ranges: &[],
            geometry: None,
        },
        Row {
            verb: "with_values",
            sample: || {
                let p = Param::new("p", 1.0, 0.0, 10.0)?;
                Ok(square()?.translate(&p, 0.0).with_values(&Values::new().set("p", 5.0)))
            },
            with: None,
            ranges: &[],
            geometry: None,
        },
    ]
}

fn drawn(drawing: &Drawing) -> Result<Vec<Shape>, Error> {
    drawing.shapes(&Values::new())
}

// Enrolment ---------------------------------------------------------------------------------------------------------

#[test]
fn the_rows_and_the_registered_verbs_are_the_same_set() {
    let registered: Vec<&str> = verbs().map(|verb| verb.name).collect();
    let rows = rows();
    let rowed: Vec<&str> = rows.iter().map(|row| row.verb).collect();
    let (registered_set, rowed_set): (BTreeSet<_>, BTreeSet<_>) =
        (registered.iter().copied().collect(), rowed.iter().copied().collect());
    assert_eq!(
        registered.len(),
        registered_set.len(),
        "a verb is registered twice: {registered:?}"
    );
    assert_eq!(rowed.len(), rowed_set.len(), "a verb has two rows: {rowed:?}");
    let without_row: Vec<_> = registered_set.difference(&rowed_set).collect();
    let without_verb: Vec<_> = rowed_set.difference(&registered_set).collect();
    assert!(
        without_row.is_empty(),
        "registered verbs without a row: {without_row:?}"
    );
    assert!(
        without_verb.is_empty(),
        "rows without a registered verb: {without_verb:?}"
    );
}

#[test]
fn every_number_argument_has_range_rows_and_every_row_names_an_argument_of_its_verb() -> Outcome {
    for row in rows() {
        let verb = verbs().find(|verb| verb.name == row.verb).ok_or("no such verb")?;
        let numbers: BTreeSet<&str> = verb
            .args
            .iter()
            .filter(|arg| arg.kind == Kind::Number)
            .map(|arg| arg.name)
            .collect();
        let ranged: BTreeSet<&str> = row.ranges.iter().map(|(argument, ..)| *argument).collect();
        let missing: Vec<_> = numbers.difference(&ranged).collect();
        assert!(
            missing.is_empty(),
            "{}: Number arguments without range rows: {missing:?}",
            row.verb
        );
        let declared: BTreeSet<&str> = verb.args.iter().map(|arg| arg.name).collect();
        let stray: Vec<_> = ranged.difference(&declared).collect();
        assert!(
            stray.is_empty(),
            "{}: range rows for arguments the verb lacks: {stray:?}",
            row.verb
        );
        for argument in &ranged {
            assert!(
                row.with.is_some_and(|with| with(argument, Expr::from(1.0)).is_ok()),
                "{}: no way to set {argument}",
                row.verb
            );
        }
        // a row that names an argument accepts something and refuses something, or it checks nothing
        for argument in &ranged {
            let outcomes: BTreeSet<bool> = row
                .ranges
                .iter()
                .filter(|(name, ..)| name == argument)
                .map(|(_, _, accepted)| *accepted)
                .collect();
            assert_eq!(
                outcomes.len(),
                2,
                "{}.{argument}: needs a value accepted and a value refused",
                row.verb
            );
        }
    }
    Ok(())
}

// 1. Determinism ----------------------------------------------------------------------------------------------------

#[test]
fn a_verb_draws_the_same_shapes_every_time_and_an_equal_drawing_built_apart_does_too() -> Outcome {
    for row in rows() {
        let (first, apart) = ((row.sample)()?, (row.sample)()?);
        assert_eq!(first.error(), None, "{}", row.verb);
        let shapes = drawn(&first)?;
        assert!(
            !shapes.is_empty(),
            "{}: a sample that draws nothing checks nothing",
            row.verb
        );
        assert_eq!(shapes, drawn(&first)?, "{}: drawn twice", row.verb);
        assert_eq!(shapes, drawn(&apart)?, "{}: built apart", row.verb);
        assert_eq!(first.parameters()?, apart.parameters()?, "{}: parameters", row.verb);
    }
    Ok(())
}

// 2. Values within range --------------------------------------------------------------------------------------------

#[test]
fn a_verb_accepts_the_edges_of_its_ranges_and_refuses_past_them_at_the_call_and_when_drawn() -> Outcome {
    // a Param wide enough to carry any finite number
    let carrier = Param::new("carrier", 0.0, -1e308, 1e308)?;
    for row in rows() {
        let Some(with) = row.with else { continue };
        for (argument, value, accepted) in row.ranges {
            let case = format!("{}.{argument} = {value}", row.verb);
            let at_call = with(argument, Expr::from(*value))?;
            // a literal argument takes no Param: there is no drawing that reads one, so nothing is checked when drawn
            let literal = verbs().any(|verb| {
                verb.name == row.verb
                    && verb
                        .args
                        .iter()
                        .any(|arg| arg.name == *argument && arg.reads == Reads::Literal)
            });
            if literal {
                assert!(
                    with(argument, Expr::from(&carrier)).is_err(),
                    "{case}: a literal took a Param"
                );
            }
            let when_drawn = if literal {
                None
            } else {
                Some(with(argument, Expr::from(&carrier))?)
            };
            let given = Values::new().set("carrier", *value);
            if *accepted {
                assert_eq!(at_call.error(), None, "{case}: at the call");
                drawn(&at_call)?;
                if let Some(when_drawn) = &when_drawn {
                    when_drawn
                        .shapes(&given)
                        .map_err(|error| format!("{case}: when drawn: {error}"))?;
                }
            } else {
                // a source is the place `source (text)`, the one step of a chain after a square `step 1 (rotate)`
                let place = if verbs().any(|verb| verb.name == row.verb && verb.role == Role::Source) {
                    format!("source ({})", row.verb)
                } else {
                    format!("step 1 ({})", row.verb)
                };
                let expected = format!("{place}.{argument}");
                let text = format!("{value}");
                let held = at_call
                    .error()
                    .cloned()
                    .ok_or_else(|| format!("{case}: no error at the call"))?;
                let (name, message) = parameter_refusal(Err::<(), _>(held))?;
                assert!(
                    name == expected && message.contains(&text),
                    "{case}: at the call: {name}: {message}"
                );
                if let Some(when_drawn) = &when_drawn {
                    assert_eq!(when_drawn.error(), None, "{case}: a Param is not checked at the call");
                    let (name, message) = parameter_refusal(when_drawn.shapes(&given))?;
                    assert!(
                        name == expected && message.contains(&text),
                        "{case}: when drawn: {name}: {message}"
                    );
                }
            }
        }
    }
    Ok(())
}

// 3. Design size ----------------------------------------------------------------------------------------------------

#[test]
fn a_verb_makes_the_same_paths_and_styles_at_any_design_size() -> Outcome {
    for row in rows() {
        let sample = (row.sample)()?;
        let bare = drawn(&sample)?;
        for (width, height) in [(1.0, 1.0), (640.0, 480.0), (100_000.0, 3.0)] {
            let framed = Drawing::new([sample.clone()]).with_size(width, height);
            assert_eq!(framed.error(), None, "{}: {width} x {height}", row.verb);
            let shapes = drawn(&framed)?;
            assert_eq!(shapes.len(), bare.len(), "{}: {width} x {height}", row.verb);
            for (framed, bare) in shapes.iter().zip(&bare) {
                assert_eq!(framed.path(), bare.path(), "{}: {width} x {height}", row.verb);
                assert_eq!(framed.style(), bare.style(), "{}: {width} x {height}", row.verb);
            }
        }
    }
    Ok(())
}

// 4. Agreement with Path --------------------------------------------------------------------------------------------

#[test]
fn a_geometry_verb_gives_the_result_written_by_hand() -> Outcome {
    for row in rows() {
        let Some(geometry) = row.geometry else { continue };
        for (start, apply, expected) in geometry.by_hand {
            let drawing = apply(Drawing::from(Path::from_svg_path(start)?))?;
            assert_eq!(drawing.error(), None, "{}", row.verb);
            assert_eq!(drawn(&drawing)?[0].path().to_svg_path(), *expected, "{}", row.verb);
        }
    }
    Ok(())
}

proptest! {
    // The step is the Path method applied to the shape's path, for shapes and numbers of every size.
    #[test]
    fn a_geometry_verb_equals_the_path_method(
        start in (-1000.0f64..1000.0, -1000.0f64..1000.0),
        steps in proptest::collection::vec(step((-1000.0f64..1000.0, -1000.0f64..1000.0)), 0..8),
        numbers in (log_uniform(-3.0, 6.0), any::<bool>(), log_uniform(-3.0, 6.0), any::<bool>()),
    ) {
        let fail = |e: Error| TestCaseError::fail(e.to_string());
        let path = build(start, &steps).map_err(fail)?;
        let a = if numbers.1 { numbers.0 } else { -numbers.0 };
        let b = if numbers.3 { numbers.2 } else { -numbers.2 };
        for row in rows() {
            let Some(geometry) = row.geometry else { continue };
            for (case, drawing, expected) in (geometry.cases)(&path, a, b).map_err(fail)? {
                let drawn = drawing.shapes(&Values::new());
                match (drawn, expected) {
                    (Ok(shapes), Ok(expected)) => prop_assert_eq!(shapes[0].path(), &expected, "{}: {}", row.verb, case),
                    (Err(_), Err(_)) => {}
                    (drawn, expected) => prop_assert!(false, "{}: {}: drawing {:?}, path {:?}", row.verb, case, drawn.map(|s| s.len()), expected.map(|_| ())),
                }
            }
        }
    }
}
