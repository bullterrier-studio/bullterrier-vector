//! The booleans of `Path` (`resolve`, `union`, `subtract`, `intersect`, `exclude`) through the public API. Expected
//! values are hand-computed areas, and for random polygons a winding number and the shoelace area written in the tests
//! (`common`), never the library's own region code.

use crate::common::region::canonical_fault;
use crate::common::{
    Corners, Outcome, Ring, clearance, log_uniform, net_area, parameter_refusal, polygons_path, rings, shoelace_area,
    winding,
};
use bullterrier_vector::{Error, Path};
use proptest::prelude::*;
use std::f64::consts::PI;

/// The design tolerance the hand cases use for straight-edged shapes: nothing in them is curved.
const TOLERANCE: f64 = 0.01;

/// The four verbs in the order the tables list them.
const VERBS: [&str; 4] = ["union", "subtract", "intersect", "exclude"];

fn apply(verb: usize, a: &Path, b: &Path, tolerance: f64) -> Result<Path, Error> {
    match verb {
        0 => a.union(b, tolerance),
        1 => a.subtract(b, tolerance),
        2 => a.intersect(b, tolerance),
        _ => a.exclude(b, tolerance),
    }
}

fn square(x: f64, y: f64, size: f64) -> Result<Path, Error> {
    polygons_path(&[square_corners(x, y, size)])
}

/// A square clockwise on screen, from its top left corner.
fn square_corners(x: f64, y: f64, size: f64) -> Corners {
    vec![(x, y), (x + size, y), (x + size, y + size), (x, y + size)]
}

fn backwards(mut corners: Corners) -> Corners {
    corners.reverse();
    corners
}

// Hand cases ---------------------------------------------------------------------------------------------------------

/// One pair of shapes and, for union, subtract, intersect and exclude in turn, the area worked out by hand and the
/// number of rings (outer contours and holes) of the result.
struct Case {
    name: &'static str,
    a: Path,
    b: Path,
    expected: [(f64, usize); 4],
}

