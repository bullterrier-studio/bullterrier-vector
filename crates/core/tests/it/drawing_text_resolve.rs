//! `resolve` on text: a glyph whose contours overlap becomes one outline, so `split` finds its true parts, while the
//! attributes of every glyph and the per-item numbers go on working. Areas come from the sampled outline (`common`)
//! and from the measures of the test font's `H`, worked out by hand below.

use crate::common::fonts::fraunces;
use crate::common::region::{canonical_fault, sampled};
use crate::common::{Outcome, net_area, rings, shoelace_area};
use bullterrier_vector::{Drawing, Error, Path, Shape, Values, by_index, text};

const TOLERANCE: f64 = 0.01;

fn drawn(drawing: &Drawing) -> Result<Vec<Shape>, Error> {
    drawing.shapes(&Values::new())
}

fn subpaths(shape: &Shape) -> usize {
    shape.path().to_svg_path().matches('M').count()
}

/// The area the outline fills where its contours do not overlap: the sampled contours added, each with its sign.
fn summed_area(path: &Path) -> Result<f64, Box<dyn std::error::Error>> {
    Ok(sampled(path, 64)?.iter().map(|ring| shoelace_area(ring)).sum())
}

/// The `H` of the test font is three contours, a crossbar and two stems, that overlap. The crossbar is
/// x 18.1 to 61.9 and 4.2 high (y -35.25 to -31.05); the stems are 11.65 wide where it crosses them, x 13.25 to 24.9
/// and 57.9 to 69.55. It covers 6.8 of the left stem and 4.0 of the right: 10.8 x 4.2 counted twice.
const H_OVERLAP: f64 = (6.8 + 4.0) * 4.2;

#[test]
fn resolve_turns_the_overlapping_contours_of_an_h_into_one_outline_of_the_same_area() -> Outcome {
    let glyph = drawn(&text("H", &fraunces()?))?;
    assert_eq!(subpaths(&glyph[0]), 3);
    assert!(
        drawn(&text("H", &fraunces()?).split())?.len() > 1,
        "the contours were taken for parts"
    );

    let resolved = drawn(&text("H", &fraunces()?).resolve(TOLERANCE))?;
    assert_eq!((resolved.len(), subpaths(&resolved[0])), (1, 1));
    let found = rings(resolved[0].path())?;
    assert_eq!(canonical_fault(&found), None);
    let expected = summed_area(glyph[0].path())? - H_OVERLAP;
    let area = net_area(&found);
    assert!((area - expected).abs() < 0.003 * expected, "{area} is not {expected}");

    let parts = drawn(&text("H", &fraunces()?).resolve(TOLERANCE).split())?;
    assert_eq!((parts.len(), parts[0].attributes().part()), (1, Some(0)));
    Ok(())
}

#[test]
fn resolve_leaves_separate_parts_and_holes_as_they_are() -> Outcome {
    // (text, parts after resolve, sub-paths of the glyph): the i is a stem and a dot, the o an outline and its hole
    let rows = [("i", 2, 2), ("o", 1, 2), ("l", 1, 1)];
    for (content, parts, contours) in rows {
        let resolved = text(content, &fraunces()?).resolve(TOLERANCE);
        assert_eq!(drawn(&resolved.split())?.len(), parts, "{content}: parts");
        assert_eq!(subpaths(&drawn(&resolved)?[0]), contours, "{content}: sub-paths");
        assert_eq!(canonical_fault(&rings(drawn(&resolved)?[0].path())?), None, "{content}");
    }
    Ok(())
}

#[test]
fn merge_joins_the_contours_it_is_given_without_uniting_them() -> Outcome {
    let merged = drawn(&text("H", &fraunces()?).split().merge())?;
    assert_eq!((merged.len(), subpaths(&merged[0])), (1, 3));
    Ok(())
}

#[test]
fn resolve_keeps_the_attributes_of_every_glyph_and_the_per_item_numbers_still_vary_them() -> Outcome {
    let source = text("Hi Tel", &fraunces()?);
    let (before, after) = (drawn(&source)?, drawn(&source.resolve(TOLERANCE))?);
    assert_eq!(after.len(), before.len());
    for (resolved, original) in after.iter().zip(&before) {
        assert_eq!(resolved.attributes(), original.attributes());
    }

    let moved = drawn(&source.resolve(TOLERANCE).translate(by_index(0.0, 40.0), 0.0))?;
    for (number, (shifted, still)) in moved.iter().zip(&after).enumerate() {
        #[expect(clippy::cast_precision_loss, reason = "five glyphs")]
        let expected = 10.0 * number as f64;
        let gap = shifted.path().bounds()?.x - still.path().bounds()?.x;
        assert!(
            (gap - expected).abs() < 1e-9,
            "glyph {number}: moved by {gap}, not {expected}"
        );
    }
    Ok(())
}
