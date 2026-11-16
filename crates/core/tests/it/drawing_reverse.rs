//! `Drawing::reverse` through the public API. The usage first: a draw-on the other way round, `reverse().slice(0, p)`.
//! Expected values are by hand; the winding is measured by the shoelace sum, which does not use `reverse`.

use crate::common::Outcome;
use bullterrier_vector::{Color, Drawing, Inputs, Path, Values, text};

fn drawn(drawing: &Drawing) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(drawing
        .shapes(&Values::new())?
        .iter()
        .map(|shape| shape.path().to_svg_path())
        .collect())
}

/// Twice the signed area of the polygon through eight evenly spaced points of a closed path of perimeter 40.
fn shoelace(path: &Path) -> Result<f64, Box<dyn std::error::Error>> {
    let points = (0..8)
        .map(|i| path.point_at(f64::from(i) / 8.0, None))
        .collect::<Result<Vec<_>, _>>()?;
    Ok((0..8)
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % 8]);
            a.x * b.y - b.x * a.y
        })
        .sum())
}

#[test]
fn an_open_line_swaps_its_ends() -> Outcome {
    let line = Drawing::from(Path::from_svg_path("M0 0 L10 0")?);
    assert_eq!(drawn(&line.reverse())?, ["M10 0 L0 0"]);
    Ok(())
}

#[test]
fn reverse_then_slice_is_the_second_half_walked_backwards() -> Outcome {
    let ell = Drawing::from(Path::from_svg_path("M0 0 L10 0 L10 10")?);
    assert_eq!(drawn(&ell.reverse().slice(0.0, 0.25))?, ["M10 10 L10 5"]);
    assert_eq!(drawn(&ell.reverse().slice(0.0, 0.75))?, ["M10 10 L10 0 L5 0"]);
    let half = drawn(&ell.reverse().slice(0.0, 0.5))?;
    assert_eq!(half.len(), 1);
    assert!(half[0].starts_with("M10 10 L10 0"), "{half:?}");
    Ok(())
}

#[test]
fn a_closed_path_stays_closed_and_its_winding_flips() -> Outcome {
    let square = Path::from_svg_path("M0 0 L10 0 L10 10 L0 10 Z")?;
    let reversed = Drawing::from(square.clone()).reverse();
    let shapes = reversed.shapes(&Values::new())?;
    let path = shapes[0].path();
    assert!(path.to_svg_path().ends_with('Z'), "{}", path.to_svg_path());
    assert!(path.contains((5.0, 5.0))?);
    assert!(!path.contains((15.0, 5.0))?);
    let (before, after) = (shoelace(&square)?, shoelace(path)?);
    assert_eq!(before, 200.0);
    assert_eq!(after, -200.0);
    Ok(())
}

#[test]
fn a_position_is_one_minus_that_position_on_the_reversed_shape() -> Outcome {
    let arch = Path::from_svg_path("M0 0 C0 10 10 10 10 0 L20 0")?;
    let reversed = Drawing::from(arch.clone()).reverse().shapes(&Values::new())?;
    for i in 0..=10 {
        let t = f64::from(i) / 10.0;
        let (a, b) = (arch.point_at(t, None)?, reversed[0].path().point_at(1.0 - t, None)?);
        assert!((a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9, "t {t}: {a:?} {b:?}");
    }
    Ok(())
}

#[test]
fn twice_gives_the_same_shapes() -> Outcome {
    let word = text("Hi", &crate::common::fonts::fraunces()?).fill(Color::hex(0xff_00_00));
    assert_eq!(
        word.reverse().reverse().shapes(&Values::new())?,
        word.shapes(&Values::new())?
    );
    Ok(())
}

#[test]
fn every_shape_is_reversed_with_its_style_and_attributes_kept() -> Outcome {
    let word = text("Hi", &crate::common::fonts::fraunces()?).fill(Color::hex(0xff_00_00));
    let (before, after) = (word.shapes(&Values::new())?, word.reverse().shapes(&Values::new())?);
    assert_eq!(before.len(), after.len());
    for (a, b) in before.iter().zip(&after) {
        assert_eq!(b.path(), &a.path().reverse());
        assert_eq!(a.style(), b.style());
        assert_eq!(a.attributes(), b.attributes());
    }
    Ok(())
}

#[test]
fn nothing_stays_nothing() -> Outcome {
    assert!(Drawing::new(Vec::<Path>::new()).reverse().shapes(&Values::new())?.is_empty());
    assert_eq!(drawn(&Drawing::from(Path::from_svg_path("")?).reverse())?, [""]);
    Ok(())
}

#[test]
fn the_step_saves_and_loads_the_same_drawing() -> Outcome {
    let drawing = Drawing::from(Path::from_svg_path("M0 0 L10 0")?).reverse();
    let json = drawing.to_json()?;
    assert!(json.contains(r#"{"reverse":{}}"#), "{json}");
    let again = Drawing::from_json(&json, &Inputs::new())?;
    assert_eq!(again.to_json()?, json);
    assert_eq!(drawn(&again)?, ["M10 0 L0 0"]);
    Ok(())
}
