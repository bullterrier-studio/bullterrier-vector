//! The style steps of a `Drawing` (`fill`, `stroke`, `opacity`, `blend`) and the same on a `Path`, through the public
//! API: what the shapes drawn carry, and the values the steps refuse at the call and when drawn. Expected values are
//! written by hand.

mod common;

use bullterrier_vector::{Blend, Cap, Color, Drawing, Error, Join, Param, Path, Stroke, StrokeStyle, Style, Values};
use common::{Outcome, parameter_refusal};

const PINK: Color = Color::hex(0xff_4f_9a);
const BLUE: Color = Color::hex(0x2a_5b_ff);

fn square() -> Result<Path, Error> {
    Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)
}

fn line() -> Result<Path, Error> {
    Path::line((0.0, 0.0), (10.0, 5.0))
}

/// The style of the only shape `drawing` draws with `values`.
fn style(drawing: &Drawing, values: &Values) -> Result<Style, Box<dyn std::error::Error>> {
    let shapes = drawing.shapes(values)?;
    match shapes.as_slice() {
        [only] => Ok(only.style().clone()),
        other => Err(format!("expected one shape, got {}", other.len()).into()),
    }
}

fn plain(drawing: &Drawing) -> Result<Style, Box<dyn std::error::Error>> {
    style(drawing, &Values::new())
}

fn held(drawing: &Drawing) -> Result<Error, Box<dyn std::error::Error>> {
    drawing
        .error()
        .cloned()
        .ok_or_else(|| "the drawing holds no error".into())
}

// Fill and stroke ---------------------------------------------------------------------------------------------------

#[test]
fn a_fill_replaces_the_default_paint_and_leaves_no_stroke() -> Outcome {
    for (case, path) in [("closed", square()?), ("open", line()?)] {
        let style = plain(&Drawing::from(path).fill(PINK))?;
        assert_eq!(style.fill(), PINK, "{case}");
        assert_eq!(
            style.stroke(),
            None,
            "{case}: once a fill is set only what is set is painted"
        );
    }
    Ok(())
}

#[test]
fn a_fill_of_none_counts_as_set_so_nothing_is_painted() -> Outcome {
    for (case, path) in [("closed", square()?), ("open", line()?)] {
        let style = plain(&Drawing::from(path).fill(Color::NONE))?;
        assert_eq!((style.fill(), style.stroke()), (Color::NONE, None), "{case}");
    }
    Ok(())
}

#[test]
fn a_stroke_alone_leaves_the_fill_off_and_takes_the_defaults() -> Outcome {
    let style = plain(&Drawing::from(square()?).stroke(BLUE))?;
    assert_eq!(style.fill(), Color::NONE);
    let stroke = style.stroke().ok_or("no stroke")?;
    assert_eq!(stroke.color(), BLUE);
    assert_eq!(stroke.width(), 2.0);
    assert_eq!((stroke.join(), stroke.cap()), (Join::Round, Cap::Round));
    assert_eq!(stroke.miter_limit(), 4.0);
    Ok(())
}

#[test]
fn fill_and_stroke_together_give_both_in_either_order() -> Outcome {
    let a = plain(&Drawing::from(square()?).fill(PINK).stroke(BLUE))?;
    let b = plain(&Drawing::from(square()?).stroke(BLUE).fill(PINK))?;
    assert_eq!(a, b);
    assert_eq!(a.fill(), PINK);
    assert_eq!(a.stroke().map(StrokeStyle::color), Some(BLUE));
    Ok(())
}

#[test]
fn a_stroke_takes_its_width_joins_caps_and_miter_limit() -> Outcome {
    let drawing = Drawing::from(line()?).stroke(
        Stroke::new(BLUE)
            .width(5.5)
            .join(Join::Bevel)
            .cap(Cap::Butt)
            .miter_limit(2.5),
    );
    let style = plain(&drawing)?;
    let stroke = style.stroke().ok_or("no stroke")?;
    assert_eq!(
        (stroke.width(), stroke.join(), stroke.cap(), stroke.miter_limit()),
        (5.5, Join::Bevel, Cap::Butt, 2.5)
    );
    Ok(())
}

