//! `Path::offset` and `Path::outline` through the public API. The oracles are written here: hand-computed areas and
//! bounds of squares, lines and corners, the distance from every vertex of a result to the polygon it came from, and for
//! random convex polygons that the grown region holds the original and the shrunk one lies inside it.

use crate::common::region::{canonical_fault, crossing_fault, nearest_edge};
use crate::common::{Corners, Outcome, Ring, net_area, parameter_refusal, polygons_path, rings};
use bullterrier_vector::{Cap, Error, Font, Join, Path, Text, Values, text};
use proptest::prelude::*;
use std::f64::consts::PI;

const TOLERANCE: f64 = 0.01;
const LIMIT: f64 = 4.0;

fn area(path: &Path) -> Result<f64, Box<dyn std::error::Error>> {
    Ok(net_area(&rings(path)?))
}

/// `(left, top, right, bottom)` of the outline.
fn extent(path: &Path) -> Result<(f64, f64, f64, f64), Error> {
    let r = path.bounds()?;
    Ok((r.x, r.y, r.x + r.width, r.y + r.height))
}

fn assert_near(actual: f64, wanted: f64, slack: f64, what: &str) {
    assert!(
        (actual - wanted).abs() <= slack,
        "{what}: {actual} is not {wanted} within {slack}"
    );
}

fn assert_extent(path: &Path, wanted: (f64, f64, f64, f64), slack: f64, what: &str) -> Outcome {
    let found = extent(path)?;
    for (got, want) in [found.0, found.1, found.2, found.3]
        .into_iter()
        .zip([wanted.0, wanted.1, wanted.2, wanted.3])
    {
        assert_near(got, want, slack, what);
    }
    Ok(())
}

fn corners_square(size: f64) -> Corners {
    vec![(0.0, 0.0), (size, 0.0), (size, size), (0.0, size)]
}

fn square(size: f64) -> Result<Path, Error> {
    polygons_path(&[corners_square(size)])
}

/// The L: a 40 by 20 bar with a 20 by 20 block on its left end going down. Area 1200, five convex corners, one reflex.
fn l_shape() -> Corners {
    vec![
        (0.0, 0.0),
        (40.0, 0.0),
        (40.0, 20.0),
        (20.0, 20.0),
        (20.0, 40.0),
        (0.0, 40.0),
    ]
}

fn perimeter(corners: &[(f64, f64)]) -> f64 {
    corners
        .iter()
        .enumerate()
        .map(|(i, &p)| crate::common::distance(p, corners[(i + 1) % corners.len()]))
        .sum()
}

// A square of 10 grown or shrunk by 2: the area each join leaves, and the bounds, which every join shares.
#[test]
fn a_square_changes_by_the_area_its_join_leaves() -> Outcome {
    // (what, join, distance, miter limit, tolerance, area wanted, how much less than wanted the arcs may leave)
    let rows = [
        ("miter out", Join::Miter, 2.0, LIMIT, TOLERANCE, 196.0, 0.0),
        ("bevel out", Join::Bevel, 2.0, LIMIT, TOLERANCE, 188.0, 0.0),
        // see `a_miter_beyond_the_limit_is_clipped_flat`: the clip at a limit of 1 leaves a nip
        // of 0.06 at each corner where a bevel leaves none
        (
            "a limit of 1 all but bevels every corner",
            Join::Miter,
            2.0,
            1.0,
            TOLERANCE,
            188.0 + 0.3,
            0.3,
        ),
        (
            "round out",
            Join::Round,
            2.0,
            LIMIT,
            TOLERANCE,
            100.0 + 80.0 + 4.0 * PI,
            2.0 * PI * 2.0 * TOLERANCE,
        ),
        (
            "round out, coarse",
            Join::Round,
            2.0,
            LIMIT,
            0.5,
            100.0 + 80.0 + 4.0 * PI,
            2.0 * PI * 2.0 * 0.5,
        ),
        (
            "round out, wide",
            Join::Round,
            20.0,
            LIMIT,
            TOLERANCE,
            100.0 + 800.0 + 400.0 * PI,
            2.0 * PI * 20.0 * TOLERANCE,
        ),
        ("miter in", Join::Miter, -2.0, LIMIT, TOLERANCE, 36.0, 0.0),
        ("bevel in", Join::Bevel, -2.0, LIMIT, TOLERANCE, 36.0, 0.0),
        ("round in", Join::Round, -2.0, LIMIT, TOLERANCE, 36.0, 0.0),
    ];
    for (what, join, distance, limit, tolerance, wanted, arcs) in rows {
        let grown = square(10.0)?.offset(distance, join, limit, tolerance)?;
        let found = area(&grown)?;
        assert!(
            found <= wanted + 1e-6 && found >= wanted - arcs - 1e-6,
            "{what}: area {found}, wanted {wanted}"
        );
        assert_extent(
            &grown,
            (-distance, -distance, 10.0 + distance, 10.0 + distance),
            1e-6,
            what,
        )?;
    }
    Ok(())
}

