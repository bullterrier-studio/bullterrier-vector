//! `Path::within` through the public API. Oracles: a horizontal line through a circle is cut at the chord worked out by
//! hand, and every point of every piece is checked against `Path::contains` of the region.

use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::Path;

fn lines(path: &Path) -> Vec<(Vec<(f64, f64)>, bool)> {
    path.to_svg_path()
        .split('M')
        .skip(1)
        .map(|part| {
            let numbers: Vec<f64> = part
                .split(|c: char| c.is_ascii_alphabetic() || c == ' ' || c == ',')
                .filter_map(|piece| piece.parse().ok())
                .collect();
            (
                numbers.chunks(2).map(|pair| (pair[0], pair[1])).collect(),
                part.contains('Z'),
            )
        })
        .collect()
}

#[test]
fn a_line_through_a_circle_is_cut_at_the_chord() -> Outcome {
    let region = Path::circle((100.0, 50.0), 30.0)?;
    let line = Path::polyline([(0.0, 50.0), (200.0, 50.0)], false)?;
    let found = lines(&line.within(&region)?);
    assert_eq!(found.len(), 1);
    let points = &found[0].0;
    assert!(
        (points[0].0 - 70.0).abs() < 0.01 && (points[points.len() - 1].0 - 130.0).abs() < 0.01,
        "{points:?}"
    );
    Ok(())
}

#[test]
fn a_loop_that_leaves_the_region_becomes_open_pieces_that_all_lie_inside() -> Outcome {
    let region = Path::rect(0.0, 0.0, 100.0, 100.0, 0.0)?;
    let ring = Path::circle((100.0, 50.0), 40.0)?;
    let found = lines(&ring.within(&region)?);
    assert_eq!(
        found.len(),
        1,
        "one arc: the loop starts inside, so its two ends are joined"
    );
    assert!(!found[0].1);
    for p in &found[0].0 {
        assert!(p.0 <= 100.0 + 1e-6, "{p:?} is outside");
        let r = libm::hypot(p.0 - 100.0, p.1 - 50.0);
        assert!((r - 40.0).abs() < 0.05);
    }
    Ok(())
}

#[test]
fn a_loop_wholly_inside_stays_closed_and_one_wholly_outside_is_dropped() -> Outcome {
    let region = Path::rect(0.0, 0.0, 100.0, 100.0, 0.0)?;
    let inside = Path::circle((50.0, 50.0), 10.0)?;
    assert!(lines(&inside.within(&region)?)[0].1);
    assert_eq!(Path::circle((300.0, 50.0), 10.0)?.within(&region)?.to_svg_path(), "");
    Ok(())
}

#[test]
fn a_hole_of_the_region_cuts_the_line() -> Outcome {
    // the nonzero rule: the inner square runs the other way
    let region = Path::start((0.0, 0.0))?
        .line_to((100.0, 0.0))?
        .line_to((100.0, 100.0))?
        .line_to((0.0, 100.0))?
        .close()?
        .move_to((40.0, 40.0))?
        .line_to((40.0, 60.0))?
        .line_to((60.0, 60.0))?
        .line_to((60.0, 40.0))?
        .close()?;
    let line = Path::polyline([(0.0, 50.0), (100.0, 50.0)], false)?;
    let found = lines(&line.within(&region)?);
    assert_eq!(found.len(), 2);
    assert!((found[0].0[1].0 - 40.0).abs() < 0.01 && (found[1].0[0].0 - 60.0).abs() < 0.01);
    Ok(())
}

#[test]
fn an_empty_region_or_path_gives_nothing() -> Outcome {
    let line = Path::polyline([(0.0, 0.0), (10.0, 0.0)], false)?;
    assert_eq!(line.within(&Path::new())?.to_svg_path(), "");
    assert_eq!(Path::new().within(&Path::circle((0.0, 0.0), 5.0)?)?.to_svg_path(), "");
    Ok(())
}

#[test]
fn a_long_slanted_line_across_many_bands_is_cut_once_per_edge() -> Outcome {
    // a tall thin rectangle: its sides span every band; the slanted line crosses each side once
    let region = Path::rect(40.0, 0.0, 20.0, 1000.0, 0.0)?;
    let line = Path::polyline([(0.0, 100.0), (100.0, 900.0)], false)?;
    let found = lines(&line.within(&region)?);
    assert_eq!(found.len(), 1);
    let points = &found[0].0;
    assert_eq!(points.len(), 2);
    assert!(
        (points[0].0 - 40.0).abs() < 1e-9 && (points[1].0 - 60.0).abs() < 1e-9,
        "{points:?}"
    );
    Ok(())
}

