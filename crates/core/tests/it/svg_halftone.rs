//! The SVG `Drawing::to_svg` writes for a halftone: the clip path and the one path of dots, read back with an XML parser
//! against the lattice worked out here from the pitch and the angle, and the picture it makes against the pixels,
//! drawn by a second renderer (`resvg`).

use crate::common::fonts::fraunces;
use crate::common::svg_oracle::{body, element, svg_agrees};
use crate::common::{Outcome, Ring, net_area, parameter_refusal, rings};
use bullterrier_vector::{Color, Deg, Drawing, Error, Halftone, Path, Shadow, Size, Stroke, Text, Values, text};
use std::f64::consts::PI;

const BLACK: Color = Color::hex(0x00_00_00);
const RED: Color = Color::hex(0xff_00_00);

fn color(text: &str) -> Result<Color, Error> {
    Color::parse(text)
}

fn rect(left: f64, top: f64, width: f64, height: f64) -> Result<Path, Error> {
    Path::rect(left, top, width, height, 0.0)
}

/// The SVG of `drawing` at `side` by `side`.
fn svg(drawing: &Drawing, side: f64) -> Result<String, Error> {
    drawing.to_svg(Size::px(side, side), &Values::new())
}

/// The dots of the SVG: the polygons of the `<path>` that the `<g clip-path>` holds.
fn dots_of(svg: &str) -> Result<Vec<Ring>, Box<dyn std::error::Error>> {
    let document = roxmltree::Document::parse(svg)?;
    let mut found = Vec::new();
    for group in document.descendants().filter(|node| {
        node.attribute("clip-path")
            .is_some_and(|clip| clip.starts_with("url(#halftone-"))
    }) {
        for path in group.children().filter(roxmltree::Node::is_element) {
            found.extend(rings(&Path::from_svg_path(
                path.attribute("d").ok_or("a dot path without data")?,
            )?)?);
        }
    }
    Ok(found)
}

/// The centre of a regular polygon: the mean of its corners.
fn centre(dot: &Ring) -> (f64, f64) {
    #[expect(clippy::cast_precision_loss, reason = "a few dozen corners")]
    let count = dot.len() as f64;
    (
        dot.iter().map(|corner| corner.0).sum::<f64>() / count,
        dot.iter().map(|corner| corner.1).sum::<f64>() / count,
    )
}

/// The lattice point `(i, j)` of a lattice `pitch` apart turned `degrees` clockwise on the screen (`y` down) about the
/// origin: the point `(i * pitch, j * pitch)` turned.
fn lattice(pitch: f64, degrees: f64, (i, j): (i32, i32)) -> (f64, f64) {
    let (sin, cos) = (libm::sin(degrees.to_radians()), libm::cos(degrees.to_radians()));
    let (x, y) = (f64::from(i) * pitch, f64::from(j) * pitch);
    (x * cos - y * sin, x * sin + y * cos)
}

/// The distance from `point` to the rectangle `[left, right] x [top, bottom]`, 0 inside.
fn from_rect(point: (f64, f64), (left, top, right, bottom): (f64, f64, f64, f64)) -> f64 {
    let (across, down) = (
        (left - point.0).max(point.0 - right).max(0.0),
        (top - point.1).max(point.1 - bottom).max(0.0),
    );
    libm::hypot(across, down)
}

/// The centres of the dots of radius `radius` that touch the rectangle: the lattice points within the radius of it.
fn expected_centres(rect: (f64, f64, f64, f64), (pitch, degrees): (f64, f64), radius: f64) -> Vec<(f64, f64)> {
    let mut found = Vec::new();
    for i in -40..40 {
        for j in -40..40 {
            let at = lattice(pitch, degrees, (i, j));
            if from_rect(at, rect) < radius {
                found.push(at);
            }
        }
    }
    sorted(found)
}

fn sorted(mut points: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    points.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    points
}

/// The radius of a dot of area `(1 - tone)` of a cell of `pitch`.
fn radius_of(pitch: f64, tone: f64) -> f64 {
    pitch * ((1.0 - tone) / PI).sqrt()
}

// The markup --------------------------------------------------------------------------------------------------------

