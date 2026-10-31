//! What `Drawing::to_rgba` strokes: the width seen through the view, the corners and ends of an outline against the
//! regions they must lie between (worked out with a brute-force distance to the centreline), and hand-worked corner,
//! cap, closed-or-open and zero-length rows. The oracles are geometry written in the test, not the rasteriser.

mod common;

use bullterrier_vector::{Cap, Color, Drawing, Join, Path, Stroke};
use common::Outcome;
use common::pixels::{Image, render};
use libm::{fabs, hypot};
use proptest::prelude::*;
use std::f64::consts::SQRT_2;

type Point = (f64, f64);

/// The picture of `path` stroked and not filled, `size` pixels square.
fn stroked(path: Path, stroke: Stroke, size: usize) -> Result<Image, Box<dyn std::error::Error>> {
    Ok(render(
        &Drawing::from(path).fill(Color::NONE).stroke(stroke),
        size,
        size,
    )?)
}

fn black() -> Color {
    Color::hex(0)
}

/// Where `point` lies relative to the segment `from`-`to`.
struct Relative {
    /// The distance to the nearest point of the segment.
    distance: f64,
    /// How far along the segment's line the point projects, from `from`; the segment is `length` long.
    along: f64,
    length: f64,
    /// How far from the segment's line the point is.
    across: f64,
}

fn relative(point: Point, from: Point, to: Point) -> Relative {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let length = hypot(dx, dy);
    let (ux, uy) = (dx / length, dy / length);
    let (px, py) = (point.0 - from.0, point.1 - from.1);
    let along = px * ux + py * uy;
    let nearest = along.clamp(0.0, length);
    Relative {
        distance: hypot(px - ux * nearest, py - uy * nearest),
        along,
        length,
        across: fabs(px * uy - py * ux),
    }
}

/// The segments of the polyline through `corners`, `closed` or not.
fn segments(corners: &[Point], closed: bool) -> Vec<(Point, Point)> {
    let count = if closed { corners.len() } else { corners.len() - 1 };
    (0..count)
        .map(|i| (corners[i], corners[(i + 1) % corners.len()]))
        .collect()
}

/// The distance from `point` to the polyline: the brute-force oracle of the round region.
fn distance(corners: &[Point], point: Point) -> f64 {
    segments(corners, false)
        .into_iter()
        .map(|(from, to)| relative(point, from, to).distance)
        .fold(f64::INFINITY, f64::min)
}

/// Whether `point` lies in the rectangle of some segment, a pixel in from its edges: drawn whatever the corners and
/// the ends are.
fn in_a_rectangle(corners: &[Point], width: f64, point: Point) -> bool {
    segments(corners, false).into_iter().any(|(from, to)| {
        let at = relative(point, from, to);
        at.along >= 1.0 && at.along <= at.length - 1.0 && at.across <= width / 2.0 - 1.0
    })
}

fn centre(column: usize, row: usize) -> Point {
    #[expect(clippy::cast_precision_loss, reason = "small pixel indices")]
    let centre = (column as f64 + 0.5, row as f64 + 0.5);
    centre
}

fn fail(error: impl std::fmt::Display) -> TestCaseError {
    TestCaseError::fail(error.to_string())
}

const JOINS: [Join; 3] = [Join::Round, Join::Miter, Join::Bevel];
const CAPS: [Cap; 3] = [Cap::Butt, Cap::Round, Cap::Square];

// each case checks ten thousand pixels, so fewer cases than the default give the same time as the other files
proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn a_stroke_lies_between_the_segment_rectangles_and_the_region_its_corners_and_ends_allow(
        corners in prop::collection::vec((15.0f64..85.0, 15.0f64..85.0), 3..6),
        width in 4.0f64..14.0,
        join in 0usize..3,
        cap in 0usize..3,
    ) {
        // segments at least as long as the stroke is wide (and 3 pixels, so that "a pixel in from each end" means
        // something): the stroker can lose a sliver of a rectangle on the inside of a sharp turn after a segment shorter
        // than half the width, which this test does not describe
        prop_assume!(corners.windows(2).all(|pair| hypot(pair[0].0 - pair[1].0, pair[0].1 - pair[1].1) >= width.max(3.0)));
        let (join, cap, miter_limit) = (JOINS[join], CAPS[cap], 4.0);
        let path = Path::polyline(corners.clone(), false).map_err(fail)?;
        let stroke = Stroke::new(black()).width(width).join(join).cap(cap).miter_limit(miter_limit);
        let image = stroked(path, stroke, 100).map_err(fail)?;
        // a bevel and a butt end stay inside the round region; a miter reaches `miter_limit` half widths, a square
        // end the corner of its square
        let reach = match (join, cap) {
            (Join::Miter, _) => miter_limit,
            (_, Cap::Square) => SQRT_2,
            _ => 1.0,
        } * width / 2.0;
        let round_throughout = join == Join::Round && cap == Cap::Round;
        for row in 0..100 {
            for column in 0..100 {
                let point = centre(column, row);
                let alpha = image.alpha(column, row);
                if in_a_rectangle(&corners, width, point) {
                    prop_assert_eq!(alpha, 255, "inside a rectangle at column {} row {}", column, row);
                }
                let away = distance(&corners, point);
                if round_throughout && away <= width / 2.0 - 1.0 {
                    prop_assert_eq!(alpha, 255, "inside the round region at column {} row {}", column, row);
                }
                if away >= reach + 1.0 {
                    prop_assert_eq!(alpha, 0, "outside every region at column {} row {}", column, row);
                }
            }
        }
    }
}