/// The leftmost x of the tip of a thin spike pointing left, grown by 1 (`offset`) or stroked 2 wide (`outline`) with a miter
/// join: the apex has the half-angle `asin(1/8)`, so a full miter reaches 8 past it.
fn spike_tip(limit: f64, outlined: bool) -> Result<f64, Error> {
    let half = 100.0 / 63.0_f64.sqrt();
    let result = if outlined {
        Path::polyline([(100.0, -half), (0.0, 0.0), (100.0, half)], false)?.outline(
            2.0,
            Join::Miter,
            Cap::Butt,
            limit,
            TOLERANCE,
        )?
    } else {
        polygons_path(&[vec![(0.0, 0.0), (100.0, -half), (100.0, half)]])?.offset(1.0, Join::Miter, limit, TOLERANCE)?
    };
    Ok(extent(&result)?.0)
}

// A miter whose ratio (1 / sin of the half-angle: 8 here) is within the limit reaches its point.
#[test]
fn a_miter_within_the_limit_reaches_its_point() -> Outcome {
    for outlined in [false, true] {
        for limit in [9.0, 30.0] {
            assert_near(
                spike_tip(limit, outlined)?,
                -8.0,
                0.02,
                &format!("limit {limit}, outlined {outlined}"),
            );
        }
    }
    Ok(())
}

// A miter beyond the limit is clipped flat at `sqrt(limit^2 - 1)` times the distance from the end of the offset edge
// (SVG 2's `miter-clip`), not bevelled as a stroke's: the engine has no bevel for it (docs/design-decisions.md,
// "Offsets (V9)"). For the spike that puts the tip at `(1 + sqrt(limit^2 - 1) * sqrt(63)) / 8`.
#[test]
fn a_miter_beyond_the_limit_is_clipped_flat() -> Outcome {
    for outlined in [false, true] {
        for limit in [7.0_f64, 4.0, 2.0] {
            let clip = (1.0 + (limit * limit - 1.0).sqrt() * 63.0_f64.sqrt()) / 8.0;
            assert_near(
                spike_tip(limit, outlined)?,
                -clip,
                0.02,
                &format!("limit {limit}, outlined {outlined}"),
            );
        }
    }
    Ok(())
}

// A round join stays a round join however sharp, and the limit does not touch it.
#[test]
fn the_miter_limit_does_not_change_a_round_or_a_bevel_join() -> Outcome {
    let shape = polygons_path(&[l_shape()])?;
    for join in [Join::Round, Join::Bevel] {
        let one = shape.offset(2.0, join, 1.0, TOLERANCE)?;
        let big = shape.offset(2.0, join, 50.0, TOLERANCE)?;
        assert_eq!(one.to_svg_path(), big.to_svg_path(), "{join:?}");
    }
    Ok(())
}

