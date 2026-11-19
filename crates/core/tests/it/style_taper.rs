//! `stroke(profile=)`: a stroke whose width follows a profile along each sub-path, drawn as a filled outline by the SVG,
//! the pixels and the mesh alike. The oracles: the region `Path::tapered_outline` paints (proven in `path_taper`) by its
//! area and a point test, a second renderer (`resvg`) for the SVG, and what a plain round stroke paints at the same
//! places (a pixel at a zero-factor tip is background, where a plain stroke covers it).

use crate::common::mesh::{area, covering, mesh_of, triangles};
use crate::common::pixels::render;
use crate::common::svg_oracle::{body, element, svg_agrees};
use crate::common::{Outcome, format_refusal, net_area, parameter_refusal, rings};
use bullterrier_vector::{
    Blend, Color, Drawing, Expr, Font, Inputs, Join, Param, Path, Shadow, Size, Stroke, Text, Values, text,
};

const BLACK: Color = Color::hex(0);
const RED: Color = Color::hex(0xff_00_00);
/// The tolerance every output draws a tapered stroke's round parts to, in design pixels.
const TOLERANCE: f64 = 0.01;
type Stops = Vec<(f64, f64)>;
const TAPER: [(f64, f64); 3] = [(0.0, 0.0), (0.5, 1.0), (1.0, 0.0)];

fn line() -> Result<Path, bullterrier_vector::Error> {
    Path::line((10.0, 50.0), (90.0, 50.0))
}

/// The line stroked 20 wide with the taper, nothing else painted.
fn brush() -> Result<Drawing, bullterrier_vector::Error> {
    Ok(Drawing::from(line()?).stroke(Stroke::new(BLACK).width(20.0).profile(TAPER)))
}

fn region() -> Result<Path, bullterrier_vector::Error> {
    line()?.tapered_outline(20.0, &TAPER, TOLERANCE)
}

// The pixels --------------------------------------------------------------------------------------------------------

#[test]
fn the_pixels_cover_the_tapered_region_and_a_zero_factor_tip_is_background() -> Outcome {
    let image = render(&brush()?, 100, 100)?;
    // mid-stroke is painted; ten pixels above the center is the edge of the widest place
    assert_eq!(image.pixel(50, 50), [0, 0, 0, 255]);
    assert!(image.alpha(50, 41) > 200 && image.alpha(50, 38) == 0);
    // near the start the disc is a fraction of a pixel wide: a plain round stroke would cover (15, 45)
    assert_eq!(image.alpha(15, 45), 0);
    let plain = render(&Drawing::from(line()?).stroke(Stroke::new(BLACK).width(20.0)), 100, 100)?;
    assert_eq!(plain.alpha(15, 45), 255);
    // the coverage is the area of the region
    let wanted = net_area(&rings(&region()?)?);
    assert!(
        (image.covered_area() - wanted).abs() < 0.01 * wanted,
        "{} {wanted}",
        image.covered_area()
    );
    Ok(())
}

#[test]
fn the_fill_is_below_the_stroke_and_a_hole_of_a_letter_stays_open() -> Outcome {
    let font = Font::bundled(Font::bundled_names().first().ok_or("no bundled font")?)?;
    let letter = text("O", Text::new(&font).size(80.0))
        .translate(10.0, 90.0)
        .fill(RED)
        .stroke(Stroke::new(BLACK).width(10.0).profile([(0.0, 1.0), (1.0, 1.0)]));
    let flat = text("O", Text::new(&font).size(80.0))
        .translate(10.0, 90.0)
        .fill(RED)
        .stroke(Stroke::new(BLACK).width(10.0));
    let (tapered, plain) = (render(&letter, 100, 100)?, render(&flat, 100, 100)?);
    // a profile of one width everywhere is a plain stroke (round joins and ends), to the anti-aliasing of the edges
    let off = tapered
        .bytes
        .iter()
        .zip(&plain.bytes)
        .filter(|(a, b)| a.abs_diff(**b) > 40)
        .count();
    assert!(off * 100 < tapered.bytes.len(), "{off} differing bytes");
    // the middle of the O is neither filled nor stroked
    let middle = letter.bounds(&Values::new())?;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a pixel inside the canvas"
    )]
    let (x, y) = (
        (middle.x + middle.width / 2.0) as usize,
        (middle.y + middle.height / 2.0) as usize,
    );
    assert_eq!(tapered.alpha(x, y), 0);
    assert!(plain.alpha(x, y) == 0 && tapered.covered_area() > 500.0);
    Ok(())
}

#[test]
fn a_sliced_path_tapers_at_the_cut_whichever_step_comes_first() -> Outcome {
    let before = Drawing::from(line()?.slice(0.0, 0.5, None)?).stroke(Stroke::new(BLACK).width(20.0).profile(TAPER));
    let after = brush()?.slice(0.0, 0.5);
    let (a, b) = (render(&before, 100, 100)?, render(&after, 100, 100)?);
    assert_eq!(a.bytes, b.bytes);
    // the tip is where the path now ends, at x = 50; the unsliced stroke is widest there
    assert_eq!(a.alpha(48, 45), 0);
    assert_eq!(render(&brush()?, 100, 100)?.alpha(48, 45), 255);
    Ok(())
}