#[test]
fn a_round_stroke_of_a_curve_is_the_ring_round_the_centreline() -> Outcome {
    // radius 30 and width 10: the centreline is flattened to within a quarter of a pixel, so a pixel centre 4 or less
    // from the circle is inside the ring and 6 or more is outside
    let image = stroked(Path::circle((50.0, 50.0), 30.0)?, Stroke::new(black()).width(10.0), 100)?;
    for row in 0..100 {
        for column in 0..100 {
            let (x, y) = centre(column, row);
            let off = fabs(hypot(x - 50.0, y - 50.0) - 30.0);
            if off <= 4.0 {
                assert_eq!(image.alpha(column, row), 255, "column {column} row {row}");
            } else if off >= 6.0 {
                assert_eq!(image.alpha(column, row), 0, "column {column} row {row}");
            }
        }
    }
    Ok(())
}

#[test]
fn a_corner_is_mitred_bevelled_or_rounded() -> Outcome {
    // the path runs right along y = 40 to (40, 40), then up: its outer corner is at (45, 45), and a width of 10 puts
    // the bevel on the line x + y = 85, the round join on the circle of radius 5 round (40, 40). The 90 degree corner
    // has a miter length of 1.414 widths. Probed pixels, by hand: (44,44) lies beyond the bevel line and the circle
    // (its nearest corner is 5.66 from the corner); (43,43) beyond the bevel line but across the circle's edge
    // (corners 4.24 to 5.66 away); (41,41) inside all three.
    let corner = Path::polyline([(10.0, 40.0), (40.0, 40.0), (40.0, 10.0)], false)?;
    // (case, join, miter limit, alpha at (44,44), alpha at (43,43), alpha at (41,41))
    let rows = [
        ("miter, limit 4", Join::Miter, 4.0, (255, 255, 255)),
        ("miter, limit 1.5 holds", Join::Miter, 1.5, (255, 255, 255)),
        ("miter, limit 1.2 is cut to a bevel", Join::Miter, 1.2, (0, 0, 255)),
        ("bevel", Join::Bevel, 4.0, (0, 0, 255)),
    ];
    for (case, join, limit, (far, middle, near)) in rows {
        let stroke = Stroke::new(black())
            .width(10.0)
            .join(join)
            .cap(Cap::Butt)
            .miter_limit(limit);
        let image = stroked(corner.clone(), stroke, 60)?;
        assert_eq!(
            (image.alpha(44, 44), image.alpha(43, 43), image.alpha(41, 41)),
            (far, middle, near),
            "{case}"
        );
    }
    let round = stroked(
        corner,
        Stroke::new(black()).width(10.0).join(Join::Round).cap(Cap::Butt),
        60,
    )?;
    // the exact share of the pixel inside the circle is 0.55; the stroker follows the arc to within a quarter of a pixel
    assert_eq!(round.alpha(44, 44), 0, "round: beyond the circle");
    assert!(
        (50..255).contains(&round.alpha(43, 43)),
        "round: across its edge: {}",
        round.alpha(43, 43)
    );
    assert_eq!(round.alpha(41, 41), 255, "round: inside");
    Ok(())
}