#[expect(clippy::too_many_lines, reason = "a table: one row per pair of shapes")]
fn cases() -> Result<Vec<Case>, Error> {
    let eight = polygons_path(&[vec![(0.0, 0.0), (4.0, 4.0), (4.0, 0.0), (0.0, 4.0)]])?;
    let lone_move = Path::start((1.0, 1.0))?;
    let a_line = Path::line((0.0, 0.0), (9.0, 9.0))?;
    let open_square = Path::polyline(square_corners(0.0, 0.0, 4.0), false)?;
    let ring = polygons_path(&[square_corners(0.0, 0.0, 10.0), backwards(square_corners(3.0, 3.0, 4.0))])?;
    let overlapping = polygons_path(&[square_corners(0.0, 0.0, 4.0), square_corners(2.0, 2.0, 4.0)])?;
    let cancelling = polygons_path(&[square_corners(0.0, 0.0, 4.0), backwards(square_corners(2.0, 2.0, 4.0))])?;
    Ok(vec![
        Case {
            name: "overlapping squares",
            a: square(0.0, 0.0, 4.0)?,
            b: square(2.0, 2.0, 4.0)?,
            expected: [(28.0, 1), (12.0, 1), (4.0, 1), (24.0, 2)],
        },
        Case {
            name: "overlapping squares, the second counter-clockwise",
            a: square(0.0, 0.0, 4.0)?,
            b: polygons_path(&[backwards(square_corners(2.0, 2.0, 4.0))])?,
            expected: [(28.0, 1), (12.0, 1), (4.0, 1), (24.0, 2)],
        },
        Case {
            name: "a square with a smaller one inside",
            a: square(0.0, 0.0, 10.0)?,
            b: square(3.0, 3.0, 4.0)?,
            expected: [(100.0, 1), (84.0, 2), (16.0, 1), (84.0, 2)],
        },
        Case {
            name: "a ring and a square filling its hole",
            a: ring.clone(),
            b: square(3.0, 3.0, 4.0)?,
            expected: [(100.0, 1), (84.0, 2), (0.0, 0), (100.0, 1)],
        },
        Case {
            name: "a ring and nothing",
            a: ring,
            b: Path::new(),
            expected: [(84.0, 2), (84.0, 2), (0.0, 0), (84.0, 2)],
        },
        Case {
            name: "squares sharing an edge",
            a: square(0.0, 0.0, 4.0)?,
            b: square(4.0, 0.0, 4.0)?,
            expected: [(32.0, 1), (16.0, 1), (0.0, 0), (32.0, 1)],
        },
        Case {
            name: "squares touching at a corner",
            a: square(0.0, 0.0, 4.0)?,
            b: square(4.0, 4.0, 4.0)?,
            expected: [(32.0, 2), (16.0, 1), (0.0, 0), (32.0, 2)],
        },
        Case {
            name: "identical squares",
            a: square(0.0, 0.0, 4.0)?,
            b: square(0.0, 0.0, 4.0)?,
            expected: [(16.0, 1), (0.0, 0), (16.0, 1), (0.0, 0)],
        },
        Case {
            name: "separate squares",
            a: square(0.0, 0.0, 4.0)?,
            b: square(10.0, 0.0, 4.0)?,
            expected: [(32.0, 2), (16.0, 1), (0.0, 0), (32.0, 2)],
        },
        Case {
            name: "a square and nothing",
            a: square(0.0, 0.0, 4.0)?,
            b: Path::new(),
            expected: [(16.0, 1), (16.0, 1), (0.0, 0), (16.0, 1)],
        },
        Case {
            name: "nothing and a square",
            a: Path::new(),
            b: square(0.0, 0.0, 4.0)?,
            expected: [(16.0, 1), (0.0, 0), (0.0, 0), (16.0, 1)],
        },
        Case {
            name: "nothing and nothing",
            a: Path::new(),
            b: Path::new(),
            expected: [(0.0, 0); 4],
        },
        Case {
            name: "a bow-tie and nothing: two triangles of 4",
            a: eight,
            b: Path::new(),
            expected: [(8.0, 2), (8.0, 2), (0.0, 0), (8.0, 2)],
        },
        Case {
            name: "two overlapping squares of one path and nothing: they join",
            a: overlapping,
            b: Path::new(),
            expected: [(28.0, 1), (28.0, 1), (0.0, 0), (28.0, 1)],
        },
        Case {
            name: "two squares of one path that run opposite ways: they cancel where they overlap",
            a: cancelling,
            b: Path::new(),
            expected: [(24.0, 2), (24.0, 2), (0.0, 0), (24.0, 2)],
        },
        Case {
            name: "an open outline fills as if closed",
            a: open_square,
            b: square(2.0, 2.0, 4.0)?,
            expected: [(28.0, 1), (12.0, 1), (4.0, 1), (24.0, 2)],
        },
        Case {
            name: "a lone move and a line fill nothing",
            a: lone_move,
            b: a_line,
            expected: [(0.0, 0); 4],
        },
    ])
}

#[test]
fn the_four_verbs_give_the_hand_computed_area_and_a_canonical_form() -> Outcome {
    for case in cases()? {
        for (verb, (area, count)) in case.expected.into_iter().enumerate() {
            let result = apply(verb, &case.a, &case.b, TOLERANCE)?;
            let found = rings(&result)?;
            let what = format!("{} of {}", VERBS[verb], case.name);
            assert!(
                (net_area(&found) - area).abs() < 1e-9,
                "{what}: area {}",
                net_area(&found)
            );
            assert_eq!(found.len(), count, "{what}: rings {found:?}");
            assert_eq!(canonical_fault(&found), None, "{what}: {found:?}");
        }
    }
    Ok(())
}

#[test]
fn resolve_is_the_union_of_a_path_with_nothing() -> Outcome {
    for case in cases()? {
        assert_eq!(
            case.a.resolve(TOLERANCE)?,
            case.a.union(&Path::new(), TOLERANCE)?,
            "{}",
            case.name
        );
    }
    Ok(())
}

#[test]
fn resolving_a_resolved_path_of_whole_numbers_changes_nothing() -> Outcome {
    for case in cases()? {
        for (verb, name) in VERBS.iter().enumerate() {
            let result = apply(verb, &case.a, &case.b, TOLERANCE)?;
            assert_eq!(result.resolve(TOLERANCE)?, result, "{name} of {}", case.name);
        }
    }
    Ok(())
}