#[test]
fn a_halftone_is_a_clip_path_of_the_shape_and_one_path_of_dots_in_the_clipped_group() -> Outcome {
    let drawing = Drawing::from(rect(0.0, 0.0, 20.0, 20.0)?)
        .halftone(Halftone::new(color("#336699")?, 0.5).pitch(10.0).angle(Deg(0.0)));
    let found = body(&drawing, 20.0)?;
    let names: Vec<&str> = found.iter().map(|element| element.name.as_str()).collect();
    assert_eq!(names, ["clipPath", "g"]);
    assert_eq!(found[0].attributes.get("id").map(String::as_str), Some("halftone-0"));
    assert_eq!(
        found[0].children,
        vec![element("path", &[("d", "M0 0 L20 0 L20 20 L0 20 Z")], vec![])]
    );
    assert_eq!(
        found[1].attributes.get("clip-path").map(String::as_str),
        Some("url(#halftone-0)")
    );
    // the dots are one path, filled with the colour and nothing else
    assert_eq!(found[1].children.len(), 1);
    let attributes = &found[1].children[0].attributes;
    assert_eq!(attributes.get("fill").map(String::as_str), Some("#336699"));
    assert_eq!(attributes.keys().collect::<Vec<_>>(), ["d", "fill"]);
    Ok(())
}

#[test]
fn a_translucent_colour_is_one_fill_opacity_on_the_path_of_dots() -> Outcome {
    let drawing = Drawing::from(rect(0.0, 0.0, 20.0, 20.0)?).halftone(Halftone::new(color("#33669980")?, 0.5));
    let text = svg(&drawing, 20.0)?;
    let document = roxmltree::Document::parse(&text)?;
    let dots = document
        .descendants()
        .find(|node| node.has_tag_name("g") && node.attribute("clip-path").is_some())
        .and_then(|group| group.first_element_child())
        .ok_or("no path of dots")?;
    assert_eq!(dots.attribute("fill-opacity"), Some(&*format!("{}", 128.0 / 255.0)));
    Ok(())
}

#[test]
fn the_dots_are_the_lattice_points_within_a_radius_of_the_shape_each_of_the_area_of_the_tone() -> Outcome {
    // (pitch, angle, tone, the rectangle): angles that are and are not quarter turns, pitches that do and do not divide
    // the rectangle, a dot at every tone from heavy to light
    let rows = [
        (10.0, 0.0, 0.5, (3.0, 5.0, 57.0, 41.0)),
        (10.0, 30.0, 0.5, (3.0, 5.0, 57.0, 41.0)),
        (7.5, 45.0, 0.8, (-12.0, 2.0, 40.0, 64.0)),
        (12.0, -20.0, 0.3, (10.0, 10.0, 90.0, 50.0)),
        (9.0, 90.0, 0.6, (0.5, 0.5, 60.5, 20.5)),
        (15.0, 405.0, 0.9, (1.0, 1.0, 59.0, 59.0)),
    ];
    for (pitch, angle, tone, bounds) in rows {
        let (left, top, right, bottom) = bounds;
        let shape = rect(left, top, right - left, bottom - top)?;
        let drawing = Drawing::from(shape).halftone(Halftone::new(BLACK, tone).pitch(pitch).angle(Deg(angle)));
        let dots = dots_of(&svg(&drawing, 100.0)?)?;
        let found = sorted(dots.iter().map(centre).collect());
        let wanted = expected_centres(bounds, (pitch, angle), radius_of(pitch, tone));
        let case = format!("pitch {pitch}, angle {angle}, tone {tone}");
        assert_eq!(found.len(), wanted.len(), "{case}: the number of dots");
        for (at, expected) in found.iter().zip(&wanted) {
            assert!(
                (at.0 - expected.0).abs() < 1e-6 && (at.1 - expected.1).abs() < 1e-6,
                "{case}: a dot at {at:?}, expected {expected:?}"
            );
        }
        // each dot has the area of the tone, to the polygon's 1% under the circle
        let area = (1.0 - tone) * pitch * pitch;
        for dot in &dots {
            let drawn = net_area(std::slice::from_ref(dot));
            assert!(
                (drawn - area).abs() < 0.01 * area,
                "{case}: a dot of {drawn}, expected {area}"
            );
        }
    }
    Ok(())
}

#[test]
fn a_dot_stays_on_the_lattice_when_the_shape_moves() -> Outcome {
    let dots = Halftone::new(BLACK, 0.5).pitch(8.0).angle(Deg(25.0));
    let here = Drawing::from(rect(10.0, 10.0, 70.0, 50.0)?).halftone(dots.clone());
    let there = Drawing::from(rect(35.0, 23.0, 70.0, 50.0)?).halftone(dots);
    let (first, second) = (dots_of(&svg(&here, 120.0)?)?, dots_of(&svg(&there, 120.0)?)?);
    // the rectangles overlap in [35, 80] x [23, 60]: a dot well inside that is in both pictures, at the same place and
    // with the same corners
    let mut shared = 0;
    for dot in &first {
        let at = centre(dot);
        if at.0 > 40.0 && at.0 < 75.0 && at.1 > 28.0 && at.1 < 55.0 {
            shared += 1;
            assert!(second.contains(dot), "a dot at {at:?} moved with the shape");
        }
    }
    assert!(shared > 10, "only {shared} dots in the overlap");
    Ok(())
}

