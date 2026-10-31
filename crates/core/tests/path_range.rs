//! The coordinate limit: every coordinate of a path, given or worked out, lies within ±1e100, so that the squares and
//! sums the measuring does stay finite. A coordinate beyond it is refused where it enters, naming the parameter and the
//! value; one within it measures, cuts and hit-tests like any other.

mod common;

use bullterrier_vector::{Error, Path, Point};
use common::{Outcome, Step, log_uniform, parameter_refusal, step};
use proptest::prelude::*;

/// Coordinates beyond the limit, from just past it to the largest a number holds.
const TOO_BIG: [f64; 4] = [1e120, 1e154, 1e200, 1e308];

/// A path to build the next segment on.
fn start() -> Result<Path, Error> {
    Path::start((1.0, 2.0))
}

#[test]
fn a_coordinate_beyond_the_limit_is_refused_by_every_builder_naming_the_parameter_and_the_value() -> Outcome {
    let begun = start()?;
    for far in TOO_BIG {
        // (what is called, the parameter it must name, the outcome)
        let rows = [
            ("Path::start", "point", Path::start((far, 0.0))),
            ("Path::start y", "point", Path::start((0.0, -far))),
            ("move_to", "point", begun.move_to((far, 0.0))),
            ("line_to", "point", begun.line_to((0.0, far))),
            ("quad_to control", "control", begun.quad_to((far, 0.0), (1.0, 1.0))),
            ("quad_to point", "point", begun.quad_to((1.0, 1.0), (-far, 0.0))),
            (
                "curve_to control1",
                "control1",
                begun.curve_to((far, 0.0), (1.0, 1.0), (2.0, 2.0)),
            ),
            (
                "curve_to control2",
                "control2",
                begun.curve_to((1.0, 1.0), (0.0, -far), (2.0, 2.0)),
            ),
            (
                "curve_to point",
                "point",
                begun.curve_to((1.0, 1.0), (2.0, 2.0), (far, far)),
            ),
            ("Path::line start", "start", Path::line((far, 0.0), (1.0, 1.0))),
            ("Path::line end", "end", Path::line((1.0, 1.0), (0.0, -far))),
            ("polyline", "points", Path::polyline([(0.0, 0.0), (far, far)], false)),
            ("rect x", "x", Path::rect(far, 0.0, 1.0, 1.0, 0.0)),
            ("rect y", "y", Path::rect(0.0, -far, 1.0, 1.0, 0.0)),
            ("rect width", "width", Path::rect(0.0, 0.0, far, 1.0, 0.0)),
            ("rect height", "height", Path::rect(0.0, 0.0, 1.0, far, 0.0)),
            ("rect radius", "radius", Path::rect(0.0, 0.0, 10.0, 10.0, far)),
            ("circle center", "center", Path::circle((far, 0.0), 1.0)),
            ("circle radius", "radius", Path::circle((0.0, 0.0), far)),
            ("polygon center", "center", Path::polygon((0.0, far), 1.0, 5)),
            ("polygon radius", "radius", Path::polygon((0.0, 0.0), far, 5)),
            ("star center", "center", Path::star((-far, 0.0), 2.0, 1.0, 5)),
            ("star outer", "outer", Path::star((0.0, 0.0), far, 1.0, 5)),
            ("star inner", "inner", Path::star((0.0, 0.0), 2.0, far, 5)),
        ];
        for (what, parameter, outcome) in rows {
            let (name, message) = parameter_refusal(outcome).map_err(|e| format!("{what} at {far}: {e}"))?;
            assert_eq!(name, parameter, "{what} at {far}");
            assert!(
                message.contains(&far.to_string()),
                "{what}: {message:?} lacks the value {far}"
            );
        }
    }
    Ok(())
}

