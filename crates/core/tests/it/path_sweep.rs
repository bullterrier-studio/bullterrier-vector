//! `Path::sweep` through the public API: the region a shape covers as it moves along a straight vector. Oracles written
//! here: a square swept along an axis is a rectangle; a convex polygon's swept area is its area plus the vector's length
//! times the polygon's width across it; and for straight-edged shapes the exact membership rule, a point `p` is swept
//! when `p` or `p - d` is in the shape or the segment between them crosses an edge of the shape, checked at seeded
//! random points against `contains` of the result.

use crate::common::{Outcome, Ring, net_area, parameter_refusal, rings};
use bullterrier_vector::{Error, Path};

const TOLERANCE: f64 = 0.01;

/// An edge of a polygon, from one corner to the next.
type Edge = ((f64, f64), (f64, f64));

fn area(path: &Path) -> Result<f64, Box<dyn std::error::Error>> {
    Ok(net_area(&rings(path)?))
}

/// A seeded stream of numbers from 0 up to 1 (splitmix64), so the sample points are the same on every run.
struct Seeded(u64);

impl Seeded {
    fn unit(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        #[expect(clippy::cast_precision_loss, reason = "53 bits of a u64 are exact in an f64")]
        let unit = ((z ^ (z >> 31)) >> 11) as f64 / 9_007_199_254_740_992.0;
        unit
    }
}

/// Whether the segments `a`-`b` and `c`-`d` meet (touching counts).
fn meet(a: (f64, f64), b: (f64, f64), c: (f64, f64), d: (f64, f64)) -> bool {
    let side = |p: (f64, f64), q: (f64, f64), r: (f64, f64)| (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0);
    let (d1, d2, d3, d4) = (side(c, d, a), side(c, d, b), side(a, b, c), side(a, b, d));
    d1 * d2 <= 0.0 && d3 * d4 <= 0.0
}

/// The exact rule for a shape of straight edges: `p` is swept by `d` when `p` or `p - d` is inside, or the segment from
/// `p - d` to `p` crosses the outline.
fn swept(shape: &Path, edges: &[Edge], p: (f64, f64), d: (f64, f64)) -> Result<bool, Error> {
    let back = (p.0 - d.0, p.1 - d.1);
    Ok(shape.contains(p)? || shape.contains(back)? || edges.iter().any(|(a, b)| meet(back, p, *a, *b)))
}

fn edges_of(path: &Path) -> Result<Vec<Edge>, Box<dyn std::error::Error>> {
    Ok(rings(path)?
        .iter()
        .flat_map(|ring: &Ring| (0..ring.len()).map(|at| (ring[at], ring[(at + 1) % ring.len()])))
        .collect())
}

/// The shapes the membership oracle is run over: concave, with a hole, and both.
fn shapes() -> Result<Vec<(&'static str, Path)>, Error> {
    Ok(vec![
        ("a star", Path::star((50.0, 50.0), 40.0, 15.0, 5)?),
        (
            "a frame with a hole",
            Path::from_svg_path("M10 10 L90 10 L90 90 L10 90 Z M30 30 L30 70 L70 70 L70 30 Z")?,
        ),
        (
            "a comb",
            Path::from_svg_path("M10 10 L90 10 L90 80 L70 80 L70 30 L50 30 L50 80 L30 80 L30 30 L10 30 Z")?,
        ),
    ])
}

/// Eight directions round the compass and a long and a short length: every edge faces some of them.
fn vectors() -> Vec<(f64, f64)> {
    let mut found = Vec::new();
    for (x, y) in [
        (1.0, 0.0),
        (1.0, 1.0),
        (0.0, 1.0),
        (-1.0, 1.0),
        (-1.0, 0.0),
        (-1.0, -1.0),
        (0.0, -1.0),
        (1.0, -1.0),
    ] {
        for length in [7.0, 45.0] {
            found.push((x * length, y * length * 0.6));
        }
    }
    found
}

