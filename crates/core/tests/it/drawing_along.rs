//! The `along` step through the public API: it carries each shape along the guide as `Path::along` does, per shape when
//! `start` reads a list, recomputes only itself when a Param it reads changes, keeps what a shape is (style, attributes,
//! how many there are), and a text carried along a circle puts its glyphs at the distances along it that their pens
//! stand at. Expected places are worked out from the angle on the circle with the test's own `sin` and `cos`.

use crate::common::fonts::fraunces;
use crate::common::{Outcome, distance, geometry_refusal, parameter_refusal};
use bullterrier_vector::{Along, Color, Drawing, Param, Path, Stats, Text, Values, text};
use std::f64::consts::PI;

fn guide() -> Result<Path, Box<dyn std::error::Error>> {
    Ok(Path::circle((0.0, 0.0), 300.0)?)
}

fn type_at(content: &str) -> Result<Drawing, Box<dyn std::error::Error>> {
    Ok(text(content, Text::new(&fraunces()?).size(40.0)).fill(Color::hex(0x22_66_cc)))
}

/// The centre of the bounds of a path.
fn centre(path: &Path) -> Result<(f64, f64), Box<dyn std::error::Error>> {
    let bounds = path.bounds()?;
    Ok((bounds.x + bounds.width / 2.0, bounds.y + bounds.height / 2.0))
}

#[test]
fn a_text_carried_along_a_circle_has_its_glyphs_at_the_distances_their_pens_stood() -> Outcome {
    let (radius, start) = (300.0, 0.2);
    let flat = type_at("HIHI HIH")?.shapes(&Values::new())?;
    let bent = type_at("HIHI HIH")?
        .along(&guide()?, Along::new().start(start))
        .shapes(&Values::new())?;
    assert_eq!(bent.len(), flat.len());
    for (flat, bent) in flat.iter().zip(&bent) {
        // a glyph's middle at (x, y) goes to the angle (start * 2 pi r + x) / r, y towards the outside where it is up
        let (x, y) = centre(flat.path())?;
        let angle = (start * 2.0 * PI * radius + x) / radius;
        let reach = radius - y;
        let expected = (reach * libm::sin(angle), -reach * libm::cos(angle));
        let found = centre(bent.path())?;
        let off = distance(found, expected);
        // the bounds of a bent glyph are not the bend of the bounds of a flat one, by a share of its size (a glyph
        // that is not the same turned, such as an A, would be off by several pixels at 70 degrees)
        assert!(
            off <= 1.5,
            "{:?}: the middle is {found:?}, expected {expected:?}",
            flat.attributes().char()
        );
    }
    Ok(())
}

#[test]
fn the_shapes_keep_their_style_attributes_and_number() -> Outcome {
    let flat = type_at("HIFH")?.shapes(&Values::new())?;
    let bent = type_at("HIFH")?
        .along(&guide()?, Along::new().start(0.1))
        .shapes(&Values::new())?;
    assert_eq!(bent.len(), flat.len());
    for (flat, bent) in flat.iter().zip(&bent) {
        assert_eq!(bent.style(), flat.style());
        assert_eq!(bent.attributes(), flat.attributes());
        assert_eq!(bent.index(), flat.index());
        assert_ne!(bent.path(), flat.path());
    }
    Ok(())
}

#[test]
fn a_list_gives_each_shape_its_own_start() -> Outcome {
    let shift = Param::new("shift", 0.0, -1.0, 1.0)?;
    // a straight guide 10 right and 20 down of the origin, 400 long: a glyph moves by its start times 400
    let east = Path::line((10.0, 20.0), (410.0, 20.0))?;
    let drawing = type_at("HI")?.along(&east, Along::new().start(&shift));
    let flat = type_at("HI")?.shapes(&Values::new())?;
    let starts = [0.0, 0.25];
    let bent = drawing.shapes(&Values::new().set("shift", starts.to_vec()))?;
    for ((flat, bent), start) in flat.iter().zip(&bent).zip(starts) {
        let (home, found) = (flat.path().bounds()?, bent.path().bounds()?);
        let (dx, dy) = (found.x - home.x, found.y - home.y);
        assert!(
            (dx - (10.0 + start * 400.0)).abs() <= 0.03 && (dy - 20.0).abs() <= 0.03,
            "start {start}: moved by ({dx}, {dy})"
        );
    }
    Ok(())
}