#[test]
fn every_join_and_cap_is_drawn_as_asked() -> Outcome {
    for join in [Join::Round, Join::Miter, Join::Bevel] {
        for cap in [Cap::Butt, Cap::Round, Cap::Square] {
            let style = plain(&Drawing::from(line()?).stroke(Stroke::new(BLUE).join(join).cap(cap)))?;
            let stroke = style.stroke().ok_or("no stroke")?;
            assert_eq!((stroke.join(), stroke.cap()), (join, cap));
        }
    }
    Ok(())
}

#[test]
fn a_colour_is_kept_as_its_bytes_whatever_the_case_it_was_written_in() -> Outcome {
    let style = plain(&Drawing::from(square()?).fill(Color::parse("#FF4F9A")?))?;
    assert_eq!(style.fill().to_string(), "#ff4f9aff");
    assert_eq!(style.fill(), Color::parse("#ff4f9a")?);
    Ok(())
}

#[test]
fn a_stroke_width_can_follow_a_param() -> Outcome {
    let pop = Param::new("pop", 1.0, 0.0, 10.0)?;
    let drawing = Drawing::from(line()?).stroke(Stroke::new(BLUE).width(24.0 * &pop));
    let width = |values: &Values| -> Result<f64, Box<dyn std::error::Error>> {
        Ok(style(&drawing, values)?.stroke().ok_or("no stroke")?.width())
    };
    assert_eq!(width(&Values::new())?, 24.0);
    assert_eq!(width(&Values::new().set("pop", 0.5))?, 12.0);
    Ok(())
}

#[test]
fn a_stroke_the_step_refuses_is_refused_at_the_call_naming_the_argument() -> Outcome {
    let base = Drawing::from(line()?);
    // (case, drawing, the argument named)
    let rows = [
        ("colour none", base.stroke(Color::NONE), "step 1 (stroke).color"),
        (
            "width 0",
            base.stroke(Stroke::new(BLUE).width(0.0)),
            "step 1 (stroke).width",
        ),
        (
            "negative width",
            base.stroke(Stroke::new(BLUE).width(-1.0)),
            "step 1 (stroke).width",
        ),
        (
            "NaN width",
            base.stroke(Stroke::new(BLUE).width(f64::NAN)),
            "step 1 (stroke).width",
        ),
        (
            "miter limit below 1",
            base.stroke(Stroke::new(BLUE).miter_limit(0.99)),
            "step 1 (stroke).miter_limit",
        ),
        (
            "negative miter limit",
            base.stroke(Stroke::new(BLUE).miter_limit(-4.0)),
            "step 1 (stroke).miter_limit",
        ),
    ];
    for (case, drawing, argument) in rows {
        let (name, message) = parameter_refusal(Err::<(), _>(held(&drawing)?))?;
        assert_eq!(name, argument, "{case}: {message}");
    }
    // the boundaries are accepted
    for accepted in [
        Stroke::new(BLUE).width(f64::MIN_POSITIVE),
        Stroke::new(BLUE).miter_limit(1.0),
        Stroke::new(BLUE).width(1e300),
    ] {
        assert_eq!(base.stroke(accepted).error(), None);
    }
    Ok(())
}

#[test]
fn a_width_or_limit_that_reads_a_param_is_refused_when_drawn() -> Outcome {
    let w = Param::new("w", 2.0, -5.0, 5.0)?;
    let drawing = Drawing::from(line()?).stroke(Stroke::new(BLUE).width(&w).miter_limit(&w + 0.5));
    assert_eq!(drawing.error(), None);
    let (name, message) = parameter_refusal(drawing.shapes(&Values::new().set("w", 0.0)))?;
    assert!(
        name == "step 1 (stroke).width" && message.starts_with("shape 0: "),
        "{name}: {message}"
    );
    // a width of 0.25 passes; the miter limit it works out to, 0.75, does not
    let (name, _) = parameter_refusal(drawing.shapes(&Values::new().set("w", 0.25)))?;
    assert_eq!(name, "step 1 (stroke).miter_limit");
    Ok(())
}