#[test]
fn a_line_is_cut_the_same_whichever_way_it_runs() -> Outcome {
    // a point on the region's own edge is inside or outside by chance, so the middle of each part decides, not its start
    let region = Path::rect(0.0, 0.0, 100.0, 100.0, 0.0)?;
    for (from, to) in [
        ((150.0, 50.0), (50.0, 50.0)),
        ((-50.0, 50.0), (50.0, 50.0)),
        ((50.0, 150.0), (50.0, 50.0)),
    ] {
        let found = lines(&Path::polyline([from, to], false)?.within(&region)?);
        assert_eq!(found.len(), 1, "{from:?} to {to:?}");
        for p in &found[0].0 {
            assert!(
                (-1e-9..=100.0 + 1e-9).contains(&p.0) && (-1e-9..=100.0 + 1e-9).contains(&p.1),
                "{p:?}"
            );
        }
        assert!(
            (libm::hypot(found[0].0[0].0 - found[0].0[1].0, found[0].0[0].1 - found[0].0[1].1) - 50.0).abs() < 1e-9
        );
    }
    Ok(())
}

/// A comb of `teeth` teeth one wide and one apart, 10 high, all in one row of the region's grid, as an SVG path.
fn comb(teeth: usize) -> Result<Path, bullterrier_vector::Error> {
    let mut data = String::from("M0 -5");
    for i in 0..teeth {
        data.push_str(&format!("L{} 0L{} 10L{} 10L{} 0", 2 * i, 2 * i, 2 * i + 1, 2 * i + 1));
    }
    data.push_str(&format!("L{} 0L{} -5Z", 2 * teeth, 2 * teeth));
    Path::from_svg_path(&data)
}

#[test]
fn a_comb_of_many_edges_is_cut_by_a_line_of_many_points_without_walking_every_edge_for_each() -> Outcome {
    // 100,000 edges all in one band and a line of 100,000 points: every edge against every point is 1e10 visits
    let teeth = 25_000;
    let region = comb(teeth)?;
    let points: Vec<(f64, f64)> = (0..=4 * teeth).map(|i| (i as f64 * 0.5, 5.0)).collect();
    let found = lines(&Path::polyline(points, false)?.within(&region)?);
    assert_eq!(found.len(), teeth, "one piece in each tooth");
    for (piece, _) in found.iter().take(50) {
        let (first, last) = (piece[0], piece[piece.len() - 1]);
        assert!((last.0 - first.0 - 1.0).abs() < 1e-9, "{piece:?}");
    }
    Ok(())
}

#[test]
fn work_past_the_budget_is_refused_naming_the_region_before_it_is_done() -> Outcome {
    // 20,000 lines each across all 50,000 teeth: 2e9 edges weighed
    let region = comb(25_000)?;
    let mut data = String::new();
    for k in 0..20_000 {
        data.push_str(&format!("M-1 {}L60000 {}", 0.0002 * f64::from(k), 0.0002 * f64::from(k)));
    }
    let (name, _) = parameter_refusal(Path::from_svg_path(&data)?.within(&region))?;
    assert_eq!(name, "region");
    Ok(())
}

#[test]
fn a_region_too_big_to_flatten_is_refused_not_taken_for_empty() -> Outcome {
    // 150,000 big curves flatten past MAX_SEGMENTS
    let mut data = String::from("M0 0");
    for _ in 0..150_000 {
        data.push_str("C0 0 1000 0 1000 1000C1000 1000 0 1000 0 0");
    }
    let region = Path::from_svg_path(&data)?;
    let line = Path::polyline([(0.0, 0.0), (10.0, 10.0)], false)?;
    let (name, _) = parameter_refusal(line.within(&region))?;
    assert_eq!(name, "region");
    Ok(())
}

#[test]
fn bands_of_lines_over_a_comb_of_many_edges_are_made_without_a_walk_of_every_edge_per_point() -> Outcome {
    use bullterrier_vector::{Bands, Field};
    let region = comb(10_000)?;
    let drawn = region.bands(&Bands::lines(1.0, Field::constant(0.5)?, (0.2, 0.4))?)?;
    // lines at y = -4.5 .. 9.5 each cut into a piece in every tooth that reaches it: the ten of the teeth, 10,000 each
    assert!(lines(&drawn).len() >= 10_000, "{}", lines(&drawn).len());
    Ok(())
}
