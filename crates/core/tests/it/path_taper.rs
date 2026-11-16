//! `Path::tapered_outline` through the public API: the region a stroke whose width follows a profile along its length
//! paints. The oracle is written here, independent of the library: a point is inside iff some point `c(s)` of a densely
//! sampled centre line has `|p - c(s)| <= r(s)`, with `r(s)` the profile's factor at `s / length` times half the width.
//! Areas are computed by hand from the definition (a kite and a sector).

use crate::common::{Outcome, net_area, parameter_refusal, rings};
use bullterrier_vector::{Error, Font, Path, Text, Values, text};

const TOLERANCE: f64 = 0.01;
/// A taper: 0 to 1 over the first 15 per cent, 1 to 0 over the last 15.
const TAPER: [(f64, f64); 4] = [(0.0, 0.0), (0.15, 1.0), (0.85, 1.0), (1.0, 0.0)];

fn area(path: &Path) -> Result<f64, Box<dyn std::error::Error>> {
    Ok(net_area(&rings(path)?))
}

/// The factor of `profile` at `position`, written from the definition: linear between stops, the end values beyond.
fn factor(profile: &[(f64, f64)], position: f64) -> f64 {
    let Some(last) = profile.iter().rposition(|&(at, _)| at <= position) else {
        return profile[0].1;
    };
    match profile.get(last + 1) {
        None => profile[last].1,
        Some(&(next_at, next)) => {
            let (at, here) = profile[last];
            here + (next - here) * (position - at) / (next_at - at)
        }
    }
}

/// The distance between two points.
fn gap(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0) * (a.0 - b.0) + (a.1 - b.1) * (a.1 - b.1)).sqrt()
}

/// A centre line as a polyline of many points (a cubic is sampled by de Casteljau) with the radius at each point.
struct Sweep {
    discs: Vec<(f64, f64, f64)>,
}

impl Sweep {
    /// The discs along `points` (closed with a line back to the first if `closed`), at most 0.02 apart.
    fn along(points: &[(f64, f64)], closed: bool, width: f64, profile: &[(f64, f64)]) -> Self {
        let mut corners = points.to_vec();
        if closed {
            corners.push(points[0]);
        }
        let lengths: Vec<f64> = corners.windows(2).map(|pair| gap(pair[0], pair[1])).collect();
        let total: f64 = lengths.iter().sum();
        let mut discs = Vec::new();
        let mut before = 0.0;
        for (pair, length) in corners.windows(2).zip(&lengths) {
            #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "a small count")]
            let steps = (length / 0.02).ceil().max(1.0) as usize;
            for step in 0..=steps {
                #[expect(clippy::cast_precision_loss, reason = "a small count")]
                let share = step as f64 / steps as f64;
                let at = (before + share * length) / total;
                discs.push((
                    pair[0].0 + share * (pair[1].0 - pair[0].0),
                    pair[0].1 + share * (pair[1].1 - pair[0].1),
                    width / 2.0 * factor(profile, at),
                ));
            }
            before += length;
        }
        Self { discs }
    }

    /// How far `point` is inside the sweep (negative: outside).
    fn depth(&self, point: (f64, f64)) -> f64 {
        self.discs
            .iter()
            .map(|&(x, y, r)| r - gap(point, (x, y)))
            .fold(f64::MIN, f64::max)
    }

    /// Every point of a grid over `[x0, x1] x [y0, y1]` that is clearly inside or outside the sweep agrees with `path`:
    /// the oracle is exact to the sampling (0.01) and the library to its tolerance (0.01), so points within 0.1 of
    /// the boundary are left out. Returns how many points were compared.
    fn agrees(&self, path: &Path, (x0, x1): (f64, f64), (y0, y1): (f64, f64), step: f64) -> Result<usize, Error> {
        let mut compared = 0;
        let mut x = x0;
        while x <= x1 {
            let mut y = y0;
            while y <= y1 {
                let depth = self.depth((x, y));
                if depth.abs() > 0.1 {
                    assert_eq!(path.contains((x, y))?, depth > 0.0, "point ({x}, {y}), depth {depth}");
                    compared += 1;
                }
                y += step;
            }
            x += step;
        }
        Ok(compared)
    }
}