// Closing (grow then shrink) fills what is narrower than 2r, opening (shrink then grow) removes it and rounds the corners.
#[test]
fn closing_fills_a_slot_and_opening_removes_a_spur() -> Outcome {
    let r = 3.0;
    let tolerance = 0.001;
    let rounded = r * r * (1.0 - PI / 4.0);
    let (round, limit) = (Join::Round, LIMIT);
    let slotted = polygons_path(&[vec![
        (0.0, 0.0),
        (19.0, 0.0),
        (19.0, 10.0),
        (21.0, 10.0),
        (21.0, 0.0),
        (40.0, 0.0),
        (40.0, 20.0),
        (0.0, 20.0),
    ]])?;
    let spurred = polygons_path(&[vec![
        (0.0, 0.0),
        (19.0, 0.0),
        (19.0, -10.0),
        (21.0, -10.0),
        (21.0, 0.0),
        (40.0, 0.0),
        (40.0, 20.0),
        (0.0, 20.0),
    ]])?;
    let l = polygons_path(&[l_shape()])?;
    let slack = |corners: &[(f64, f64)]| 2.0 * perimeter(corners) * tolerance;
    // (what, path, shrinks first, area wanted, slack)
    let rows = [
        (
            "closing fills a slot narrower than 2r",
            &slotted,
            false,
            800.0,
            slack(&l_shape()) * 2.0,
        ),
        (
            "opening removes a spur and rounds four corners",
            &spurred,
            true,
            800.0 - 4.0 * rounded,
            0.5,
        ),
        (
            "closing an L fills a fillet in its reflex corner",
            &l,
            false,
            1200.0 + rounded,
            slack(&l_shape()),
        ),
        (
            "opening an L rounds its five convex corners",
            &l,
            true,
            1200.0 - 5.0 * rounded,
            slack(&l_shape()),
        ),
    ];
    for (what, path, opening, wanted, slack) in rows {
        let (first, second) = if opening { (-r, r) } else { (r, -r) };
        let result = path
            .offset(first, round, limit, tolerance)?
            .offset(second, round, limit, tolerance)?;
        assert_near(area(&result)?, wanted, slack, what);
    }
    // with miter joins the corners come back exactly
    let rectangle = Path::rect(0.0, 0.0, 40.0, 20.0, 0.0)?;
    for (first, second) in [(r, -r), (-r, r)] {
        let result =
            rectangle
                .offset(first, Join::Miter, limit, tolerance)?
                .offset(second, Join::Miter, limit, tolerance)?;
        assert_near(area(&result)?, 800.0, 1e-6, "a rectangle through miters");
    }
    Ok(())
}

// Every vertex of a round-joined result lies |d| from the polygon (the offset of a region is where the distance to it is
// |d|), and no point of an edge is nearer than |d| less the tolerance.
#[test]
fn every_vertex_of_a_round_offset_lies_one_distance_from_the_outline() -> Outcome {
    let star: Corners = (0..10)
        .map(|k| {
            let radius = if k % 2 == 0 { 30.0 } else { 12.0 };
            let angle = f64::from(k) * PI / 5.0;
            (50.0 + radius * libm::cos(angle), 50.0 + radius * libm::sin(angle))
        })
        .collect();
    for (what, source) in [("star", star), ("L", l_shape())] {
        let path = polygons_path(std::slice::from_ref(&source))?;
        for distance in [2.0, -2.0, 6.0, -3.5] {
            let result = path.offset(distance, Join::Round, LIMIT, TOLERANCE)?;
            let found = rings(&result)?;
            assert!(!found.is_empty(), "{what} by {distance}");
            for ring in &found {
                for (index, &vertex) in ring.iter().enumerate() {
                    let at = nearest_edge(std::slice::from_ref(&source), vertex);
                    assert_near(
                        at,
                        distance.abs(),
                        TOLERANCE,
                        &format!("{what} by {distance}: vertex {vertex:?}"),
                    );
                    let next = ring[(index + 1) % ring.len()];
                    let middle = (vertex.0.midpoint(next.0), vertex.1.midpoint(next.1));
                    let between = nearest_edge(std::slice::from_ref(&source), middle);
                    assert!(
                        between >= distance.abs() - TOLERANCE,
                        "{what} by {distance}: edge {vertex:?}-{next:?} comes within {between}"
                    );
                }
            }
            assert_eq!(canonical_fault(&found), None, "{what} by {distance}");
            assert_eq!(crossing_fault(&found, 1e-9), None, "{what} by {distance}");
        }
    }
    Ok(())
}

#[test]
fn an_inset_lies_inside_the_original_and_the_bounds_shrink_on_both_sides() -> Outcome {
    let bar = Path::rect(0.0, 0.0, 40.0, 20.0, 0.0)?;
    let inset = bar.offset(-3.0, Join::Miter, LIMIT, TOLERANCE)?;
    assert_extent(&inset, (3.0, 3.0, 37.0, 17.0), 1e-6, "bar")?;
    let l = polygons_path(&[l_shape()])?;
    let inset = l.offset(-4.0, Join::Round, LIMIT, TOLERANCE)?;
    for ring in rings(&inset)? {
        for vertex in ring {
            assert!(l.contains(vertex)?, "{vertex:?} is outside the L");
        }
    }
    Ok(())
}