#[test]
fn a_bow_tie_and_a_square_combine_by_the_hand_computed_areas() -> Outcome {
    let eight = polygons_path(&[vec![(0.0, 0.0), (4.0, 4.0), (4.0, 0.0), (0.0, 4.0)]])?;
    let middle = square(1.0, 1.0, 2.0)?;
    // the two triangles are 8 together; the square covers 1 of each side's triangle
    for (verb, area) in [10.0, 6.0, 2.0, 8.0].into_iter().enumerate() {
        let found = rings(&apply(verb, &eight, &middle, TOLERANCE)?)?;
        assert!((net_area(&found) - area).abs() < 1e-9, "{}: {found:?}", VERBS[verb]);
        assert_eq!(canonical_fault(&found), None, "{}: {found:?}", VERBS[verb]);
    }
    Ok(())
}

#[test]
fn the_result_is_the_same_whatever_order_and_start_the_input_was_written_in() -> Outcome {
    let one = polygons_path(&[square_corners(0.0, 0.0, 4.0), square_corners(2.0, 2.0, 4.0)])?;
    let other = polygons_path(&[
        vec![(6.0, 6.0), (2.0, 6.0), (2.0, 2.0), (6.0, 2.0)],
        vec![(4.0, 4.0), (0.0, 4.0), (0.0, 0.0), (4.0, 0.0)],
    ])?;
    // the same two squares, the second pair written with other starts, other order, and the first the other way round
    assert_eq!(one.resolve(TOLERANCE)?, other.resolve(TOLERANCE)?);
    Ok(())
}

#[test]
fn contours_come_out_in_the_canonical_order_each_outer_followed_by_its_holes() -> Outcome {
    // written in no order, every contour started at a corner that is not its topmost and run the wrong way for its role
    let messy = polygons_path(&[
        // hole at (12, 2), run clockwise
        vec![(12.0, 6.0), (12.0, 2.0), (16.0, 2.0), (16.0, 6.0)],
        // the big outer, run counter-clockwise from its bottom right
        vec![(20.0, 20.0), (20.0, 0.0), (0.0, 0.0), (0.0, 20.0)],
        // hole at (5, 10)
        vec![(9.0, 14.0), (5.0, 14.0), (5.0, 10.0), (9.0, 10.0)],
        // an outer to the right whose top is as high as the big one: it comes after it
        vec![(25.0, 2.0), (27.0, 2.0), (27.0, 0.0), (25.0, 0.0)],
        // an outer higher than all
        vec![(31.0, -1.0), (34.0, -1.0), (34.0, -5.0), (31.0, -5.0)],
        // hole at (2, 2), run clockwise
        vec![(2.0, 2.0), (6.0, 2.0), (6.0, 6.0), (2.0, 6.0)],
    ])?;
    let found = rings(&messy.resolve(TOLERANCE)?)?;
    let starts: Vec<(f64, f64)> = found.iter().map(|ring| ring[0]).collect();
    assert_eq!(
        starts,
        [
            (31.0, -5.0), // the highest outer
            (0.0, 0.0),   // the big outer
            (2.0, 2.0),   // its holes: top to bottom, then left to right
            (12.0, 2.0),
            (5.0, 10.0),
            (25.0, 0.0), // the outer that starts level with the big one but further right
        ]
    );
    // a square from its top left corner clockwise, a hole from its top left corner counter-clockwise
    assert_eq!(found[0], [(31.0, -5.0), (34.0, -5.0), (34.0, -1.0), (31.0, -1.0)]);
    assert_eq!(found[2], [(2.0, 2.0), (2.0, 6.0), (6.0, 6.0), (6.0, 2.0)]);
    assert_eq!(canonical_fault(&found), None);
    Ok(())
}

/// What is wrong with the polygon `ring` as a flattening of the circle of `radius` round the origin that the library's
/// cubic Bezier circle makes, flattened to within `tolerance`, if anything.
fn circle_fault(ring: &Ring, radius: f64, tolerance: f64) -> Option<String> {
    // a cubic quarter circle bulges out by at most 2.7e-4 of the radius; the corners are on that curve, and a chord's
    // middle is within `tolerance` of it
    let bulge = 3.0e-4 * radius;
    for (index, &(x, y)) in ring.iter().enumerate() {
        let (next_x, next_y) = ring[(index + 1) % ring.len()];
        let corner = libm::hypot(x, y);
        let middle = libm::hypot(f64::midpoint(x, next_x), f64::midpoint(y, next_y));
        if (corner - radius).abs() > bulge + 1e-9 * radius {
            return Some(format!("corner {index} is {corner} from the centre"));
        }
        if middle < radius - bulge - tolerance - 1e-9 * radius {
            return Some(format!(
                "the edge from corner {index} cuts {} off the circle",
                radius - middle
            ));
        }
    }
    None
}

