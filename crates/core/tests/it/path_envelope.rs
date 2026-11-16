//! `Path::envelope` and `Path::arch` through the public API. The oracle is the map written out here from the
//! definition: a point `(x, y)` of the frame goes to `lerp(T(u), B(u), v)` with `u = (x - x0) / width`, `v = (y - y0) /
//! height`, `T` and `B` the points `u` of the way along the arc length of the top and the bottom curve (read with the
//! public `point_at`) blended with the frame's straight edge by `amount`; the output polyline is checked against the
//! exact image of a dense sampling of the original, by brute force. Expected vertices are computed by hand.

use crate::common::{Outcome, geometry_refusal, has_vertex, lines, parameter_refusal, to_lines};
use bullterrier_vector::{Path, Rect};

const FRAME: Rect = Rect {
    x: 0.0,
    y: 0.0,
    width: 100.0,
    height: 100.0,
};

/// The point `along` of the way along `path` by arc length; outside 0 to 1 the path goes on straight in the direction it
/// ends or begins in (the direction read over a thousandth of a pixel).
fn at(path: &Path, along: f64) -> Result<(f64, f64), Box<dyn std::error::Error>> {
    let point = |share: f64| path.point_at(share, None).map(|point| (point.x, point.y));
    if (0.0..=1.0).contains(&along) {
        return Ok(point(along)?);
    }
    let (edge, inside) = if along < 0.0 { (0.0, 1e-5) } else { (1.0, 1.0 - 1e-5) };
    let (end, near) = (point(edge)?, point(inside)?);
    let length = path.length();
    let gap = libm::hypot(end.0 - near.0, end.1 - near.1);
    let beyond = (along - edge).abs() * length / gap;
    Ok((end.0 + (end.0 - near.0) * beyond, end.1 + (end.1 - near.1) * beyond))
}

/// Where the definition sends `(x, y)` of `frame`.
fn exact(
    point: (f64, f64),
    frame: Rect,
    top: &Path,
    bottom: Option<&Path>,
    amount: f64,
) -> Result<(f64, f64), Box<dyn std::error::Error>> {
    let (u, v) = ((point.0 - frame.x) / frame.width, (point.1 - frame.y) / frame.height);
    let mix = |straight: (f64, f64), curve: (f64, f64)| {
        (
            straight.0 + (curve.0 - straight.0) * amount,
            straight.1 + (curve.1 - straight.1) * amount,
        )
    };
    let t = mix((frame.x + u * frame.width, frame.y), at(top, u)?);
    let straight_bottom = (frame.x + u * frame.width, frame.y + frame.height);
    let b = match bottom {
        Some(bottom) => mix(straight_bottom, at(bottom, u)?),
        None => straight_bottom,
    };
    Ok((t.0 + (b.0 - t.0) * v, t.1 + (b.1 - t.1) * v))
}

/// Every exact image of a dense sampling of `path` is near the enveloped outline: 0.05 pixel and 0.2% of how far the
/// map moved the point.
fn assert_matches_the_definition(path: &Path, frame: Rect, top: &Path, bottom: Option<&Path>, amount: f64) -> Outcome {
    assert_within(path, frame, top, bottom, amount, (0.05, 0.002))
}

/// As [`assert_matches_the_definition`] with the tolerance `(pixels, share of the displacement)` given.
fn assert_within(
    path: &Path,
    frame: Rect,
    top: &Path,
    bottom: Option<&Path>,
    amount: f64,
    (pixels, share): (f64, f64),
) -> Outcome {
    let found = lines(&path.envelope(top, bottom, Some(frame), amount)?)?;
    for index in 0..=400 {
        let original = at(path, f64::from(index) / 400.0)?;
        let image = exact(original, frame, top, bottom, amount)?;
        let moved = libm::hypot(image.0 - original.0, image.1 - original.1);
        let off = to_lines(image, &found);
        assert!(
            off <= pixels + share * moved,
            "sample {index}: the exact image {image:?} of {original:?} is {off} from the output"
        );
    }
    Ok(())
}

fn hill() -> Result<Path, bullterrier_vector::Error> {
    Path::from_svg_path("M0 60 C30 -40 70 -40 100 60")
}

fn glyph() -> Result<Path, bullterrier_vector::Error> {
    Path::from_svg_path("M10 90 L10 10 L55 10 Q85 10 85 35 Q85 60 55 60 L30 60 L30 90 Z")
}

