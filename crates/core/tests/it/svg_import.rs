//! `Drawing::from_svg` through the public API. Oracles: the SVG that `Drawing::to_svg` writes (exported, then read
//! back), hand-worked coordinates for the transforms, and hand-written documents for ids and groups.

use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Color, Drawing, Error, Paint, Path, Point, Select, Shape, Size, Stroke, Values};

const OPEN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100">"#;

fn svg(body: &str) -> String {
    format!("{OPEN}{body}</svg>")
}

fn shapes(text: &str) -> Result<Vec<Shape>, Error> {
    Drawing::from_svg(text)?.shapes(&Values::new())
}

fn only(shapes: Vec<Shape>) -> Result<Shape, Box<dyn std::error::Error>> {
    match <[Shape; 1]>::try_from(shapes) {
        Ok([one]) => Ok(one),
        Err(other) => Err(format!("expected one shape, got {}", other.len()).into()),
    }
}

type Pair = (f64, f64);

fn near(got: Point, want: (f64, f64)) -> bool {
    (got.x - want.0).abs() < 1e-9 && (got.y - want.1).abs() < 1e-9
}

// Round trip: what to_svg writes ------------------------------------------------------------------------------------

#[test]
fn a_drawing_exported_to_svg_is_imported_with_its_paths_and_paints() -> Outcome {
    let drawing = Drawing::new([
        Drawing::from(Path::rect(10.0, 20.0, 30.0, 40.0, 0.0)?).fill(Color::parse("#336699")?),
        Drawing::from(Path::line((5.0, 6.0), (70.0, 80.0))?).stroke(Stroke::new(Color::parse("#ff0000")?).width(3.0)),
        Drawing::from(Path::circle((100.0, 50.0), 12.0)?)
            .fill(Color::parse("#00ff0080")?)
            .opacity(0.5),
    ])
    .with_size(200.0, 100.0);
    let text = drawing.to_svg(Size::px(200.0, 100.0), &Values::new())?;
    let imported = Drawing::from_svg(&text)?;
    assert_eq!(imported.size(), Some((200.0, 100.0)));
    let (before, after) = (drawing.shapes(&Values::new())?, imported.shapes(&Values::new())?);
    assert_eq!(before.len(), after.len());
    for (was, now) in before.iter().zip(&after) {
        assert_eq!(now.path().to_svg_path(), was.path().to_svg_path());
        assert_eq!(now.style().fill(), was.style().fill());
        assert_eq!(now.style().stroke(), was.style().stroke());
        assert!((now.style().opacity() - was.style().opacity()).abs() < 1e-12);
    }
    Ok(())
}

// Ids, classes and groups -------------------------------------------------------------------------------------------

/// A logo of 42 contours: 30 in the group `mark` inside the group `logo`, 10 in `logo` itself behind an unnamed
/// group, one outside any group and one with no id.
fn logo() -> String {
    let rect = |index: usize| {
        format!(
            r#"<rect id="part-{index}" class="c{}" x="{}" y="0" width="5" height="5"/>"#,
            index % 3,
            index * 10
        )
    };
    let mark: String = (0..30).map(rect).collect();
    let behind: String = (30..40).map(rect).collect();
    svg(&format!(
        r#"<g id="logo"><g id="mark">{mark}</g><g>{behind}</g></g>{}<rect x="900" y="0" width="5" height="5"/><rect id="loose" x="910" y="0" width="5" height="5"/>"#,
        rect(40),
    ))
}

