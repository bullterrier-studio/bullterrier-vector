//! The booleans on real outlines: a sample of the glyphs of each bundled font (the letters with counters and dots, the
//! accents, the punctuation, a ligature), at the ends of every variable axis. Every glyph is resolved, and every
//! pair of a sample of pairs from different fonts goes through the four verbs. The oracles are written in the tests:
//! the exact area of a set rule over the outline sampled into straight edges, the winding number at random points
//! (a fixed xorshift, so a failure repeats), and a search for crossing edges.
//!
//! The bounds. A glyph is drawn 100 pixels to the em, resolved to 0.01, and the oracle samples a curve at `STEPS`
//! points: a boundary may be 0.01 (flattening) plus `SAMPLING` (the sampling, rounded up from a quadratic's
//! `|P0 - 2 P1 + P2| / (4 STEPS^2)` with the controls at most 50 apart) from where the oracle puts it. So a probe must
//! be farther than `REACH` from every edge, and an area may differ by `REACH` times the perimeter.

use crate::common::region::{canonical_fault, crossing_fault, exact_areas, inside, nearest_edge, sampled};
use crate::common::{Corners, Outcome, net_area, rings};
use bullterrier_vector::{Font, Path, Text, Values, text};

const TOLERANCE: f64 = 0.01;
const STEPS: usize = 32;
const SAMPLING: f64 = 0.0125;
const REACH: f64 = TOLERANCE + SAMPLING;
const EM: f64 = 100.0;
/// Letters with counters (`B`, `8`, `g`, `Q`), dots (`i`, `j`, `!`, `?`), accents (`É`, `Å`, `ø`, `ñ`), marks that
/// overlap their own parts (`&`, `%`, `@`, `$`), and the `fi`, `ffi` ligatures.
const CONTENT: &str = "BiJj!%&8ÉÅøñgQ@$?fiffi";

/// A set rule: whether a point is kept, given whether it is in the first region and in the second.
type Rule = fn(bool, bool) -> bool;

/// One glyph: where it came from, and its outline as the font draws it.
struct Glyph {
    label: String,
    path: Path,
    polys: Vec<Corners>,
}

/// The settings of a font's axes to draw at: the defaults, each axis alone at each end, and all at one end.
fn locations(font: &Font) -> Vec<Vec<(String, f64)>> {
    let axes = font.axes();
    let at = |pick: &dyn Fn(usize) -> f64| {
        axes.iter()
            .enumerate()
            .map(|(i, a)| (a.name().to_owned(), pick(i)))
            .collect()
    };
    let mut found: Vec<Vec<(String, f64)>> = vec![at(&|i| axes[i].default())];
    for (index, axis) in axes.iter().enumerate() {
        for end in [axis.min(), axis.max()] {
            found.push(at(&|i| if i == index { end } else { axes[i].default() }));
        }
    }
    found.push(at(&|i| axes[i].min()));
    found.push(at(&|i| axes[i].max()));
    found
}

fn corpus() -> Result<Vec<Glyph>, Box<dyn std::error::Error>> {
    let mut found = Vec::new();
    for name in Font::bundled_names() {
        let font = Font::bundled(name)?;
        for (number, location) in locations(&font).into_iter().enumerate() {
            let options = Text::new(&font).size(EM).axes(location);
            for (glyph, shape) in text(CONTENT, options).shapes(&Values::new())?.iter().enumerate() {
                let path = shape.path().clone();
                let polys = sampled(&path, STEPS)?;
                if !polys.is_empty() {
                    found.push(Glyph {
                        label: format!("{name} at location {number}, glyph {glyph}"),
                        path,
                        polys,
                    });
                }
            }
        }
    }
    Ok(found)
}

/// A fixed xorshift, so a failing probe is found again.
struct Dice(u64);

impl Dice {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        #[expect(clippy::cast_precision_loss, reason = "the top 53 bits")]
        let share = (self.0 >> 11) as f64 / (1_u64 << 53) as f64;
        share
    }
}

impl Dice {
    /// One of `of`.
    fn pick<'a, T>(&mut self, of: &[&'a T]) -> &'a T {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss,
            reason = "a share of a count"
        )]
        let index = (self.next() * of.len() as f64) as usize;
        of[index.min(of.len() - 1)]
    }
}

fn perimeter(polys: &[Corners]) -> f64 {
    polys
        .iter()
        .map(|ring| {
            ring.iter()
                .enumerate()
                .map(|(i, &p)| crate::common::distance(p, ring[(i + 1) % ring.len()]))
                .sum::<f64>()
        })
        .sum()
}

fn bounds(polys: &[Corners]) -> ((f64, f64), (f64, f64)) {
    let mut low = (f64::INFINITY, f64::INFINITY);
    let mut high = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for &(x, y) in polys.iter().flatten() {
        low = (low.0.min(x), low.1.min(y));
        high = (high.0.max(x), high.1.max(y));
    }
    (low, high)
}