#[test]
fn a_curve_is_flattened_to_within_the_tolerance() -> Outcome {
    let circle = Path::circle((0.0, 0.0), 100.0)?;
    let mut corner_counts = Vec::new();
    for tolerance in [0.01, 1.0] {
        let found = rings(&circle.union(&Path::new(), tolerance)?)?;
        assert_eq!(found.len(), 1);
        assert_eq!(circle_fault(&found[0], 100.0, tolerance), None, "tolerance {tolerance}");
        corner_counts.push(found[0].len());
    }
    // known bad for the checker: a flattening to 1.0 is too coarse for 0.01
    let coarse = rings(&circle.resolve(1.0)?)?;
    assert!(circle_fault(&coarse[0], 100.0, 0.01).is_some());
    assert!(
        corner_counts[0] > 2 * corner_counts[1],
        "the finer tolerance gives more corners: {corner_counts:?}"
    );
    Ok(())
}

#[test]
fn a_tolerance_that_is_not_a_positive_number_within_the_coordinate_limit_is_refused() -> Outcome {
    let a = square(0.0, 0.0, 4.0)?;
    let b = square(2.0, 2.0, 4.0)?;
    let rows = [
        (0.0, "zero"),
        (-1.0, "negative"),
        (f64::NAN, "NaN"),
        (f64::INFINITY, "infinite"),
        (1.0e101, "beyond the coordinate limit"),
    ];
    for (tolerance, what) in rows {
        for (verb, name) in VERBS.iter().enumerate() {
            let (parameter, message) = parameter_refusal(apply(verb, &a, &b, tolerance))?;
            assert_eq!(parameter, "tolerance", "{name} with a tolerance that is {what}");
            assert!(message.contains(&tolerance.to_string()), "{message}");
        }
        let (parameter, _) = parameter_refusal(a.resolve(tolerance))?;
        assert_eq!(parameter, "tolerance", "resolve with a tolerance that is {what}");
    }
    // known good: the largest tolerance there is
    a.union(&b, 1.0e100)?;
    Ok(())
}

#[test]
fn shapes_out_at_the_coordinate_limit_combine() -> Outcome {
    let far = 1.0e100;
    let big = polygons_path(&[vec![(-far, -far), (far, -far), (far, far), (-far, far)]])?;
    let small = square(0.0, 0.0, 1.0)?;
    let found = rings(&big.union(&small, TOLERANCE)?)?;
    assert_eq!(found.len(), 1);
    assert!((net_area(&found) / (4.0 * far * far) - 1.0).abs() < 1e-12);
    // the small square's area is far below what the big one's area resolves
    let found = rings(&big.subtract(&small, TOLERANCE)?)?;
    assert!((net_area(&found) / (4.0 * far * far) - 1.0).abs() < 1e-12);
    Ok(())
}

#[test]
fn shapes_of_any_size_combine_to_the_same_relative_area() -> Outcome {
    for size in [1.0e-9, 1.0e-3, 1.0, 1.0e6, 1.0e30] {
        let a = square(0.0, 0.0, 4.0 * size)?;
        let b = square(2.0 * size, 2.0 * size, 4.0 * size)?;
        for (verb, area) in [28.0, 12.0, 4.0, 24.0].into_iter().enumerate() {
            let found = rings(&apply(verb, &a, &b, size)?)?;
            let expected = area * size * size;
            assert!(
                (net_area(&found) - expected).abs() <= 1e-12 * expected,
                "{} at size {size}: {}",
                VERBS[verb],
                net_area(&found)
            );
        }
    }
    Ok(())
}

// Regressions: inputs the fuzz found, as they were minimised ---------------------------------------------------------