#[test]
fn only_id_selects_each_part_of_a_logo_and_group_gives_its_path() -> Outcome {
    let logo = Drawing::from_svg(&logo())?;
    assert_eq!(logo.shapes(&Values::new())?.len(), 43);
    for index in 0..41usize {
        let part = only(
            logo.only(Select::new().attribute("id", format!("part-{index}")))
                .shapes(&Values::new())?,
        )?;
        assert_eq!(
            part.path().bounds()?.x,
            f64::from(u32::try_from(index)?) * 10.0,
            "part-{index}"
        );
        assert_eq!(part.attributes().id(), Some(format!("part-{index}").as_str()));
        assert_eq!(part.attributes().class(), Some(format!("c{}", index % 3).as_str()));
        let group = match index {
            0..30 => Some("logo/mark"),
            30..40 => Some("logo"),
            _ => None,
        };
        assert_eq!(part.attributes().group(), group, "part-{index}");
    }
    Ok(())
}

#[test]
fn an_element_without_an_id_has_its_index_and_no_id() -> Outcome {
    let all = shapes(&logo())?;
    let plain = &all[41];
    assert_eq!(plain.index(), 41);
    assert_eq!(plain.attributes().id(), None);
    assert_eq!(plain.attributes().class(), None);
    assert_eq!(plain.attributes().group(), None);
    Ok(())
}