/// The glyph corpus is not empty and has the shapes the test names: a sample that quietly shrank would test nothing.
#[test]
fn the_corpus_has_glyphs_with_counters_and_dots_from_every_bundled_font() -> Outcome {
    let glyphs = corpus()?;
    for name in Font::bundled_names() {
        let count = glyphs.iter().filter(|g| g.label.starts_with(name)).count();
        assert!(count > 100, "{name}: {count} glyphs");
    }
    // `B` has two counters: a resolved outline of it has holes (counter-clockwise rings)
    let b = glyphs
        .iter()
        .find(|g| g.label.ends_with("location 0, glyph 0"))
        .ok_or("no B")?;
    let found = rings(&b.path.resolve(TOLERANCE)?)?;
    assert_eq!(
        found
            .iter()
            .filter(|ring| crate::common::shoelace_area(ring) < 0.0)
            .count(),
        2
    );
    Ok(())
}

#[test]
fn every_glyph_resolves_to_its_own_nonzero_fill_with_no_overlap_left() -> Outcome {
    let mut dice = Dice(0x9E37_79B9_7F4A_7C15);
    for glyph in corpus()? {
        let label = &glyph.label;
        let found = rings(&glyph.path.resolve(TOLERANCE)?)?;
        assert_eq!(canonical_fault(&found), None, "{label}");
        assert_eq!(crossing_fault(&found, 1e-9), None, "{label}");
        let exact = exact_areas(&glyph.polys, &[], &[|a, _| a])[0];
        let bound = REACH * perimeter(&glyph.polys);
        assert!(
            (net_area(&found) - exact).abs() <= bound,
            "{label}: area {}, exact {exact}, bound {bound}",
            net_area(&found)
        );
        let (low, high) = bounds(&glyph.polys);
        let mut checked = 0;
        for _ in 0..300 {
            let point = (
                low.0 - 2.0 + dice.next() * (high.0 - low.0 + 4.0),
                low.1 - 2.0 + dice.next() * (high.1 - low.1 + 4.0),
            );
            if nearest_edge(&glyph.polys, point) > REACH {
                checked += 1;
                assert_eq!(
                    inside(&found, point),
                    inside(&glyph.polys, point),
                    "{label} at {point:?}"
                );
            }
        }
        assert!(checked > 100, "{label}: only {checked} probes were clear of the edges");
    }
    Ok(())
}

#[test]
fn glyphs_of_different_fonts_combine_by_every_verb_as_the_set_rule_says() -> Outcome {
    let glyphs = corpus()?;
    let (from_first, from_second): (Vec<&Glyph>, Vec<&Glyph>) =
        glyphs.iter().partition(|g| g.label.starts_with("bricolage"));
    let mut dice = Dice(0xD1B5_4A32_D192_ED03);
    let verbs: [(&str, Rule); 4] = [
        ("union", |a, b| a || b),
        ("subtract", |a, b| a && !b),
        ("intersect", |a, b| a && b),
        ("exclude", |a, b| a != b),
    ];
    let mut overlapping = 0;
    for _ in 0..24 {
        let (a, b) = (dice.pick(&from_first), dice.pick(&from_second));
        // the second moved so that its middle lands within 25 of the first's middle
        let (a_box, b_box) = (bounds(&a.polys), bounds(&b.polys));
        let dx = f64::midpoint(a_box.0.0, a_box.1.0) - f64::midpoint(b_box.0.0, b_box.1.0) + dice.next() * 50.0 - 25.0;
        let dy = f64::midpoint(a_box.0.1, a_box.1.1) - f64::midpoint(b_box.0.1, b_box.1.1) + dice.next() * 50.0 - 25.0;
        let b_path = b.path.translate(dx, dy)?;
        let b_polys = sampled(&b_path, STEPS)?;
        let rules: Vec<_> = verbs.iter().map(|v| v.1).collect();
        let exact = exact_areas(&a.polys, &b_polys, &rules);
        let bound = REACH * (perimeter(&a.polys) + perimeter(&b_polys));
        overlapping += usize::from(exact[2] > 1.0);
        let (low, high) = bounds(&[a.polys.clone(), b_polys.clone()].concat());
        let both: Vec<Corners> = a.polys.iter().chain(&b_polys).cloned().collect();
        for (number, (name, rule)) in verbs.iter().enumerate() {
            let label = format!("{name} of ({}) and ({}) moved by ({dx}, {dy})", a.label, b.label);
            let found = rings(&match number {
                0 => a.path.union(&b_path, TOLERANCE)?,
                1 => a.path.subtract(&b_path, TOLERANCE)?,
                2 => a.path.intersect(&b_path, TOLERANCE)?,
                _ => a.path.exclude(&b_path, TOLERANCE)?,
            })?;
            assert_eq!(canonical_fault(&found), None, "{label}");
            assert_eq!(crossing_fault(&found, 1e-9), None, "{label}");
            assert!(
                (net_area(&found) - exact[number]).abs() <= bound,
                "{label}: area {}, exact {}, bound {bound}",
                net_area(&found),
                exact[number]
            );
            for _ in 0..150 {
                let point = (
                    low.0 + dice.next() * (high.0 - low.0),
                    low.1 + dice.next() * (high.1 - low.1),
                );
                if nearest_edge(&both, point) > REACH {
                    let want = rule(inside(&a.polys, point), inside(&b_polys, point));
                    assert_eq!(inside(&found, point), want, "{label} at {point:?}");
                }
            }
        }
    }
    assert!(overlapping > 8, "only {overlapping} of the pairs overlapped");
    Ok(())
}