#[test]
fn a_coordinate_beyond_the_limit_is_refused_in_path_data_naming_data_the_command_and_the_value() -> Outcome {
    for far in TOO_BIG {
        let written = format!("{far:e}");
        // (path data with the number in a place a coordinate comes from, the command at fault)
        let rows = [
            (format!("M{written} 0"), 'M'),
            (format!("M0 0 L{written} 0"), 'L'),
            (format!("M0 0 H{written}"), 'H'),
            (format!("M0 0 V-{written}"), 'V'),
            (format!("M0 0 Q{written} 0 1 1"), 'Q'),
            (format!("M0 0 Q1 1 {written} 0"), 'Q'),
            (format!("M0 0 C{written} 0 1 1 2 2"), 'C'),
            (format!("M0 0 C1 1 {written} 0 2 2"), 'C'),
            (format!("M0 0 C1 1 2 2 {written} 0"), 'C'),
            (format!("M0 0 S{written} 0 1 1"), 'S'),
            (format!("M0 0 S1 1 {written} 0"), 'S'),
            (format!("M0 0 T{written} 0"), 'T'),
            (format!("M0 0 A1 1 0 0 1 {written} 0"), 'A'),
            (format!("m{written} 0"), 'm'),
            (format!("M0 0 l0 {written}"), 'l'),
        ];
        for (data, command) in rows {
            let (name, message) = parameter_refusal(Path::from_svg_path(&data)).map_err(|e| format!("{data}: {e}"))?;
            assert_eq!(name, "data", "{data}");
            assert!(message.contains(&format!("`{command}`")), "{data}: {message:?}");
            assert!(
                message.contains(&far.to_string()),
                "{data}: {message:?} lacks the value {far}"
            );
        }
    }
    Ok(())
}

#[test]
fn path_data_that_once_made_slicing_panic_is_refused_when_read() -> Outcome {
    // the finding that led to the limit: with a sub-path of no length first, slicing it once panicked
    let (name, message) = parameter_refusal(Path::from_svg_path("M0 0 M1 0 L1e200 0"))?;
    assert_eq!(
        (name.as_str(), message.contains("`L` at byte 10")),
        ("data", true),
        "{message}"
    );
    Ok(())
}

#[test]
fn a_coordinate_the_path_data_works_out_beyond_the_limit_is_refused_though_every_number_in_it_is_within() -> Outcome {
    // (data, the command at fault): sums of relative moves, reflected handles, and an arc whose radii are scaled up beyond the limit
    let rows = [
        ("M9e99 0 l9e99 0", 'l'),
        ("M0 0 C-9e99 0 -9e99 0 9e99 0 S 0 0 0 0", 'S'),
        ("M0 0 Q-9e99 0 9e99 0 T 0 0", 'T'),
        ("M9e99 0 A1e77 1 0 0 1 9e99 1e99", 'A'),
    ];
    for (data, command) in rows {
        let (name, message) = parameter_refusal(Path::from_svg_path(data)).map_err(|e| format!("{data}: {e}"))?;
        assert_eq!(name, "data", "{data}");
        assert!(message.contains(&format!("`{command}`")), "{data}: {message:?}");
    }
    // the same shapes one step inside are read
    for data in [
        "M9e99 0 l-9e99 0",
        "M0 0 C-3e99 0 -3e99 0 3e99 0 S 0 0 0 0",
        "M-9e99 0 A5e99 5e99 0 1 1 9e99 0",
        "M0 0 A1e77 1 0 0 1 0 1",
        "M1e100 -1e100 L-1e100 1e100",
    ] {
        Path::from_svg_path(data).map_err(|e| format!("{data}: {e}"))?;
    }
    Ok(())
}