/// The points of the cubic `p0 p1 p2 p3`, 4000 of them.
fn cubic(corners: [(f64, f64); 4]) -> Vec<(f64, f64)> {
    let mix = |from: (f64, f64), to: (f64, f64), t: f64| (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t);
    (0..=4000)
        .map(|i| {
            let t = f64::from(i) / 4000.0;
            let (ab, bc, cd) = (
                mix(corners[0], corners[1], t),
                mix(corners[1], corners[2], t),
                mix(corners[2], corners[3], t),
            );
            mix(mix(ab, bc, t), mix(bc, cd, t), t)
        })
        .collect()
}

fn line(length: f64) -> Result<Path, Error> {
    Path::line((0.0, 0.0), (length, 0.0))
}

// Examples: what the position domain is -------------------------------------------------------------------------------

#[test]
fn example_one_a_brush_stroke_on_an_open_wavy_path_thins_to_points_at_both_ends() -> Outcome {
    let wave = Path::from_svg_path("M0 0 C30 40 70 -40 100 0")?;
    let outline = wave.tapered_outline(12.0, &TAPER, TOLERANCE)?;
    let sweep = Sweep::along(
        &cubic([(0.0, 0.0), (30.0, 40.0), (70.0, -40.0), (100.0, 0.0)]),
        false,
        12.0,
        &TAPER,
    );
    assert!(sweep.agrees(&outline, (-10.0, 110.0), (-30.0, 30.0), 0.7)? > 2000);
    // closed at the tips: nothing sticks out past the ends of the path
    let bounds = outline.bounds()?;
    assert!(bounds.x > -0.5 && bounds.x + bounds.width < 100.5, "{bounds:?}");
    Ok(())
}

#[test]
fn example_two_each_dash_of_a_dashed_line_tapers_at_its_own_ends() -> Outcome {
    // dashes at 0..30, 40..70 and 80..100, each measured from its own start
    let dashed = line(100.0)?.dash(&[30.0, 10.0], 0.0, None)?;
    let outline = dashed.tapered_outline(10.0, &[(0.0, 0.0), (0.5, 1.0), (1.0, 0.0)], TOLERANCE)?;
    // the second dash is 30 long: 0.5 along it (x = 55) it is full width, 0.5 pixel after its start it is a hair
    assert!(outline.contains((55.0, 4.0))?);
    assert!(
        !outline.contains((40.5, 2.0))?,
        "a taper over the whole line would be 4 wide here"
    );
    assert!(!outline.contains((69.5, 2.0))?);
    // the last, shorter dash peaks at its own middle (x = 90), not at the middle of the line
    assert!(outline.contains((90.0, 4.0))?);
    assert!(!outline.contains((80.5, 2.0))?);
    Ok(())
}

#[test]
fn example_three_a_closed_ring_tapers_from_its_written_start_round_to_the_start_again() -> Outcome {
    // a 40 square written from its top left corner; the profile peaks half way round, at the opposite corner
    let square = [(0.0, 0.0), (40.0, 0.0), (40.0, 40.0), (0.0, 40.0)];
    let profile = [(0.0, 0.0), (0.5, 1.0), (1.0, 0.0)];
    let ring = Path::polyline(square, true)?.tapered_outline(6.0, &profile, TOLERANCE)?;
    // the seam: both ends of the profile meet at the written start, where the ring pinches to a point; opposite it, it
    // is full width, and the whole ring agrees with the oracle
    assert!(!ring.contains((-0.7, -0.7))?);
    assert!(ring.contains((40.0 + 2.0, 40.0 + 2.0))?);
    let sweep = Sweep::along(&square, true, 6.0, &profile);
    assert!(sweep.agrees(&ring, (-6.0, 46.0), (-6.0, 46.0), 0.5)? > 5000);
    Ok(())
}