// Opacity and blend -------------------------------------------------------------------------------------------------

#[test]
fn opacity_multiplies() -> Outcome {
    let base = Drawing::from(square()?);
    assert_eq!(plain(&base)?.opacity(), 1.0);
    assert_eq!(plain(&base.opacity(0.5))?.opacity(), 0.5);
    assert_eq!(plain(&base.opacity(0.5).opacity(0.5))?.opacity(), 0.25);
    assert_eq!(plain(&base.opacity(0.0))?.opacity(), 0.0);
    assert_eq!(plain(&base.opacity(1.0))?.opacity(), 1.0);
    // it leaves the paint alone
    assert_eq!(plain(&base.opacity(0.5))?.fill(), Color::hex(0));
    Ok(())
}

#[test]
fn an_alpha_outside_zero_to_one_is_refused_at_the_call_and_when_drawn() -> Outcome {
    let base = Drawing::from(square()?);
    for (case, alpha) in [
        ("above", 1.0001),
        ("below", -0.0001),
        ("NaN", f64::NAN),
        ("infinite", f64::INFINITY),
    ] {
        let drawing = base.opacity(alpha);
        let (name, message) = parameter_refusal(Err::<(), _>(held(&drawing)?))?;
        assert_eq!(name, "step 1 (opacity).alpha", "{case}: {message}");
    }
    let fade = Param::new("fade", 0.5, -5.0, 5.0)?;
    let drawing = base.opacity(&fade);
    assert_eq!(plain(&drawing)?.opacity(), 0.5);
    let (name, message) = parameter_refusal(drawing.shapes(&Values::new().set("fade", 2.0)))?;
    assert!(
        name == "step 1 (opacity).alpha" && message.contains("shape 0"),
        "{name}: {message}"
    );
    Ok(())
}

#[test]
fn blend_sets_the_mode() -> Outcome {
    let base = Drawing::from(square()?);
    assert_eq!(plain(&base)?.blend(), Blend::Normal);
    assert_eq!(plain(&base.blend(Blend::Multiply))?.blend(), Blend::Multiply);
    assert_eq!(
        plain(&base.blend(Blend::Multiply).blend(Blend::Normal))?.blend(),
        Blend::Normal
    );
    Ok(())
}

// On a Path ---------------------------------------------------------------------------------------------------------

#[test]
fn a_style_step_on_a_path_is_the_same_step_on_a_drawing_of_it() -> Outcome {
    let path = square()?;
    let fade = Param::new("fade", 0.5, 0.0, 1.0)?;
    let none = Values::new();
    let pairs = [
        (path.fill(PINK), Drawing::from(path.clone()).fill(PINK)),
        (path.stroke(BLUE), Drawing::from(path.clone()).stroke(BLUE)),
        (
            path.stroke(Stroke::new(BLUE).width(3.0)),
            Drawing::from(path.clone()).stroke(Stroke::new(BLUE).width(3.0)),
        ),
        (path.opacity(0.5), Drawing::from(path.clone()).opacity(0.5)),
        (
            path.blend(Blend::Multiply),
            Drawing::from(path.clone()).blend(Blend::Multiply),
        ),
    ];
    for (on_path, on_drawing) in pairs {
        assert_eq!(on_path.shapes(&none)?, on_drawing.shapes(&none)?);
    }
    let faded = Values::new().set("fade", 0.25);
    assert_eq!(
        path.opacity(&fade).shapes(&faded)?,
        Drawing::from(path.clone()).opacity(&fade).shapes(&faded)?
    );
    assert_eq!(path.opacity(&fade).shapes(&faded)?[0].style().opacity(), 0.25);
    // and the drawing it returns is one of that path
    assert_eq!(path.fill(PINK).shapes(&Values::new())?[0].path(), &path);
    Ok(())
}
