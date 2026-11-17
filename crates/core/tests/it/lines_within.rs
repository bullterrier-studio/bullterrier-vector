//! `Path::within` through the public API. Oracles: a horizontal line through a circle is cut at the chord worked out by
//! hand, and every point of every piece is checked against `Path::contains` of the region.

use crate::common::Outcome;
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
