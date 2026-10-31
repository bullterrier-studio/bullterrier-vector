//! Properties of `Drawing::to_mesh` that hold for any drawing: the same drawing gives the same buffers, a sized drawing
//! at twice the size has positions exactly twice as large, refilling replaces what was there, and `generation` moves
//! exactly when the contents do.

mod common;

use bullterrier_vector::{Blend, Color, Drawing, Mesh, Param, Path, Size, Stroke, Values};
use common::mesh::{check_layout, mesh_of};
use common::{Corners, Outcome};
use proptest::prelude::*;

/// A scene with curves, a self-overlapping outline, a stroke with round joins and a multiply, in a 100 by 50 frame.
fn scene() -> Result<Drawing, bullterrier_vector::Error> {
    Ok(Drawing::new([
        Drawing::from(Path::circle((30.0, 25.0), 12.0)?).fill(Color::parse("#33669980")?),
        Drawing::from(Path::star((60.0, 25.0), 20.0, 8.0, 5)?)
            .fill(Color::hex(0x00ff_0000))
            .stroke(Stroke::new(Color::hex(0x0000_00ff)).width(3.0))
            .opacity(0.7),
        Drawing::from(Path::polyline(
            [(5.0, 5.0), (95.0, 45.0), (95.0, 5.0), (5.0, 45.0)],
            false,
        )?)
        .fill(Color::NONE)
        .stroke(Stroke::new(Color::hex(0x0000_aa00)).width(4.0))
        .blend(Blend::Multiply),
    ])
    .with_size(100.0, 50.0))
}

#[test]
fn the_same_drawing_gives_the_same_buffers_whether_the_mesh_is_new_or_refilled() -> Outcome {
    let fresh = mesh_of(&scene()?, 200.0, 100.0)?;
    assert_ne!(fresh.indices().len(), 0);
    let again = mesh_of(&scene()?, 200.0, 100.0)?;
    let mut refilled = mesh_of(
        &Drawing::from(Path::circle((5.0, 5.0), 4.0)?).fill(Color::hex(0)),
        10.0,
        10.0,
    )?;
    scene()?.to_mesh(Size::px(200.0, 100.0), &Values::new(), &mut refilled)?;
    for other in [&again, &refilled] {
        assert_eq!(other.vertices(), fresh.vertices());
        assert_eq!(other.indices(), fresh.indices());
        assert_eq!(other.ranges(), fresh.ranges());
    }
    Ok(())
}

/// Whether `double` is `single` drawn twice as large: positions exactly twice, everything else equal.
fn is_double_of(double: &Mesh, single: &Mesh) -> bool {
    double.indices() == single.indices()
        && double.ranges() == single.ranges()
        && double.vertices().len() == single.vertices().len()
        && double
            .vertices()
            .chunks(8)
            .zip(single.vertices().chunks(8))
            .all(|(two, one)| {
                two[0].to_bits() == (2.0 * one[0]).to_bits()
                    && two[1].to_bits() == (2.0 * one[1]).to_bits()
                    && two[2..] == one[2..]
            })
}

#[test]
fn a_sized_drawing_at_twice_the_size_has_positions_exactly_twice_as_large() -> Outcome {
    // 100 by 50 frames: at 200 by 100 they fill the output; at 200 by 200 they are centred down by 50
    for (case, (width, height)) in [
        ("filling", (200.0, 100.0)),
        ("centred", (200.0, 200.0)),
        ("smaller", (50.0, 25.0)),
    ] {
        let single = mesh_of(&scene()?, width, height)?;
        let double = mesh_of(&scene()?, width * 2.0, height * 2.0)?;
        assert!(single.indices().len() > 60, "{case}");
        assert!(is_double_of(&double, &single), "{case}");
    }
    Ok(())
}

#[test]
fn a_scale_that_is_not_a_power_of_two_scales_positions_to_f32_precision() -> Outcome {
    let single = mesh_of(&scene()?, 100.0, 50.0)?;
    let triple = mesh_of(&scene()?, 300.0, 150.0)?;
    assert!(single.indices().len() > 60);
    assert_eq!(single.vertices().len(), triple.vertices().len());
    for (three, one) in triple.vertices().chunks(8).zip(single.vertices().chunks(8)) {
        assert!((f64::from(three[0]) - 3.0 * f64::from(one[0])).abs() < 1e-4);
        assert!((f64::from(three[1]) - 3.0 * f64::from(one[1])).abs() < 1e-4);
    }
    Ok(())
}