// By hand --------------------------------------------------------------------------------------------------------------

#[test]
fn a_vertical_line_goes_between_the_curves_by_hand() -> Outcome {
    // the top runs (0, 0) to (100, 20), the bottom is the frame's own: at x = 50, u = 0.5, T = (50, 10), B = (50, 100)
    let top = Path::line((0.0, 0.0), (100.0, 20.0))?;
    let line = Path::line((50.0, 0.0), (50.0, 100.0))?;
    let found = lines(&line.envelope(&top, None, Some(FRAME), 1.0)?)?;
    assert!(has_vertex(&found, (50.0, 10.0)), "{found:?}");
    assert!(has_vertex(&found, (50.0, 100.0)), "{found:?}");
    // v = 0.5 is the middle of the two
    assert!(to_lines((50.0, 55.0), &found) < 1e-9, "{found:?}");
    Ok(())
}

#[test]
fn the_corners_of_the_frame_go_to_the_ends_of_the_curves() -> Outcome {
    let top = Path::from_svg_path("M0 0 L40 0 L40 30")?;
    let bottom = Path::line((0.0, 100.0), (100.0, 100.0))?;
    let frame = Path::rect(0.0, 0.0, 100.0, 100.0, 0.0)?;
    let found = lines(&frame.envelope(&top, Some(&bottom), Some(FRAME), 1.0)?)?;
    assert!(found.iter().all(|line| line.1), "closed stays closed");
    for corner in [(0.0, 0.0), (40.0, 30.0), (100.0, 100.0), (0.0, 100.0)] {
        assert!(has_vertex(&found, corner), "no vertex at {corner:?}: {found:?}");
    }
    // half way along the top, 35 of its 70 long: the middle of the first edge is not it, the point 35 along is (35, 0)
    assert!(to_lines((35.0, 0.0), &found) < 1e-6);
    Ok(())
}

#[test]
fn past_the_ends_of_an_open_curve_the_envelope_goes_on_straight() -> Outcome {
    // a frame twice as wide as the curve's fraction reaches: x = 150 is u = 1.5, half a curve length past the end
    let bent_up = Path::line((0.0, 0.0), (100.0, -100.0))?;
    let wide = Rect { width: 100.0, ..FRAME };
    let probe = Path::line((150.0, 0.0), (150.0, 0.0))?;
    let found = lines(&probe.envelope(&bent_up, None, Some(wide), 1.0)?)?;
    // T(1.5) is 1.5 lengths along the slanted line: (150, -150); the bottom is straight at y = 100
    assert!(has_vertex(&found, (150.0, -150.0)), "{found:?}");
    Ok(())
}

#[test]
fn an_amount_blends_each_edge_between_the_straight_one_and_the_curve() -> Outcome {
    let top = Path::line((0.0, 0.0), (100.0, 20.0))?;
    let line = Path::line((50.0, 0.0), (50.0, 100.0))?;
    // the top at u = 0.5 is (50, 10) at amount 1, (50, 0) at amount 0, (50, 5) at amount 0.5, (50, -10) at amount -1
    for (amount, y) in [(1.0, 10.0), (0.0, 0.0), (0.5, 5.0), (-1.0, -10.0), (2.0, 20.0)] {
        let found = lines(&line.envelope(&top, None, Some(FRAME), amount)?)?;
        assert!(has_vertex(&found, (50.0, y)), "amount {amount}: {found:?}");
    }
    Ok(())
}

#[test]
fn the_bottom_curve_is_blended_too() -> Outcome {
    let bottom = Path::line((0.0, 100.0), (100.0, 140.0))?;
    let top = Path::line((0.0, 0.0), (100.0, 0.0))?;
    let line = Path::line((50.0, 0.0), (50.0, 100.0))?;
    // at u = 0.5 the bottom is (50, 120); half way to it from the straight (50, 100) is (50, 110)
    for (amount, y) in [(1.0, 120.0), (0.5, 110.0), (0.0, 100.0)] {
        let found = lines(&line.envelope(&top, Some(&bottom), Some(FRAME), amount)?)?;
        assert!(has_vertex(&found, (50.0, y)), "amount {amount}: {found:?}");
    }
    Ok(())
}

