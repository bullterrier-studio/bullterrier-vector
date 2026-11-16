//! `Path::morph` through the public API: the outline `fraction` of the way from one path to another. The oracles are
//! written here from the definition: hand-computed vertices, the closed form of a lerp, the shoelace area, and a brute
//! force over `Path::contains` on a grid of probes that keep clear of every edge (a boundary may be a flattening away).

use crate::common::{Line, Outcome, has_vertex, lines, net_area, parameter_refusal, rings, shoelace_area, to_lines};
use bullterrier_vector::{Error, Path};
use proptest::prelude::*;

/// Probes within this of an edge of either outline are skipped: the outline may be a flattening away from the form.
const MARGIN: f64 = 0.05;

fn svg(data: &str) -> Result<Path, Error> {
    Path::from_svg_path(data)
}

/// A rectangle running clockwise on screen (`y` down): an outer contour.
fn rect(x: f64, y: f64, width: f64, height: f64) -> String {
    format!(
        "M{x} {y} L{} {y} L{} {} L{x} {} Z",
        x + width,
        x + width,
        y + height,
        y + height
    )
}

/// A rectangle running counter-clockwise: a hole.
fn hole(x: f64, y: f64, width: f64, height: f64) -> String {
    format!(
        "M{x} {y} L{x} {} L{} {} L{} {y} Z",
        y + height,
        x + width,
        y + height,
        x + width
    )
}

/// The corners of a regular polygon of `sides` about `center` at `radius`, clockwise on screen, the first at the
/// corner number `first` of the ring.
fn ring_points(center: (f64, f64), radius: f64, sides: u32, first: u32) -> Vec<(f64, f64)> {
    (0..sides)
        .map(|k| {
            let angle = std::f64::consts::TAU * f64::from((k + first) % sides) / f64::from(sides);
            (
                center.0 + radius * libm::cos(angle),
                center.1 + radius * libm::sin(angle),
            )
        })
        .collect()
}

/// A regular polygon with `sides` straight sides, clockwise on screen, from its corner number `first`.
fn ngon(center: (f64, f64), radius: f64, sides: u32, first: u32) -> Result<Path, Error> {
    Path::polyline(ring_points(center, radius, sides, first), true)
}

/// The same polygon running the other way round.
fn ngon_reversed(center: (f64, f64), radius: f64, sides: u32) -> Result<Path, Error> {
    let mut points = ring_points(center, radius, sides, 0);
    points.reverse();
    Path::polyline(points, true)
}

/// The paths side by side as one path, their sub-paths in order.
fn together(parts: &[&Path]) -> Result<Path, Error> {
    let mut data = String::new();
    for part in parts {
        data.push_str(&part.to_svg_path());
        data.push(' ');
    }
    svg(&data)
}

/// The sub-paths of a path, as the result of `morph` is made of straight lines.
fn outline(path: &Path) -> Result<Vec<Line>, Box<dyn std::error::Error>> {
    lines(path)
}

/// A square grid of `count` x `count` probes over `from` to `to`, off the lattice of the numbers the tests use.
fn probes(from: (f64, f64), to: (f64, f64), count: u32) -> Vec<(f64, f64)> {
    let mut found = Vec::new();
    for row in 0..count {
        for column in 0..count {
            found.push((
                from.0 + (to.0 - from.0) * (f64::from(column) + 0.4137) / f64::from(count),
                from.1 + (to.1 - from.1) * (f64::from(row) + 0.5291) / f64::from(count),
            ));
        }
    }
    found
}

/// The corners of the box all the outlines fit in, 5 pixels wider all round.
fn box_of(found: &[&[Line]]) -> ((f64, f64), (f64, f64)) {
    let mut low = (f64::INFINITY, f64::INFINITY);
    let mut high = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for &(x, y) in found.iter().flat_map(|lines| lines.iter()).flat_map(|line| &line.0) {
        low = (low.0.min(x), low.1.min(y));
        high = (high.0.max(x), high.1.max(y));
    }
    ((low.0 - 5.0, low.1 - 5.0), (high.0 + 5.0, high.1 + 5.0))
}

/// How many probes (of `count` x `count` over the box of both outlines) are clear of every edge of both and were
/// compared; fails at the first where the two paths disagree about being filled.
fn compare_fill(a: &Path, b: &Path, count: u32) -> Result<usize, Box<dyn std::error::Error>> {
    let (la, lb) = (outline(a)?, outline(b)?);
    let (from, to) = box_of(&[&la, &lb]);
    let mut compared = 0;
    for probe in probes(from, to, count) {
        if to_lines(probe, &la) < MARGIN || to_lines(probe, &lb) < MARGIN {
            continue;
        }
        assert_eq!(
            a.contains(probe)?,
            b.contains(probe)?,
            "the two disagree at {probe:?}\n{}\n{}",
            a.to_svg_path(),
            b.to_svg_path()
        );
        compared += 1;
    }
    Ok(compared)
}