#[test]
fn a_ring_grows_outward_and_its_hole_shrinks_until_it_closes() -> Outcome {
    let outer = vec![(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)];
    let hole = vec![(6.0, 6.0), (6.0, 14.0), (14.0, 14.0), (14.0, 6.0)];
    let ring = polygons_path(&[outer, hole])?;
    // (distance, rings wanted, area wanted, bounds wanted)
    let rows = [
        (1.0, 2, 22.0 * 22.0 - 6.0 * 6.0, (-1.0, -1.0, 21.0, 21.0)),
        (-1.0, 2, 18.0 * 18.0 - 10.0 * 10.0, (1.0, 1.0, 19.0, 19.0)),
        (4.5, 1, 29.0 * 29.0, (-4.5, -4.5, 24.5, 24.5)),
    ];
    for (distance, count, wanted, bounds) in rows {
        let result = ring.offset(distance, Join::Miter, LIMIT, TOLERANCE)?;
        let found = rings(&result)?;
        assert_eq!(found.len(), count, "by {distance}");
        assert_near(net_area(&found), wanted, 1e-6, &format!("by {distance}"));
        assert_extent(&result, bounds, 1e-6, &format!("by {distance}"))?;
        assert_eq!(canonical_fault(&found), None, "by {distance}");
    }
    Ok(())
}