#[test]
fn near_is_read_for_each_shape_too() -> Outcome {
    // a unit dot at the origin, carried along a circle: near the bottom, it starts at the bottom
    let dot = Drawing::from(Path::start((0.0, 0.0))?.line_to((1.0, 0.0))?);
    let bent = dot
        .along(&guide()?, Along::new().near((0.0, 1000.0)))
        .shapes(&Values::new())?;
    let (x, y) = centre(bent[0].path())?;
    assert!(x.abs() <= 1.0 && (y - 300.0).abs() <= 0.1, "({x}, {y})");
    Ok(())
}

#[test]
fn changing_a_param_of_the_bend_recomputes_only_the_bend() -> Outcome {
    let cycle = Param::new("cycle", 0.0, 0.0, 1.0)?;
    let drawing = type_at("HIFH")?.along(&guide()?, Along::new().start(&cycle));
    let stats = |values: &Values| -> Result<Stats, Box<dyn std::error::Error>> {
        drawing.shapes(values)?;
        Ok(drawing.stats())
    };
    // the text and its fill read no Param, so a later draw starts from the fill: the two results kept are that and the draw
    let counts = |computed, reused| Stats {
        computed,
        reused,
        entries: 2,
    };
    assert_eq!(
        stats(&Values::new())?,
        counts(3, 0),
        "cold: the text, its fill and the bend"
    );
    assert_eq!(
        stats(&Values::new().set("cycle", 0.5))?,
        counts(1, 1),
        "cycle changed: the bend"
    );
    assert_eq!(
        stats(&Values::new().set("cycle", 0.5))?,
        counts(0, 1),
        "the same again: none"
    );
    Ok(())
}

#[test]
fn a_guide_that_is_empty_or_has_no_length_is_refused_when_drawn() -> Outcome {
    let dot = Drawing::from(Path::line((0.0, 0.0), (10.0, 0.0))?);
    for (case, guide) in [("empty", Path::new()), ("a point", Path::start((1.0, 1.0))?)] {
        let drawing = dot.along(&guide, Along::new());
        assert_eq!(drawing.error(), None, "{case}: a guide is not measured at the call");
        let (verb, _) = geometry_refusal(drawing.shapes(&Values::new()))?;
        assert!(verb.contains("along"), "{case}: {verb}");
    }
    Ok(())
}

#[test]
fn bending_more_than_ten_million_edges_in_a_draw_is_refused_naming_the_step_and_the_counter() -> Outcome {
    // 50,000 edges of 1,000 pixels, each wound round a circle 100 across about once and a half, are halved to the
    // depth cap, 256 pieces each: 12.8 million in all
    let zigzag: Vec<(f64, f64)> = (0..50_000u32)
        .map(|index| (if index % 2 == 0 { 0.0 } else { 1000.0 }, 0.0))
        .collect();
    let drawing = Drawing::from(Path::polyline(zigzag, false)?).along(&Path::circle((0.0, 0.0), 100.0)?, Along::new());
    let (parameter, message) = parameter_refusal(drawing.shapes(&Values::new()))?;
    assert!(parameter.contains("along"), "{parameter}");
    assert!(message.contains("MAX_SEGMENTS 10000000"), "{message}");
    Ok(())
}

#[test]
fn walking_a_long_guide_for_every_shape_is_charged_to_the_draw() -> Outcome {
    // 1,000 shapes, each walking a closed guide of 20,000 edges from the point nearest (0, 0): 20 million edges
    let zigzag: Vec<(f64, f64)> = (0..20_000u32)
        .map(|index| (f64::from(index), if index % 2 == 0 { 0.0 } else { 10.0 }))
        .collect();
    let guide = Path::polyline(zigzag, true)?;
    let points = (0..1000u32)
        .map(|index| Ok(Drawing::from(Path::start((f64::from(index), 0.0))?)))
        .collect::<Result<Vec<_>, bullterrier_vector::Error>>()?;
    let drawing = Drawing::new(points).along(&guide, Along::new().near((0.0, 0.0)));
    let (parameter, message) = parameter_refusal(drawing.shapes(&Values::new()))?;
    assert!(parameter.contains("along"), "{parameter}");
    assert!(message.contains("MAX_SEGMENTS"), "{message}");
    Ok(())
}