/// `compare_fill` that also requires that a fair share of the probes were compared (the grid is not mostly edge).
fn assert_same_fill(a: &Path, b: &Path) -> Outcome {
    let compared = compare_fill(a, b, 40)?;
    assert!(compared > 600, "only {compared} of 1600 probes were compared");
    Ok(())
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn assert_vertex(found: &[Line], at: (f64, f64)) {
    assert!(has_vertex(found, at), "no vertex at {at:?} in {found:?}");
}

// Shapes -------------------------------------------------------------------------------------------------------------------

/// A square with a square hole.
fn ring_a() -> Result<Path, Error> {
    svg(&format!(
        "{} {}",
        rect(0.0, 0.0, 100.0, 100.0),
        hole(30.0, 30.0, 40.0, 40.0)
    ))
}

/// A seven-sided polygon with a five-sided hole, elsewhere and bigger.
fn ring_b() -> Result<Path, Error> {
    let outer = ngon((160.0, 70.0), 90.0, 7, 0)?;
    let inner = ngon_reversed((160.0, 70.0), 30.0, 5)?;
    together(&[&outer, &inner])
}

/// A pentagon with a triangular and a rectangular hole.
fn letter_a() -> Result<Path, Error> {
    svg(&format!(
        "M0 0 L80 0 L100 40 L80 100 L0 100 Z M20 20 L20 45 L45 32 Z {}",
        hole(20.0, 60.0, 30.0, 25.0)
    ))
}

/// A different pentagon with a rectangular hole.
fn letter_b() -> Result<Path, Error> {
    svg(&format!(
        "M10 5 L90 0 L95 60 L50 110 L5 70 Z {}",
        hole(40.0, 30.0, 25.0, 25.0)
    ))
}

/// Two overlapping rectangles that run the same way: a stem and a crossbar that is a contour of its own, as a letter A
/// that is not resolved.
fn unresolved_a() -> Result<Path, Error> {
    svg(&format!(
        "{} {}",
        rect(0.0, 0.0, 30.0, 100.0),
        rect(0.0, 35.0, 100.0, 20.0)
    ))
}

// The two ends ---------------------------------------------------------------------------------------------------------------

#[test]
fn fraction_0_fills_what_self_fills_and_1_what_the_target_fills() -> Outcome {
    let pairs = [
        ("an outer with a hole", ring_a()?, ring_b()?),
        ("letters with an unmatched hole", letter_a()?, letter_b()?),
        (
            "an unmatched hole (a ring to a disc)",
            ring_a()?,
            ngon((50.0, 50.0), 45.0, 64, 0)?,
        ),
        (
            "an unmatched outer (two squares to one)",
            svg(&format!(
                "{} {}",
                rect(0.0, 0.0, 40.0, 40.0),
                rect(60.0, 0.0, 40.0, 40.0)
            ))?,
            svg(&rect(10.0, 10.0, 80.0, 80.0))?,
        ),
        (
            "unresolved overlapping outers to a circle",
            unresolved_a()?,
            ngon((50.0, 50.0), 45.0, 48, 0)?,
        ),
    ];
    for (label, a, b) in &pairs {
        for (from, to, name) in [(a, b, "forward"), (b, a, "backward")] {
            assert_same_fill(&from.morph(to, 0.0)?, from).map_err(|e| format!("{label}, {name}, fraction 0: {e}"))?;
            assert_same_fill(&from.morph(to, 1.0)?, to).map_err(|e| format!("{label}, {name}, fraction 1: {e}"))?;
        }
    }
    Ok(())
}

#[test]
fn at_the_ends_the_vertices_of_the_source_and_the_target_are_there_and_nothing_strays_from_their_outline() -> Outcome {
    let square = svg(&rect(0.0, 0.0, 10.0, 10.0))?;
    let pentagon = svg("M20 0 L35 10 L30 30 L10 30 L5 10 Z")?;
    let (square_lines, pentagon_lines) = (outline(&square)?, outline(&pentagon)?);
    for (at, whole) in [(0.0, &square_lines), (1.0, &pentagon_lines)] {
        let found = outline(&square.morph(&pentagon, at)?)?;
        assert_eq!(found.len(), 1);
        assert!(found[0].1, "closed stays closed");
        for corner in &whole[0].0 {
            assert_vertex(&found, *corner);
        }
        for vertex in &found[0].0 {
            assert!(to_lines(*vertex, whole) < 1e-9, "{vertex:?} is off the outline at {at}");
        }
    }
    Ok(())
}

#[test]
fn morphing_a_path_to_itself_fills_what_it_fills_at_every_fraction() -> Outcome {
    for path in [ring_a()?, letter_a()?, ring_b()?] {
        for t in [0.0, 0.3, 0.5, 1.0, 1.7, -0.6] {
            assert_same_fill(&path.morph(&path, t)?, &path).map_err(|e| format!("fraction {t}: {e}"))?;
        }
    }
    Ok(())
}

// The middle -----------------------------------------------------------------------------------------------------------------

#[test]
fn every_vertex_between_concentric_regular_polygons_is_at_the_lerped_radius() -> Outcome {
    // the corners of a regular polygon are on its circle exactly, so the lerped radius holds to 1e-9; the second case
    // starts the target 8 corners of 64 on, which is 16 alignment steps of 128: a start that is not found leaves the
    // corners paired with ones a turn of 45 degrees off and the radius short by about 0.7%
    let centre = (50.0, 60.0);
    for first in [0, 8] {
        let (small, big) = (ngon(centre, 20.0, 64, 0)?, ngon(centre, 60.0, 64, first)?);
        for t in [0.0, 0.25, 1.0, 1.5, -0.5] {
            let radius = lerp(20.0, 60.0, t);
            let found = outline(&small.morph(&big, t)?)?;
            assert_eq!(found.len(), 1);
            for vertex in &found[0].0 {
                let off = libm::hypot(vertex.0 - centre.0, vertex.1 - centre.1) - radius;
                assert!(off.abs() < 1e-9, "start {first}, fraction {t}: radius off by {off}");
            }
        }
    }
    Ok(())
}

#[test]
fn between_concentric_circles_every_vertex_is_near_the_lerped_radius() -> Outcome {
    // a looser companion of the exact test above, on `Path::circle`, whose flattening is off its circle
    let centre = (50.0, 60.0);
    let (small, big) = (Path::circle(centre, 20.0)?, Path::circle(centre, 60.0)?);
    for t in [0.0, 0.25, 1.0, 1.5, -0.5] {
        let radius = lerp(20.0, 60.0, t);
        let found = outline(&small.morph(&big, t)?)?;
        assert_eq!(found.len(), 1);
        // a point of a circle's flattening is inside the curve by at most 0.01, and the curve (four cubic arcs) is off the
        // true circle by at most 0.028% of the radius; the result is the lerp of one point of each, so it strays from
        // the lerped radius by at most |1 - t| and |t| times those
        let slack = ((1.0 - t).abs() + t.abs()) * (0.01 + 2.8e-4 * 60.0) + 1e-9;
        for vertex in &found[0].0 {
            let off = libm::hypot(vertex.0 - centre.0, vertex.1 - centre.1) - radius;
            assert!(off.abs() <= slack, "fraction {t}: radius off by {off}");
        }
    }
    Ok(())
}

#[test]
fn rectangles_of_one_shape_morph_to_the_lerped_rectangle() -> Outcome {
    // the same aspect ratio, so the corners of both sit at the same shares of the way round
    let (small, big) = (svg(&rect(0.0, 0.0, 10.0, 20.0))?, svg(&rect(20.0, 10.0, 30.0, 60.0))?);
    let at = 0.25;
    let found = outline(&small.morph(&big, at)?)?;
    assert_eq!(found.len(), 1);
    let (x, y, width, height) = (
        lerp(0.0, 20.0, at),
        lerp(0.0, 10.0, at),
        lerp(10.0, 30.0, at),
        lerp(20.0, 60.0, at),
    );
    for corner in [(x, y), (x + width, y), (x + width, y + height), (x, y + height)] {
        assert_vertex(&found, corner);
    }
    for vertex in &found[0].0 {
        let on_a_side = (vertex.0 - x).abs() < 1e-9
            || (vertex.0 - x - width).abs() < 1e-9
            || (vertex.1 - y).abs() < 1e-9
            || (vertex.1 - y - height).abs() < 1e-9;
        assert!(on_a_side, "{vertex:?} is not on the lerped rectangle");
    }
    let area = shoelace_area(&found[0].0);
    assert!((area - width * height).abs() < 1e-9, "{area}");
    Ok(())
}

#[test]
fn contours_pair_by_where_they_are_and_not_by_their_order() -> Outcome {
    let left_right = svg(&format!(
        "{} {}",
        rect(0.0, 0.0, 20.0, 20.0),
        rect(200.0, 0.0, 20.0, 20.0)
    ))?;
    let right_left = svg(&format!(
        "{} {}",
        rect(200.0, 0.0, 20.0, 20.0),
        rect(0.0, 0.0, 20.0, 20.0)
    ))?;
    // each square stays where it is: at the middle there is still a square at each end
    let middle = left_right.morph(&right_left, 0.5)?;
    assert_same_fill(&middle, &left_right)?;
    Ok(())
}

#[test]
fn an_outer_pairs_with_an_outer_and_a_hole_with_a_hole() -> Outcome {
    // the ring's hole is nearer the other path's outer contour in the box than its hole is, and still pairs with the
    // hole: at the middle the hole is the midway square
    let a = svg(&format!(
        "{} {}",
        rect(0.0, 0.0, 100.0, 100.0),
        hole(40.0, 40.0, 20.0, 20.0)
    ))?;
    let b = svg(&format!(
        "{} {}",
        rect(0.0, 0.0, 100.0, 100.0),
        hole(10.0, 10.0, 60.0, 60.0)
    ))?;
    let middle = a.morph(&b, 0.5)?;
    // the hole of the middle is the lerp of the two holes: (25, 25) to (65, 65)
    assert!(!middle.contains((30.0, 30.0))?);
    assert!(!middle.contains((60.0, 60.0))?);
    assert!(middle.contains((20.0, 20.0))?);
    assert!(middle.contains((70.0, 70.0))?);
    assert!(middle.contains((90.0, 90.0))?);
    Ok(())
}

#[test]
fn the_start_of_the_target_is_chosen_so_that_the_two_lie_over_each_other() -> Outcome {
    // the same octagon written from another corner: carried to itself at the middle, not twisted into a smaller one
    let a = ngon((50.0, 50.0), 40.0, 8, 0)?;
    for first in 1..8 {
        let b = ngon((50.0, 50.0), 40.0, 8, first)?;
        let found = outline(&a.morph(&b, 0.5)?)?;
        let wanted = outline(&a)?;
        for corner in &wanted[0].0 {
            assert_vertex(&found, *corner);
        }
        for vertex in &found[0].0 {
            assert!(to_lines(*vertex, &wanted) < 1e-9, "{vertex:?} strays, start {first}");
        }
    }
    Ok(())
}

#[test]
fn a_circle_drawn_the_other_way_round_morphs_to_a_ring_whose_hole_stays_a_hole() -> Outcome {
    let disc = ngon_reversed((100.0, 100.0), 40.0, 64)?;
    let ring = together(&[
        &ngon((100.0, 100.0), 60.0, 64, 0)?,
        &ngon_reversed((100.0, 100.0), 20.0, 64)?,
    ])?;
    // at the middle the outer is 50 wide and the hole, grown from the point at the middle, 10
    for (from, to, t) in [(&disc, &ring, 0.5), (&ring, &disc, 0.5)] {
        let middle = from.morph(to, t)?;
        assert!(!middle.contains((100.0, 100.0))?, "the hole is a hole");
        assert!(!middle.contains((104.0, 97.0))?, "the hole is a hole");
        assert!(middle.contains((130.0, 100.0))?, "the body is filled");
        assert!(middle.contains((100.0, 70.0))?, "the body is filled");
        assert!(!middle.contains((100.0, 40.0))?, "outside the outer");
    }
    Ok(())
}

#[test]
fn the_area_of_an_unmatched_hole_shrinks_as_the_square_of_what_is_left() -> Outcome {
    // the same outer on both sides, a 20 x 20 hole on the one: it shrinks about its centre (50, 50) to nothing
    let with_hole = svg(&format!(
        "{} {}",
        rect(0.0, 0.0, 100.0, 100.0),
        hole(40.0, 40.0, 20.0, 20.0)
    ))?;
    let plain = svg(&rect(0.0, 0.0, 100.0, 100.0))?;
    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let wanted = 10_000.0 - (1.0 - t) * (1.0 - t) * 400.0;
        let found = with_hole.morph(&plain, t)?;
        // brute force: the cells of 0.5 whose centres are filled (every edge sits on a multiple of 0.5)
        let mut cells = 0.0;
        for row in 0..200 {
            for column in 0..200 {
                let centre = (
                    f64::midpoint(f64::from(column), 0.5),
                    f64::midpoint(f64::from(row), 0.5),
                );
                if found.contains(centre)? {
                    cells += 0.25;
                }
            }
        }
        assert!(
            (cells - wanted).abs() < 1e-9,
            "fraction {t}: {cells} cells, wanted {wanted}"
        );
        assert!((net_area(&rings(&found)?) - wanted).abs() < 1e-6, "fraction {t}");
        // and the other way round it grows from nothing
        let back = plain.morph(&with_hole, 1.0 - t)?;
        assert!((net_area(&rings(&back)?) - wanted).abs() < 1e-6, "fraction {t}");
    }
    Ok(())
}