#[test]
#[expect(clippy::unreadable_literal, reason = "the numbers of a failing run, as they were")]
fn an_outline_that_hardly_leaves_a_line_has_no_contour_that_rounds_to_a_line() -> Outcome {
    // five corners on the line y = 0.7 x + 3, each a few units in the last place off it, written in no order: the
    // outline crosses itself, and its pieces enclose some 1e-15. The engine keeps the pieces (its grid is finer than a
    // number); written as numbers, two of the three corners of a piece are in line with the third.
    let outline = Path::polyline(
        [
            (3.1147088378134846, 5.180296186469447),
            (7.931189843963503, 8.551832890774438),
            (8.207948272450677, 8.745563790715485),
            (5.227353250142145, 6.659147275099491),
            (9.136855905853253, 9.395799134097295),
        ],
        true,
    )?;
    let found = rings(&outline.resolve(TOLERANCE)?)?;
    assert_eq!(canonical_fault(&found), None, "{found:?}");
    assert!(net_area(&found).abs() < 1e-12, "{found:?}");
    Ok(())
}

#[test]
#[expect(clippy::unreadable_literal, reason = "the numbers of a failing run, as they were")]
fn a_curve_that_runs_out_and_back_along_one_line_leaves_no_contour_that_rounding_has_turned_over() -> Outcome {
    // a cubic whose controls are its end and its start: flattened, its points lie on the line to within rounding, and
    // the pieces of it that cross enclose some 1e-15; a piece that rounding has turned over would be a hole of an
    // outer contour that is not there
    let end = (94.0263645004414, 80.97995546626417);
    let out_and_back = Path::start((0.0, 0.0))?.curve_to(end, (0.0, 0.0), end)?.close()?;
    let (low, high) = (224.99441768192287, 824.9944176819229);
    let square = square(low, low, high - low)?;
    let found = rings(&out_and_back.union(&square, TOLERANCE)?)?;
    assert_eq!(canonical_fault(&found), None, "{found:?}");
    assert!(
        (net_area(&found) - (high - low) * (high - low)).abs() < 1e-6,
        "{found:?}"
    );
    Ok(())
}

// Properties ---------------------------------------------------------------------------------------------------------

/// Closed polygons on a coarse lattice, so that edges coincide, touch and cross at corners, and a polygon may wind
/// either way and cross itself.
fn polygons() -> impl Strategy<Value = Vec<Corners>> {
    let corner = (0..60_i32, 0..60_i32).prop_map(|(x, y)| (f64::from(x), f64::from(y)));
    prop::collection::vec(prop::collection::vec(corner, 3..8), 1..4)
}

fn points() -> impl Strategy<Value = Vec<(f64, f64)>> {
    prop::collection::vec((-5.0..65.0_f64, -5.0..65.0_f64), 60)
}

fn fail(error: &impl ToString) -> TestCaseError {
    TestCaseError::fail(error.to_string())
}

/// Whether `a` and `b` fill the same area, give or take the rounding of the engine's grid. Not the same corners:
/// the grid is chosen from the extent of what the engine is given, and where two contours touch at a point that was
/// rounded, a second run may join them or leave them apart.
fn same_area(a: &Path, b: &Path) -> Result<bool, TestCaseError> {
    let (a, b) = (
        net_area(&rings(a).map_err(|e| fail(&e))?),
        net_area(&rings(b).map_err(|e| fail(&e))?),
    );
    Ok((a - b).abs() <= 1e-9 * (1.0 + a.abs()))
}

/// The winding number of `polygons` round `point`, summed over the polygons.
fn winding_of(polygons: &[Corners], point: (f64, f64)) -> i32 {
    polygons.iter().map(|corners| winding(corners, point)).sum()
}

/// The distance from `point` to the nearest edge of any of `polygons`.
fn nearest_edge(polygons: &[Corners], point: (f64, f64)) -> f64 {
    polygons
        .iter()
        .map(|corners| clearance(corners, point))
        .fold(f64::INFINITY, f64::min)
}