#[test]
fn example_three_text_o_tapers_each_contour_at_its_own_start() -> Outcome {
    let font = Font::bundled(Font::bundled_names().first().ok_or("no bundled font")?)?;
    let letter = text("O", Text::new(&font).size(100.0)).shapes(&Values::new())?;
    let o = letter.first().ok_or("no glyph")?.path();
    let contours = o.split().len();
    assert!(contours >= 1);
    let tapered = o.tapered_outline(8.0, &TAPER, TOLERANCE)?;
    let plain = o.outline(
        8.0,
        bullterrier_vector::Join::Round,
        bullterrier_vector::Cap::Round,
        4.0,
        TOLERANCE,
    )?;
    // a taper paints less than the full width, and more than nothing
    let (thin, full) = (area(&tapered)?, area(&plain)?);
    assert!(thin > 0.5 * full && thin < 0.98 * full, "{thin} against {full}");
    Ok(())
}

// By hand ------------------------------------------------------------------------------------------------------------

#[test]
fn a_line_that_thins_to_a_point_paints_a_kite_and_the_sector_behind_it() -> Outcome {
    // a point at one end, a disc of radius r = 3 at the other, d = 5 apart (a 3-4-5 triangle): the tangent is t = 4
    // long, the kite is r t = 12, and the disc beyond it is a sector of angle pi + 2 asin(3/5)
    let outline = line(5.0)?.tapered_outline(6.0, &[(0.0, 0.0), (1.0, 1.0)], 0.001)?;
    let wanted = 12.0 + 0.5 * 9.0 * (std::f64::consts::PI + 2.0 * 0.643_501_108_793_284_4);
    let found = area(&outline)?;
    assert!((found - wanted).abs() < 0.03, "{found} is not {wanted}");
    // closed at the tip: the point at the start is the left-most of the outline
    assert!(outline.bounds()?.x > -1e-6);
    Ok(())
}

#[test]
fn a_profile_of_one_everywhere_paints_a_plain_round_capped_stroke() -> Outcome {
    let found = area(&line(100.0)?.tapered_outline(10.0, &[(0.0, 1.0), (1.0, 1.0)], TOLERANCE)?)?;
    let wanted = 100.0 * 10.0 + std::f64::consts::PI * 25.0;
    // the corners are on the circles, so the polygon is inside by at most tolerance x the length of the arcs (31)
    assert!((found - wanted).abs() < 0.35, "{found} is not {wanted}");
    // and a single stop is the same
    let single = area(&line(100.0)?.tapered_outline(10.0, &[(0.4, 1.0)], TOLERANCE)?)?;
    assert!((single - found).abs() < 1e-6, "{single} against {found}");
    Ok(())
}

#[test]
fn a_profile_of_one_everywhere_matches_the_plain_outline_of_a_curve() -> Outcome {
    let wave = Path::from_svg_path("M0 0 C30 40 70 -40 100 0")?;
    let plain = area(&wave.outline(
        8.0,
        bullterrier_vector::Join::Round,
        bullterrier_vector::Cap::Round,
        4.0,
        TOLERANCE,
    )?)?;
    let found = area(&wave.tapered_outline(8.0, &[(0.0, 1.0), (1.0, 1.0)], TOLERANCE)?)?;
    assert!((found - plain).abs() < 0.002 * plain, "{found} against {plain}");
    Ok(())
}

#[test]
fn the_factors_scale_the_width_and_the_end_values_hold_outside_the_listed_range() -> Outcome {
    // 0.5 times a width of 10 from x = 0 to 40, held past the first and last stop: a stadium of radius 2.5
    let held = line(100.0)?.tapered_outline(10.0, &[(0.4, 0.5), (0.6, 0.5)], TOLERANCE)?;
    let found = area(&held)?;
    let wanted = 100.0 * 5.0 + std::f64::consts::PI * 2.5 * 2.5;
    assert!((found - wanted).abs() < 0.2, "{found} is not {wanted}");
    Ok(())
}