#[test]
fn a_closed_curve_wraps_with_its_seam_at_its_start() -> Outcome {
    // a circle starts at its top: u = 0 and u = 1 are both its top, u = 0.5 is its bottom
    let ring = Path::circle((50.0, 50.0), 40.0)?;
    let probe = Path::line((50.0, 0.0), (50.0, 0.0))?;
    let found = lines(&probe.envelope(&ring, None, Some(FRAME), 1.0)?)?;
    assert!(has_vertex(&found, (50.0, 90.0)), "u = 0.5 is the bottom: {found:?}");
    let probe = Path::line((0.0, 0.0), (0.0, 0.0))?;
    let found = lines(&probe.envelope(&ring, None, Some(FRAME), 1.0)?)?;
    assert!(to_lines((50.0, 10.0), &found) < 1e-6, "u = 0 is the seam: {found:?}");
    let probe = Path::line((100.0, 0.0), (100.0, 0.0))?;
    let found = lines(&probe.envelope(&ring, None, Some(FRAME), 1.0)?)?;
    assert!(
        to_lines((50.0, 10.0), &found) < 1e-6,
        "u = 1 wraps to the seam: {found:?}"
    );
    Ok(())
}

// Identities ---------------------------------------------------------------------------------------------------------

#[test]
fn the_frames_own_straight_edges_are_the_identity_at_any_amount() -> Outcome {
    let top = Path::line((0.0, 0.0), (100.0, 0.0))?;
    let bottom = Path::line((0.0, 100.0), (100.0, 100.0))?;
    let shape = glyph()?;
    for amount in [1.0, 0.0, 0.3, -2.0] {
        let found = lines(&shape.envelope(&top, Some(&bottom), Some(FRAME), amount)?)?;
        for index in 0..=200 {
            let original = at(&shape, f64::from(index) / 200.0)?;
            assert!(
                to_lines(original, &found) < 0.02,
                "amount {amount}: {original:?} is gone"
            );
        }
    }
    Ok(())
}

#[test]
fn an_amount_of_zero_leaves_the_shape_where_it_is_whatever_the_curves() -> Outcome {
    let shape = glyph()?;
    let found = lines(&shape.envelope(&hill()?, Some(&hill()?.translate(0.0, 100.0)?), Some(FRAME), 0.0)?)?;
    for index in 0..=200 {
        let original = at(&shape, f64::from(index) / 200.0)?;
        assert!(to_lines(original, &found) < 0.02, "{original:?} moved");
    }
    Ok(())
}

#[test]
fn points_on_the_top_edge_of_the_frame_lie_on_the_top_curve() -> Outcome {
    let top = hill()?;
    let edge = Path::line((0.0, 0.0), (100.0, 0.0))?;
    let found = lines(&edge.envelope(&top, None, Some(FRAME), 1.0)?)?;
    for index in 0..=400 {
        let on_top = at(&top, f64::from(index) / 400.0)?;
        assert!(to_lines(on_top, &found) <= 0.01, "{index}");
    }
    // and every vertex of the output is on the top curve
    let dense = lines(&top.resample(0.01)?)?;
    for vertex in &found[0].0 {
        assert!(to_lines(*vertex, &dense) <= 0.01, "{vertex:?}");
    }
    Ok(())
}

#[test]
fn a_bottom_the_same_arc_down_by_the_frame_height_keeps_vertical_strokes_their_length() -> Outcome {
    // the same arc, one frame height lower: every column keeps its height and stays vertical
    let top = Path::arc((50.0, 150.0), 90.0, 200.0, 340.0)?;
    let bottom = top.translate(0.0, 40.0)?;
    let frame = Rect {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 40.0,
    };
    let stroke = Path::line((37.0, 10.0), (37.0, 30.0))?;
    let found = lines(&stroke.envelope(&top, Some(&bottom), Some(frame), 1.0)?)?;
    let points = &found[0].0;
    let length: f64 = points
        .windows(2)
        .map(|pair| libm::hypot(pair[1].0 - pair[0].0, pair[1].1 - pair[0].1))
        .sum();
    assert!((length - 20.0).abs() < 1e-6, "length {length}");
    assert!(
        points.iter().all(|point| (point.0 - points[0].0).abs() < 1e-6),
        "the column bent: {points:?}"
    );
    Ok(())
}

// The definition ------------------------------------------------------------------------------------------------------

#[test]
fn a_glyph_and_a_circle_match_the_definition_between_two_curves() -> Outcome {
    let s_curve = Path::from_svg_path("M0 0 C40 -50 60 50 100 0")?;
    let bottom = Path::from_svg_path("M0 100 Q50 160 100 100")?;
    for amount in [1.0, 0.6, -0.5] {
        assert_matches_the_definition(&glyph()?, FRAME, &s_curve, Some(&bottom), amount)?;
        assert_matches_the_definition(&Path::circle((50.0, 50.0), 50.0)?, FRAME, &s_curve, None, amount)?;
    }
    Ok(())
}