#[test]
fn a_contour_without_a_partner_never_comes_back_through_its_point() -> Outcome {
    // the same outer on both sides, a 20 x 20 hole at the middle on the one: past 1 the hole is gone, not reflected
    // through its point, and before 0 a hole that only the target has is absent, not reflected
    let with_hole = svg(&format!(
        "{} {}",
        rect(0.0, 0.0, 100.0, 100.0),
        hole(40.0, 40.0, 20.0, 20.0)
    ))?;
    let plain = svg(&rect(0.0, 0.0, 100.0, 100.0))?;
    for t in [1.0, 1.3, 2.0] {
        let found = with_hole.morph(&plain, t)?;
        for probe in [(50.0, 50.0), (45.0, 45.0), (55.0, 55.0)] {
            assert!(found.contains(probe)?, "source hole at fraction {t}, probe {probe:?}");
        }
    }
    for t in [0.0, -0.3, -1.0] {
        let found = plain.morph(&with_hole, t)?;
        for probe in [(50.0, 50.0), (45.0, 45.0), (55.0, 55.0)] {
            assert!(found.contains(probe)?, "target hole at fraction {t}, probe {probe:?}");
        }
    }
    // an outer that only the source has stays a point past 1, an outer that only the target has before 0
    let two = svg(&format!(
        "{} {}",
        rect(0.0, 0.0, 40.0, 40.0),
        rect(200.0, 0.0, 20.0, 20.0)
    ))?;
    let one = svg(&rect(0.0, 0.0, 40.0, 40.0))?;
    assert!(!two.morph(&one, 1.5)?.contains((210.0, 10.0))?, "leaving outer past 1");
    assert!(
        !one.morph(&two, -0.5)?.contains((210.0, 10.0))?,
        "arriving outer before 0"
    );
    Ok(())
}