#[test]
fn a_sloped_profile_holds_its_end_values_outside_the_range_and_does_not_extend_the_slope() -> Outcome {
    // width 10 on a line of 100, factors 0.2 at 25 and 1 at 75: radius 1 up to x = 25, 1 to 5 over 50, then 5 to the end.
    // By hand: three hulls of two discs (stadium r 1 over 25, the slanted hull of discs 1 and 5 a distance 50 apart with
    // tangent t = sqrt(50^2 - 4^2) and half-angle a = asin(4 / 50), stadium r 5 over 25) less the two discs they share.
    let outline = line(100.0)?.tapered_outline(10.0, &[(0.25, 0.2), (0.75, 1.0)], TOLERANCE)?;
    let a = (4.0_f64 / 50.0).asin();
    let t = (50.0_f64 * 50.0 - 16.0).sqrt();
    let pi = std::f64::consts::PI;
    let first = 25.0 * 2.0 + pi;
    let slanted = t * 6.0 + 0.5 * 25.0 * (pi + 2.0 * a) + 0.5 * (pi - 2.0 * a);
    let last = 25.0 * 10.0 + pi * 25.0;
    let wanted = first + slanted + last - pi - 25.0 * pi;
    let found = area(&outline)?;
    assert!((found - wanted).abs() < 0.3, "{found} is not {wanted}");
    Ok(())
}

#[test]
fn a_symmetric_profile_gives_a_mirror_symmetric_outline() -> Outcome {
    let profile = [(0.0, 0.2), (0.3, 1.0), (0.7, 1.0), (1.0, 0.2)];
    let outline = line(100.0)?.tapered_outline(10.0, &profile, TOLERANCE)?;
    let mut x = -6.0;
    while x <= 106.0 {
        let mut y = 0.0;
        while y <= 7.0 {
            let (here, across, along) = (
                outline.contains((x, y))?,
                outline.contains((x, -y))?,
                outline.contains((100.0 - x, y))?,
            );
            // the boundary is a polygon: points within a tolerance of it may differ
            let depth = Sweep::along(&[(0.0, 0.0), (100.0, 0.0)], false, 10.0, &profile).depth((x, y));
            if depth.abs() > 0.1 {
                assert_eq!((here, across), (along, here), "({x}, {y})");
            }
            y += 0.37;
        }
        x += 1.3;
    }
    Ok(())
}

#[test]
fn a_step_in_the_profile_is_a_step_in_the_width() -> Outcome {
    let profile = [(0.0, 1.0), (0.5, 1.0), (0.5, 0.2), (1.0, 0.2)];
    let outline = line(100.0)?.tapered_outline(10.0, &profile, TOLERANCE)?;
    assert!(outline.contains((40.0, 4.0))?);
    assert!(outline.contains((60.0, 0.9))?);
    assert!(!outline.contains((60.0, 1.3))?);
    Ok(())
}

#[test]
fn an_all_zero_profile_leaves_an_empty_path_and_a_lone_point_nothing() -> Outcome {
    assert_eq!(line(10.0)?.tapered_outline(4.0, &[(0.0, 0.0)], TOLERANCE)?, Path::new());
    assert_eq!(Path::new().tapered_outline(4.0, &[(0.0, 1.0)], TOLERANCE)?, Path::new());
    assert_eq!(
        Path::from_svg_path("M3 4")?.tapered_outline(4.0, &[(0.0, 1.0)], TOLERANCE)?,
        Path::new()
    );
    Ok(())
}