#[test]
fn a_long_diagonal_edge_and_a_tight_arc_match_the_definition() -> Outcome {
    // a diagonal across a wide frame is the hardest edge for the cut: its image is bilinear in the frame
    let wide = Rect {
        x: 0.0,
        y: 0.0,
        width: 1000.0,
        height: 100.0,
    };
    let diagonal = Path::line((0.0, 0.0), (1000.0, 100.0))?;
    let ring = Path::arc((500.0, 400.0), 380.0, 190.0, 350.0)?;
    assert_matches_the_definition(&diagonal, wide, &ring, None, 1.0)?;
    // a line has no flattening of its own, so the cut alone is measured, to a tenth of the pixel the rest allows
    assert_within(&diagonal, wide, &ring, None, 1.0, (0.01, 0.0))?;
    assert_matches_the_definition(&diagonal, wide, &ring, Some(&ring.translate(0.0, 100.0)?), 1.0)?;
    Ok(())
}

#[test]
fn a_frame_that_is_not_the_bounds_still_follows_the_definition() -> Outcome {
    // the shape reaches outside the frame: u and v leave 0..1 and the map goes on (curves straight past their ends)
    let frame = Rect {
        x: 20.0,
        y: 20.0,
        width: 60.0,
        height: 60.0,
    };
    assert_matches_the_definition(&glyph()?, frame, &hill()?, None, 1.0)
}

// The default frame --------------------------------------------------------------------------------------------------

#[test]
fn the_default_frame_is_the_bounds_of_the_path() -> Outcome {
    let shape = glyph()?;
    let bounds = shape.bounds()?;
    let top = hill()?;
    assert_eq!(
        shape.envelope(&top, None, None, 1.0)?,
        shape.envelope(&top, None, Some(bounds), 1.0)?
    );
    assert_eq!(shape.arch(0.2, None)?, shape.arch(0.2, Some(bounds))?);
    Ok(())
}

#[test]
fn swapping_top_and_bottom_mirrors_the_result() -> Outcome {
    // v = 0 lies on `top`: a `bottom` curve above `top` turns the frame over, each point of the swapped call being the
    // original's with v replaced by 1 - v. Top (0,0)-(100,20), bottom (0,100)-(100,140), column x = 50: T = (50, 10), B = (50, 120)
    let top = Path::line((0.0, 0.0), (100.0, 20.0))?;
    let bottom = Path::line((0.0, 100.0), (100.0, 140.0))?;
    let column = Path::line((50.0, 0.0), (50.0, 100.0))?;
    let upright = lines(&column.envelope(&top, Some(&bottom), Some(FRAME), 1.0)?)?;
    let flipped = lines(&column.envelope(&bottom, Some(&top), Some(FRAME), 1.0)?)?;
    // heights 0, 25, 75, 100 of the column go to 10 + 110 v for the upright call and to 120 - 110 v for the flipped one
    for (height, y) in [(0.0, 10.0), (25.0, 37.5), (75.0, 92.5), (100.0, 120.0)] {
        assert!(has_vertex(&upright, (50.0, y)), "upright {height}: {upright:?}");
        assert!(has_vertex(&flipped, (50.0, 130.0 - y)), "flipped {height}: {flipped:?}");
    }
    Ok(())
}

// Arch -----------------------------------------------------------------------------------------------------------------

#[test]
fn an_arch_lifts_the_middle_of_the_band_by_the_bend_times_the_width() -> Outcome {
    // a bend of 0.2 on a frame 100 wide: the middle of the top and of the bottom edge rise 20 and the corners stay
    let rect = Path::rect(0.0, 0.0, 100.0, 100.0, 0.0)?;
    let found = lines(&rect.arch(0.2, Some(FRAME))?)?;
    for vertex in [(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0), (50.0, -20.0), (50.0, 80.0)] {
        assert!(has_vertex(&found, vertex), "no vertex at {vertex:?}");
    }
    assert!(found.iter().all(|line| line.1));
    // a quarter of the way along, 4 * 0.2 * 100 * 0.25 * 0.75 = 15 up, both edges
    assert!(has_vertex(&found, (25.0, -15.0)), "{found:?}");
    assert!(has_vertex(&found, (25.0, 85.0)), "{found:?}");
    // a negative bend sags the same
    let found = lines(&rect.arch(-0.2, Some(FRAME))?)?;
    assert!(has_vertex(&found, (50.0, 20.0)), "{found:?}");
    assert!(has_vertex(&found, (50.0, 120.0)), "{found:?}");
    Ok(())
}

