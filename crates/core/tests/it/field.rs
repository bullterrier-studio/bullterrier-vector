//! `Field` through the public API. Oracles are closed forms: the distance to a circle is `|q - c| - r`, a ring's
//! normalised distance is `(R - rho) / (R - r)`, a ramp is a projection.

use crate::common::Outcome;
use bullterrier_vector::{Error, Field, Path, Point};

const RADIUS: f64 = 50.0;

fn circle() -> Result<Path, Error> {
    Path::circle((100.0, 100.0), RADIUS)
}

/// A ring: an outer circle clockwise and an inner one counter-clockwise (reversed).
fn ring(outer: f64, inner: f64) -> Result<Path, Error> {
    let hole = Path::circle((100.0, 100.0), inner)?.reverse();
    let outer = Path::circle((100.0, 100.0), outer)?;
    Path::from_svg_path(&format!("{} {}", outer.to_svg_path(), hole.to_svg_path()))
}

fn radius_of(q: (f64, f64)) -> f64 {
    libm::hypot(q.0 - 100.0, q.1 - 100.0)
}

#[test]
fn signed_distance_of_a_circle_is_the_analytic_one() -> Outcome {
    let field = Field::distance(&circle()?, false)?;
    for q in [
        (100.0, 100.0),
        (130.0, 100.0),
        (100.0, 190.0),
        (20.0, 20.0),
        (300.0, -40.0),
    ] {
        let want = radius_of(q) - RADIUS;
        let got = field.at(Point::new(q.0, q.1));
        assert!((got - want).abs() < 0.05, "at {q:?}: {got} against {want}");
    }
    Ok(())
}

#[test]
fn normalised_distance_of_a_ring_runs_from_outer_edge_to_hole_edge() -> Outcome {
    let field = Field::distance(&ring(80.0, 30.0)?, true)?;
    for rho in [32.0, 45.0, 55.0, 70.0, 78.0] {
        let want = (80.0 - rho) / (80.0 - 30.0);
        let got = field.at(Point::new(100.0 + rho, 100.0));
        assert!((got - want).abs() < 0.01, "rho {rho}: {got} against {want}");
    }
    assert_eq!(field.at(Point::new(100.0, 100.0)), 0.0, "inside the hole");
    assert_eq!(field.at(Point::new(400.0, 100.0)), 0.0, "outside");
    Ok(())
}

#[test]
fn normalised_distance_without_a_hole_is_one_at_the_deepest_point() -> Outcome {
    let field = Field::distance(&circle()?, true)?;
    assert!((field.at(Point::new(100.0, 100.0)) - 1.0).abs() < 0.02);
    // never above 1, wherever the deepest point is estimated to lie
    for step in 0..50 {
        let at = Point::new(100.0 + f64::from(step) * 0.07, 100.0 - f64::from(step) * 0.05);
        assert!(field.at(at) <= 1.0, "{} at {at:?}", field.at(at));
    }
    let half = field.at(Point::new(125.0, 100.0));
    assert!((half - 0.5).abs() < 0.02, "{half}");
    Ok(())
}

#[test]
fn ramp_projects_onto_the_line_and_holds_beyond_it() -> Outcome {
    let field = Field::ramp((0.0, 0.0), (100.0, 0.0))?;
    assert_eq!(field.at(Point::new(25.0, 70.0)), 0.25);
    assert_eq!(field.at(Point::new(-30.0, 0.0)), 0.0);
    assert_eq!(field.at(Point::new(900.0, 5.0)), 1.0);
    assert!(Field::ramp((1.0, 1.0), (1.0, 1.0)).is_err());
    Ok(())
}

#[test]
fn distance_of_an_empty_path_is_a_geometry_error() {
    assert!(matches!(
        Field::distance(&Path::new(), false),
        Err(Error::Geometry { .. })
    ));
}