#[test]
fn an_inset_past_the_collapse_is_an_empty_path() -> Outcome {
    for (what, path, distance) in [
        ("a square", square(10.0)?, -6.0),
        ("a thin bar", Path::rect(0.0, 0.0, 40.0, 4.0, 0.0)?, -3.0),
        ("a circle", Path::circle((0.0, 0.0), 5.0)?, -9.0),
    ] {
        for join in [Join::Round, Join::Miter, Join::Bevel] {
            assert_eq!(
                path.offset(distance, join, LIMIT, TOLERANCE)?,
                Path::new(),
                "{what}, {join:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn an_offset_of_nothing_is_nothing_and_by_zero_is_the_resolved_path() -> Outcome {
    assert_eq!(Path::new().offset(3.0, Join::Round, LIMIT, TOLERANCE)?, Path::new());
    let bow_tie = Path::polyline([(0.0, 0.0), (10.0, 10.0), (10.0, 0.0), (0.0, 10.0)], true)?;
    for path in [square(10.0)?, bow_tie, Path::circle((5.0, 5.0), 5.0)?] {
        let same = path.offset(0.0, Join::Miter, LIMIT, TOLERANCE)?;
        assert_eq!(same.to_svg_path(), path.resolve(TOLERANCE)?.to_svg_path());
    }
    Ok(())
}

#[test]
fn an_open_outline_is_offset_as_if_closed() -> Outcome {
    let corners = [(0.0, 0.0), (30.0, 0.0), (10.0, 20.0)];
    let open = Path::polyline(corners, false)?.offset(2.0, Join::Round, LIMIT, TOLERANCE)?;
    let closed = Path::polyline(corners, true)?.offset(2.0, Join::Round, LIMIT, TOLERANCE)?;
    assert_eq!(open.to_svg_path(), closed.to_svg_path());
    Ok(())
}

#[test]
fn the_same_call_twice_writes_the_same_path_data() -> Outcome {
    let path = Path::circle((5.0, 5.0), 5.0)?.union(&square(8.0)?, TOLERANCE)?;
    let line = Path::polyline([(0.0, 0.0), (20.0, 5.0), (30.0, -4.0)], false)?;
    for join in [Join::Round, Join::Miter, Join::Bevel] {
        let once = path.offset(1.5, join, LIMIT, TOLERANCE)?.to_svg_path();
        assert_eq!(
            once,
            path.offset(1.5, join, LIMIT, TOLERANCE)?.to_svg_path(),
            "offset, {join:?}"
        );
        let once = line.outline(3.0, join, Cap::Round, LIMIT, TOLERANCE)?.to_svg_path();
        assert_eq!(
            once,
            line.outline(3.0, join, Cap::Round, LIMIT, TOLERANCE)?.to_svg_path(),
            "outline, {join:?}"
        );
    }
    Ok(())
}

// A line of 10 stroked 4 wide: the area is the bar, plus what the caps add.
#[test]
fn the_outline_of_a_line_is_a_bar_with_the_caps_it_is_given() -> Outcome {
    let line = Path::line((0.0, 0.0), (10.0, 0.0))?;
    let r = 2.0;
    // (cap, area wanted, how much less the arcs may leave, bounds wanted)
    let rows = [
        (Cap::Butt, 40.0, 0.0, (0.0, -2.0, 10.0, 2.0)),
        (Cap::Square, 56.0, 0.0, (-2.0, -2.0, 12.0, 2.0)),
        (
            Cap::Round,
            40.0 + PI * r * r,
            2.0 * PI * r * TOLERANCE,
            (-2.0, -2.0, 12.0, 2.0),
        ),
    ];
    for (cap, wanted, arcs, bounds) in rows {
        let result = line.outline(4.0, Join::Round, cap, LIMIT, TOLERANCE)?;
        let found = area(&result)?;
        assert!(
            found <= wanted + 1e-6 && found >= wanted - arcs - 1e-6,
            "{cap:?}: area {found}, wanted {wanted}"
        );
        // the mid-point of an arc is not always a vertex of it
        assert_extent(
            &result,
            bounds,
            if arcs == 0.0 { 1e-6 } else { TOLERANCE },
            &format!("{cap:?}"),
        )?;
        assert_eq!(rings(&result)?.len(), 1, "{cap:?}");
    }
    Ok(())
}

// An open L (10 right, 10 down) stroked 2 wide with butt caps: the joins differ by the corner square.
#[test]
fn the_outline_of_a_corner_has_the_area_its_join_leaves() -> Outcome {
    let corner = Path::polyline([(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)], false)?;
    let rows = [
        (Join::Miter, 40.0, 0.0),
        (Join::Bevel, 39.5, 0.0),
        (Join::Round, 40.0 - (1.0 - PI / 4.0), 2.0 * PI * 0.5 * TOLERANCE),
    ];
    for (join, wanted, arcs) in rows {
        let result = corner.outline(2.0, join, Cap::Butt, LIMIT, TOLERANCE)?;
        let found = area(&result)?;
        assert!(
            found <= wanted + 1e-6 && found >= wanted - arcs - 1e-6,
            "{join:?}: area {found}, wanted {wanted}"
        );
    }
    Ok(())
}

#[test]
fn a_closed_outline_is_stroked_into_a_ring_whichever_way_it_runs() -> Outcome {
    let forward = square(10.0)?;
    let backward = polygons_path(&[corners_square(10.0).into_iter().rev().collect()])?;
    for path in [forward, backward] {
        let result = path.outline(2.0, Join::Miter, Cap::Butt, LIMIT, TOLERANCE)?;
        let found = rings(&result)?;
        assert_eq!(found.len(), 2, "an outer ring and a hole");
        assert_near(net_area(&found), 12.0 * 12.0 - 8.0 * 8.0, 1e-6, "the ring");
        assert_extent(&result, (-1.0, -1.0, 11.0, 11.0), 1e-6, "the ring")?;
        assert_eq!(canonical_fault(&found), None);
    }
    let circle = Path::circle((0.0, 0.0), 10.0)?.outline(2.0, Join::Round, Cap::Round, LIMIT, TOLERANCE)?;
    // the outer and inner edge each stray from the circles by at most the tolerance
    assert_near(
        area(&circle)?,
        PI * (121.0 - 81.0),
        2.0 * PI * 20.0 * TOLERANCE,
        "the circle",
    );
    Ok(())
}

#[test]
fn the_strokes_of_separate_sub_paths_are_united() -> Outcome {
    // two bars of a cross: the overlap is one region, not counted twice
    let cross = Path::line((0.0, 5.0), (10.0, 5.0))?
        .move_to((5.0, 0.0))?
        .line_to((5.0, 10.0))?;
    let result = cross.outline(2.0, Join::Miter, Cap::Butt, LIMIT, TOLERANCE)?;
    assert_near(area(&result)?, 20.0 + 20.0 - 4.0, 1e-6, "the cross");
    assert_eq!(rings(&result)?.len(), 1);
    Ok(())
}

#[test]
fn the_parameters_outside_what_the_verbs_accept_are_refused_by_name() -> Outcome {
    let shape = square(10.0)?;
    let rejected = |value: f64| format!("{value}");
    // (parameter, distance or width, miter limit, tolerance)
    let offsets = [
        ("distance", f64::NAN, LIMIT, TOLERANCE),
        ("distance", f64::INFINITY, LIMIT, TOLERANCE),
        ("distance", 1e101, LIMIT, TOLERANCE),
        ("distance", -1e101, LIMIT, TOLERANCE),
        ("miter_limit", 1.0, 0.5, TOLERANCE),
        ("miter_limit", 1.0, f64::NAN, TOLERANCE),
        ("miter_limit", 1.0, f64::INFINITY, TOLERANCE),
        ("tolerance", 1.0, LIMIT, 0.0),
        ("tolerance", 1.0, LIMIT, -0.01),
        ("tolerance", 1.0, LIMIT, f64::NAN),
        ("tolerance", 1.0, LIMIT, 1e101),
    ];
    for (parameter, distance, limit, tolerance) in offsets {
        let (named, message) = parameter_refusal(shape.offset(distance, Join::Round, limit, tolerance))?;
        let value = match parameter {
            "distance" => distance,
            "miter_limit" => limit,
            _ => tolerance,
        };
        assert_eq!(named, parameter, "offset {parameter} {value}");
        assert!(
            message.contains(&rejected(value)),
            "offset {parameter} {value}: {message}"
        );
    }
    let outlines = [
        ("width", 0.0, LIMIT, TOLERANCE),
        ("width", -1.0, LIMIT, TOLERANCE),
        ("width", f64::NAN, LIMIT, TOLERANCE),
        ("width", f64::INFINITY, LIMIT, TOLERANCE),
        ("width", 1e101, LIMIT, TOLERANCE),
        ("miter_limit", 1.0, 0.99, TOLERANCE),
        ("miter_limit", 1.0, f64::NAN, TOLERANCE),
        ("tolerance", 1.0, LIMIT, 0.0),
        ("tolerance", 1.0, LIMIT, f64::NAN),
    ];
    for (parameter, width, limit, tolerance) in outlines {
        let (named, message) = parameter_refusal(shape.outline(width, Join::Round, Cap::Round, limit, tolerance))?;
        let value = match parameter {
            "width" => width,
            "miter_limit" => limit,
            _ => tolerance,
        };
        assert_eq!(named, parameter, "outline {parameter} {value}");
        assert!(
            message.contains(&rejected(value)),
            "outline {parameter} {value}: {message}"
        );
    }
    // the parameters are checked whatever the path
    assert_eq!(
        parameter_refusal(Path::new().offset(f64::NAN, Join::Round, LIMIT, TOLERANCE))?.0,
        "distance"
    );
    assert_eq!(
        parameter_refusal(Path::new().outline(0.0, Join::Round, Cap::Butt, LIMIT, TOLERANCE))?.0,
        "width"
    );
    // a miter limit of exactly 1 is the least allowed
    shape.offset(1.0, Join::Miter, 1.0, TOLERANCE)?;
    Ok(())
}

// A shape at the limit of the numbers: grown to just inside ±1e100 it is drawn, grown past it is refused, shrunk past nothing it
// is gone.
#[test]
fn a_shape_near_the_limit_of_the_numbers_is_grown_refused_or_gone() -> Outcome {
    let big = Path::rect(-1e99, -1e99, 2e99, 2e99, 0.0)?;
    for join in [Join::Round, Join::Miter, Join::Bevel] {
        let grown = big.offset(8e99, join, LIMIT, TOLERANCE)?;
        assert_extent(&grown, (-9e99, -9e99, 9e99, 9e99), 1e84, &format!("{join:?}"))?;
        parameter_refusal(big.offset(9.5e99, join, LIMIT, TOLERANCE))?;
        assert_eq!(big.offset(-2e99, join, LIMIT, TOLERANCE)?, Path::new(), "{join:?}");
        let stroked = big.outline(1e99, join, Cap::Square, LIMIT, TOLERANCE)?;
        assert_extent(&stroked, (-1.5e99, -1.5e99, 1.5e99, 1.5e99), 1e84, &format!("{join:?}"))?;
    }
    Ok(())
}

/// The corners of a convex polygon on a circle of radius `radius` round the middle: `weights` are the shares of the full
/// turn between neighbours, so that no two corners are close.
fn convex(weights: &[f64], radius: f64, turn: f64) -> Corners {
    let total: f64 = weights.iter().sum();
    let mut angle = turn;
    weights
        .iter()
        .map(|weight| {
            let corner = (60.0 + radius * libm::cos(angle), 60.0 + radius * libm::sin(angle));
            angle += 2.0 * PI * weight / total;
            corner
        })
        .collect()
}

proptest! {
    #[test]
    fn a_convex_polygon_grown_holds_its_corners_and_shrunk_lies_inside_it(
        weights in prop::collection::vec(1.0_f64..3.0, 3..9),
        radius in 15.0_f64..50.0,
        turn in 0.0_f64..std::f64::consts::TAU,
        distance in 0.5_f64..5.0,
        join in prop::sample::select(vec![Join::Round, Join::Miter, Join::Bevel]),
    ) {
        let source = convex(&weights, radius, turn);
        let path = polygons_path(std::slice::from_ref(&source)).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let fail = |e: &dyn std::fmt::Display| TestCaseError::fail(e.to_string());
        let grown = path.offset(distance, join, LIMIT, TOLERANCE).map_err(|e| fail(&e))?;
        for &corner in &source {
            prop_assert!(grown.contains(corner).map_err(|e| fail(&e))?, "{corner:?} left out of the grown polygon");
        }
        let shrunk = path.offset(-distance, join, LIMIT, TOLERANCE).map_err(|e| fail(&e))?;
        let inner = rings(&shrunk).map_err(|e| fail(&e))?;
        for vertex in inner.iter().flatten() {
            prop_assert!(path.contains(*vertex).map_err(|e| fail(&e))?, "{vertex:?} is outside the source");
        }
        let (before, after, within) = (
            area(&path).map_err(|e| fail(&e))?,
            area(&grown).map_err(|e| fail(&e))?,
            area(&shrunk).map_err(|e| fail(&e))?,
        );
        prop_assert!(within < before && before < after, "{within} < {before} < {after}");
    }
}

fn locations(font: &Font) -> Vec<Vec<(String, f64)>> {
    let axes = font.axes();
    let at = |pick: &dyn Fn(usize) -> f64| {
        axes.iter()
            .enumerate()
            .map(|(i, a)| (a.name().to_owned(), pick(i)))
            .collect()
    };
    vec![at(&|i| axes[i].default()), at(&|i| axes[i].max())]
}

// Every glyph of a sample of both bundled fonts (at the default and the heaviest setting, 100 pixels to the em), grown and
// shrunk by 3 with each join, and stroked 6 wide: no panic, and a canonical result that does not cross itself. (A mitre with
// the engine's near-straight cutoff off panics inside it on these outlines.)
#[test]
fn glyphs_of_both_bundled_fonts_survive_offsets_and_outlines() -> Outcome {
    let mut runs = 0;
    for name in Font::bundled_names() {
        let font = Font::bundled(name)?;
        for location in locations(&font) {
            let options = Text::new(&font).size(100.0).axes(location);
            for (glyph, shape) in text("BiJj!%&8ÉÅøñgQ@$?fiffi", options)
                .shapes(&Values::new())?
                .iter()
                .enumerate()
            {
                let path = shape.path();
                let results = [
                    ("round out", path.offset(3.0, Join::Round, LIMIT, TOLERANCE)?),
                    ("round in", path.offset(-3.0, Join::Round, LIMIT, TOLERANCE)?),
                    ("miter out", path.offset(3.0, Join::Miter, LIMIT, TOLERANCE)?),
                    ("bevel in", path.offset(-3.0, Join::Bevel, LIMIT, TOLERANCE)?),
                    (
                        "miter stroke",
                        path.outline(6.0, Join::Miter, Cap::Butt, LIMIT, TOLERANCE)?,
                    ),
                    (
                        "round stroke",
                        path.outline(6.0, Join::Round, Cap::Round, LIMIT, TOLERANCE)?,
                    ),
                ];
                for (what, result) in &results {
                    let found: Vec<Ring> = rings(result)?;
                    let label = format!("{name} glyph {glyph}, {what}");
                    assert_eq!(canonical_fault(&found), None, "{label}");
                    assert_eq!(crossing_fault(&found, 1e-9), None, "{label}");
                    runs += 1;
                }
            }
        }
    }
    assert!(runs > 400, "{runs} runs");
    Ok(())
}