#[test]
fn a_ring_morphed_to_an_l_leaves_no_speck_outside_the_l_shaped_lerp() -> Outcome {
    // the hole of the ring is at (50, 50), where the L is empty: wound, its reversed contour paints outside the moving
    // body (proven at 0.9 and 0.95); subtracted, it cannot
    let outer = svg(&rect(0.0, 0.0, 100.0, 100.0))?;
    let ring = svg(&format!(
        "{} {}",
        rect(0.0, 0.0, 100.0, 100.0),
        hole(35.0, 35.0, 30.0, 30.0)
    ))?;
    let l = svg("M0 0 L40 0 L40 60 L100 60 L100 100 L0 100 Z")?;
    for t in [0.9, 0.95] {
        let found = ring.morph(&l, t)?;
        let body = outer.morph(&l, t)?;
        let (shape, plain) = (outline(&body)?, outline(&found)?);
        // the hole is the ring's hole shrunk to (50, 50): a square of 30 (1 - t) about it
        let half = 15.0 * (1.0 - t);
        let mut compared = 0;
        for probe in probes((-5.0, -5.0), (105.0, 105.0), 60) {
            let near_hole = (probe.0 - 50.0).abs() - half;
            let near_hole = near_hole.max((probe.1 - 50.0).abs() - half);
            if to_lines(probe, &shape) < MARGIN || to_lines(probe, &plain) < MARGIN || near_hole.abs() < MARGIN {
                continue;
            }
            let wanted = body.contains(probe)? && near_hole > 0.0;
            assert_eq!(found.contains(probe)?, wanted, "fraction {t}, probe {probe:?}");
            compared += 1;
        }
        assert!(compared > 1500, "{compared}");
        // by hand: far from the L's body and from the old square's middle, nothing is painted
        assert!(!found.contains((80.0, 20.0))?);
        assert!(found.contains((20.0, 20.0))?);
        assert!(found.contains((20.0, 80.0))?);
    }
    Ok(())
}