#[test]
fn an_arch_keeps_the_height_of_the_frame_and_a_negative_bend_mirrors_a_positive_one() -> Outcome {
    // a word's frame, 365 by 150: a bend of 0.3 moves the middle 0.3 * 365 = 109.5, and +0.3 and -0.3 keep its 150 height
    let frame = Rect {
        x: 0.0,
        y: 0.0,
        width: 365.0,
        height: 150.0,
    };
    let rect = Path::rect(0.0, 0.0, 365.0, 150.0, 0.0)?;
    let (up, down) = (lines(&rect.arch(0.3, Some(frame))?)?, lines(&rect.arch(-0.3, Some(frame))?)?);
    for vertex in [(182.5, -109.5), (182.5, 40.5)] {
        assert!(has_vertex(&up, vertex), "no vertex at {vertex:?}: {up:?}");
    }
    for vertex in [(182.5, 109.5), (182.5, 259.5)] {
        assert!(has_vertex(&down, vertex), "no vertex at {vertex:?}: {down:?}");
    }
    Ok(())
}

#[test]
fn an_arch_follows_the_formula_at_every_height() -> Outcome {
    // x stays; y = y - 4 bend W u (1 - u), the same lift at every height
    let shape = glyph()?;
    let bend = 0.35;
    let found = lines(&shape.arch(bend, Some(FRAME))?)?;
    for index in 0..=400 {
        let (x, y) = at(&shape, f64::from(index) / 400.0)?;
        let u = x / 100.0;
        let image = (x, y - 4.0 * bend * 100.0 * u * (1.0 - u));
        assert!(to_lines(image, &found) <= 0.05, "sample {index}: {image:?}");
    }
    Ok(())
}

#[test]
fn a_bend_of_zero_is_the_identity() -> Outcome {
    let shape = glyph()?;
    let found = lines(&shape.arch(0.0, None)?)?;
    for index in 0..=200 {
        let original = at(&shape, f64::from(index) / 200.0)?;
        assert!(to_lines(original, &found) < 0.02, "{original:?}");
    }
    Ok(())
}

// Empty, lone, closed ---------------------------------------------------------------------------------------------------

#[test]
fn an_empty_path_stays_empty_and_a_lone_point_is_moved() -> Outcome {
    assert_eq!(Path::new().envelope(&hill()?, None, Some(FRAME), 1.0)?, Path::new());
    assert_eq!(Path::new().arch(0.2, None)?, Path::new());
    let found = lines(&Path::from_svg_path("M50 0")?.arch(0.2, Some(FRAME))?)?;
    assert!(has_vertex(&found, (50.0, -20.0)), "{found:?}");
    Ok(())
}

#[test]
fn an_open_path_stays_open() -> Outcome {
    let found = lines(&Path::from_svg_path("M0 0 L100 50 L0 100")?.arch(0.2, Some(FRAME))?)?;
    assert!(found.iter().all(|line| !line.1));
    Ok(())
}

// Refusals --------------------------------------------------------------------------------------------------------------

#[test]
fn a_frame_with_no_area_or_beyond_the_limit_is_refused_naming_frame() -> Outcome {
    let shape = glyph()?;
    let top = hill()?;
    let bad = |rect: Rect| parameter_refusal(shape.envelope(&top, None, Some(rect), 1.0)).map(|found| found.0);
    assert_eq!(bad(Rect { width: 0.0, ..FRAME })?, "frame");
    assert_eq!(bad(Rect { height: 0.0, ..FRAME })?, "frame");
    assert_eq!(bad(Rect { width: -5.0, ..FRAME })?, "frame");
    assert_eq!(bad(Rect { x: f64::NAN, ..FRAME })?, "frame");
    assert_eq!(
        bad(Rect {
            height: f64::INFINITY,
            ..FRAME
        })?,
        "frame"
    );
    assert_eq!(
        bad(Rect {
            width: 1.1e100,
            ..FRAME
        })?,
        "frame"
    );
    assert_eq!(
        parameter_refusal(shape.arch(0.2, Some(Rect { width: 0.0, ..FRAME })))?.0,
        "frame"
    );
    // a line has no height of its own to frame by
    let line = Path::line((0.0, 0.0), (100.0, 0.0))?;
    assert_eq!(parameter_refusal(line.arch(0.2, None))?.0, "frame");
    assert_eq!(parameter_refusal(line.envelope(&top, None, None, 1.0))?.0, "frame");
    // an explicit frame is checked even where there is nothing to bend
    assert_eq!(
        parameter_refusal(Path::new().arch(0.2, Some(Rect { width: 0.0, ..FRAME })))?.0,
        "frame"
    );
    Ok(())
}

