//! What `Drawing::to_mesh` makes of a fill: the triangles cover the region the nonzero rule fills, each point once,
//! and nothing else. The oracles are a winding number written in the tests (random polygons with holes and
//! self-overlap), closed-form areas and distances, never the tessellator.

mod common;

use bullterrier_vector::{Color, Drawing, Path};
use common::mesh::{area, check_layout, covering, mesh_of, triangles};
use common::{Corners, Outcome, clearance, winding};
use proptest::prelude::*;
use std::f64::consts::PI;

fn fill(path: Path) -> Drawing {
    Drawing::from(path).fill(Color::hex(0x0033_6699))
}

/// The closed polygons `polygons` as the sub-paths of one path.
fn polygons_path(polygons: &[Corners]) -> Result<Path, bullterrier_vector::Error> {
    let mut path = Path::new();
    for corners in polygons {
        path = path.move_to(corners[0])?;
        for corner in &corners[1..] {
            path = path.line_to(*corner)?;
        }
        path = path.close()?;
    }
    Ok(path)
}

fn polygons() -> impl Strategy<Value = Vec<Corners>> {
    let corner = (0..100_i32, 0..100_i32).prop_map(|(x, y)| (f64::from(x), f64::from(y)));
    prop::collection::vec(prop::collection::vec(corner, 3..8), 1..4)
}

fn points() -> impl Strategy<Value = Vec<(f64, f64)>> {
    prop::collection::vec((-5.0..105.0_f64, -5.0..105.0_f64), 60)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    #[test]
    fn a_point_is_covered_once_where_the_nonzero_rule_says_inside_and_never_elsewhere(
        polygons in polygons(),
        points in points(),
    ) {
        let mesh = mesh_of(&fill(polygons_path(&polygons).map_err(|e| TestCaseError::fail(e.to_string()))?), 100.0, 100.0)
            .map_err(|e| TestCaseError::fail(e.to_string()))?;
        check_layout(&mesh).map_err(TestCaseError::fail)?;
        let triangles = triangles(&mesh);
        let mut decided = 0;
        for point in points {
            // a point on an outline may be either: judge those away from every edge only
            if polygons.iter().any(|corners| clearance(corners, point) < 0.05) {
                continue;
            }
            let inside = polygons.iter().map(|corners| winding(corners, point)).sum::<i32>() != 0;
            let Some(covered) = covering(&triangles, point) else { continue };
            decided += 1;
            prop_assert_eq!(covered, usize::from(inside), "point {:?} of {:?}", point, polygons);
        }
        prop_assert!(decided > 10, "only {} points were decided", decided);
    }
}

#[test]
fn the_triangles_of_a_rectangle_have_its_area_exactly_and_those_of_a_bow_tie_its_two_wings() -> Outcome {
    let rectangle = mesh_of(&fill(Path::rect(10.0, 20.0, 30.0, 40.0, 0.0)?), 100.0, 100.0)?;
    assert!((triangles(&rectangle).iter().map(area).sum::<f64>() - 1200.0).abs() < 1e-4);
    // two triangles meeting at (50, 50): each wing is 100 * 50 / 2 = 2500 whichever way the outline runs
    let bow_tie = Path::polyline([(0.0, 0.0), (100.0, 100.0), (100.0, 0.0), (0.0, 100.0)], true)?;
    let mesh = mesh_of(&fill(bow_tie), 100.0, 100.0)?;
    assert!((triangles(&mesh).iter().map(area).sum::<f64>() - 5000.0).abs() < 1e-3);
    Ok(())
}

#[test]
fn curves_are_filled_to_the_flatness_of_the_other_outputs() -> Outcome {
    // (case, path, the area it encloses worked in closed form); the flattening keeps within 0.01 of the curve, which
    // loses under 0.05% of a circle of radius 10
    let rows = [
        (
            "a circle of radius 10",
            Path::circle((50.0, 50.0), 10.0)?,
            PI * 100.0,
            0.0005,
        ),
        (
            "a circle of radius 30",
            Path::circle((50.0, 50.0), 30.0)?,
            PI * 900.0,
            0.0005,
        ),
        (
            "a rectangle with rounded corners",
            Path::rect(10.0, 10.0, 40.0, 20.0, 5.0)?,
            800.0 - 25.0 * (4.0 - PI),
            0.0005,
        ),
    ];
    for (case, path, expected, tolerance) in rows {
        let mesh = mesh_of(&fill(path), 100.0, 100.0)?;
        let total: f64 = triangles(&mesh).iter().map(area).sum();
        assert!(
            ((total - expected) / expected).abs() < tolerance,
            "{case}: {total} against {expected}"
        );
    }
    Ok(())
}

#[test]
fn a_circle_is_covered_once_inside_and_not_at_all_outside() -> Outcome {
    let mesh = mesh_of(&fill(Path::circle((50.0, 50.0), 20.0)?), 100.0, 100.0)?;
    let triangles = triangles(&mesh);
    let mut decided = 0;
    for column in 0..100 {
        for row in 0..100 {
            let point = (f64::from(column) + 0.37, f64::from(row) + 0.61);
            let distance = common::distance(point, (50.0, 50.0));
            // the flattened circle lies within 0.01 of the true one
            if (distance - 20.0).abs() < 0.05 {
                continue;
            }
            let Some(covered) = covering(&triangles, point) else {
                continue;
            };
            decided += 1;
            assert_eq!(covered, usize::from(distance < 20.0), "{point:?}");
        }
    }
    assert!(decided > 9000, "{decided}");
    Ok(())
}

#[test]
fn a_hole_and_an_overlap_follow_the_nonzero_rule() -> Outcome {
    // an outer square with an inner one wound the other way is a frame; two squares wound the same way overlap: the
    // overlap is inside once, where even-odd would leave it out
    let outer = vec![(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)];
    let hole = vec![(25.0, 25.0), (25.0, 75.0), (75.0, 75.0), (75.0, 25.0)];
    let same_way = vec![(25.0, 25.0), (75.0, 25.0), (75.0, 75.0), (25.0, 75.0)];
    let rows = [
        ("a frame", vec![outer.clone(), hole], (50.3, 49.1), 0, (12.3, 37.1), 1),
        ("an overlap", vec![outer, same_way], (50.3, 49.1), 1, (12.3, 37.1), 1),
    ];
    for (case, polygons, probe, expected, other, expected_other) in rows {
        let mesh = mesh_of(&fill(polygons_path(&polygons)?), 100.0, 100.0)?;
        let triangles = triangles(&mesh);
        assert_eq!(covering(&triangles, probe), Some(expected), "{case} at {probe:?}");
        assert_eq!(covering(&triangles, other), Some(expected_other), "{case} at {other:?}");
    }
    Ok(())
}

#[test]
fn an_open_sub_path_fills_as_if_closed() -> Outcome {
    let open = Path::polyline([(10.0, 10.0), (90.0, 10.0), (50.0, 90.0)], false)?;
    let mesh = mesh_of(&fill(open), 100.0, 100.0)?;
    assert!((triangles(&mesh).iter().map(area).sum::<f64>() - 3200.0).abs() < 1e-3);
    Ok(())
}