#[test]
fn a_shape_translated_with_its_target_morphs_to_the_same_shape_translated() -> Outcome {
    let (dx, dy) = (137.5, -42.25);
    let a = together(&[&ngon((50.0, 50.0), 40.0, 6, 0)?, &ngon_reversed((50.0, 50.0), 15.0, 4)?])?;
    let b = svg(&format!(
        "M5 0 L95 10 L80 90 L20 100 L0 40 Z {}",
        hole(30.0, 30.0, 40.0, 40.0)
    ))?;
    for t in [0.0, 0.3, 1.0, 1.6] {
        let moved = outline(&a.translate(dx, dy)?.morph(&b.translate(dx, dy)?, t)?)?;
        let plain = outline(&a.morph(&b, t)?)?;
        assert_eq!(moved.len(), plain.len());
        for (m, p) in moved.iter().zip(&plain) {
            assert_eq!(m.0.len(), p.0.len(), "fraction {t}");
            assert_eq!(m.1, p.1);
            for (mv, pv) in m.0.iter().zip(&p.0) {
                assert!(
                    (mv.0 - pv.0 - dx).abs() < 1e-9 && (mv.1 - pv.1 - dy).abs() < 1e-9,
                    "fraction {t}: {mv:?} against {pv:?}"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn morphing_there_at_a_fraction_is_morphing_back_at_the_rest() -> Outcome {
    let (a, b) = (letter_a()?, letter_b()?);
    for t in [0.25, 0.5, 0.8] {
        let (there, back) = (a.morph(&b, t)?, b.morph(&a, 1.0 - t)?);
        assert_same_fill(&there, &back).map_err(|e| format!("fraction {t}: {e}"))?;
    }
    Ok(())
}

#[test]
fn a_morph_is_the_same_every_time() -> Outcome {
    let (a, b) = (letter_a()?, letter_b()?);
    assert_eq!(a.morph(&b, 0.37)?, a.morph(&b, 0.37)?);
    Ok(())
}

// Open sub-paths -------------------------------------------------------------------------------------------------------------

#[test]
fn two_open_paths_are_joined_end_to_end_at_the_shares_of_their_lengths() -> Outcome {
    // A is 10 long, B is two legs of 14.14: the shares 0, 1/2 and 1 are three points; A at 1/2 is (5, 0)
    let a = svg("M0 0 L10 0")?;
    let b = svg("M0 10 L10 20 L20 10")?;
    let found = outline(&a.morph(&b, 0.5)?)?;
    assert_eq!(found.len(), 1);
    assert!(!found[0].1, "an open path stays open");
    assert_eq!(found[0].0.len(), 3, "{:?}", found[0].0);
    for (vertex, wanted) in found[0].0.iter().zip([(0.0, 5.0), (7.5, 10.0), (15.0, 5.0)]) {
        assert!(
            (vertex.0 - wanted.0).abs() < 1e-9 && (vertex.1 - wanted.1).abs() < 1e-9,
            "{vertex:?} is not {wanted:?}"
        );
    }
    // written the other way round, B is the same line: the ends are matched, not the starts
    let turned = svg("M20 10 L10 20 L0 10")?;
    assert_eq!(outline(&a.morph(&turned, 0.5)?)?, found);
    Ok(())
}

#[test]
fn an_open_path_next_to_a_closed_one_stays_open_and_collapses_when_there_is_nothing_to_pair_with() -> Outcome {
    let square = rect(0.0, 0.0, 100.0, 100.0);
    let with_line = svg(&format!("{square} M10 10 L30 10"))?;
    let plain = svg(&square)?;
    // the line is at (20, 10) of the square's box: it shrinks to that point
    let found = outline(&with_line.morph(&plain, 0.5)?)?;
    assert_eq!(found.len(), 2);
    assert_eq!(found.iter().filter(|line| line.1).count(), 1);
    let open = found.iter().find(|line| !line.1).ok_or("no open sub-path")?;
    assert_eq!(open.0.len(), 2);
    for (vertex, wanted) in open.0.iter().zip([(15.0, 10.0), (25.0, 10.0)]) {
        assert!(
            (vertex.0 - wanted.0).abs() < 1e-9 && (vertex.1 - wanted.1).abs() < 1e-9,
            "{vertex:?}"
        );
    }
    // at 1 it is one point, still an open sub-path
    let end = outline(&with_line.morph(&plain, 1.0)?)?;
    let open = end.iter().find(|line| !line.1).ok_or("no open sub-path")?;
    assert!(
        open.0
            .iter()
            .all(|v| (v.0 - 20.0).abs() < 1e-9 && (v.1 - 10.0).abs() < 1e-9)
    );
    // and the way back grows it from that point
    let start = outline(&plain.morph(&with_line, 0.0)?)?;
    let open = start.iter().find(|line| !line.1).ok_or("no open sub-path")?;
    assert!(
        open.0
            .iter()
            .all(|v| (v.0 - 20.0).abs() < 1e-9 && (v.1 - 10.0).abs() < 1e-9)
    );
    Ok(())
}

// Nothing, a point, no area --------------------------------------------------------------------------------------------------

#[test]
fn an_empty_path_on_either_side_is_a_shape_that_shrinks_or_grows_about_its_own_centre() -> Outcome {
    assert_eq!(Path::new().morph(&Path::new(), 0.5)?, Path::new());
    let square = svg(&rect(10.0, 20.0, 40.0, 60.0))?;
    for (from, to, t, half) in [(&Path::new(), &square, 0.5, 0.5), (&square, &Path::new(), 0.25, 0.75)] {
        // the centre of the four corners is (30, 50): the square is scaled about it to the share `half`
        let found = outline(&from.morph(to, t)?)?;
        assert_eq!(found.len(), 1);
        for corner in [(10.0, 20.0), (50.0, 20.0), (50.0, 80.0), (10.0, 80.0)] {
            let moved = (30.0 + (corner.0 - 30.0) * half, 50.0 + (corner.1 - 50.0) * half);
            assert_vertex(&found, moved);
        }
    }
    assert_same_fill(&Path::new().morph(&square, 1.0)?, &square)?;
    assert_same_fill(&square.morph(&Path::new(), 0.0)?, &square)?;
    Ok(())
}

#[test]
fn a_lone_point_moves_to_the_target_and_a_shape_shrinks_to_a_point() -> Outcome {
    // a point against an open line pairs with it: the line grows from the point
    let point = svg("M10 10")?;
    let line = svg("M0 0 L10 0 L10 10")?;
    let start = outline(&point.morph(&line, 0.0)?)?;
    assert!(
        start[0]
            .0
            .iter()
            .all(|v| (v.0 - 10.0).abs() < 1e-9 && (v.1 - 10.0).abs() < 1e-9)
    );
    let end = outline(&point.morph(&line, 1.0)?)?;
    for corner in [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)] {
        assert_vertex(&end, corner);
    }
    // a closed square next to a point: it shrinks to the place in the point's box that it has in its own (the box of a
    // point is the point), and the point, open, collapses to the square's top left corner at its own place
    let square = svg(&rect(0.0, 0.0, 10.0, 10.0))?;
    let dot = svg("M50 50")?;
    let found = outline(&square.morph(&dot, 0.5)?)?;
    let closed = found.iter().find(|line| line.1).ok_or("no closed sub-path")?;
    for (vertex, corner) in closed
        .0
        .iter()
        .zip([(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)])
    {
        let wanted = (f64::midpoint(corner.0, 50.0), f64::midpoint(corner.1, 50.0));
        assert!(
            (vertex.0 - wanted.0).abs() < 1e-9 && (vertex.1 - wanted.1).abs() < 1e-9,
            "{vertex:?}"
        );
    }
    let open = found.iter().find(|line| !line.1).ok_or("no open sub-path")?;
    assert!(
        open.0
            .iter()
            .all(|v| (v.0 - 25.0).abs() < 1e-9 && (v.1 - 25.0).abs() < 1e-9),
        "{open:?}"
    );
    Ok(())
}

#[test]
fn a_closed_contour_with_no_area_is_an_outer_that_a_shape_grows_from() -> Outcome {
    let flat = svg("M0 0 L10 0 Z")?;
    let square = svg(&rect(0.0, 0.0, 10.0, 10.0))?;
    // it fills nothing at its end, and the whole square at the other
    let start = flat.morph(&square, 0.0)?;
    for probe in probes((-2.0, -2.0), (12.0, 12.0), 20) {
        assert!(!start.contains(probe)?, "{probe:?}");
    }
    assert_same_fill(&flat.morph(&square, 1.0)?, &square)?;
    // between them it is paired with the square, as an outer (a hole, it would be cut from a square that grows from a
    // point). By hand: the square's start that lies nearest the flat one's out-and-back run is the middle of its left
    // edge (7/8 of the way round), so the pairs are (0,0)-(0,5), (2.5,0)-(0,0), (7.5,0)-(10,0), (10,0)-(10,5),
    // (7.5,0)-(10,10) and (2.5,0)-(0,10), and the outline at the middle is (0,2.5), (1.25,0), (8.75,0), (10,2.5),
    // (8.75,5), (1.25,5): the square grown from a point at (5,0) would not reach (1,1)
    let middle = flat.morph(&square, 0.5)?;
    assert!(middle.contains((1.0, 1.0))?);
    assert!(middle.contains((5.0, 2.5))?);
    assert!(!middle.contains((0.5, 0.5))?);
    assert!(!middle.contains((9.5, 4.5))?);
    let back = square.morph(&flat, 1.0)?;
    for probe in probes((-2.0, -2.0), (12.0, 12.0), 20) {
        assert!(!back.contains(probe)?, "{probe:?}");
    }
    Ok(())
}

// Errors ---------------------------------------------------------------------------------------------------------------------

#[test]
fn a_fraction_that_is_not_a_number_is_refused_by_name() -> Outcome {
    let (a, b) = (letter_a()?, letter_b()?);
    for t in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(parameter_refusal(a.morph(&b, t))?.0, "fraction", "{t}");
    }
    assert_eq!(
        parameter_refusal(Path::new().morph(&Path::new(), f64::NAN))?.0,
        "fraction"
    );
    Ok(())
}

#[test]
fn a_point_that_would_pass_the_limit_is_refused_by_name() -> Outcome {
    let (a, b) = (svg(&rect(0.0, 0.0, 10.0, 10.0))?, svg(&rect(0.0, 0.0, 11.0, 11.0))?);
    assert_eq!(parameter_refusal(a.morph(&b, 1e300))?.0, "fraction");
    assert_eq!(parameter_refusal(a.morph(&b, -1e101))?.0, "fraction");
    // big coordinates: the lerp overflows to infinity and to a number that is not one
    let (big, bigger) = (
        svg(&rect(-9e99, -9e99, 1.8e100, 1.8e100))?,
        svg(&rect(-9.5e99, -9e99, 1.9e100, 1.8e100))?,
    );
    assert_eq!(parameter_refusal(big.morph(&bigger, 1e300))?.0, "fraction");
    assert_eq!(parameter_refusal(big.morph(&bigger, 3.0))?.0, "fraction");
    // the limit itself is within it
    assert!(big.morph(&bigger, 1.0).is_ok());
    Ok(())
}

#[test]
fn a_pair_of_paths_with_more_than_ten_million_edges_in_all_is_refused_at_once() -> Outcome {
    // 2.6 million points each: flattening them is 5.2 million edges, within the limit, and the lists the morph would
    // make are as many again: refused by the count before any is made
    let points =
        |offset: f64| (0..2_600_000_u32).map(move |k| (f64::from(k), if k % 2 == 0 { offset } else { offset + 1.0 }));
    let (a, b) = (Path::polyline(points(0.0), false)?, Path::polyline(points(5.0), false)?);
    let (name, message) = parameter_refusal(a.morph(&b, 0.5))?;
    assert_eq!(name, "fraction");
    assert!(message.contains("MAX_SEGMENTS"), "{message}");
    // half of it is made
    let (half_a, half_b) = (
        Path::polyline(points(0.0).take(1_000_000), false)?,
        Path::polyline(points(5.0).take(1_000_000), false)?,
    );
    assert!(half_a.morph(&half_b, 0.5).is_ok());
    Ok(())
}

#[test]
fn many_contours_are_counted_in_all() -> Outcome {
    // 4,000 squares each way are 16 million pairs to rank: they count against the same limit
    let squares = |count: u32| -> Result<Path, Error> {
        let mut data = String::new();
        for k in 0..count {
            data.push_str(&rect(f64::from(k) * 3.0, 0.0, 2.0, 2.0));
        }
        svg(&data)
    };
    let (few, many) = (squares(40)?, squares(4000)?);
    assert!(few.morph(&few, 0.5).is_ok());
    assert_eq!(parameter_refusal(many.morph(&many, 0.5))?.0, "fraction");
    Ok(())
}

// Properties ---------------------------------------------------------------------------------------------------------------

/// A convex polygon: corners on a circle at angles about a `sides`th of the way round from each other, with jitter.
fn convex() -> impl Strategy<Value = Vec<(f64, f64)>> {
    (
        30.0..70.0_f64,
        30.0..70.0_f64,
        12.0..40.0_f64,
        0.0..6.0_f64,
        prop::collection::vec(0.0..0.4_f64, 3..10),
    )
        .prop_map(|(x, y, radius, turn, jitter)| {
            let sides = f64::from(u32::try_from(jitter.len()).unwrap_or(3));
            jitter
                .iter()
                .enumerate()
                .map(|(k, shake)| {
                    let k = f64::from(u32::try_from(k).unwrap_or(0));
                    let angle = turn + (k + shake) * std::f64::consts::TAU / sides;
                    (x + radius * libm::cos(angle), y + radius * libm::sin(angle))
                })
                .collect()
        })
}

#[expect(clippy::needless_pass_by_value, reason = "it is the function of map_err")]
fn fail(error: impl ToString) -> TestCaseError {
    TestCaseError::fail(error.to_string())
}

proptest! {
    /// Two convex polygons: the ends fill as the polygons do.
    #[test]
    fn the_ends_of_a_random_pair_of_polygons_fill_as_the_polygons_do(a in convex(), b in convex()) {
        let (a, b) = (Path::polyline(a, true).map_err(fail)?, Path::polyline(b, true).map_err(fail)?);
        let start = a.morph(&b, 0.0).map_err(fail)?;
        let end = a.morph(&b, 1.0).map_err(fail)?;
        compare_fill(&start, &a, 24).map_err(fail)?;
        compare_fill(&end, &b, 24).map_err(fail)?;
    }

    /// Every vertex of the result is the lerp of a point of each outline: so for a fraction from 0 to 1 the box of the
    /// result lies inside the lerp of the two boxes.
    #[test]
    fn the_box_of_a_random_pair_is_inside_the_lerp_of_their_boxes(a in convex(), b in convex(), t in 0.0..=1.0_f64) {
        let (a, b) = (Path::polyline(a, true).map_err(fail)?, Path::polyline(b, true).map_err(fail)?);
        let (ba, bb) = (outline(&a).map_err(fail)?, outline(&b).map_err(fail)?);
        let found = outline(&a.morph(&b, t).map_err(fail)?).map_err(fail)?;
        let extent = |lines: &[Line], pick: fn(&(f64, f64)) -> f64| -> (f64, f64) {
            let values = lines.iter().flat_map(|line| &line.0).map(pick);
            values.fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), v| (low.min(v), high.max(v)))
        };
        for pick in [(|p: &(f64, f64)| p.0) as fn(&(f64, f64)) -> f64, |p: &(f64, f64)| p.1] {
            let ((la, ha), (lb, hb), (lf, hf)) = (extent(&ba, pick), extent(&bb, pick), extent(&found, pick));
            prop_assert!(lf >= lerp(la, lb, t) - 1e-9, "{lf} below {}", lerp(la, lb, t));
            prop_assert!(hf <= lerp(ha, hb, t) + 1e-9, "{hf} above {}", lerp(ha, hb, t));
        }
    }
}

#[test]
fn an_island_inside_an_unmatched_hole_stays_filled_where_it_is_at_fraction_0() -> Outcome {
    // a square with a hole that has a smaller square inside it (an outer in a hole, as in an "@"): the hole has no
    // partner in the plain square, and the island of the source must not be cut away with it
    let ringed = svg(&format!(
        "{} {} {}",
        rect(0.0, 0.0, 100.0, 100.0),
        hole(20.0, 20.0, 60.0, 60.0),
        rect(40.0, 40.0, 20.0, 20.0)
    ))?;
    let plain = svg(&rect(0.0, 0.0, 100.0, 100.0))?;
    let found = ringed.morph(&plain, 0.0)?;
    assert!(found.contains((50.0, 50.0))?, "the island is filled");
    assert!(!found.contains((30.0, 30.0))?, "the hole is empty");
    assert!(found.contains((10.0, 10.0))?, "the ring is filled");
    assert_same_fill(&found, &ringed)?;
    let found = plain.morph(&ringed, 1.0)?;
    assert_same_fill(&found, &ringed)?;
    Ok(())
}