#[test]
fn an_end_is_cut_square_rounded_or_extended() -> Outcome {
    // a line from (20, 50) to (60, 50), 10 wide: rows 45 to 54. Probed pixels (column, row) and their alpha under a
    // butt, a round and a square end, worked by hand: a square end reaches 5 further (columns 15 to 19 and 60 to 64),
    // a round end is a disc of radius 5 (a pixel is clear if its nearest corner is 5 or more from the end point)
    let line = Path::line((20.0, 50.0), (60.0, 50.0))?;
    // (column, row, butt, round, square)
    let probes = [
        (20, 50, 255, 255, 255),
        (19, 50, 0, 255, 255),
        (16, 50, 0, 255, 255),
        (15, 45, 0, 0, 255),
        (14, 50, 0, 0, 0),
        (59, 50, 255, 255, 255),
        (60, 50, 0, 255, 255),
        (64, 45, 0, 0, 255),
        (65, 50, 0, 0, 0),
    ];
    for (index, cap) in [Cap::Butt, Cap::Round, Cap::Square].into_iter().enumerate() {
        let image = stroked(line.clone(), Stroke::new(black()).width(10.0).cap(cap), 80)?;
        for probe in probes {
            let expected = [probe.2, probe.3, probe.4][index];
            assert_eq!(
                image.alpha(probe.0, probe.1),
                expected,
                "{cap:?} at column {} row {}",
                probe.0,
                probe.1
            );
        }
    }
    Ok(())
}

#[test]
fn a_closed_outline_has_a_corner_where_an_open_one_has_two_ends() -> Outcome {
    // a square of side 40 from (20, 20), stroked 10 wide with mitres and butt ends: the corner at its start has an
    // outer point at (15, 15), covered only if the outline is closed
    let corners = [(20.0, 20.0), (60.0, 20.0), (60.0, 60.0), (20.0, 60.0)];
    let stroke = || Stroke::new(black()).width(10.0).join(Join::Miter).cap(Cap::Butt);
    let closed = stroked(Path::polyline(corners, true)?, stroke(), 80)?;
    let mut back_to_the_start = corners.to_vec();
    back_to_the_start.push(corners[0]);
    let open = stroked(Path::polyline(back_to_the_start, false)?, stroke(), 80)?;
    assert_eq!(closed.alpha(15, 15), 255, "closed: mitred at the start");
    assert_eq!(open.alpha(15, 15), 0, "open: two butt ends, no corner");
    // the other corners are joined either way
    assert_eq!(closed.alpha(64, 64), 255);
    assert_eq!(open.alpha(64, 64), 255);
    Ok(())
}

#[test]
fn a_zero_length_segment_is_a_disc_a_square_or_nothing_by_its_cap() -> Outcome {
    let dot = Path::start((50.0, 50.0))?.line_to((50.0, 50.0))?;
    let round = stroked(dot.clone(), Stroke::new(black()).width(10.0).cap(Cap::Round), 100)?;
    // a disc of radius 5: the pixels a pixel inside it are opaque, those beyond are clear
    assert_eq!(
        (round.alpha(49, 49), round.alpha(53, 50), round.alpha(50, 53)),
        (255, 255, 255)
    );
    assert_eq!(
        (round.alpha(56, 50), round.alpha(50, 44), round.alpha(54, 54)),
        (0, 0, 0)
    );
    // a square cap leaves the square of side 10 (columns and rows 45 to 54), corners included
    let square = stroked(dot.clone(), Stroke::new(black()).width(10.0).cap(Cap::Square), 100)?;
    assert_eq!(
        (square.alpha(45, 45), square.alpha(54, 54), square.alpha(54, 45)),
        (255, 255, 255)
    );
    assert_eq!(
        (square.alpha(44, 50), square.alpha(55, 50), square.alpha(50, 55)),
        (0, 0, 0)
    );
    let butt = stroked(dot, Stroke::new(black()).width(10.0).cap(Cap::Butt), 100)?;
    assert!(butt.bytes.iter().all(|&byte| byte == 0), "a butt cap draws nothing");
    Ok(())
}

#[test]
fn the_width_is_seen_through_the_view_and_the_stroke_colour_keeps_its_alpha() -> Outcome {
    // a line from (2, 10) to (18, 10), 3 wide, in a 20 by 20 frame drawn 40 by 40: the view doubles everything, so
    // the line is from (4, 20) to (36, 20) and 6 wide: rows 17 to 22, columns 4 to 35 (butt ends)
    let line = Path::line((2.0, 10.0), (18.0, 10.0))?;
    let stroke = Stroke::new(Color::parse("#2a5bff80")?).width(3.0).cap(Cap::Butt);
    let drawing = Drawing::from(line)
        .fill(Color::NONE)
        .stroke(stroke)
        .with_size(20.0, 20.0);
    let image = render(&drawing, 40, 40)?;
    for (column, row, alpha) in [
        (4, 17, 128),
        (35, 22, 128),
        (20, 20, 128),
        (3, 20, 0),
        (36, 20, 0),
        (20, 16, 0),
        (20, 23, 0),
    ] {
        assert_eq!(image.alpha(column, row), alpha, "column {column} row {row}");
    }
    let [red, green, blue, _] = image.pixel(20, 20);
    // straight alpha: the colour is the stroke's, within a level of what a half-covered pixel can hold
    assert!(
        (i32::from(red) - 0x2a).abs() <= 2
            && (i32::from(green) - 0x5b).abs() <= 2
            && (i32::from(blue) - 0xff).abs() <= 2
    );
    Ok(())
}