// The SVG -----------------------------------------------------------------------------------------------------------

#[test]
fn a_profile_stroke_is_a_filled_path_of_the_region_and_a_fill_is_written_below_it() -> Outcome {
    let outline = region()?.to_svg_path();
    assert_eq!(
        body(&brush()?, 100.0)?,
        vec![element("path", &[("d", &outline), ("fill", "#000000")], vec![])]
    );
    let both = Drawing::from(Path::rect(10.0, 40.0, 80.0, 20.0, 0.0)?)
        .fill(RED)
        .stroke(Stroke::new(BLACK).width(10.0).profile([(0.0, 1.0)]));
    let written = body(&both, 100.0)?;
    assert_eq!(written.len(), 2);
    assert_eq!(written[0].attributes.get("fill").map(String::as_str), Some("#ff0000"));
    assert!(!written[0].attributes.contains_key("stroke"));
    assert_eq!(written[1].attributes.get("fill").map(String::as_str), Some("#000000"));
    assert!(!written[1].attributes.contains_key("stroke-width"));
    Ok(())
}

#[test]
fn the_written_region_has_the_area_of_the_kernel_region_on_any_canvas() -> Outcome {
    for side in [100.0, 300.0] {
        let written = body(&brush()?, side)?;
        let d = written[0].attributes.get("d").ok_or("no d")?;
        let drawn = net_area(&rings(&Path::from_svg_path(d)?)?);
        // a drawing without a frame is at one pixel to the design pixel, whatever the canvas
        let wanted = net_area(&rings(&region()?)?);
        assert!((drawn - wanted).abs() < 1e-6 * wanted, "{side}: {drawn} {wanted}");
    }
    Ok(())
}

#[test]
fn the_group_applies_a_blend_once_to_the_fill_and_the_stroke() -> Outcome {
    let drawing = Drawing::from(Path::rect(10.0, 40.0, 80.0, 20.0, 0.0)?)
        .fill(RED)
        .stroke(Stroke::new(BLACK).width(10.0).profile(TAPER))
        .blend(Blend::Multiply);
    let written = body(&drawing, 100.0)?;
    assert_eq!(written.len(), 1);
    assert_eq!(written[0].name, "g");
    assert_eq!(written[0].children.len(), 2);
    Ok(())
}

#[test]
fn the_svg_draws_what_the_pixels_draw_through_every_effect() -> Outcome {
    let paper = Drawing::from(Path::rect(0.0, 0.0, 120.0, 120.0, 0.0)?).fill(Color::hex(0xee_ee_cc));
    let blob = || Path::rect(20.0, 40.0, 80.0, 30.0, 0.0);
    let half = Color::parse("#0000ff80")?;
    let stroke = |paint: Color| Stroke::new(paint).width(16.0).profile(TAPER);
    let ramp =
        bullterrier_vector::Gradient::linear((20.0, 0.0), (100.0, 0.0), [(0.0, RED), (1.0, Color::hex(0x00_00_ff))])?;
    let rows = [
        ("stroke alone", Drawing::from(line()?).stroke(stroke(BLACK))),
        (
            "fill, translucent stroke, shadow, blend",
            Drawing::from(blob()?)
                .fill(RED)
                .stroke(stroke(half))
                .shadow(Shadow::new().dx(4.0).dy(6.0).blur(8.0))
                .blend(Blend::Multiply),
        ),
        (
            "hard shadow and opacity",
            Drawing::from(blob()?)
                .fill(RED)
                .stroke(stroke(half))
                .shadow(Shadow::new().dx(6.0).dy(6.0).blur(0.0))
                .opacity(0.6),
        ),
        (
            "gradient stroke",
            Drawing::from(blob()?)
                .fill(Color::NONE)
                .stroke(Stroke::new(ramp).width(16.0).profile(TAPER)),
        ),
        (
            "knockout",
            Drawing::new([
                Drawing::from(Path::rect(0.0, 0.0, 120.0, 120.0, 0.0)?).fill(RED),
                Drawing::from(blob()?)
                    .fill(Color::NONE)
                    .stroke(stroke(BLACK))
                    .knockout(),
            ]),
        ),
    ];
    for (case, drawing) in rows {
        let drawing = Drawing::new([paper.clone(), drawing]);
        let agreement = svg_agrees(&drawing, 120, 120)?;
        // a tip thinner than a pixel is covered by less than the 24 levels that make an edge, and the two renderers
        // cover it differently by up to 10 levels (measured on a half-transparent stroke); a missing or doubled
        // translucent stroke is 128 levels
        assert!(
            agreement.interior <= 10 && agreement.edge_over_40 * 100 <= agreement.edge_pixels,
            "{case}: {agreement:?}"
        );
    }
    Ok(())
}