#[test]
fn a_transform_that_carries_a_point_beyond_the_limit_is_refused_naming_its_parameter() -> Outcome {
    let line = Path::line((0.0, 0.0), (10.0, 5.0))?;
    for far in TOO_BIG {
        let rows = [
            ("translate dx", "dx", line.translate(far, 0.0)),
            ("translate dy", "dy", line.translate(0.0, -far)),
            ("rotate about", "about", line.rotate(30.0, Some(Point::new(far, 0.0)))),
            ("scale about", "about", line.scale(2.0, Some(Point::new(0.0, -far)))),
            ("map_points", "fn", line.map_points(|_| Ok(Point::new(far, 0.0)))),
        ];
        for (what, parameter, outcome) in rows {
            let (name, message) = parameter_refusal(outcome).map_err(|e| format!("{what} at {far}: {e}"))?;
            assert_eq!(name, parameter, "{what} at {far}");
            assert!(
                message.contains(&far.to_string()),
                "{what}: {message:?} lacks the value {far}"
            );
        }
    }
    // points within the limit that the move, turn, scaling or constructor carries beyond it
    let edge = |x: f64, y: f64| Path::line((x, y), (0.0, 0.0));
    let rows = [
        ("translate", "dx", edge(9e99, 0.0)?.translate(9e99, 0.0)),
        (
            "rotate",
            "angle",
            edge(9e99, 9e99)?.rotate(45.0, Some(Point::new(0.0, 0.0))),
        ),
        ("scale", "by", edge(1e99, 0.0)?.scale(1e5, Some(Point::new(0.0, 0.0)))),
        ("rect", "width", Path::rect(9e99, 0.0, 9e99, 1.0, 0.0)),
        ("circle", "radius", Path::circle((9e99, 0.0), 9e99)),
        ("polygon", "radius", Path::polygon((9e99, 0.0), 9e99, 4)),
        ("star", "outer", Path::star((0.0, 9e99), 9e99, 1.0, 5)),
    ];
    for (what, parameter, outcome) in rows {
        let (name, message) = parameter_refusal(outcome).map_err(|e| format!("{what}: {e}"))?;
        assert_eq!(name, parameter, "{what}");
        assert!(message.contains("too large"), "{what}: {message:?}");
    }
    Ok(())
}

#[test]
fn a_query_point_beyond_the_limit_is_refused_naming_it() -> Outcome {
    let line = Path::line((0.0, 0.0), (10.0, 5.0))?;
    for far in TOO_BIG {
        let rows = [
            ("locate", "point", line.locate((far, 0.0), None).map(drop)),
            ("contains", "point", line.contains((0.0, -far)).map(drop)),
            (
                "locate near",
                "near",
                line.locate((1.0, 1.0), Some(Point::new(far, 0.0))).map(drop),
            ),
            (
                "point_at near",
                "near",
                line.point_at(0.5, Some(Point::new(0.0, far))).map(drop),
            ),
            (
                "tangent_at near",
                "near",
                line.tangent_at(0.5, Some(Point::new(far, far))).map(drop),
            ),
            (
                "slice near",
                "near",
                line.slice(0.0, 0.5, Some(Point::new(-far, 0.0))).map(drop),
            ),
        ];
        for (what, parameter, outcome) in rows {
            let (name, message) = parameter_refusal(outcome).map_err(|e| format!("{what} at {far}: {e}"))?;
            assert_eq!(name, parameter, "{what} at {far}");
            assert!(
                message.contains(&far.to_string()),
                "{what}: {message:?} lacks the value {far}"
            );
        }
    }
    Ok(())
}

#[test]
fn the_limit_is_1e100_exactly_in_both_signs() -> Outcome {
    let beyond = f64::from_bits(1e100_f64.to_bits() + 1);
    Path::start((1e100, -1e100))?;
    Path::start((-1e100, 1e100))?;
    Path::rect(1e100, -1e100, 0.0, 0.0, 0.0)?;
    Path::line((0.0, 0.0), (1.0, 1.0))?.translate(1e100, -1e100)?;
    Path::from_svg_path("M1e100 -1e100 L-1e100 1e100")?;
    for (what, parameter, outcome) in [
        ("start", "point", Path::start((beyond, 0.0))),
        ("start negative", "point", Path::start((0.0, -beyond))),
        ("rect", "x", Path::rect(beyond, 0.0, 0.0, 0.0, 0.0)),
        (
            "translate",
            "dx",
            Path::line((0.0, 0.0), (1.0, 1.0))?.translate(beyond, 0.0),
        ),
        ("data", "data", Path::from_svg_path(&format!("M{beyond:e} 0"))),
    ] {
        let (name, _) = parameter_refusal(outcome).map_err(|e| format!("{what}: {e}"))?;
        assert_eq!(name, parameter, "{what}");
    }
    Ok(())
}

