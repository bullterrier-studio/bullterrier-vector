//! A flow along the outline of a stroke does not flip where the stroke's two edges meet, and hatching along it leaves
//! no stub. Oracles: the angle of each edge worked out by hand from the polygon's corners, a brute-force nearest edge
//! computed here, and lengths measured on the output.

use crate::common::{Outcome, lines, to_segment};
use bullterrier_vector::{Field, Flow, Font, Hatch, Path, Text, Values, text};

/// The distance between two directions without sign, in degrees (at most 90).
fn apart(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(180.0);
    d.min(180.0 - d)
}

fn length(points: &[(f64, f64)]) -> f64 {
    points
        .windows(2)
        .map(|w| libm::hypot(w[1].0 - w[0].0, w[1].1 - w[0].1))
        .sum()
}

#[test]
fn the_direction_turns_smoothly_across_the_middle_of_a_tapering_stroke() -> Outcome {
    // left edge straight down, right edge leaning: they differ by 21.8 degrees, and the nearest edge changes
    // where the two are equally far
    let stroke = Path::from_svg_path("M0 0 L80 0 L200 300 L0 300 Z")?;
    let flow = Flow::contour(&stroke)?;
    let leaning = libm::atan2(300.0, 120.0).to_degrees();
    assert!(apart(flow.at((3.0, 150.0)), 90.0) < 1.5, "at the left edge");
    assert!(apart(flow.at((137.0, 150.0)), leaning) < 1.5, "at the right edge");
    let mut worst = 0.0_f64;
    let mut previous = flow.at((5.0, 150.0));
    let mut x = 5.25;
    while x < 135.0 {
        let angle = flow.at((x, 150.0));
        worst = worst.max(apart(angle, previous));
        previous = angle;
        x += 0.25;
    }
    assert!(
        worst < 1.0,
        "the direction jumps {worst} degrees between samples a quarter pixel apart"
    );
    Ok(())
}

/// An L of two strokes 60 wide, as a polygon.
const L: [(f64, f64); 6] = [
    (0.0, 0.0),
    (60.0, 0.0),
    (60.0, 240.0),
    (200.0, 240.0),
    (200.0, 300.0),
    (0.0, 300.0),
];

#[test]
fn lines_run_along_each_stroke() -> Outcome {
    let shape = Path::from_svg_path("M0 0 L60 0 L60 240 L200 240 L200 300 L0 300 Z")?;
    let result = lines(&shape.hatch(&Hatch::new(6.0).flow(Flow::contour(&shape)?), None)?)?;
    let (mut along, mut total, mut longest) = (0.0, 0.0, 0.0_f64);
    for (points, _) in &result {
        longest = longest.max(length(points));
        for pair in points.windows(2) {
            let mid = (f64::midpoint(pair[0].0, pair[1].0), f64::midpoint(pair[0].1, pair[1].1));
            let size = libm::hypot(pair[1].0 - pair[0].0, pair[1].1 - pair[0].1);
            // the brute-force nearest edge and its angle
            let (mut best, mut tangent) = (f64::MAX, 0.0);
            for index in 0..L.len() {
                let (a, b) = (L[index], L[(index + 1) % L.len()]);
                let d = to_segment(mid, a, b);
                if d < best {
                    best = d;
                    tangent = libm::atan2(b.1 - a.1, b.0 - a.0).to_degrees();
                }
            }
            let corner = L
                .iter()
                .map(|c| libm::hypot(mid.0 - c.0, mid.1 - c.1))
                .fold(f64::MAX, f64::min);
            // away from the corners, where a direction has to turn
            if best < 25.0 && corner > 30.0 {
                total += size;
                if apart(
                    libm::atan2(pair[1].1 - pair[0].1, pair[1].0 - pair[0].0).to_degrees(),
                    tangent,
                ) < 15.0
                {
                    along += size;
                }
            }
        }
    }
    assert!(
        total > 500.0 && along / total > 0.97,
        "{along} of {total} runs along its edge"
    );
    assert!(
        longest > 230.0,
        "no line runs the length of the stem: longest {longest}"
    );
    Ok(())
}

fn letters() -> Result<Vec<Path>, bullterrier_vector::Error> {
    let font = Font::bundled("bricolage_grotesque")?;
    let word = text(
        "FLOW",
        Text::new(&font)
            .size(330.0)
            .anchor((540.0, 420.0))
            .axes([("opsz", 96.0), ("wdth", 75.0), ("wght", 800.0)]),
    )
    .resolve(0.01);
    Ok(word.shapes(&Values::new())?.iter().map(|s| s.path().clone()).collect())
}

#[test]
fn a_headline_hatched_along_its_letters_has_no_stub_a_line_under_three_spacings_ends_at_the_outline_both_ends()
-> Outcome {
    let letters = letters()?;
    let whole = Path::from_svg_path(&letters.iter().map(Path::to_svg_path).collect::<Vec<_>>().join(" "))?;
    let flow = Flow::contour(&whole)?;
    let edge = Field::distance(&whole, false)?;
    for (near, far) in [(6.0, 6.0), (3.0, 9.0)] {
        let hatch = Hatch::between(near, far, Field::distance(&whole, true)?).flow(flow.clone());
        let (mut found, mut chords) = (0, 0);
        for letter in &letters {
            for (points, closed) in lines(&letter.hatch(&hatch, None)?)? {
                found += 1;
                if closed || length(&points) >= 3.0 * near {
                    continue;
                }
                // a short line is a chord of a corner: it leaves the letter at both ends, within one step of its edge
                chords += 1;
                for end in [points[0], points[points.len() - 1]] {
                    assert!(
                        edge.at(end).abs() < 0.3 * far,
                        "a line {} long at spacing {near}..{far} ends {} from the outline",
                        length(&points),
                        edge.at(end)
                    );
                }
                assert!(length(&points) >= near, "a fragment {} long", length(&points));
            }
        }
        assert!(found > 40, "{found} lines");
        assert!(chords * 6 <= found, "{chords} of {found} lines are short chords");
    }
    Ok(())
}