// The mesh ----------------------------------------------------------------------------------------------------------

#[test]
fn the_mesh_covers_the_region_once_and_nothing_else() -> Outcome {
    let mesh = mesh_of(&brush()?, 100.0, 100.0)?;
    let found = triangles(&mesh);
    let total: f64 = found.iter().map(area).sum();
    let wanted = net_area(&rings(&region()?)?);
    assert!((total - wanted).abs() < 1e-3 * wanted, "{total} {wanted}");
    assert_eq!(covering(&found, (50.0, 55.0)), Some(1));
    assert_eq!(covering(&found, (15.0, 45.0)), Some(0));
    assert_eq!(covering(&found, (50.0, 38.0)), Some(0));
    Ok(())
}

// The step, the document, the limits ---------------------------------------------------------------------------------

#[test]
fn a_miter_join_or_square_cap_with_a_profile_is_refused_in_the_step() -> Outcome {
    let base = Drawing::from(line()?);
    for stroke in [
        Stroke::new(BLACK).join(Join::Miter).profile(TAPER),
        Stroke::new(BLACK).cap(bullterrier_vector::Cap::Square).profile(TAPER),
    ] {
        let error = base.stroke(stroke).error().map(ToString::to_string);
        assert!(error.ok_or("no error")?.contains("stroke"), "named the step");
    }
    let stops = vec![(0.5, 1.0); 1001];
    assert!(base.stroke(Stroke::new(BLACK).profile(stops)).error().is_some());
    // a position the kernel refuses is refused as the stroke is read, naming the step
    let (place, _) = parameter_refusal(
        base.stroke(Stroke::new(BLACK).profile([(2.0, 1.0)]))
            .shapes(&Values::new()),
    )?;
    assert!(place.contains("profile"), "{place}");
    Ok(())
}

#[test]
fn an_outline_past_the_limit_is_refused_by_every_output_naming_the_shape() -> Outcome {
    let drawing = Drawing::from(line()?).stroke(Stroke::new(BLACK).width(1e14).profile([(0.0, 1.0)]));
    let (place, message) = parameter_refusal(drawing.to_svg(Size::px(100.0, 100.0), &Values::new()))?;
    assert!(
        place.contains("shape 0") && message.contains("MAX_SEGMENTS"),
        "{place}: {message}"
    );
    assert!(drawing.to_rgba(Size::px(100.0, 100.0), &Values::new()).is_err());
    assert!(mesh_of(&drawing, 100.0, 100.0).is_err());
    Ok(())
}

#[test]
fn a_profile_reads_a_param_and_saves_and_loads_with_the_stroke() -> Outcome {
    let peak = Param::new("peak", 1.0, 0.0, 1.0)?;
    let drawing = Drawing::from(line()?).stroke(
        Stroke::new(BLACK)
            .width(20.0)
            .profile([(Expr::from(0.0), Expr::from(0.0)), (Expr::from(1.0), Expr::from(&peak))]),
    );
    let at = |value: f64| -> Result<Option<Stops>, Box<dyn std::error::Error>> {
        Ok(drawing.shapes(&Values::new().set("peak", value))?[0]
            .style()
            .stroke()
            .and_then(|stroke| stroke.profile().map(<[_]>::to_vec)))
    };
    assert_eq!(at(0.5)?, Some(vec![(0.0, 0.0), (1.0, 0.5)]));
    let json = brush()?.to_json()?;
    assert!(json.contains(r#""profile":[[0,0],[0.5,1],[1,0]]"#), "{json}");
    let again = Drawing::from_json(&json, &Inputs::new())?;
    assert_eq!(again.to_json()?, json);
    assert_eq!(again.shapes(&Values::new())?, brush()?.shapes(&Values::new())?);
    let bad = json.replace(
        r#""profile":[[0,0],[0.5,1],[1,0]]"#,
        r#""profile":[[0,1]],"join":"miter""#,
    );
    let (pointer, _) = format_refusal(Drawing::from_json(&bad, &Inputs::new()))?;
    assert!(pointer.contains("join"), "{pointer}");
    Ok(())
}

#[test]
fn a_drawing_made_from_a_shape_keeps_its_profile_and_the_cache_tells_profiles_apart() -> Outcome {
    let shape = brush()?.shapes(&Values::new())?.remove(0);
    let again = Drawing::from(shape.clone()).shapes(&Values::new())?.remove(0);
    assert_eq!(again.style(), shape.style());
    let other = Drawing::from(line()?).stroke(Stroke::new(BLACK).width(20.0).profile([(0.0, 1.0)]));
    let (a, b) = (render(&brush()?, 100, 100)?, render(&other, 100, 100)?);
    assert_ne!(a.bytes, b.bytes);
    Ok(())
}