/// Whether `found` is within a relative `tolerance` of `expected`.
fn near(found: f64, expected: f64, tolerance: f64) -> bool {
    (found - expected).abs() <= tolerance * expected.abs()
}

#[test]
fn a_line_of_1e99_measures_exactly_like_a_short_one() -> Outcome {
    let big = 1e99;
    let line = Path::line((-big, 0.0), (big, 0.0))?;
    assert!(near(line.length(), 2.0 * big, 1e-12), "length {}", line.length());
    assert_eq!(line.point_at(0.0, None)?, Point::new(-big, 0.0));
    assert_eq!(line.point_at(1.0, None)?, Point::new(big, 0.0));
    assert_eq!(line.tangent_at(0.0, None)?, Point::new(1.0, 0.0));
    assert_eq!(line.tangent_at(1.0, None)?, Point::new(1.0, 0.0));
    assert!(near(line.locate((0.0, 5.0), None)?, 0.5, 1e-12));
    assert_eq!(line.locate((big, big), None)?, 1.0);
    let bounds = line.bounds()?;
    assert_eq!((bounds.x, bounds.width, bounds.height), (-big, 2.0 * big, 0.0));
    let half = line.slice(0.0, 0.5, None)?;
    assert_eq!(half.point_at(0.0, None)?, Point::new(-big, 0.0));
    assert!(
        half.point_at(1.0, None)?.x.abs() <= 1e-12 * big,
        "{:?}",
        half.to_svg_path()
    );
    Ok(())
}

#[test]
fn a_sub_path_of_no_length_before_a_long_line_is_passed_over_when_slicing() -> Outcome {
    let path = Path::from_svg_path("M0 0 M1 0 L1e99 0")?;
    let half = path.slice(0.0, 0.5, None)?;
    assert_eq!(half.point_at(0.0, None)?, Point::new(1.0, 0.0));
    assert!(
        near(half.point_at(1.0, None)?.x, 5e98, 1e-12),
        "{:?}",
        half.to_svg_path()
    );
    assert_eq!(path.point_at(0.0, None)?, Point::new(1.0, 0.0));
    Ok(())
}

#[test]
fn shapes_of_1e99_keep_their_proportions() -> Outcome {
    let big = 1e99;
    let square = Path::rect(-big, -big, 2.0 * big, 2.0 * big, 0.0)?;
    assert!(near(square.length(), 8.0 * big, 1e-12));
    assert!(square.contains((0.0, 0.0))?);
    assert!(!square.contains((0.0, 3.0 * big / 2.0 * 1.5))?);
    let circle = Path::circle((0.0, 0.0), big)?;
    let turn = 2.0 * std::f64::consts::PI * big;
    assert!(near(circle.length(), turn, 1e-3), "length {}", circle.length());
    let top = circle.point_at(0.0, None)?;
    assert_eq!((top.x, top.y), (0.0, -big));
    let direction = circle.tangent_at(0.0, None)?;
    assert!(
        near(
            (direction.x * direction.x + direction.y * direction.y).sqrt(),
            1.0,
            1e-12
        ),
        "{direction:?}"
    );
    Ok(())
}

/// A coordinate whose size is spread over the orders of magnitude, mostly within the limit and sometimes far beyond
/// it, with either sign.
fn coordinate() -> impl Strategy<Value = f64> + Clone {
    (
        prop_oneof![4 => log_uniform(-3.0, 100.0), 1 => log_uniform(100.0, 308.0)],
        any::<bool>(),
    )
        .prop_map(|(size, negative)| if negative { -size } else { size })
}