#[test]
fn numbers_that_are_not_numbers_are_refused_by_name() -> Outcome {
    let shape = glyph()?;
    let top = hill()?;
    for amount in [f64::NAN, f64::INFINITY] {
        assert_eq!(parameter_refusal(shape.envelope(&top, None, None, amount))?.0, "amount");
    }
    for bend in [f64::NAN, f64::NEG_INFINITY] {
        assert_eq!(parameter_refusal(shape.arch(bend, None))?.0, "bend");
    }
    // a point sent beyond 1e100
    assert_eq!(parameter_refusal(shape.arch(1e100, None))?.0, "bend");
    assert_eq!(parameter_refusal(shape.envelope(&top, None, None, 1e308))?.0, "amount");
    Ok(())
}

#[test]
fn a_curve_with_no_length_is_a_geometry_error_like_along() -> Outcome {
    let shape = glyph()?;
    let nothing = Path::from_svg_path("M5 5")?;
    assert_eq!(
        geometry_refusal(shape.envelope(&Path::new(), None, None, 1.0))?.0,
        "envelope"
    );
    assert_eq!(
        geometry_refusal(shape.envelope(&nothing, None, None, 1.0))?.0,
        "envelope"
    );
    assert_eq!(
        geometry_refusal(shape.envelope(&hill()?, Some(&nothing), None, 1.0))?.0,
        "envelope"
    );
    Ok(())
}

#[test]
fn a_cut_into_too_many_pieces_is_refused_before_it_is_made() -> Outcome {
    // 1e9 long in a frame 1 wide: 256 pieces of the width each is 2.6e11 pieces
    let long = Path::line((0.0, 0.0), (1e9, 0.5))?;
    let frame = Rect {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
    };
    let (name, message) = parameter_refusal(long.arch(0.2, Some(frame)))?;
    assert!(name == "frame" && message.contains("MAX_SEGMENTS"), "{name}: {message}");
    let (name, message) = parameter_refusal(long.envelope(&hill()?, None, Some(frame), 1.0))?;
    assert!(name == "frame" && message.contains("MAX_SEGMENTS"), "{name}: {message}");
    Ok(())
}

// An absolute budget ------------------------------------------------------------------------------------------------

#[test]
fn a_wide_arch_and_a_wide_envelope_are_within_a_twentieth_of_a_pixel_of_the_map() -> Outcome {
    let frame = Rect {
        x: 0.0,
        y: 0.0,
        width: 8000.0,
        height: 100.0,
    };
    let top = Path::line((0.0, 0.0), (8000.0, 0.0))?;
    let found = lines(&top.arch(1.0, Some(frame))?)?;
    let worst = (0..=8000)
        .map(|index| {
            let x = f64::from(index);
            let u = x / 8000.0;
            to_lines((x, -4.0 * 8000.0 * u * (1.0 - u)), &found)
        })
        .fold(0.0_f64, f64::max);
    assert!(worst <= 0.05, "the arch is {worst} pixels from the output");
    let curve = Path::from_svg_path("M0 1800 C900 -1200 2100 -1200 3000 1800")?;
    let frame = Rect {
        x: 0.0,
        y: 0.0,
        width: 3000.0,
        height: 100.0,
    };
    let edge = Path::line((0.0, 0.0), (3000.0, 0.0))?;
    let found = lines(&edge.envelope(&curve, None, Some(frame), 1.0)?)?;
    let mut worst = 0.0_f64;
    for index in 0..=3000 {
        let along = f64::from(index) / 3000.0;
        let image = at(&curve, along)?;
        worst = worst.max(to_lines(image, &found));
    }
    // the guide is flattened within 0.01 pixel
    assert!(worst <= 0.06, "the envelope is {worst} pixels from the output");
    Ok(())
}
