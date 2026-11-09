//! `clip` and `mask`: a shape cut to a region, or faded by the luminance of other shapes, in the pixels, the SVG, the
//! mesh and the documents. The oracles are by construction: a clip to a pixel-aligned rectangle leaves the unclipped
//! picture inside and the backdrop outside; a flat mask of a grey `g` gives the alpha `g` (the luminance
//! coefficients 0.2126, 0.7152 and 0.0722 worked out by hand in the rows); a gradient mask fades as the gradient does.

#![expect(clippy::cast_precision_loss, reason = "pixel indices are small")]

use crate::common::fonts::fraunces;
use crate::common::mesh::mesh_of;
use crate::common::pixels::{Image, render};
use crate::common::svg_oracle::svg_agrees;
use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{
    Blend, Color, Drawing, Error, Gradient, Inputs, Mesh, Path, Shadow, Size, Stroke, Text, Values, text,
};

const PAPER: &str = "#e8a060";
const INK: &str = "#3070c0";

fn color(text: &str) -> Result<Color, Error> {
    Color::parse(text)
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> Result<Path, Error> {
    Path::rect(x, y, width, height, 0.0)
}

fn solid(path: Path, fill: &str) -> Result<Drawing, Error> {
    Ok(Drawing::from(path).fill(color(fill)?))
}

fn paper() -> Result<Drawing, Error> {
    solid(rect(0.0, 0.0, 40.0, 40.0)?, PAPER)
}

fn disc() -> Result<Drawing, Error> {
    solid(Path::circle((20.0, 20.0), 16.0)?, INK)
}

fn whole() -> Result<Drawing, Error> {
    solid(rect(0.0, 0.0, 40.0, 40.0)?, "#ff0000")
}

/// Whether the two images are the same at every pixel `inside` says is to be compared.
fn same_everywhere(left: &Image, right: &Image, inside: impl Fn(usize, usize) -> bool) -> bool {
    (0..left.height).all(|y| (0..left.width).all(|x| !inside(x, y) || left.pixel(x, y) == right.pixel(x, y)))
}

// Clip ----------------------------------------------------------------------------------------------------------------

#[test]
fn a_clip_to_a_pixel_aligned_rectangle_keeps_the_picture_inside_and_the_backdrop_outside() -> Outcome {
    let clipped = render(
        &Drawing::new([paper()?, disc()?.clip(rect(10.0, 10.0, 20.0, 20.0)?)]),
        40,
        40,
    )?;
    let unclipped = render(&Drawing::new([paper()?, disc()?]), 40, 40)?;
    let backdrop = render(&paper()?, 40, 40)?;
    let inside = |x: usize, y: usize| (10..30).contains(&x) && (10..30).contains(&y);
    assert!(
        same_everywhere(&clipped, &unclipped, inside),
        "inside the region: as unclipped"
    );
    assert!(
        same_everywhere(&clipped, &backdrop, |x, y| !inside(x, y)),
        "outside: the backdrop"
    );
    // the clip cut something: the disc reaches outside the rectangle
    assert!(!same_everywhere(&unclipped, &backdrop, |x, y| !inside(x, y)));
    Ok(())
}

#[test]
fn a_clip_cuts_a_shape_whose_region_edge_is_a_curve_with_the_coverage_of_the_curve() -> Outcome {
    // a disc of radius 10 clips a full square: the coverage is the disc's area, pi r^2, to a pixel's tenth
    let image = render(&whole()?.clip(Path::circle((20.0, 20.0), 10.0)?), 40, 40)?;
    let expected = std::f64::consts::PI * 100.0;
    assert!(
        (image.covered_area() - expected).abs() < 1.0,
        "{}",
        image.covered_area()
    );
    Ok(())
}

#[test]
fn a_clip_region_of_several_overlapping_shapes_is_their_union_whichever_way_round_they_run() -> Outcome {
    let one = Path::from_svg_path("M0 0 L10 0 L10 10 L0 10 Z")?;
    // the second runs the other way: joined as they are, the overlap would wind to zero and be a hole
    let other = Path::from_svg_path("M5 5 L5 15 L15 15 L15 5 Z")?;
    let region = Drawing::new([Drawing::from(one), Drawing::from(other)]);
    let image = render(&whole()?.clip(region), 40, 40)?;
    for (x, y, alpha) in [
        (2, 2, 255),
        (7, 7, 255),
        (12, 12, 255),
        (12, 2, 0),
        (2, 12, 0),
        (20, 20, 0),
    ] {
        assert_eq!(image.alpha(x, y), alpha, "pixel {x},{y}");
    }
    Ok(())
}

#[test]
fn a_region_that_fills_nothing_leaves_nothing_drawn() -> Outcome {
    for (case, region) in [
        ("an empty path", Path::new()),
        ("an open line", Path::from_svg_path("M0 0 L40 40")?),
    ] {
        let image = render(&whole()?.clip(region), 40, 40)?;
        assert!(image.all_pixels_are([0, 0, 0, 0]), "{case}");
    }
    Ok(())
}

#[test]
fn a_second_clip_cuts_to_both_regions() -> Outcome {
    let drawing = whole()?
        .clip(rect(0.0, 0.0, 20.0, 40.0)?)
        .clip(rect(10.0, 0.0, 30.0, 40.0)?);
    let image = render(&drawing, 40, 40)?;
    for (x, alpha) in [(5, 0), (9, 0), (10, 255), (19, 255), (20, 0), (35, 0)] {
        assert_eq!(image.alpha(x, 20), alpha, "column {x}");
    }
    Ok(())
}

#[test]
fn a_clip_cuts_the_shadow_with_the_shape() -> Outcome {
    let square = || solid(rect(10.0, 10.0, 10.0, 10.0)?, "#000000");
    let shadowed = |blur: f64| -> Result<Drawing, Error> {
        Ok(square()?.shadow(Shadow::new().dx(8.0).dy(0.0).blur(blur).color(color("#000000")?)))
    };
    let right_half = rect(20.0, 0.0, 20.0, 40.0)?;
    for blur in [0.0, 6.0] {
        let free = render(&shadowed(blur)?, 40, 40)?;
        let cut = render(&shadowed(blur)?.clip(right_half.clone()), 40, 40)?;
        // the shadow reaches x 18 to 28 and the shape x 10 to 20
        assert!(free.alpha(18, 15) > 0, "blur {blur}: the shadow is there to be cut");
        assert_eq!(cut.alpha(18, 15), 0, "blur {blur}: the shadow left of the region");
        assert_eq!(cut.alpha(15, 15), 0, "blur {blur}: the shape left of the region");
        assert!(cut.alpha(24, 15) > 0, "blur {blur}: the shadow inside the region stays");
        assert_eq!(cut.alpha(24, 15), free.alpha(24, 15), "blur {blur}");
    }
    Ok(())
}

#[test]
fn a_knockout_ignores_a_clip_and_a_mask() -> Outcome {
    let cut = || -> Result<Drawing, Error> { Ok(disc()?.knockout()) };
    let plain = render(&Drawing::new([paper()?, cut()?]), 40, 40)?;
    for (case, drawing) in [
        ("a clip", cut()?.clip(rect(0.0, 0.0, 5.0, 5.0)?)),
        ("a mask", cut()?.mask(solid(rect(0.0, 0.0, 40.0, 40.0)?, "#000000")?)),
    ] {
        let image = render(&Drawing::new([paper()?, drawing]), 40, 40)?;
        assert!(same_everywhere(&image, &plain, |_, _| true), "{case}");
    }
    assert_eq!(plain.alpha(20, 20), 0, "the knockout did cut");
    Ok(())
}

#[test]
fn a_clip_is_per_shape_and_in_the_design_frame() -> Outcome {
    // the region stays where it is when the shape is moved after the clip, as a gradient does
    let moved = whole()?.clip(rect(0.0, 0.0, 10.0, 40.0)?).translate(5.0, 0.0);
    let image = render(&moved, 40, 40)?;
    assert_eq!(image.alpha(7, 20), 255);
    assert_eq!(image.alpha(12, 20), 0);
    Ok(())
}

// Mask ----------------------------------------------------------------------------------------------------------------

#[test]
fn a_flat_mask_sets_the_opacity_by_its_luminance_times_its_alpha() -> Outcome {
    // luminance = 0.2126 R + 0.7152 G + 0.0722 B of the sRGB values, on the colour premultiplied by alpha
    let rows = [
        ("black", "#000000", 0),
        ("dark grey", "#404040", 64),
        ("mid grey", "#808080", 128),
        ("light grey", "#c8c8c8", 200),
        ("white", "#ffffff", 255),
        ("red: 0.2126 * 255", "#ff0000", 54),
        ("green: 0.7152 * 255", "#00ff00", 182),
        ("blue: 0.0722 * 255", "#0000ff", 18),
        ("white at half alpha", "#ffffff80", 128),
        ("mid grey at half alpha: 0.5 * 0.5 * 255", "#80808080", 64),
        ("nothing", "#ffffff00", 0),
    ];
    for (case, mask, alpha) in rows {
        let drawing = whole()?.mask(solid(rect(0.0, 0.0, 40.0, 40.0)?, mask)?);
        let pixel = render(&drawing, 40, 40)?.pixel(20, 20);
        assert!(
            pixel[3].abs_diff(alpha) <= 1,
            "{case}: wanted alpha {alpha}, got {pixel:?}"
        );
        if alpha > 20 {
            assert!(
                pixel[0] >= 253 && pixel[1] <= 2 && pixel[2] <= 2,
                "{case}: the colour stays: {pixel:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn a_white_mask_changes_nothing_and_a_black_one_hides_everything() -> Outcome {
    let with = |fill: &str| -> Result<Image, Error> {
        render(
            &Drawing::new([paper()?, disc()?.mask(solid(rect(-10.0, -10.0, 60.0, 60.0)?, fill)?)]),
            40,
            40,
        )
    };
    let free = render(&Drawing::new([paper()?, disc()?]), 40, 40)?;
    // a layer composites an edge pixel a level off a direct draw at most (the rasteriser's rounding)
    let (white, free_bytes) = (with("#ffffff")?, &free.bytes);
    let most = white
        .bytes
        .iter()
        .zip(free_bytes)
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap_or(0);
    assert!(most <= 1, "white: {most} levels");
    assert!(
        same_everywhere(&with("#000000")?, &render(&paper()?, 40, 40)?, |_, _| true),
        "black"
    );
    Ok(())
}

#[test]
fn a_mask_that_covers_part_of_the_canvas_cuts_the_rest_on_a_canvas_that_is_not_square() -> Outcome {
    let left = solid(rect(0.0, 0.0, 24.0, 30.0)?, "#ffffff")?;
    let drawing = solid(rect(0.0, 0.0, 48.0, 30.0)?, "#ff0000")?.mask(left);
    let image = render(&drawing, 48, 30)?;
    for (x, alpha) in [(2, 255), (22, 255), (26, 0), (46, 0)] {
        assert_eq!(image.alpha(x, 15), alpha, "column {x}");
    }
    Ok(())
}

#[test]
fn a_gradient_mask_fades_the_shape_as_the_gradient_does() -> Outcome {
    let fade = Gradient::linear(
        (0.0, 0.0),
        (40.0, 0.0),
        [(0.0, color("#000000")?), (1.0, color("#ffffff")?)],
    )?;
    let image = render(
        &whole()?.mask(Drawing::from(rect(0.0, 0.0, 40.0, 40.0)?).fill(fade)),
        40,
        40,
    )?;
    for x in 0..40 {
        // sRGB values interpolated, so the luminance is the position
        let wanted = 255.0 * (x as f64 + 0.5) / 40.0;
        let got = f64::from(image.alpha(x, 20));
        assert!((got - wanted).abs() <= 2.0, "column {x}: wanted {wanted}, got {got}");
    }
    Ok(())
}

#[test]
fn a_mask_is_taken_as_painted_with_its_opacity_and_its_own_clip() -> Outcome {
    let half = solid(rect(0.0, 0.0, 40.0, 40.0)?, "#ffffff")?.opacity(0.5);
    assert!(render(&whole()?.mask(half), 40, 40)?.alpha(20, 20).abs_diff(128) <= 1);
    let clipped = solid(rect(0.0, 0.0, 40.0, 40.0)?, "#ffffff")?.clip(rect(0.0, 0.0, 20.0, 40.0)?);
    let image = render(&whole()?.mask(clipped), 40, 40)?;
    assert_eq!((image.alpha(10, 20), image.alpha(30, 20)), (255, 0));
    Ok(())
}

#[test]
fn a_mask_fades_each_shape_on_its_own_not_the_drawing_as_a_group() -> Outcome {
    // two opaque squares overlap at the middle; a half mask gives each alpha 0.5, which painted over each other make
    // 1 - 0.5 * 0.5 = 0.75 there (a group fading would make 0.5)
    let both = Drawing::new([
        solid(rect(0.0, 0.0, 30.0, 40.0)?, "#ff0000")?,
        solid(rect(10.0, 0.0, 30.0, 40.0)?, "#0000ff")?,
    ]);
    let image = render(&both.mask(solid(rect(0.0, 0.0, 40.0, 40.0)?, "#808080")?), 40, 40)?;
    assert!(image.alpha(5, 20).abs_diff(128) <= 1, "one square");
    assert!(image.alpha(20, 20).abs_diff(192) <= 2, "{:?}", image.pixel(20, 20));
    Ok(())
}

#[test]
fn a_mask_fades_the_shadow_with_the_shape() -> Outcome {
    let shadowed = solid(rect(10.0, 10.0, 10.0, 10.0)?, "#000000")?
        .shadow(Shadow::new().dx(8.0).dy(0.0).blur(0.0).color(color("#000000")?));
    let right_half = solid(rect(20.0, 0.0, 20.0, 40.0)?, "#ffffff")?;
    let image = render(&shadowed.mask(right_half), 40, 40)?;
    assert_eq!((image.alpha(15, 15), image.alpha(18, 15)), (0, 0));
    assert_eq!(image.alpha(24, 15), 255);
    Ok(())
}

#[test]
fn a_mask_holds_no_masks_and_a_shape_takes_one() -> Outcome {
    let masked = || -> Result<Drawing, Error> { Ok(whole()?.mask(solid(rect(0.0, 0.0, 40.0, 40.0)?, "#ffffff")?)) };
    for (case, drawing) in [
        ("a mask that is masked", whole()?.mask(masked()?)),
        (
            "a second mask",
            masked()?.mask(solid(rect(0.0, 0.0, 40.0, 40.0)?, "#ffffff")?),
        ),
    ] {
        let (name, message) = parameter_refusal(drawing.shapes(&Values::new()))?;
        assert!(
            name.contains("mask") && name.ends_with("source"),
            "{case}: {name}: {message}"
        );
    }
    // a mask that is clipped is fine
    whole()?
        .mask(whole()?.clip(rect(0.0, 0.0, 5.0, 5.0)?))
        .shapes(&Values::new())?;
    Ok(())
}

#[test]
fn shapes_of_two_steps_are_cut_each_by_its_own_clip_and_mask() -> Outcome {
    let top = solid(rect(0.0, 0.0, 40.0, 20.0)?, "#ff0000")?;
    let bottom = solid(rect(0.0, 20.0, 40.0, 20.0)?, "#0000ff")?;
    let drawing = Drawing::new([
        top.clip(rect(0.0, 0.0, 10.0, 40.0)?),
        bottom.clip(rect(30.0, 0.0, 10.0, 40.0)?),
        top.translate(0.0, 0.0)
            .mask(solid(rect(0.0, 0.0, 40.0, 40.0)?, "#404040")?),
        bottom.mask(solid(rect(0.0, 0.0, 40.0, 40.0)?, "#c0c0c0")?),
    ]);
    // the clips leave the left strip of the top and the right strip of the bottom; the masks fade each whole shape
    let image = render(&drawing, 40, 40)?;
    assert_eq!(image.pixel(5, 10), [255, 0, 0, 255], "top, left: its clip");
    assert_eq!(image.pixel(35, 30), [0, 0, 255, 255], "bottom, right: its clip");
    // the faded shapes show over them; away from the clips they are alone
    assert!(image.alpha(20, 10).abs_diff(64) <= 1, "top, middle: its mask");
    assert!(image.alpha(20, 30).abs_diff(192) <= 1, "bottom, middle: its mask");
    Ok(())
}

// The shapes ----------------------------------------------------------------------------------------------------------

#[test]
fn every_shape_of_a_step_shares_one_region_and_one_mask() -> Outcome {
    let stacked = Drawing::new([whole()?, disc()?]);
    let clipped = stacked.clip(rect(0.0, 0.0, 10.0, 10.0)?).shapes(&Values::new())?;
    assert!(clipped.iter().all(|shape| shape.style().clip().is_some()));
    let (first, second) = (clipped[0].style().clip(), clipped[1].style().clip());
    assert!(std::ptr::eq(first.ok_or("no clip")?, second.ok_or("no clip")?));
    let masked = stacked.mask(whole()?).shapes(&Values::new())?;
    assert_eq!(masked[0].style().mask().map(<[_]>::len), Some(1));
    let (first, second) = (masked[0].style().mask(), masked[1].style().mask());
    assert!(std::ptr::eq(first.ok_or("no mask")?, second.ok_or("no mask")?));
    // the mask's shapes are as painted
    assert_eq!(
        masked[0].style().mask().map(|shapes| shapes[0].style().fill().clone()),
        Some(color("#ff0000")?.into())
    );
    Ok(())
}

#[test]
fn a_shape_with_a_clip_and_a_mask_put_back_into_a_drawing_draws_as_it_was() -> Outcome {
    let fade = Gradient::linear(
        (0.0, 0.0),
        (40.0, 0.0),
        [(0.0, color("#000000")?), (1.0, color("#ffffff")?)],
    )?;
    let drawing = Drawing::new([
        disc()?
            .stroke(Stroke::new(color("#202020")?).width(3.0))
            .opacity(0.8)
            .blend(Blend::Multiply)
            .shadow(Shadow::new().dx(4.0).blur(0.0))
            .clip(rect(5.0, 5.0, 30.0, 30.0)?)
            .mask(Drawing::from(rect(0.0, 0.0, 40.0, 40.0)?).fill(fade)),
        whole()?.clip(Path::circle((20.0, 20.0), 8.0)?),
    ]);
    let shapes = drawing.shapes(&Values::new())?;
    for shape in &shapes {
        let again = Drawing::from(shape.clone()).shapes(&Values::new())?;
        assert_eq!(again.len(), 1);
        assert_eq!(again[0].path(), shape.path());
        assert_eq!(again[0].style(), shape.style(), "shape {}", shape.index());
    }
    Ok(())
}

// The documents -------------------------------------------------------------------------------------------------------

#[test]
fn a_clip_and_a_mask_write_their_drawing_and_read_back_to_the_same_shapes() -> Outcome {
    let drawing = Drawing::new([
        whole()?.clip(rect(0.0, 0.0, 10.0, 40.0)?),
        disc()?.mask(Drawing::new([
            solid(rect(0.0, 0.0, 20.0, 40.0)?, "#ffffff")?,
            solid(rect(20.0, 0.0, 20.0, 40.0)?, "#808080")?,
        ])),
    ]);
    let text = drawing.to_json()?;
    assert!(
        text.contains(r#""clip":{"region""#) && text.contains(r#""mask":{"source""#),
        "{text}"
    );
    let again = Drawing::from_json(&text, &Inputs::new())?;
    assert_eq!(again.to_json()?, text);
    assert_eq!(again.shapes(&Values::new())?, drawing.shapes(&Values::new())?);
    Ok(())
}

// The SVG -------------------------------------------------------------------------------------------------------------

fn agree(case: &str, ink: Drawing) -> Outcome {
    let paper = solid(rect(0.0, 0.0, 120.0, 120.0)?, PAPER)?;
    let agreement = svg_agrees(&Drawing::new([paper, ink]), 120, 120)?;
    assert!(
        agreement.interior <= 4 && agreement.edge_over_40 * 100 <= agreement.edge_pixels.max(1),
        "{case}: {agreement:?}"
    );
    Ok(())
}

fn blob() -> Result<Drawing, Error> {
    solid(Path::circle((60.0, 60.0), 40.0)?, INK)
}

fn fade_across() -> Result<Gradient, Error> {
    Gradient::linear(
        (20.0, 0.0),
        (100.0, 0.0),
        [(0.0, color("#000000")?), (1.0, color("#ffffff")?)],
    )
}

#[test]
fn the_svg_cuts_and_fades_as_the_pixels_do() -> Outcome {
    let region = || Path::circle((50.0, 60.0), 30.0);
    let hole = Drawing::from(rect(0.0, 0.0, 120.0, 120.0)?).fill(fade_across()?);
    let shadow =
        || -> Result<Shadow, Error> { Ok(Shadow::new().dx(14.0).dy(10.0).blur(8.0).color(color("#802020c0")?)) };
    let rows = [
        ("clip", blob()?.clip(region()?)),
        ("a clip that fills nothing", blob()?.clip(Path::new())),
        ("mask", blob()?.mask(solid(rect(0.0, 0.0, 120.0, 120.0)?, "#a0a0a0")?)),
        ("mask with a gradient", blob()?.mask(hole.clone())),
        (
            "mask of coloured, translucent shapes",
            blob()?.mask(Drawing::new([
                solid(Path::circle((40.0, 60.0), 30.0)?, "#ff8000c0")?,
                solid(Path::circle((80.0, 60.0), 30.0)?, "#00a0ff")?.opacity(0.7),
            ])),
        ),
        (
            "clip with a hard shadow",
            blob()?.shadow(shadow()?.blur(0.0)).clip(region()?),
        ),
        ("clip with a blurred shadow", blob()?.shadow(shadow()?).clip(region()?)),
        (
            "mask with a shadow",
            blob()?.shadow(shadow()?.blur(0.0)).mask(hole.clone()),
        ),
        (
            "clip, with opacity and blend",
            blob()?.opacity(0.7).blend(Blend::Multiply).clip(region()?),
        ),
        (
            "mask, with opacity and blend",
            blob()?.opacity(0.7).blend(Blend::Multiply).mask(hole.clone()),
        ),
        ("both", blob()?.clip(region()?).mask(hole.clone())),
        (
            "a stroke",
            blob()?
                .stroke(Stroke::new(color("#202020")?).width(6.0))
                .clip(region()?),
        ),
    ];
    for (case, ink) in rows {
        agree(case, ink)?;
    }
    Ok(())
}

/// Every `id` of the SVG of `drawing`, in order.
fn ids(drawing: &Drawing) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let svg = drawing.to_svg(Size::px(120.0, 120.0), &Values::new())?;
    let document = roxmltree::Document::parse(&svg)?;
    Ok(document
        .descendants()
        .filter_map(|node| node.attribute("id").map(str::to_owned))
        .collect())
}

#[test]
fn what_a_mask_draws_has_ids_of_its_own() -> Outcome {
    let shadow = Shadow::new().dx(5.0).dy(5.0).blur(6.0).color(color("#00000080")?);
    let gradient = |shift: f64| {
        Gradient::linear(
            (shift, 0.0),
            (shift + 50.0, 0.0),
            [(0.0, color("#000000")?), (1.0, color("#ffffff")?)],
        )
    };
    // shape 0 of the drawing and shape 0 of the mask both have a blurred shadow and a gradient fill
    let source = Drawing::from(rect(0.0, 0.0, 120.0, 120.0)?)
        .fill(gradient(0.0)?)
        .shadow(shadow.clone())
        .stroke(Stroke::new(gradient(10.0)?));
    let drawing = Drawing::new([blob()?
        .fill(gradient(20.0)?)
        .stroke(Stroke::new(gradient(30.0)?))
        .shadow(shadow)
        .mask(source)]);
    let found = ids(&drawing)?;
    // the mask, and a shadow and two gradients each for the mask's shape and the drawing's
    assert_eq!(found.len(), 7, "{found:?}");
    let mut sorted = found.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        found.len(),
        "every id of the document is its own: {found:?}"
    );
    Ok(())
}

#[test]
fn the_ids_of_a_clip_in_a_mask_are_their_own_too() -> Outcome {
    let source = solid(rect(0.0, 0.0, 120.0, 120.0)?, "#ffffff")?.clip(Path::circle((60.0, 60.0), 50.0)?);
    let found = ids(&blob()?.clip(Path::circle((50.0, 60.0), 30.0)?).mask(source))?;
    let mut sorted = found.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!((sorted.len(), found.len()), (found.len(), 3), "{found:?}");
    Ok(())
}

// Text: a clip or a mask is made once ---------------------------------------------------------------------------------

/// Eighty-eight glyphs of the test font in one line.
fn headline() -> Result<Drawing, Error> {
    let font = fraunces()?;
    Ok(text("HIFCTAVeflo".repeat(8), Text::new(&font).size(30.0))
        .fill(color("#c02020")?)
        .translate(0.0, 80.0))
}

fn fade_text() -> Result<Drawing, Error> {
    let across = Gradient::linear(
        (0.0, 0.0),
        (1900.0, 0.0),
        [(0.0, color("#ffffff")?), (1.0, color("#000000")?)],
    )?;
    Ok(Drawing::from(rect(0.0, 0.0, 1920.0, 1080.0)?).fill(across))
}

#[test]
fn a_headline_faded_by_one_mask_writes_one_mask_that_every_glyph_uses() -> Outcome {
    let shapes = headline()?.shapes(&Values::new())?;
    assert!(shapes.len() >= 80);
    let drawing = headline()?.mask(fade_text()?).clip(rect(0.0, 0.0, 1000.0, 200.0)?);
    let svg = drawing.to_svg(Size::px(1920.0, 1080.0), &Values::new())?;
    let document = roxmltree::Document::parse(&svg)?;
    let count = |name: &str| document.descendants().filter(|node| node.has_tag_name(name)).count();
    assert_eq!((count("mask"), count("clipPath")), (1, 1));
    let users = document
        .descendants()
        .filter(|node| {
            node.attribute("mask")
                .is_some_and(|value| value.starts_with("url(#mask-"))
        })
        .count();
    assert!(users >= 80, "{users} elements use the mask");
    Ok(())
}

#[test]
fn a_headline_faded_by_one_mask_draws_at_full_hd_and_equals_its_glyphs_masked_one_by_one() -> Outcome {
    // one mask is built, and charged once: 88 charges of 2 million pixels would pass MAX_FILTER_PIXELS
    let faded = headline()?.mask(fade_text()?);
    let whole_image = render(&faded, 1920, 1080)?;
    assert!(whole_image.covered_area() > 100.0);
    // the same at a size where each glyph can be drawn apart: the glyph masked alone, laid over the ones before
    let small = |drawing: &Drawing| render(drawing, 480, 270);
    let together = small(&faded)?;
    let shapes = headline()?.shapes(&Values::new())?;
    let mut laid = vec![[0.0_f64; 4]; 480 * 270];
    for shape in &shapes {
        let one = small(&Drawing::from(shape.clone()).mask(fade_text()?))?;
        for (at, pixel) in one.bytes.as_chunks::<4>().0.iter().enumerate() {
            let alpha = f64::from(pixel[3]) / 255.0;
            let below = laid[at];
            let out_alpha = alpha + below[3] * (1.0 - alpha);
            for channel in 0..3 {
                // premultiplied colour
                laid[at][channel] = f64::from(pixel[channel]) / 255.0 * alpha + below[channel] * (1.0 - alpha);
            }
            laid[at][3] = out_alpha;
        }
    }
    let mut worst = 0.0_f64;
    for (at, pixel) in together.bytes.as_chunks::<4>().0.iter().enumerate() {
        worst = worst.max((f64::from(pixel[3]) / 255.0 - laid[at][3]).abs() * 255.0);
    }
    assert!(worst <= 2.0, "the largest difference in alpha: {worst}");
    Ok(())
}

#[test]
fn a_masked_headline_rebuilt_from_its_shapes_still_makes_one_mask_and_one_clip() -> Outcome {
    // `Drawing::from(shape)` rebuilds each shape's own step: the masks are equal, though no longer one allocation
    let faded = headline()?.mask(fade_text()?).clip(rect(0.0, 0.0, 1000.0, 200.0)?);
    let rebuilt = Drawing::new(faded.shapes(&Values::new())?.into_iter().map(Drawing::from));
    let size = Size::px(1920.0, 1080.0);
    let svg = rebuilt.to_svg(size, &Values::new())?;
    let document = roxmltree::Document::parse(&svg)?;
    let count = |name: &str| document.descendants().filter(|node| node.has_tag_name(name)).count();
    assert_eq!((count("mask"), count("clipPath")), (1, 1));
    // 88 masks of 2 million pixels would pass MAX_FILTER_PIXELS
    let bytes = rebuilt.to_rgba(size, &Values::new())?;
    assert!(
        bytes == faded.to_rgba(size, &Values::new())?,
        "the rebuilt headline draws differently"
    );
    Ok(())
}

#[test]
fn a_mask_costs_the_canvas_once_for_each_distinct_mask_and_is_refused_past_the_budget() -> Outcome {
    // two masks of 8200 by 8200 pixels pass 2^27 pixels together, one does not
    let width = 8200.0;
    let strip = |top: f64| rect(0.0, top, 50.0, 50.0);
    let mask = |left: f64| solid(rect(left, 0.0, 50.0, 50.0)?, "#ffffff");
    let size = Size::px(width, width);
    // two shapes share the one mask of their step: charged once
    let one = Drawing::new([solid(strip(0.0)?, "#ff0000")?, solid(strip(100.0)?, "#ff0000")?]).mask(mask(0.0)?);
    one.to_rgba(size, &Values::new())?;
    let two = Drawing::new([
        solid(strip(0.0)?, "#ff0000")?.mask(mask(0.0)?),
        solid(strip(100.0)?, "#ff0000")?.mask(mask(10.0)?),
    ]);
    let (name, message) = parameter_refusal(two.to_rgba(size, &Values::new()))?;
    assert!(
        message.contains("MAX_FILTER_PIXELS") && message.contains("mask"),
        "{name}: {message}"
    );
    Ok(())
}

// The mesh ------------------------------------------------------------------------------------------------------------

#[test]
fn a_mesh_refuses_a_clip_and_a_mask_naming_the_step_and_the_shape() -> Outcome {
    let plain = || solid(rect(0.0, 0.0, 10.0, 10.0)?, "#ff0000");
    let cases = [
        ("clip", plain()?.clip(rect(0.0, 0.0, 5.0, 5.0)?)),
        ("mask", plain()?.mask(solid(rect(0.0, 0.0, 5.0, 5.0)?, "#ffffff")?)),
    ];
    for (word, drawing) in cases {
        let mut mesh = Mesh::new();
        let both = Drawing::new([plain()?, drawing]);
        let (name, message) = parameter_refusal(both.to_mesh(Size::px(20.0, 20.0), &Values::new(), &mut mesh))?;
        assert!(
            name == word && message.contains("shape 1") && message.contains("to_rgba"),
            "{word}: {name}: {message}"
        );
        assert_eq!(mesh.vertices(), []);
    }
    // a knockout ignores them, so it is not refused
    mesh_of(&plain()?.knockout().clip(rect(0.0, 0.0, 5.0, 5.0)?), 20.0, 20.0)?;
    Ok(())
}