#[test]
fn a_line_to_its_own_start_paints_nothing_as_the_plain_outline_does() -> Outcome {
    let dot = Path::line((5.0, 5.0), (5.0, 5.0))?;
    let plain = dot.outline(
        6.0,
        bullterrier_vector::Join::Round,
        bullterrier_vector::Cap::Round,
        4.0,
        TOLERANCE,
    )?;
    assert_eq!(plain, Path::new());
    assert_eq!(dot.tapered_outline(6.0, &[(0.0, 1.0)], TOLERANCE)?, Path::new());
    Ok(())
}

#[test]
fn positions_are_measured_along_each_sub_path_on_its_own() -> Outcome {
    // two lines of different lengths far apart: each tapers over its own length, so the whole is the two alone
    let both = Path::from_svg_path("M0 0 L100 0 M0 50 L20 50")?;
    let found = area(&both.tapered_outline(10.0, &TAPER, TOLERANCE)?)?;
    let alone = area(&line(100.0)?.tapered_outline(10.0, &TAPER, TOLERANCE)?)?
        + area(&Path::line((0.0, 50.0), (20.0, 50.0))?.tapered_outline(10.0, &TAPER, TOLERANCE)?)?;
    assert!((found - alone).abs() < 1e-6, "{found} against {alone}");
    Ok(())
}

// Refusals ------------------------------------------------------------------------------------------------------------

#[test]
fn a_profile_the_rules_refuse_is_a_parameter_error_naming_it() -> Outcome {
    let nan = f64::NAN;
    let many = vec![(0.5, 1.0); 1001];
    let bad: [(&str, &[(f64, f64)]); 9] = [
        ("empty", &[]),
        ("position below 0", &[(-0.1, 1.0)]),
        ("position above 1", &[(0.0, 1.0), (1.1, 1.0)]),
        ("position NaN", &[(nan, 1.0)]),
        ("positions going back", &[(0.5, 1.0), (0.4, 1.0)]),
        ("factor below 0", &[(0.0, -0.1)]),
        ("factor NaN", &[(0.0, nan)]),
        ("factor infinite", &[(0.0, f64::INFINITY)]),
        ("more than 1000 stops", &many),
    ];
    for (what, profile) in bad {
        let (parameter, _) = parameter_refusal(line(10.0)?.tapered_outline(4.0, profile, TOLERANCE))?;
        assert_eq!(parameter, "profile", "{what}");
    }
    // 1000 stops are fine
    assert!(line(10.0)?.tapered_outline(4.0, &many[..1000], TOLERANCE).is_ok());
    Ok(())
}

#[test]
fn a_width_or_tolerance_the_rules_refuse_names_itself() -> Outcome {
    let profile = [(0.0, 1.0)];
    for width in [0.0, -1.0, f64::NAN, 1.1e100] {
        let (parameter, _) = parameter_refusal(line(10.0)?.tapered_outline(width, &profile, TOLERANCE))?;
        assert_eq!(parameter, "width", "{width}");
    }
    for tolerance in [0.0, -1.0, f64::NAN, 1.1e100] {
        let (parameter, _) = parameter_refusal(line(10.0)?.tapered_outline(4.0, &profile, tolerance))?;
        assert_eq!(parameter, "tolerance", "{tolerance}");
    }
    Ok(())
}

#[test]
fn a_huge_width_at_a_fine_tolerance_is_refused_before_anything_is_built() -> Outcome {
    // a disc of radius 5e8 within 1e-9 is about 1.4 billion corners
    let (parameter, message) = parameter_refusal(line(10.0)?.tapered_outline(1e9, &[(0.0, 1.0)], 1e-9))?;
    assert_eq!(parameter, "width");
    assert!(message.contains("MAX_SEGMENTS"), "{message}");
    // and a width that makes a coordinate past 1e100
    let (parameter, _) = parameter_refusal(line(10.0)?.tapered_outline(1e100, &[(0.0, 1e100)], 1e98))?;
    assert_eq!(parameter, "width");
    Ok(())
}