#[test]
fn group_and_class_select_by_the_whole_text() -> Outcome {
    let logo = Drawing::from_svg(&logo())?;
    let count = |name: &str, value: &str| -> Result<usize, Error> {
        Ok(logo
            .only(Select::new().attribute(name, value))
            .shapes(&Values::new())?
            .len())
    };
    assert_eq!(count("group", "logo/mark")?, 30);
    assert_eq!(count("group", "logo")?, 10);
    assert_eq!(count("group", "mark")?, 0);
    assert_eq!(count("class", "c0")?, 14);
    // an id of one character is text too
    let one = Drawing::from_svg(&svg(r#"<rect id="a" width="1" height="1"/>"#))?;
    assert_eq!(
        one.only(Select::new().attribute("id", "a"))
            .shapes(&Values::new())?
            .len(),
        1
    );
    Ok(())
}

#[test]
fn a_one_character_text_asked_of_a_character_attribute_is_that_character() -> Outcome {
    let drawing = Drawing::from(Path::rect(0.0, 0.0, 1.0, 1.0, 0.0)?);
    // the same words select by `char` whether they are typed as a character or as text
    assert!(drawing.only(Select::new().attribute("char", "O")).error().is_none());
    assert!(drawing.only(Select::new().attribute("char", 'O')).error().is_none());
    let by_long_text = drawing.only(Select::new().attribute("char", "OO"));
    assert!(
        by_long_text
            .error()
            .is_some_and(|error| format!("{error}").contains("one character"))
    );
    Ok(())
}

#[test]
fn a_text_attribute_asked_as_a_number_and_a_number_attribute_asked_as_text_are_refused() -> Outcome {
    let logo = Drawing::from_svg(&logo())?;
    let by_number = logo.only(Select::new().attribute("id", 3_u32));
    assert!(
        by_number
            .error()
            .is_some_and(|error| format!("{error}").contains("text")),
        "{:?}",
        by_number.error()
    );
    let by_text = logo.only(Select::new().attribute("glyph", "abc"));
    assert!(
        by_text
            .error()
            .is_some_and(|error| format!("{error}").contains("whole number"))
    );
    Ok(())
}

#[test]
fn split_and_merge_keep_the_ids() -> Outcome {
    let drawing = Drawing::from_svg(&svg(
        r#"<g id="two"><path id="pair" d="M0 0 L5 0 L5 5 Z M20 0 L25 0 L25 5 Z"/><rect id="solo" x="40" width="5" height="5"/></g>"#,
    ))?;
    let split = drawing.split();
    let pair = split
        .only(Select::new().attribute("id", "pair"))
        .shapes(&Values::new())?;
    assert_eq!(pair.len(), 2);
    assert_eq!(pair[0].attributes().group(), Some("two"));
    let merged = split.merge_by("id").shapes(&Values::new())?;
    let ids: Vec<_> = merged
        .iter()
        .map(|shape| shape.attributes().id().map(str::to_owned))
        .collect();
    assert_eq!(ids, vec![Some("pair".to_owned()), Some("solo".to_owned())]);
    Ok(())
}

#[test]
fn the_text_attributes_survive_json() -> Outcome {
    let drawing = Drawing::from_svg(&logo())?;
    let again = Drawing::from_json(&drawing.to_json()?, &bullterrier_vector::Inputs::new())?;
    assert_eq!(again.shapes(&Values::new())?, drawing.shapes(&Values::new())?);
    let part = again
        .only(Select::new().attribute("id", "part-7"))
        .shapes(&Values::new())?;
    assert_eq!(part[0].attributes().group(), Some("logo/mark"));
    Ok(())
}

// Transforms --------------------------------------------------------------------------------------------------------

/// The ends of the line `M1 2 L3 4` after `transform`, on its own or on the group round it.
fn ends(transform: &str, on_group: bool) -> Result<(Point, Point), Box<dyn std::error::Error>> {
    let (group, element) = if on_group { (transform, "") } else { ("", transform) };
    let line = only(shapes(&svg(&format!(
        r#"<g transform="{group}"><path transform="{element}" fill="none" d="M1 2 L3 4"/></g>"#
    )))?)?;
    Ok((line.path().point_at(0.0, None)?, line.path().point_at(1.0, None)?))
}

#[test]
fn transforms_are_worked_into_the_points() -> Outcome {
    let rows: [(&str, Pair, Pair); 12] = [
        ("translate(5 7)", (6.0, 9.0), (8.0, 11.0)),
        ("translate(5)", (6.0, 2.0), (8.0, 4.0)),
        ("scale(2 3)", (2.0, 6.0), (6.0, 12.0)),
        ("scale(2)", (2.0, 4.0), (6.0, 8.0)),
        ("rotate(90)", (-2.0, 1.0), (-4.0, 3.0)),
        ("rotate(90 1 2)", (1.0, 2.0), (-1.0, 4.0)),
        ("skewX(45)", (3.0, 2.0), (7.0, 4.0)),
        ("skewY(45)", (1.0, 3.0), (3.0, 7.0)),
        ("matrix(1 0 0 1 10 20)", (11.0, 22.0), (13.0, 24.0)),
        ("matrix(0 1 -1 0 0 0)", (-2.0, 1.0), (-4.0, 3.0)),
        // the first listed applies last: scale, then move
        ("translate(10 0) scale(2)", (12.0, 4.0), (16.0, 8.0)),
        ("translate(10,0),scale(2)", (12.0, 4.0), (16.0, 8.0)),
    ];
    for (transform, start, end) in rows {
        let (from, to) = ends(transform, false)?;
        assert!(near(from, start) && near(to, end), "{transform}: {from:?} {to:?}");
        let (from, to) = ends(transform, true)?;
        assert!(
            near(from, start) && near(to, end),
            "on a group, {transform}: {from:?} {to:?}"
        );
    }
    Ok(())
}

#[test]
fn a_child_transform_applies_before_its_parents() -> Outcome {
    let line = only(shapes(&svg(
        r#"<g transform="translate(10 0)"><path transform="scale(2)" fill="none" d="M1 2 L3 4"/></g>"#,
    ))?)?;
    assert!(near(line.path().point_at(0.0, None)?, (12.0, 4.0)));
    assert!(near(line.path().point_at(1.0, None)?, (16.0, 8.0)));
    Ok(())
}

#[test]
fn the_view_box_corner_becomes_the_origin_and_its_size_the_drawings() -> Outcome {
    let drawing = Drawing::from_svg(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="999" height="999" viewBox="10 20 100 50"><rect x="15" y="25" width="5" height="6"/></svg>"#,
    )?;
    assert_eq!(drawing.size(), Some((100.0, 50.0)));
    let bounds = only(drawing.shapes(&Values::new())?)?.path().bounds()?;
    assert_eq!((bounds.x, bounds.y, bounds.width, bounds.height), (5.0, 5.0, 5.0, 6.0));
    Ok(())
}

#[test]
fn without_a_view_box_the_width_and_height_are_the_size() -> Outcome {
    let drawing = Drawing::from_svg(r#"<svg xmlns="http://www.w3.org/2000/svg" width="64px" height="32"/>"#)?;
    assert_eq!(drawing.size(), Some((64.0, 32.0)));
    assert_eq!(drawing.shapes(&Values::new())?.len(), 0);
    Ok(())
}

// Paint -------------------------------------------------------------------------------------------------------------

#[test]
fn an_element_with_no_paint_is_filled_black_and_not_stroked() -> Outcome {
    let shape = only(shapes(&svg(r#"<rect width="4" height="4"/>"#))?)?;
    assert_eq!(shape.style().fill(), &Paint::Color(Color::parse("#000000")?));
    assert!(shape.style().stroke().is_none());
    Ok(())
}

#[test]
fn paint_is_handed_down_groups_and_the_style_attribute_wins_over_the_attributes() -> Outcome {
    let shape = only(shapes(&svg(
        r##"<g fill="#ff0000" stroke="#0000ff" stroke-width="3" stroke-linejoin="bevel" stroke-linecap="square" stroke-miterlimit="7"><g fill-opacity="0.5"><rect width="4" height="4" fill="#00ff00" style="stroke-width: 5; fill:#102030"/></g></g>"##,
    ))?)?;
    assert_eq!(shape.style().fill(), &Paint::Color(Color::parse("#10203080")?));
    let stroke = shape.style().stroke().ok_or("no stroke")?;
    assert_eq!(stroke.paint(), &Paint::Color(Color::parse("#0000ff")?));
    assert_eq!(stroke.width(), 5.0);
    assert_eq!(stroke.miter_limit(), 7.0);
    assert_eq!(stroke.join().name(), "bevel");
    assert_eq!(stroke.cap().name(), "square");
    Ok(())
}

#[test]
fn svgs_own_stroke_defaults_apply() -> Outcome {
    let shape = only(shapes(&svg(r##"<rect width="4" height="4" stroke="#000"/>"##))?)?;
    let stroke = shape.style().stroke().ok_or("no stroke")?;
    assert_eq!(
        (
            stroke.width(),
            stroke.join().name(),
            stroke.cap().name(),
            stroke.miter_limit()
        ),
        (1.0, "miter", "butt", 4.0)
    );
    assert_eq!(stroke.paint(), &Paint::Color(Color::parse("#000000")?));
    Ok(())
}

#[test]
fn a_stroke_follows_an_even_scale_and_a_turn_and_refuses_a_stretch() -> Outcome {
    let scaled = only(shapes(&svg(
        r##"<path transform="scale(3) rotate(30)" stroke="#000" stroke-width="2" d="M0 0 L5 0"/>"##,
    ))?)?;
    let width = scaled.style().stroke().ok_or("no stroke")?.width();
    assert!((width - 6.0).abs() < 1e-9, "{width}");
    let mirrored = only(shapes(&svg(
        r##"<path transform="scale(-2 2)" stroke="#000" stroke-width="2" d="M0 0 L5 0"/>"##,
    ))?)?;
    assert!((mirrored.style().stroke().ok_or("no stroke")?.width() - 4.0).abs() < 1e-9);
    let (name, message) = parameter_refusal(shapes(&svg(
        r##"<path transform="scale(2 3)" stroke="#000" d="M0 0 L5 0"/>"##,
    )))?;
    assert_eq!(name, "svg");
    assert!(message.contains("<path>") && message.contains("stroke"), "{message}");
    // unstroked, a stretch is fine
    shapes(&svg(r#"<path transform="scale(2 3)" d="M0 0 L5 5 L5 0 Z"/>"#))?;
    Ok(())
}

#[test]
fn an_element_takes_its_own_opacity() -> Outcome {
    let shape = only(shapes(&svg(r#"<rect width="4" height="4" opacity="0.25"/>"#))?)?;
    assert!((shape.style().opacity() - 0.25).abs() < 1e-12);
    Ok(())
}

// The other shapes --------------------------------------------------------------------------------------------------

#[test]
fn every_shape_element_is_read_as_its_geometry() -> Outcome {
    let rows: [(&str, [f64; 4]); 7] = [
        (r#"<path d="M1 2 L11 2 L11 8 Z"/>"#, [1.0, 2.0, 10.0, 6.0]),
        (
            r#"<rect x="1" y="2" width="10" height="6" rx="2"/>"#,
            [1.0, 2.0, 10.0, 6.0],
        ),
        (r#"<circle cx="10" cy="20" r="5"/>"#, [5.0, 15.0, 10.0, 10.0]),
        (r#"<ellipse cx="10" cy="20" rx="6" ry="2"/>"#, [4.0, 18.0, 12.0, 4.0]),
        (r#"<line x1="1" y1="2" x2="4" y2="8"/>"#, [1.0, 2.0, 3.0, 6.0]),
        (r#"<polyline points="1,2 4,2 4,8"/>"#, [1.0, 2.0, 3.0, 6.0]),
        (r#"<polygon points="1 2 4 2 4 8"/>"#, [1.0, 2.0, 3.0, 6.0]),
    ];
    for (element, want) in rows {
        let bounds = only(shapes(&svg(element))?)?.path().bounds()?;
        let got = [bounds.x, bounds.y, bounds.width, bounds.height];
        assert!(
            got.iter().zip(want).all(|(g, w)| (g - w).abs() < 1e-9),
            "{element}: {got:?}"
        );
    }
    Ok(())
}

// What is skipped, what is refused ----------------------------------------------------------------------------------

#[test]
fn what_does_not_draw_is_skipped() -> Outcome {
    let skipped = svg(
        r#"<!-- a comment --><title>t</title><desc>d</desc><metadata><x/></metadata><defs/><defs>
        </defs><foo:bar xmlns:foo="urn:other"><rect width="1" height="1"/></foo:bar>
        <rect width="3" height="3" fill-rule="nonzero" data-x="1" aria-hidden="true" style="isolation:isolate" clip-path="none" display="inline"/>"#,
    );
    assert_eq!(shapes(&skipped)?.len(), 1);
    Ok(())
}

/// What each case is, the document or the body of one, and the words its error must hold.
const NOT_READ: [(&str, &str, &[&str]); 27] = [
    (
        "evenodd",
        r#"<path id="ring" fill-rule="evenodd" d="M0 0 L5 0 L5 5 Z"/>"#,
        &["<path id=\"ring\">", "fill-rule=\"evenodd\"", "nonzero", "split"],
    ),
    (
        "evenodd in style",
        r#"<path style="fill-rule:evenodd" d="M0 0 L5 0 L5 5 Z"/>"#,
        &["fill-rule=\"evenodd\""],
    ),
    ("use", r##"<use href="#a"/>"##, &["<use>", "write the shape out"]),
    ("text", "<text>hi</text>", &["<text>", "outlines"]),
    ("style element", "<style>.a{fill:red}</style>", &["<style>", "CSS"]),
    (
        "linear gradient",
        r#"<linearGradient id="g"/>"#,
        &["<linearGradient id=\"g\">", "gradients"],
    ),
    (
        "gradient fill",
        r#"<rect width="1" height="1" fill="url(#g)"/>"#,
        &["fill=\"url(#g)\"", "gradients"],
    ),
    (
        "clip path attribute",
        r#"<rect width="1" height="1" clip-path="url(#c)"/>"#,
        &["clip-path", "clip paths"],
    ),
    ("clip path element", "<clipPath/>", &["<clipPath>", "clip paths"]),
    (
        "mask",
        r#"<rect width="1" height="1" mask="url(#m)"/>"#,
        &["mask", "masks"],
    ),
    (
        "filter",
        r#"<rect width="1" height="1" filter="url(#f)"/>"#,
        &["filter"],
    ),
    (
        "dashes",
        r#"<rect width="1" height="1" stroke-dasharray="2 1"/>"#,
        &["stroke-dasharray", "dashes"],
    ),
    (
        "hidden",
        r#"<rect width="1" height="1" display="none"/>"#,
        &["display=\"none\"", "delete"],
    ),
    (
        "defs with content",
        r#"<defs><rect id="a" width="1" height="1"/></defs>"#,
        &["<defs>", "<rect id=\"a\">"],
    ),
    (
        "group opacity",
        r#"<g opacity="0.5"><rect width="1" height="1"/></g>"#,
        &["<g>", "opacity", "set opacity on the shapes"],
    ),
    ("nested svg", "<svg/>", &["<svg>", "nested"]),
    (
        "colour name",
        r#"<rect width="1" height="1" fill="red"/>"#,
        &["fill=\"red\"", "#rrggbb"],
    ),
    (
        "percent",
        r#"<rect width="50%" height="1"/>"#,
        &["width=\"50%\"", "pixels"],
    ),
    (
        "bad transform",
        r#"<rect width="1" height="1" transform="spin(3)"/>"#,
        &["transform", "spin"],
    ),
    (
        "two radii",
        r#"<rect width="9" height="9" rx="1" ry="2"/>"#,
        &["rx", "ry"],
    ),
    ("image", "<image/>", &["<image>"]),
    (
        "stylesheet instruction",
        r#"<?xml-stylesheet type="text/css" href="a.css"?><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"/>"#,
        &["xml-stylesheet", "CSS"],
    ),
    (
        "paint order",
        r#"<rect width="1" height="1" paint-order="stroke"/>"#,
        &["paint-order=\"stroke\""],
    ),
    (
        "non-scaling stroke",
        r#"<rect width="1" height="1" vector-effect="non-scaling-stroke"/>"#,
        &["vector-effect"],
    ),
    (
        "transform origin attribute",
        r#"<rect width="1" height="1" transform="rotate(45)" transform-origin="5 5"/>"#,
        &["transform-origin=\"5 5\"", "transform"],
    ),
    (
        "transform origin in style",
        r#"<rect width="1" height="1" style="transform-origin:5px 5px"/>"#,
        &["transform-origin=\"5px 5px\""],
    ),
    (
        "css transform",
        r#"<rect width="1" height="1" style="transform:rotate(45deg)"/>"#,
        &["transform", "style"],
    ),
];

#[test]
fn what_is_not_read_is_an_error_that_names_it() -> Outcome {
    for (rule, body, words) in NOT_READ {
        let text = if body.starts_with("<?xml") {
            body.to_owned()
        } else {
            svg(body)
        };
        let (name, message) = parameter_refusal(shapes(&text)).map_err(|error| format!("{rule}: {error}"))?;
        assert_eq!(name, "svg", "{rule}");
        for word in words {
            assert!(message.contains(word), "{rule}: {message:?} lacks {word:?}");
        }
    }
    Ok(())
}

#[test]
fn a_document_that_is_not_an_svg_is_an_error() {
    for (rule, text) in [
        ("not xml", "<svg"),
        ("no namespace", r#"<svg viewBox="0 0 1 1"/>"#),
        ("another root", r#"<html xmlns="http://www.w3.org/2000/svg"/>"#),
        ("no size", r#"<svg xmlns="http://www.w3.org/2000/svg"/>"#),
    ] {
        assert!(
            matches!(Drawing::from_svg(text), Err(Error::Parameter { .. })),
            "{rule}"
        );
    }
}

#[test]
fn a_doctype_is_refused_with_what_to_do() -> Outcome {
    for (rule, text) in [
        (
            "with an entity",
            r#"<!DOCTYPE svg [<!ENTITY a "b">]><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"/>"#,
        ),
        (
            "an Illustrator header",
            r#"<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"/>"#,
        ),
    ] {
        let (name, message) = parameter_refusal(Drawing::from_svg(text)).map_err(|error| format!("{rule}: {error}"))?;
        assert_eq!(name, "svg", "{rule}");
        assert!(
            message.contains("<!DOCTYPE>") && message.contains("remove"),
            "{rule}: {message:?}"
        );
    }
    Ok(())
}

#[test]
fn a_rounded_corner_larger_than_half_a_side_is_cut_to_half_the_shorter_side() -> Outcome {
    // 10 by 4: half the shorter side is 2, so any larger radius is the radius 2
    let want = Path::rect(0.0, 0.0, 10.0, 4.0, 2.0)?.to_svg_path();
    for corners in [r#"rx="50""#, r#"ry="30""#, r#"rx="3" ry="3""#] {
        let rect = only(shapes(&svg(&format!(r#"<rect width="10" height="4" {corners}/>"#)))?)?;
        assert_eq!(rect.path().to_svg_path(), want, "{corners}");
    }
    Ok(())
}

// Limits ------------------------------------------------------------------------------------------------------------

#[test]
fn the_most_elements_are_read_and_one_more_is_refused() -> Outcome {
    let with = |count: usize| svg(&r#"<rect width="1" height="1"/>"#.repeat(count - 1));
    let most = 20_000;
    assert_eq!(shapes(&with(most))?.len(), most - 1);
    let (_, message) = parameter_refusal(shapes(&with(most + 1)))?;
    assert!(message.contains("MAX_SVG_ELEMENTS"), "{message}");
    Ok(())
}

#[test]
fn the_deepest_nesting_is_read_and_one_deeper_is_refused() -> Outcome {
    // the root is level 1, the groups the next levels, the rect the last
    let nested = |levels: usize| {
        let groups = levels - 2;
        svg(&format!(
            "{}{}{}",
            "<g>".repeat(groups),
            r#"<rect width="1" height="1"/>"#,
            "</g>".repeat(groups)
        ))
    };
    let deepest = 128;
    assert_eq!(shapes(&nested(deepest))?.len(), 1);
    let (_, message) = parameter_refusal(shapes(&nested(deepest + 1)))?;
    assert!(message.contains("MAX_SVG_DEPTH"), "{message}");
    Ok(())
}

#[test]
fn a_document_at_the_element_limit_still_saves_to_json_and_loads_again() -> Outcome {
    let body = r##"<rect width="1" height="1" fill="#112233" stroke="#445566" opacity="0.5"/>"##.repeat(19_999);
    let drawing = Drawing::from_svg(&svg(&body))?;
    let again = Drawing::from_json(&drawing.to_json()?, &bullterrier_vector::Inputs::new())?;
    assert_eq!(again.shapes(&Values::new())?.len(), 19_999);
    Ok(())
}

#[test]
fn a_document_a_node_past_the_limit_is_refused_before_it_is_read_into_a_tree() -> Outcome {
    // `<a/>` is not an element the import reads: only the count of nodes can refuse this document first
    let head = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1">"#;
    let (_, message) = parameter_refusal(Drawing::from_svg(&format!("{head}{}</svg>", "<a/>".repeat(80_000))))?;
    assert!(
        message.contains("MAX_SVG_ELEMENTS") && !message.contains("not read"),
        "{message}"
    );
    Ok(())
}

#[test]
fn a_document_of_the_most_bytes_is_read_and_one_byte_more_is_refused() -> Outcome {
    let (head, tail) = (
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1">"#,
        "</svg>",
    );
    let most = 16 * 1024 * 1024;
    let padded = |bytes: usize| format!("{head}{}{tail}", " ".repeat(bytes - head.len() - tail.len()));
    assert_eq!(shapes(&padded(most))?.len(), 0);
    let (_, message) = parameter_refusal(Drawing::from_svg(&padded(most + 1)))?;
    assert!(message.contains("MAX_DOCUMENT_BYTES"), "{message}");
    Ok(())
}