fn polygon() -> impl Strategy<Value = Corners> {
    let corner = (0..100_i32, 0..100_i32).prop_map(|(x, y)| (f64::from(x), f64::from(y)));
    prop::collection::vec(corner, 3..9)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn any_polygon_filled_and_stroked_doubles_with_the_output_and_keeps_the_layout(
        corners in polygon(),
        width in 1.0..8.0_f64,
    ) {
        let build = || -> Result<Drawing, bullterrier_vector::Error> {
            Ok(Drawing::from(Path::polyline(corners.clone(), true)?)
                .fill(Color::hex(0x0033_6699))
                .stroke(Stroke::new(Color::hex(0)).width(width))
                .with_size(100.0, 100.0))
        };
        let fail = |e: bullterrier_vector::Error| TestCaseError::fail(e.to_string());
        let single = mesh_of(&build().map_err(fail)?, 100.0, 100.0).map_err(fail)?;
        let double = mesh_of(&build().map_err(fail)?, 200.0, 200.0).map_err(fail)?;
        check_layout(&single).map_err(TestCaseError::fail)?;
        prop_assert!(single.indices().len() >= 6, "a polygon fills at least one triangle");
        prop_assert!(is_double_of(&double, &single));
    }
}

#[test]
fn refilling_replaces_the_contents_whatever_they_were() -> Outcome {
    let small = Drawing::from(Path::rect(1.0, 1.0, 3.0, 3.0, 0.0)?).fill(Color::hex(0x00ff_0000));
    let expected = mesh_of(&small, 10.0, 10.0)?;
    assert_eq!(expected.indices().len(), 6);
    let mut mesh = mesh_of(&scene()?, 200.0, 100.0)?;
    small.to_mesh(Size::px(10.0, 10.0), &Values::new(), &mut mesh)?;
    assert_eq!(mesh.vertices(), expected.vertices());
    assert_eq!(mesh.indices(), expected.indices());
    assert_eq!(mesh.ranges(), expected.ranges());
    Ok(())
}

/// A red square moved right by the Param `dx` (default 0) and at the opacity of the Param `fade` (default 1).
fn moving() -> Result<Drawing, bullterrier_vector::Error> {
    let dx = Param::new("dx", 0.0, 0.0, 50.0)?;
    let fade = Param::new("fade", 1.0, 0.0, 1.0)?;
    Ok(Drawing::from(Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)?)
        .fill(Color::hex(0x00ff_0000))
        .translate(&dx, 0.0)
        .opacity(&fade))
}

#[test]
fn generation_moves_exactly_when_the_contents_change() -> Outcome {
    let drawing = moving()?;
    let mut mesh = Mesh::new();
    // (case, the values, the generation after) in order; the first fill that makes contents is 1
    let rows = [
        ("an empty mesh filled with an empty picture stays 0", None, 0),
        ("the first contents", Some(Values::new()), 1),
        ("the same values again", Some(Values::new()), 1),
        ("the same values said out loud", Some(Values::new().set("dx", 0.0)), 1),
        ("a position changed", Some(Values::new().set("dx", 5.0)), 2),
        ("and again", Some(Values::new().set("dx", 5.0)), 2),
        (
            "an opacity changed",
            Some(Values::new().set("dx", 5.0).set("fade", 0.5)),
            3,
        ),
        ("back to the first contents", Some(Values::new()), 4),
        ("a picture with nothing in it", Some(Values::new().set("fade", 0.0)), 5),
        ("and again nothing", Some(Values::new().set("fade", 0.0)), 5),
    ];
    for (case, values, generation) in rows {
        match values {
            Some(values) => drawing.to_mesh(Size::px(100.0, 100.0), &values, &mut mesh)?,
            None => Drawing::new(Vec::<Drawing>::new()).to_mesh(Size::px(10.0, 10.0), &Values::new(), &mut mesh)?,
        }
        assert_eq!(mesh.generation(), generation, "{case}");
    }
    Ok(())
}

#[test]
fn equal_contents_from_a_different_drawing_keep_the_generation() -> Outcome {
    let (first, second) = (Color::hex(0x00ff_0000), Color::hex(0x00ff_0000));
    let made_one_way = Drawing::from(Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)?).fill(first);
    let made_another = Drawing::new([Drawing::from(Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)?).fill(second)]);
    let mut mesh = mesh_of(&made_one_way, 20.0, 20.0)?;
    assert_eq!(mesh.generation(), 1);
    made_another.to_mesh(Size::px(20.0, 20.0), &Values::new(), &mut mesh)?;
    assert_eq!(mesh.generation(), 1);
    Ok(())
}
