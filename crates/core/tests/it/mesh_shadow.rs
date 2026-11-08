//! A shadow in the mesh `Drawing::to_mesh` fills: with no blur an offset copy of the shape's triangles under it; with a
//! blur it is refused. Expected buffers are those of the hand-built copy, and colours worked by hand.

use crate::common::mesh::{FLOATS, check_layout, mesh_of, same_triangles, vertex};
use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Blend, Cap, Color, Drawing, Error, Mesh, Path, Shadow, Size, Stroke, Values};

const RED: Color = Color::hex(0xff_00_00);

fn color(text: &str) -> Result<Color, Error> {
    Color::parse(text)
}

fn rect(left: f64, top: f64, width: f64, height: f64) -> Result<Path, Error> {
    Path::rect(left, top, width, height, 0.0)
}

#[test]
fn a_shadow_without_a_blur_is_the_triangles_of_the_shape_moved_and_tinted_under_it() -> Outcome {
    let tint = color("#10203040")?;
    let path = rect(10.0, 10.0, 30.0, 20.0)?;
    let drawing = Drawing::from(path.clone())
        .fill(RED)
        .shadow(Shadow::new().dx(5.5).dy(-3.0).blur(0.0).color(tint));
    let by_hand = Drawing::new([
        Drawing::from(path.translate(5.5, -3.0)?).fill(tint),
        Drawing::from(path).fill(RED),
    ]);
    let (mesh, copy) = (mesh_of(&drawing, 60.0, 60.0)?, mesh_of(&by_hand, 60.0, 60.0)?);
    check_layout(&mesh)?;
    same_triangles(&mesh, &copy)?;
    // and the shadow carries the item of its shape
    assert!(
        (0..mesh.vertices().len() / FLOATS).all(|at| u32::try_from(at).is_ok_and(|at| vertex(&mesh, at).item == 0))
    );
    Ok(())
}

#[test]
fn the_shadow_of_a_stroke_is_the_stroke_moved_under_the_fill_and_stroke() -> Outcome {
    let tint = color("#000000ff")?;
    let path = rect(10.0, 10.0, 30.0, 20.0)?;
    let stroke = |colour| Stroke::new(colour).width(4.0).cap(Cap::Butt);
    let drawing = Drawing::from(path.clone())
        .fill(RED)
        .stroke(stroke(color("#0000ff")?))
        .shadow(Shadow::new().dx(2.0).dy(9.0).blur(0.0).color(tint));
    // each layer of the shape casts a layer of its own, in the tint
    let by_hand = Drawing::new([
        Drawing::from(path.translate(2.0, 9.0)?).fill(tint),
        Drawing::from(path.translate(2.0, 9.0)?)
            .fill(Color::NONE)
            .stroke(stroke(tint)),
        Drawing::from(path).fill(RED).stroke(stroke(color("#0000ff")?)),
    ]);
    same_triangles(&mesh_of(&drawing, 60.0, 60.0)?, &mesh_of(&by_hand, 60.0, 60.0)?)
}

#[test]
fn the_shadow_takes_the_opacity_and_the_blend_of_its_shape() -> Outcome {
    // a tint of alpha 0.8 under an opacity of 0.5: a vertex alpha of 0.4, premultiplied black; the shape's red at 0.5.
    // Both in one multiply range
    let drawing = Drawing::from(rect(10.0, 10.0, 20.0, 20.0)?)
        .fill(RED)
        .shadow(Shadow::new().dx(5.0).dy(0.0).blur(0.0).color(color("#000000cc")?))
        .opacity(0.5)
        .blend(Blend::Multiply);
    let mesh = mesh_of(&drawing, 60.0, 60.0)?;
    check_layout(&mesh)?;
    assert_eq!(mesh.ranges(), [0, 12, 1], "one range of two layers, multiply");
    let (shadow, shape) = (vertex(&mesh, 0).rgba, vertex(&mesh, 4).rgba);
    let near = |got: [f32; 4], want: [f64; 4]| {
        got.iter()
            .zip(want)
            .all(|(got, want)| (f64::from(*got) - want).abs() < 1e-6)
    };
    assert!(near(shadow, [0.0, 0.0, 0.0, 0.4]), "{shadow:?}");
    assert!(near(shape, [0.5, 0.0, 0.0, 0.5]), "{shape:?}");
    Ok(())
}

#[test]
fn a_shadow_with_a_blur_is_refused_naming_the_step_and_the_blur() -> Outcome {
    let rows = [
        ("the default blur", Shadow::new(), "blur 16"),
        ("a small blur", Shadow::new().blur(0.5), "blur 0.5"),
    ];
    for (case, shadow, shown) in rows {
        let drawing = Drawing::from(rect(10.0, 10.0, 20.0, 20.0)?).fill(RED).shadow(shadow);
        let mut mesh = Mesh::new();
        let (name, message) = parameter_refusal(drawing.to_mesh(Size::px(60.0, 60.0), &Values::new(), &mut mesh))?;
        assert!(name == "shadow" && message.contains(shown), "{case}: {name}: {message}");
        assert!(mesh.vertices().is_empty(), "{case}: the mesh is left as it was");
    }
    Ok(())
}

#[test]
fn a_shape_that_casts_nothing_is_not_refused_for_its_blur() -> Outcome {
    let shadow = || Shadow::new().blur(16.0);
    for (case, drawing) in [
        (
            "no fill and no stroke",
            Drawing::from(rect(10.0, 10.0, 20.0, 20.0)?)
                .fill(Color::NONE)
                .shadow(shadow()),
        ),
        ("an empty path", Drawing::from(Path::new()).fill(RED).shadow(shadow())),
    ] {
        let mesh = mesh_of(&drawing, 60.0, 60.0)?;
        assert!(mesh.indices().is_empty(), "{case}");
    }
    Ok(())
}

#[test]
fn an_offset_beyond_what_the_tessellator_holds_is_refused_naming_the_shape() -> Outcome {
    for (case, shadow) in [
        ("dx", Shadow::new().dx(2.0e7).blur(0.0)),
        ("dy", Shadow::new().dy(-2.0e7).blur(0.0)),
    ] {
        let drawing = Drawing::from(rect(10.0, 10.0, 20.0, 20.0)?).fill(RED).shadow(shadow);
        let (name, message) = parameter_refusal(mesh_of(&drawing, 60.0, 60.0))?;
        assert!(
            name == "shape 0" && message.contains("shadow"),
            "{case}: {name}: {message}"
        );
    }
    Ok(())
}