#[test]
fn a_round_stroke_covers_the_area_of_its_ring_and_its_dot_to_the_flattening() -> Outcome {
    // (rule, design frame side, canvas side, scale)
    for (rule, scale) in [("at 0.5", 0.5), ("at 1", 1.0), ("at 3", 3.0)] {
        // a circle of radius 10 stroked 4 wide is the ring between radii 8 and 12: pi (12^2 - 8^2) = 80 pi square
        // pixels at scale 1; a zero-length round end 4 wide is a disc of radius 2. Each outline is within 0.01 design
        // pixels of the true curve, and an edge pixel's coverage is within 0.002 of exact.
        let side = 60.0 * scale;
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "30, 60 and 180")]
        let pixels = side as usize;
        let ring = Drawing::from(Path::circle((30.0, 30.0), 10.0)?)
            .fill(Color::NONE)
            .stroke(Stroke::new(black()).width(4.0))
            .with_size(60.0, 60.0);
        let covered = render(&ring, pixels, pixels)?.covered_area();
        let area = 80.0 * std::f64::consts::PI * scale * scale;
        let edges = 2.0 * std::f64::consts::PI * (12.0 + 8.0) * scale;
        let slack = edges * (0.01 * scale + 0.002);
        assert!(
            (covered - area).abs() <= slack,
            "{rule}: ring covers {covered}, area {area} within {slack}"
        );

        let dot = Drawing::from(Path::start((30.0, 30.0))?.line_to((30.0, 30.0))?)
            .fill(Color::NONE)
            .stroke(Stroke::new(black()).width(4.0).cap(Cap::Round))
            .with_size(60.0, 60.0);
        let covered = render(&dot, pixels, pixels)?.covered_area();
        let area = 4.0 * std::f64::consts::PI * scale * scale;
        let slack = 2.0 * std::f64::consts::PI * 2.0 * scale * (0.01 * scale + 0.002);
        assert!(
            (covered - area).abs() <= slack,
            "{rule}: dot covers {covered}, area {area} within {slack}"
        );
    }
    Ok(())
}

#[test]
fn a_corner_covers_the_exact_area_of_its_join_at_any_scale() -> Outcome {
    // two rectangles of length L and width w meet at a right angle: they cover 2 L w, overlapping in a square of side
    // w/2, and the join fills the outer corner square of side w/2 with a quarter disc (round), a triangle (bevel) or the
    // whole square (miter). Worked from L = 40 and w = 30 (r = 15); the stroke is within 0.01 design pixels of that, and
    // an edge pixel's coverage within 0.004 (the perimeter, and a diagonal edge crosses more pixels), the arc twice.
    let (length, width, half) = (40.0, 30.0, 15.0);
    let base = 2.0 * length * width - half * half;
    // (join, the area the join adds to the two rectangles, the length of its curved edge)
    let joins = [
        (
            Join::Round,
            std::f64::consts::PI * half * half / 4.0,
            std::f64::consts::PI * half / 2.0,
        ),
        (Join::Bevel, half * half / 2.0, 0.0),
        (Join::Miter, half * half, 0.0),
    ];
    let corner = Path::polyline([(30.0, 60.0), (70.0, 60.0), (70.0, 20.0)], false)?;
    for scale in [0.5, 1.0, 3.0] {
        for (join, added, arc) in joins {
            let stroke = Stroke::new(black())
                .width(width)
                .join(join)
                .cap(Cap::Butt)
                .miter_limit(4.0);
            let drawing = Drawing::from(corner.clone())
                .fill(Color::NONE)
                .stroke(stroke)
                .with_size(100.0, 100.0);
            #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "50, 100 and 300")]
            let pixels = (100.0 * scale) as usize;
            let covered = render(&drawing, pixels, pixels)?.covered_area();
            let area = (base + added) * scale * scale;
            let slack = (4.0 * length + 3.0 * width) * scale * 0.004 + arc * scale * 0.02 * scale;
            assert!(
                (covered - area).abs() <= slack,
                "{join:?} at {scale}: covers {covered}, area {area} within {slack}"
            );
        }
    }
    Ok(())
}