/// A point whose coordinates are as for [`coordinate`].
fn corner() -> impl Strategy<Value = (f64, f64)> + Clone {
    (coordinate(), coordinate())
}

/// A path from `(0, 0)` with each of `steps` that the library accepts; a step it refuses leaves the path as it was.
fn accepted(steps: &[Step]) -> Result<Path, Error> {
    let mut path = Path::start((0.0, 0.0))?;
    for step in steps {
        let next = match *step {
            Step::Move(p) => path.move_to(p),
            Step::Line(p) => path.line_to(p),
            Step::Quad(c, p) => path.quad_to(c, p),
            Step::Curve(a, b, p) => path.curve_to(a, b, p),
            Step::Close => path.close(),
        };
        if let Ok(next) = next {
            path = next;
        }
    }
    Ok(path)
}

fn finite_pair(point: Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

/// Every verb on `path` gives finite numbers, and none panics.
fn check_every_verb(path: &Path) -> Result<(), TestCaseError> {
    let length = path.length();
    prop_assert!(length.is_finite() && length >= 0.0, "length {}", length);
    let bounds = path.bounds().map_err(|e| TestCaseError::fail(e.to_string()))?;
    for number in [bounds.x, bounds.y, bounds.width, bounds.height] {
        prop_assert!(number.is_finite(), "bounds {:?}", bounds);
    }
    for at in [0.0, 0.5, 1.0] {
        let point = path
            .point_at(at, None)
            .map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert!(finite_pair(point), "point_at({}) {:?}", at, point);
        match path.tangent_at(at, None) {
            Ok(direction) => {
                prop_assert!(length > 0.0, "a direction on a path of no length");
                let norm = (direction.x * direction.x + direction.y * direction.y).sqrt();
                prop_assert!((norm - 1.0).abs() <= 1e-9, "tangent_at({}) {:?}", at, direction);
            }
            Err(Error::Geometry { .. }) => prop_assert!(length == 0.0, "no direction on a path with length"),
            Err(other) => return Err(TestCaseError::fail(other.to_string())),
        }
        let found = path
            .locate(point, None)
            .map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert!((0.0..=1.0).contains(&found), "locate {}", found);
        path.contains(point).map_err(|e| TestCaseError::fail(e.to_string()))?;
    }
    for (from, to) in [(0.0, 0.5), (1.0 / 3.0, 1.0)] {
        let piece = path
            .slice(from, to, None)
            .map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert!(piece.length().is_finite(), "slice length");
        prop_assert!(
            piece
                .bounds()
                .is_ok_and(|b| b.width.is_finite() && b.height.is_finite())
        );
    }
    let reversed = path.reverse();
    prop_assert!(reversed.length().is_finite());
    for part in path.split() {
        prop_assert!(part.length().is_finite());
    }
    prop_assert!(!path.to_svg_path().contains("inf"));
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    // Whatever the library accepts it can measure, cut and hit-test: no number it gives is NaN or infinite, and nothing
    // panics, at every size up to the limit (and with sizes beyond it, which must be refused, never half-accepted).
    #[test]
    fn any_accepted_path_is_measured_cut_and_hit_tested_with_finite_numbers(
        steps in prop::collection::vec(step(corner()), 1..7),
        shift in (coordinate(), coordinate()),
        turn in -720.0f64..720.0,
        factor in log_uniform(-3.0, 5.0),
    ) {
        let path = accepted(&steps).map_err(|e| TestCaseError::fail(e.to_string()))?;
        check_every_verb(&path)?;
        // a transform that is accepted gives a path just as sound
        for moved in [
            path.translate(shift.0, shift.1),
            path.rotate(turn, None),
            path.scale(factor, None),
        ]
        .into_iter()
        .flatten()
        {
            check_every_verb(&moved)?;
        }
    }
}
