//! What `Drawing::to_mesh` makes of a stroke: the triangles cover the region the stroke paints, each point once even
//! where the stroke overlaps itself. The oracles are a brute-force distance to the centreline and hand-worked corner,
//! cap and zero-length rows, never the tessellator.

use crate::common::mesh::{area, check_layout, covering, mesh_of, triangles};
use crate::common::{Corners, Outcome};
use bullterrier_vector::{Cap, Color, Drawing, Join, Path, Stroke};
use libm::hypot;
use proptest::prelude::*;

type Point = (f64, f64);

/// The drawing of `path` stroked and not filled.
fn stroked(path: Path, stroke: Stroke) -> Drawing {
    Drawing::from(path).fill(Color::NONE).stroke(stroke)
}

fn black() -> Color {
    Color::hex(0)
}

/// The distance from `point` to the segment `from`-`to`.
fn segment_distance(point: Point, from: Point, to: Point) -> f64 {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let along = (((point.0 - from.0) * dx + (point.1 - from.1) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    hypot(point.0 - (from.0 + dx * along), point.1 - (from.1 + dy * along))
}

/// The distance from `point` to the polyline through `corners`, `closed` or not.
fn centreline_distance(corners: &[Point], closed: bool, point: Point) -> f64 {
    let count = if closed { corners.len() } else { corners.len() - 1 };
    (0..count)
        .map(|i| segment_distance(point, corners[i], corners[(i + 1) % corners.len()]))
        .fold(f64::INFINITY, f64::min)
}

fn corners() -> impl Strategy<Value = Corners> {
    // whole numbers make corners meet and edges run along one another; fractions put them in general position
    let whole = (0..100_i32, 0..100_i32).prop_map(|(x, y)| (f64::from(x), f64::from(y)));
    let corner = prop_oneof![whole, (5.0..95.0_f64, 5.0..95.0_f64)];
    // no two neighbours the same: a zero-length segment is a different case (below)
    prop::collection::vec(corner, 2..8).prop_filter("neighbours differ", |corners| {
        corners.windows(2).all(|pair| pair[0] != pair[1]) && corners[0] != corners[corners.len() - 1]
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]

    #[test]
    fn a_round_stroke_covers_each_point_within_half_its_width_of_the_centreline_once_and_no_other(
        corners in corners(),
        closed in any::<bool>(),
        width in 2.0..22.0_f64,
        points in prop::collection::vec((-10.0..110.0_f64, -10.0..110.0_f64), 120),
    ) {
        let path = Path::polyline(corners.clone(), closed).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let stroke = Stroke::new(black()).width(width).join(Join::Round).cap(Cap::Round);
        let mesh = mesh_of(&stroked(path, stroke), 100.0, 100.0).map_err(|e| TestCaseError::fail(e.to_string()))?;
        check_layout(&mesh).map_err(TestCaseError::fail)?;
        let triangles = triangles(&mesh);
        let mut decided = 0;
        for point in points {
            let distance = centreline_distance(&corners, closed, point);
            // the arcs are flattened to within 0.01 of the true ones
            if (distance - width / 2.0).abs() < 0.05 {
                continue;
            }
            let Some(covered) = covering(&triangles, point) else { continue };
            decided += 1;
            prop_assert_eq!(
                covered,
                usize::from(distance < width / 2.0),
                "point {:?} at {} from the centreline of {:?}, closed {}, width {}",
                point, distance, corners, closed, width
            );
        }
        prop_assert!(decided > 20, "only {} points were decided", decided);
    }
}

/// A stroke `width` 10 along `path`, with this join and cap.
fn ten(path: Path, join: Join, cap: Cap) -> Drawing {
    stroked(path, Stroke::new(black()).width(10.0).join(join).cap(cap))
}

/// A case: what is drawn, and points with how many triangles cover each (0 or 1).
type Row = (&'static str, Drawing, Vec<(Point, usize)>);

/// Checks every probe of every row.
fn check_rows(rows: Vec<Row>) -> Outcome {
    for (case, drawing, probes) in rows {
        let mesh = mesh_of(&drawing, 100.0, 100.0)?;
        check_layout(&mesh)?;
        let triangles = triangles(&mesh);
        for (point, expected) in probes {
            assert_eq!(covering(&triangles, point), Some(expected), "{case}: at {point:?}");
        }
    }
    Ok(())
}

fn line() -> Result<Path, bullterrier_vector::Error> {
    Path::line((20.0, 50.0), (80.0, 50.0))
}

/// Strokes that doubled back at a sharp turn and were once covered three times in places: (case, corners, closed,
/// width), found by drawing 20,000 random round strokes.
#[test]
fn round_strokes_that_double_back_at_sharp_turns_cover_each_point_once() -> Outcome {
    let rows: [(&str, Corners, bool, f64); 6] = [
        (
            "an open turn of 40 degrees",
            vec![(10.0, 10.0), (80.0, 80.0), (40.0, 30.0)],
            false,
            18.0,
        ),
        (
            "a closed triangle",
            vec![(80.0, 50.0), (10.0, 60.0), (40.0, 50.0)],
            true,
            18.0,
        ),
        (
            "a closed quadrilateral",
            vec![
                (52.394_969_201_438_97, 25.524_624_780_930_83),
                (74.011_205_871_579_3, 41.982_331_588_308_625),
                (62.030_205_902_895_53, 24.311_937_545_198_212),
                (38.002_251_687_709_23, 38.682_759_388_020_18),
            ],
            true,
            19.0,
        ),
        (
            "an open zig-zag",
            vec![
                (88.243_755_967_896_33, 24.921_443_596_327_01),
                (40.309_525_162_098_22, 46.052_886_312_093_34),
                (22.674_047_735_079_483, 88.450_773_092_632_57),
                (55.701_851_194_157_1, 46.463_555_194_553_16),
            ],
            false,
            20.0,
        ),
        (
            "a closed sliver",
            vec![
                (32.542_984_082_051_056, 44.618_589_627_473_93),
                (10.953_165_615_993_123, 89.843_372_587_532_83),
                (54.468_830_081_986_74, 38.525_553_961_768_4),
                (39.961_516_840_739_38, 38.561_393_359_194_03),
            ],
            true,
            16.0,
        ),
        (
            "an open turn back by about 170 degrees",
            vec![
                (82.0, 68.0),
                (49.157_006_523_411_766, 5.0),
                (77.501_713_592_125_18, 69.026_048_892_878_89),
                (64.0, 0.0),
            ],
            false,
            19.377_304_158_942_497,
        ),
    ];
    for (case, corners, closed, width) in rows {
        let stroke = Stroke::new(black()).width(width).join(Join::Round).cap(Cap::Round);
        let mesh = mesh_of(&stroked(Path::polyline(corners.clone(), closed)?, stroke), 100.0, 100.0)?;
        check_layout(&mesh)?;
        let triangles = triangles(&mesh);
        let mut decided = 0;
        for x in 0..160 {
            for y in 0..160 {
                // a grid that avoids whole numbers, where the corners lie
                let point = (f64::from(x) * 0.7 - 6.0 + 0.113, f64::from(y) * 0.7 - 6.0 + 0.071);
                let distance = centreline_distance(&corners, closed, point);
                if (distance - width / 2.0).abs() < 0.05 {
                    continue;
                }
                let Some(covered) = covering(&triangles, point) else {
                    continue;
                };
                decided += 1;
                assert_eq!(covered, usize::from(distance < width / 2.0), "{case}: at {point:?}");
            }
        }
        assert!(decided > 10_000, "{case}: only {decided} points were decided");
    }
    Ok(())
}

#[test]
fn repeated_and_collinear_corners_leave_round_strokes_whole() -> Outcome {
    // (case, the corners drawn, the corners the stroke runs through, closed)
    let rows: [(&str, Corners, Corners, bool); 3] = [
        (
            "repeats and a straight run",
            vec![
                (20.0, 50.0),
                (20.0, 50.0),
                (50.0, 50.0),
                (80.0, 50.0),
                (80.0, 50.0),
                (50.0, 80.0),
            ],
            vec![(20.0, 50.0), (50.0, 50.0), (80.0, 50.0), (50.0, 80.0)],
            false,
        ),
        (
            "a closed outline that repeats its start",
            vec![(20.0, 20.0), (80.0, 30.0), (40.0, 80.0), (20.0, 20.0)],
            vec![(20.0, 20.0), (80.0, 30.0), (40.0, 80.0)],
            true,
        ),
        (
            "a path that turns straight back",
            vec![(20.0, 50.0), (80.0, 50.0), (30.0, 50.0)],
            vec![(20.0, 50.0), (80.0, 50.0), (30.0, 50.0)],
            false,
        ),
    ];
    for (case, drawn, run, closed) in rows {
        let stroke = Stroke::new(black()).width(12.0).join(Join::Round).cap(Cap::Round);
        let mesh = mesh_of(&stroked(Path::polyline(drawn, closed)?, stroke), 100.0, 100.0)?;
        check_layout(&mesh)?;
        let triangles = triangles(&mesh);
        for x in 0..120 {
            for y in 0..120 {
                let point = (f64::from(x) * 0.8 + 0.113, f64::from(y) * 0.8 + 0.071);
                let distance = centreline_distance(&run, closed, point);
                if (distance - 6.0).abs() < 0.05 {
                    continue;
                }
                let Some(covered) = covering(&triangles, point) else {
                    continue;
                };
                assert_eq!(covered, usize::from(distance < 6.0), "{case}: at {point:?}");
            }
        }
    }
    Ok(())
}

#[test]
fn a_smooth_curve_gets_no_extra_triangles_for_round_joins() -> Outcome {
    // the corners of a flattened circle turn by a few degrees, where a bevel is within 0.01 of the arc: a round join
    // adds nothing there, while the same stroke's two sharp corners do add their wedges
    let count = |join| -> Result<usize, Box<dyn std::error::Error>> {
        let circle = Path::circle((50.0, 50.0), 30.0)?;
        let mesh = mesh_of(&ten(circle, join, Cap::Butt), 100.0, 100.0)?;
        Ok(mesh.indices().len() / 3)
    };
    assert_eq!(count(Join::Round)?, count(Join::Bevel)?);
    let corner = |join| -> Result<usize, Box<dyn std::error::Error>> {
        let path = Path::polyline([(20.0, 20.0), (80.0, 20.0), (80.0, 80.0)], false)?;
        Ok(mesh_of(&ten(path, join, Cap::Butt), 100.0, 100.0)?.indices().len() / 3)
    };
    assert!(corner(Join::Round)? > corner(Join::Bevel)?);
    Ok(())
}

#[test]
fn ends_and_corners_cover_what_svg_paints() -> Outcome {
    let corner = || Path::polyline([(20.0, 20.0), (80.0, 20.0), (80.0, 80.0)], false);
    let square = || Path::polyline([(20.0, 20.0), (80.0, 20.0), (80.0, 80.0), (20.0, 80.0)], true);
    check_rows(vec![
        (
            "butt ends stop at the end points",
            ten(line()?, Join::Miter, Cap::Butt),
            vec![
                ((50.0, 53.0), 1),
                ((50.0, 56.0), 0),
                ((17.0, 50.0), 0),
                ((83.0, 50.0), 0),
                ((22.0, 54.0), 1),
            ],
        ),
        (
            "square ends reach half a width beyond",
            ten(line()?, Join::Miter, Cap::Square),
            vec![
                ((17.0, 50.0), 1),
                ((83.0, 54.0), 1),
                ((14.0, 50.0), 0),
                ((17.0, 56.0), 0),
            ],
        ),
        (
            "round ends are discs",
            ten(line()?, Join::Miter, Cap::Round),
            vec![
                ((18.0, 53.0), 1),
                ((16.0, 56.0), 0),
                ((14.0, 50.0), 0),
                ((84.0, 50.0), 1),
            ],
        ),
        (
            "a miter fills the corner",
            ten(corner()?, Join::Miter, Cap::Butt),
            vec![
                ((84.2, 16.4), 1),
                ((86.0, 14.0), 0),
                ((50.0, 26.0), 0),
                ((50.0, 23.0), 1),
            ],
        ),
        (
            "a bevel cuts it",
            ten(corner()?, Join::Bevel, Cap::Butt),
            vec![((82.0, 19.0), 1), ((84.0, 16.0), 0)],
        ),
        (
            "a round join is a disc",
            ten(corner()?, Join::Round, Cap::Butt),
            vec![((83.0, 17.0), 1), ((84.0, 16.0), 0)],
        ),
        (
            "a closed outline is a ring, joined at its start too",
            ten(square()?, Join::Miter, Cap::Butt),
            vec![
                ((50.0, 50.0), 0),
                ((23.0, 50.0), 1),
                ((27.0, 50.0), 0),
                ((16.0, 17.0), 1),
                ((14.0, 50.0), 0),
            ],
        ),
    ])
}

#[test]
fn zero_length_sub_paths_leave_what_their_ends_say() -> Outcome {
    let dot = || Path::start((50.0, 50.0))?.line_to((50.0, 50.0));
    check_rows(vec![
        (
            "a zero-length sub-path under a round end is a disc",
            ten(dot()?, Join::Miter, Cap::Round),
            vec![((53.0, 50.0), 1), ((56.0, 50.0), 0), ((54.0, 54.0), 0)],
        ),
        (
            "and under a square end a square",
            ten(dot()?, Join::Miter, Cap::Square),
            vec![((54.0, 54.0), 1), ((56.0, 50.0), 0)],
        ),
        (
            "and under a butt end nothing",
            ten(dot()?, Join::Miter, Cap::Butt),
            vec![((50.0, 50.0), 0)],
        ),
        (
            "a dot inside a stroke, and one that straddles its end, are covered once",
            ten(
                line()?
                    .move_to((50.0, 50.0))?
                    .line_to((50.0, 50.0))?
                    .move_to((80.0, 50.0))?
                    .line_to((80.0, 50.0))?,
                Join::Miter,
                Cap::Round,
            ),
            vec![
                ((50.0, 53.0), 1),
                ((82.0, 52.0), 1),
                ((83.0, 53.0), 1),
                ((86.0, 50.0), 0),
            ],
        ),
    ])
}

#[test]
fn the_triangles_of_a_stroke_have_the_area_of_its_outline() -> Outcome {
    // length 60 by width 10; the ends add a square (10 by 5 each) or half a disc (25 pi / 2 each)
    let line = || Path::line((20.0, 50.0), (80.0, 50.0));
    let rows = [
        ("butt", Cap::Butt, 600.0),
        ("square", Cap::Square, 700.0),
        ("round", Cap::Round, 600.0 + 25.0 * std::f64::consts::PI),
    ];
    for (case, cap, expected) in rows {
        let mesh = mesh_of(&ten(line()?, Join::Miter, cap), 100.0, 100.0)?;
        let total: f64 = triangles(&mesh).iter().map(area).sum();
        assert!(
            ((total - expected) / expected).abs() < 0.001,
            "{case}: {total} against {expected}"
        );
    }
    Ok(())
}

#[test]
fn a_stroke_over_a_fill_is_a_layer_of_its_own() -> Outcome {
    // a circle filled and stroked: at its rim both paint, so the fill's triangles and the stroke's overlap there; at
    // the centre only the fill does, and a point just outside the rim only the stroke
    let circle = Drawing::from(Path::circle((50.0, 50.0), 30.0)?)
        .fill(Color::hex(0x0000_ff00))
        .stroke(Stroke::new(black()).width(10.0));
    let mesh = mesh_of(&circle, 100.0, 100.0)?;
    let all = triangles(&mesh);
    assert_eq!(covering(&all, (50.0, 50.0)), Some(1));
    assert_eq!(covering(&all, (50.0, 21.0)), Some(2));
    assert_eq!(covering(&all, (50.0, 17.0)), Some(1));
    assert_eq!(covering(&all, (50.0, 12.0)), Some(0));
    Ok(())
}