#[test]
fn a_drawing_at_twice_the_size_has_numbers_twice_as_large() -> Outcome {
    let drawing = Drawing::from(Path::circle((50.0, 50.0), 40.0)?)
        .halftone(Halftone::new(RED, 0.4).pitch(9.0).angle(Deg(33.0)))
        .stroke(Stroke::new(BLACK).width(3.0))
        .with_size(100.0, 100.0);
    let (small, large) = (svg(&drawing, 100.0)?, svg(&drawing, 200.0)?);
    let (small_dots, large_dots) = (dots_of(&small)?, dots_of(&large)?);
    assert!(small_dots.len() > 30);
    let doubled: Vec<Ring> = small_dots
        .iter()
        .map(|dot| dot.iter().map(|corner| (corner.0 * 2.0, corner.1 * 2.0)).collect())
        .collect();
    assert_eq!(large_dots, doubled);
    assert!(small.contains(r#"stroke-width="3""#) && large.contains(r#"stroke-width="6""#));
    Ok(())
}

// The group ---------------------------------------------------------------------------------------------------------

#[test]
fn opacity_and_blend_are_on_a_group_of_the_dots_and_the_stroke_over_them() -> Outcome {
    let dots = Halftone::new(RED, 0.5).pitch(10.0);
    let stroke = Stroke::new(BLACK).width(2.0);
    let rows = [
        (
            "opacity",
            Drawing::from(rect(0.0, 0.0, 30.0, 30.0)?)
                .halftone(dots.clone())
                .stroke(stroke.clone())
                .opacity(0.5),
            "opacity",
            "0.5",
        ),
        (
            "multiply",
            Drawing::from(rect(0.0, 0.0, 30.0, 30.0)?)
                .halftone(dots)
                .stroke(stroke)
                .blend(bullterrier_vector::Blend::Multiply),
            "style",
            "mix-blend-mode:multiply",
        ),
    ];
    for (case, drawing, attribute, value) in rows {
        let found = body(&drawing, 40.0)?;
        let group = found
            .iter()
            .find(|element| element.attributes.contains_key(attribute))
            .ok_or("no group")?;
        let children: Vec<&str> = group.children.iter().map(|child| child.name.as_str()).collect();
        assert_eq!(children, ["g", "path"], "{case}");
        assert_eq!(
            group.attributes.get(attribute).map(String::as_str),
            Some(value),
            "{case}"
        );
    }
    // the stroke is the last thing drawn, over the dots, and has no fill
    let found = body(
        &Drawing::from(rect(0.0, 0.0, 30.0, 30.0)?)
            .halftone(Halftone::new(RED, 0.5).pitch(10.0))
            .stroke(Stroke::new(BLACK).width(2.0))
            .opacity(0.5),
        40.0,
    )?;
    let outline = found[1].children.last().ok_or("no stroke")?;
    assert_eq!(outline.attributes.get("fill").map(String::as_str), Some("none"));
    assert_eq!(outline.attributes.get("stroke").map(String::as_str), Some("#000000"));
    Ok(())
}

#[test]
fn a_hard_shadow_is_a_tinted_copy_of_the_dots_moved_under_the_screen() -> Outcome {
    let drawing = Drawing::from(rect(0.0, 0.0, 30.0, 30.0)?)
        .halftone(Halftone::new(RED, 0.5).pitch(10.0))
        .shadow(Shadow::new().dx(4.0).dy(6.0).blur(0.0).color(color("#0000ff80")?));
    let found = body(&drawing, 40.0)?;
    let names: Vec<&str> = found.iter().map(|element| element.name.as_str()).collect();
    assert_eq!(names, ["clipPath", "g", "g"]);
    let copy = &found[1];
    assert_eq!(
        copy.attributes.get("transform").map(String::as_str),
        Some("translate(4 6)")
    );
    assert_eq!(
        copy.attributes.get("opacity").map(String::as_str),
        Some(&*format!("{}", 128.0 / 255.0))
    );
    assert_eq!(
        copy.children[0].children[0].attributes.get("fill").map(String::as_str),
        Some("#0000ff")
    );
    assert_eq!(found[2].attributes.keys().collect::<Vec<_>>(), ["clip-path"]);
    Ok(())
}

#[test]
fn a_tone_of_zero_is_a_plain_fill_and_a_tone_of_one_a_fill_of_none() -> Outcome {
    let square = || -> Result<Drawing, Error> { Ok(Drawing::from(rect(0.0, 0.0, 30.0, 30.0)?)) };
    for (case, tone, plain) in [("solid", 0.0, RED), ("bare", 1.0, Color::NONE)] {
        let screened = body(&square()?.halftone(Halftone::new(RED, tone)), 40.0)?;
        assert_eq!(screened, body(&square()?.fill(plain), 40.0)?, "{case}");
        let stroked = body(&square()?.halftone(Halftone::new(RED, tone)).stroke(BLACK), 40.0)?;
        assert_eq!(
            stroked,
            body(&square()?.fill(plain).stroke(BLACK), 40.0)?,
            "{case} with a stroke"
        );
    }
    Ok(())
}

// Against the pixels ------------------------------------------------------------------------------------------------

#[test]
fn the_svg_shows_what_the_pixels_show() -> Outcome {
    let paper = Drawing::from(rect(0.0, 0.0, 120.0, 120.0)?).fill(color("#fff6e0")?);
    let font = fraunces()?;
    let letter = text("o", Text::new(&font).size(150.0)).translate(10.0, 90.0);
    let rows = [
        (
            "a disc",
            Drawing::from(Path::circle((60.0, 60.0), 50.0)?).halftone(Halftone::new(RED, 0.5).pitch(6.0)),
        ),
        (
            "a letter with a hole",
            letter.halftone(Halftone::new(BLACK, 0.4).pitch(5.0).angle(Deg(15.0))),
        ),
        (
            "a translucent colour that overlaps itself",
            Drawing::from(Path::circle((60.0, 60.0), 50.0)?)
                .halftone(Halftone::new(color("#00008080")?, 0.05).pitch(12.0)),
        ),
        (
            "dots with a stroke, at half opacity",
            Drawing::from(Path::circle((60.0, 60.0), 40.0)?)
                .halftone(Halftone::new(RED, 0.5).pitch(7.0))
                .stroke(Stroke::new(BLACK).width(4.0))
                .opacity(0.5),
        ),
        (
            "dots multiplied over paper",
            Drawing::from(Path::circle((60.0, 60.0), 40.0)?)
                .halftone(Halftone::new(color("#ff8800")?, 0.3).pitch(8.0))
                .blend(bullterrier_vector::Blend::Multiply),
        ),
        (
            "dots with a hard shadow",
            Drawing::from(Path::circle((50.0, 50.0), 35.0)?)
                .halftone(Halftone::new(RED, 0.5).pitch(6.0))
                .shadow(Shadow::new().dx(8.0).dy(10.0).blur(0.0).color(color("#0000ff99")?)),
        ),
        (
            "dots with a blurred shadow",
            Drawing::from(Path::circle((50.0, 50.0), 35.0)?)
                .halftone(Halftone::new(RED, 0.5).pitch(6.0))
                .shadow(Shadow::new().dx(4.0).dy(6.0).blur(10.0)),
        ),
        (
            "a knockout of dots",
            Drawing::from(rect(0.0, 0.0, 120.0, 120.0)?)
                .fill(RED)
                .knockout()
                .halftone(Halftone::new(BLACK, 0.5).pitch(10.0)),
        ),
    ];
    for (case, drawing) in rows {
        let drawing = Drawing::new([paper.clone(), drawing]).with_size(120.0, 120.0);
        let agreement = svg_agrees(&drawing, 120, 120)?;
        // the two renderers cover a curved edge differently (resvg's coverage is off by up to a sixth of the range
        // there): a pixel in a hundred of the edges may differ by more than 40 levels
        assert!(
            agreement.interior <= 4 && agreement.edge_over_40 * 100 <= agreement.edge_pixels,
            "{case}: {agreement:?}"
        );
    }
    Ok(())
}

#[test]
fn a_pitch_that_asks_for_millions_of_dots_is_refused_naming_the_pitch() -> Outcome {
    let drawing = Drawing::from(Path::circle((500.0, 500.0), 500.0)?).halftone(Halftone::new(BLACK, 0.5).pitch(0.01));
    let (name, message) = parameter_refusal(svg(&drawing, 100.0))?;
    assert!(
        name.contains("pitch") && message.contains("MAX_SEGMENTS"),
        "{name}: {message}"
    );
    Ok(())
}