#[test]
fn a_point_is_swept_exactly_when_some_point_of_the_way_back_is_in_the_shape() -> Outcome {
    let mut random = Seeded(7);
    for (what, shape) in shapes()? {
        let edges = edges_of(&shape)?;
        for d in vectors() {
            let sweep = shape.sweep(d.0, d.1, TOLERANCE)?;
            for _ in 0..400 {
                let p = (-60.0 + 220.0 * random.unit(), -60.0 + 220.0 * random.unit());
                assert_eq!(
                    sweep.contains(p)?,
                    swept(&shape, &edges, p, d)?,
                    "{what} swept by {d:?}, at {p:?}"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn a_square_swept_along_an_axis_is_a_rectangle() -> Outcome {
    let square = Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)?;
    for ((dx, dy), (x, y, w, h)) in [
        ((30.0, 0.0), (0.0, 0.0, 40.0, 10.0)),
        ((-30.0, 0.0), (-30.0, 0.0, 40.0, 10.0)),
        ((0.0, 25.0), (0.0, 0.0, 10.0, 35.0)),
        ((0.0, -25.0), (0.0, -25.0, 10.0, 35.0)),
    ] {
        assert_eq!(
            square.sweep(dx, dy, TOLERANCE)?.to_svg_path(),
            Path::rect(x, y, w, h, 0.0)?.resolve(TOLERANCE)?.to_svg_path(),
            "({dx}, {dy})"
        );
    }
    Ok(())
}

#[test]
fn a_convex_polygon_grows_by_the_length_times_its_width_across_the_vector() -> Outcome {
    // a triangle and a hexagon, swept along a diagonal: the area grows by |d| times the width of the polygon measured
    // across d (the spread of its corners along the normal of d)
    let triangle = Path::polyline([(0.0, 0.0), (40.0, 0.0), (10.0, 30.0)], true)?;
    let hexagon = Path::polygon((50.0, 50.0), 20.0, 6)?;
    for shape in [triangle, hexagon] {
        let corners = rings(&shape.resolve(TOLERANCE)?)?.remove(0);
        for (dx, dy) in [(30.0, 40.0), (-12.0, 5.0), (0.0, -9.0)] {
            let length = f64::sqrt(dx * dx + dy * dy);
            let normal = (-dy / length, dx / length);
            let across: Vec<f64> = corners.iter().map(|c| c.0 * normal.0 + c.1 * normal.1).collect();
            let width = across.iter().copied().fold(f64::NEG_INFINITY, f64::max)
                - across.iter().copied().fold(f64::INFINITY, f64::min);
            let wanted = area(&shape)?.abs() + length * width;
            let found = area(&shape.sweep(dx, dy, TOLERANCE)?)?.abs();
            assert!(
                (found - wanted).abs() < 1e-6 * wanted,
                "({dx}, {dy}): {found}, wanted {wanted}"
            );
        }
    }
    Ok(())
}

#[test]
fn no_vector_is_the_region_the_shape_fills() -> Outcome {
    for (what, shape) in shapes()? {
        assert_eq!(
            shape.sweep(0.0, 0.0, TOLERANCE)?.to_svg_path(),
            shape.resolve(TOLERANCE)?.to_svg_path(),
            "{what}"
        );
    }
    // curves and overlaps come back as the region they fill, as `offset(0)` gives it
    let overlapping = Path::from_svg_path("M0 0 L20 0 L20 20 L0 20 Z M10 10 L30 10 L30 30 L10 30 Z")?;
    for shape in [Path::circle((5.0, 5.0), 5.0)?, overlapping] {
        assert_ne!(shape.resolve(TOLERANCE)?.to_svg_path(), shape.to_svg_path());
        assert_eq!(
            shape.sweep(0.0, 0.0, TOLERANCE)?.to_svg_path(),
            shape.resolve(TOLERANCE)?.to_svg_path()
        );
    }
    assert_eq!(Path::new().sweep(5.0, 5.0, TOLERANCE)?.to_svg_path(), "");
    Ok(())
}

#[test]
fn a_long_sweep_fills_a_hole_and_a_short_one_keeps_it() -> Outcome {
    let frame = Path::from_svg_path("M10 10 L90 10 L90 90 L10 90 Z M30 30 L30 70 L70 70 L70 30 Z")?;
    // the hole's left wall moves from 30 to 40
    assert!(frame.sweep(10.0, 0.0, TOLERANCE)?.contains((35.0, 50.0))?);
    assert!(!frame.sweep(10.0, 0.0, TOLERANCE)?.contains((45.0, 50.0))?);
    // 40 to the right covers the hole, 40 wide, from its left wall
    assert!(frame.sweep(40.0, 0.0, TOLERANCE)?.contains((50.0, 50.0))?);
    assert_eq!(rings(&frame.sweep(40.0, 0.0, TOLERANCE)?)?.len(), 1);
    Ok(())
}

#[test]
fn a_curve_is_swept_within_the_tolerance() -> Outcome {
    // a disc of radius 20 swept 50 to the right is a stadium: 20^2 pi + 50 * 40
    let disc = Path::circle((0.0, 0.0), 20.0)?;
    let found = area(&disc.sweep(50.0, 0.0, TOLERANCE)?)?.abs();
    let wanted = 400.0 * std::f64::consts::PI + 2000.0;
    // flattened within 0.01 of the circle: the area is short by at most the perimeter times the tolerance
    assert!(
        found <= wanted && found > wanted - 40.0 * std::f64::consts::PI * TOLERANCE,
        "{found}"
    );
    Ok(())
}

#[test]
fn what_it_cannot_sweep_is_refused_naming_the_argument() -> Outcome {
    let square = Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)?;
    for (dx, dy, name) in [
        (f64::NAN, 0.0, "dx"),
        (f64::INFINITY, 0.0, "dx"),
        (0.0, f64::NAN, "dy"),
        (2e100, 0.0, "dx"),
        (0.0, -2e100, "dy"),
    ] {
        assert_eq!(
            parameter_refusal(square.sweep(dx, dy, TOLERANCE))?.0,
            name,
            "({dx}, {dy})"
        );
    }
    for tolerance in [0.0, -1.0, f64::NAN] {
        assert_eq!(parameter_refusal(square.sweep(1.0, 1.0, tolerance))?.0, "tolerance");
    }
    // a square swept a long way is a few edges, accepted (far enough, 1e90, and the engine's grid, a 2^-60 of the
    // extent, is wider than the square: the boolean limit, api.md)
    assert!(square.sweep(1e6, 1e6, TOLERANCE)?.bounds()?.width > 9e5);
    // a disc of 70,000 edges swept past itself on a diagonal: the box of every parallelogram meets nearly every other,
    // billions of pairs that may cross, past MAX_SEGMENTS
    let fine = Path::circle((0.0, 0.0), 1000.0)?;
    let (name, message) = parameter_refusal(fine.sweep(5000.0, 5000.0, 1e-6))?;
    assert_eq!(name, "dx", "{message}");
    Ok(())
}