/// A star-shaped polygon, so a simple one: a corner in each of `n` equal sectors, the angle and the radius random.
fn star_shaped() -> impl Strategy<Value = Corners> {
    (4..12_usize, 10.0..50.0_f64, 10.0..50.0_f64).prop_flat_map(|(n, cx, cy)| {
        prop::collection::vec((0.0..0.8_f64, 5.0..25.0_f64), n).prop_map(move |corners| {
            corners
                .iter()
                .enumerate()
                .map(|(index, (share, radius))| {
                    #[expect(clippy::cast_precision_loss, reason = "a dozen corners")]
                    let angle = 2.0 * PI * (index as f64 + share) / n as f64;
                    (cx + radius * libm::cos(angle), cy + radius * libm::sin(angle))
                })
                .collect()
        })
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn a_point_clear_of_every_edge_is_in_the_result_when_the_set_rule_says_so(
        a in polygons(),
        b in polygons(),
        probes in points(),
    ) {
        let (path_a, path_b) = (polygons_path(&a)?, polygons_path(&b)?);
        let results: Vec<Path> = (0..4).map(|verb| apply(verb, &path_a, &path_b, TOLERANCE)).collect::<Result<_, _>>()?;
        for point in probes {
            if nearest_edge(&a, point).min(nearest_edge(&b, point)) < 1e-6 {
                continue;
            }
            let (in_a, in_b) = (winding_of(&a, point) != 0, winding_of(&b, point) != 0);
            let wanted = [in_a || in_b, in_a && !in_b, in_a && in_b, in_a != in_b];
            for verb in 0..4 {
                prop_assert_eq!(
                    results[verb].contains(point)?,
                    wanted[verb],
                    "{} at {:?}: in a {}, in b {}", VERBS[verb], point, in_a, in_b
                );
            }
        }
    }

    #[test]
    fn the_result_is_canonical_and_resolving_it_again_keeps_the_area(a in polygons(), b in polygons()) {
        let (path_a, path_b) = (polygons_path(&a)?, polygons_path(&b)?);
        for (verb, name) in VERBS.iter().enumerate() {
            let result = apply(verb, &path_a, &path_b, TOLERANCE)?;
            prop_assert_eq!(canonical_fault(&rings(&result).map_err(|e| fail(&e))?), None);
            prop_assert!(same_area(&result, &result.resolve(TOLERANCE)?)?, "{name} resolved again");
        }
        let resolved = path_a.resolve(TOLERANCE)?;
        prop_assert_eq!(canonical_fault(&rings(&resolved).map_err(|e| fail(&e))?), None);
        prop_assert!(same_area(&resolved, &resolved.resolve(TOLERANCE)?)?);
    }

    #[test]
    fn the_result_does_not_depend_on_where_each_polygon_starts_or_the_order_they_are_written_in(
        a in polygons(),
        turns in prop::collection::vec(0..8_usize, 3),
    ) {
        let mut shuffled: Vec<Corners> = a.iter().rev().cloned().collect();
        for (corners, turn) in shuffled.iter_mut().zip(turns.iter().cycle()) {
            let by = turn % corners.len();
            corners.rotate_left(by);
        }
        prop_assert_eq!(
            polygons_path(&a)?.resolve(TOLERANCE)?.to_svg_path(),
            polygons_path(&shuffled)?.resolve(TOLERANCE)?.to_svg_path()
        );
    }

    #[test]
    fn the_areas_of_the_four_results_add_up_like_sets(a in star_shaped(), b in star_shaped()) {
        let (path_a, path_b) = (polygons_path(std::slice::from_ref(&a))?, polygons_path(std::slice::from_ref(&b))?);
        let (area_a, area_b) = (shoelace_area(&a).abs(), shoelace_area(&b).abs());
        let area = |verb: usize| -> Result<f64, TestCaseError> {
            let found = rings(&apply(verb, &path_a, &path_b, TOLERANCE)?).map_err(|e| fail(&e))?;
            Ok(net_area(&found))
        };
        let (union, subtract, intersect, exclude) = (area(0)?, area(1)?, area(2)?, area(3)?);
        let slack = 1e-9 * (area_a + area_b);
        prop_assert!((union - (area_a + area_b - intersect)).abs() <= slack, "union {union}");
        prop_assert!((subtract - (area_a - intersect)).abs() <= slack, "subtract {subtract}");
        prop_assert!((exclude - (union - intersect)).abs() <= slack, "exclude {exclude}");
        prop_assert!(intersect <= area_a.min(area_b) + slack && union + slack >= area_a.max(area_b));
    }

    #[test]
    fn a_circle_of_any_size_is_flattened_to_within_the_tolerance(
        radius in log_uniform(-1.0, 4.0),
        share in 0.001..0.5_f64,
    ) {
        let tolerance = share * radius;
        let found = rings(&Path::circle((0.0, 0.0), radius)?.resolve(tolerance)?).map_err(|e| fail(&e))?;
        prop_assert_eq!(found.len(), 1);
        prop_assert_eq!(circle_fault(&found[0], radius, tolerance), None);
    }
}
