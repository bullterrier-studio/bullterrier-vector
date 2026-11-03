//! How `Drawing::to_mesh` lays shapes in the buffers: the colour of a vertex, the item and order it carries, the
//! layers a shape makes (a fill, then a stroke), and the draw ranges that merge the layers of one blend. Expected
//! values are worked by hand from the colours and counts written in each test.

use crate::common::Outcome;
use crate::common::mesh::{check_layout, corners, mesh_of, vertex};
use bullterrier_vector::{Blend, Color, Drawing, Path, Stroke};

fn rect(x: f64) -> Result<Path, bullterrier_vector::Error> {
    Path::rect(x, 0.0, 8.0, 8.0, 0.0)
}

fn near(left: f32, right: f64) -> bool {
    (f64::from(left) - right).abs() < 1e-6
}

#[test]
fn a_colour_is_premultiplied_and_its_alpha_is_the_colours_times_the_shapes_opacity() -> Outcome {
    // (case, fill, opacity, the premultiplied r, g, b, a worked by hand); 0x80 / 255 is 0.501960..., halved 0.250980...
    let half = 128.0 / 255.0 * 0.5;
    let rows = [
        ("opaque", "#336699", 1.0, [0.2, 0.4, 0.6, 1.0]),
        ("opacity", "#336699", 0.5, [0.1, 0.2, 0.3, 0.5]),
        (
            "alpha",
            "#33669980",
            1.0,
            [
                0.2 * 128.0 / 255.0,
                0.4 * 128.0 / 255.0,
                0.6 * 128.0 / 255.0,
                128.0 / 255.0,
            ],
        ),
        ("both", "#33669980", 0.5, [0.2 * half, 0.4 * half, 0.6 * half, half]),
    ];
    for (case, color, opacity, expected) in rows {
        let drawing = Drawing::from(rect(0.0)?).fill(Color::parse(color)?).opacity(opacity);
        let mesh = mesh_of(&drawing, 10.0, 10.0)?;
        assert_eq!(mesh.indices().len(), 6, "{case}");
        for index in 0..4 {
            let rgba = vertex(&mesh, index).rgba;
            for (channel, (got, want)) in rgba.into_iter().zip(expected).enumerate() {
                assert!(near(got, want), "{case}: channel {channel} is {got}, wanted {want}");
            }
        }
    }
    Ok(())
}

#[test]
fn every_vertex_carries_its_shapes_index_and_order() -> Outcome {
    let red = Color::hex(0x00ff_0000);
    // a stack ranks its four shapes' order 0, 1/3, 2/3, 1, the empty path too: it draws nothing but keeps its place
    let stack = Drawing::new([
        Drawing::from(rect(0.0)?).fill(red),
        Drawing::from(Path::new()).fill(red),
        Drawing::from(rect(20.0)?).fill(red),
        Drawing::from(rect(40.0)?).fill(red),
    ]);
    let mesh = mesh_of(&stack, 100.0, 100.0)?;
    check_layout(&mesh)?;
    let seen: Vec<(u32, f32)> = (0..12)
        .map(|index| (vertex(&mesh, index).item, vertex(&mesh, index).order))
        .collect();
    let expected: [(u32, f64); 3] = [(0, 0.0), (2, 2.0 / 3.0), (3, 1.0)];
    for (shape, (item, order)) in expected.into_iter().enumerate() {
        for pair in &seen[shape * 4..shape * 4 + 4] {
            assert!(
                pair.0 == item && (f64::from(pair.1) - order).abs() < 1e-6,
                "shape {shape}: {pair:?}"
            );
        }
    }
    // a drawing of one path has no order attribute: 0
    let lone = mesh_of(&Drawing::from(rect(0.0)?).fill(red), 100.0, 100.0)?;
    assert_eq!(vertex(&lone, 0).order.to_bits(), 0);
    Ok(())
}

#[test]
fn a_shape_is_a_fill_layer_then_a_stroke_layer_in_paint_order() -> Outcome {
    let green = Color::hex(0x0000_ff00);
    let red = Color::hex(0x00ff_0000);
    let drawing = Drawing::new([
        Drawing::from(rect(0.0)?)
            .fill(green)
            .stroke(Stroke::new(red).width(2.0)),
        Drawing::from(rect(20.0)?)
            .fill(Color::NONE)
            .stroke(Stroke::new(red).width(2.0)),
    ]);
    let mesh = mesh_of(&drawing, 100.0, 100.0)?;
    check_layout(&mesh)?;
    // the colour and shape of each triangle, in order: green fill of shape 0, its red stroke, then shape 1's red stroke
    let mut runs: Vec<(u32, u32)> = Vec::new();
    for number in 0..mesh.indices().len() / 3 {
        let [a, b, c] = corners(&mesh, number);
        assert!(a.paint() == b.paint() && b.paint() == c.paint(), "triangle {number}");
        let run = (a.rgba[1].to_bits(), a.item);
        if runs.last() != Some(&run) {
            runs.push(run);
        }
    }
    // green is g = 1 and red g = 0: one run each, never interleaved
    assert_eq!(runs, [(1.0_f32.to_bits(), 0), (0, 0), (0, 1)]);
    // both layers are normal blend: one range over the lot
    assert_eq!(mesh.ranges(), [0, u32::try_from(mesh.indices().len())?, 0]);
    Ok(())
}

#[test]
fn the_layers_of_one_blend_merge_into_a_range_and_a_change_of_blend_starts_a_new_one() -> Outcome {
    let red = Color::hex(0x00ff_0000);
    let shape =
        |x: f64, blend: Blend| Ok::<_, bullterrier_vector::Error>(Drawing::from(rect(x)?).fill(red).blend(blend));
    // each rectangle is 6 indices
    let drawing = Drawing::new([
        shape(0.0, Blend::Normal)?,
        shape(10.0, Blend::Normal)?,
        shape(20.0, Blend::Multiply)?,
        shape(30.0, Blend::Normal)?,
        shape(40.0, Blend::Multiply)?,
        shape(50.0, Blend::Multiply)?,
    ]);
    let mesh = mesh_of(&drawing, 100.0, 100.0)?;
    check_layout(&mesh)?;
    assert_eq!(mesh.ranges(), [0, 12, 0, 12, 6, 1, 18, 6, 0, 24, 12, 1]);
    Ok(())
}

#[test]
fn the_fill_and_the_stroke_of_a_multiply_shape_are_one_multiply_range() -> Outcome {
    let drawing = Drawing::from(rect(0.0)?)
        .fill(Color::hex(0x0000_ff00))
        .stroke(Stroke::new(Color::hex(0x00ff_0000)).width(2.0))
        .blend(Blend::Multiply);
    let mesh = mesh_of(&drawing, 100.0, 100.0)?;
    assert_eq!(mesh.ranges(), [0, u32::try_from(mesh.indices().len())?, 1]);
    Ok(())
}

#[test]
fn a_shapes_triangles_use_its_own_vertices_and_a_later_shape_comes_later_in_the_buffers() -> Outcome {
    let drawing = Drawing::new([
        Drawing::from(rect(0.0)?).fill(Color::hex(0x00ff_0000)),
        Drawing::from(Path::circle((50.0, 50.0), 10.0)?).fill(Color::hex(0x0000_00ff)),
    ]);
    let mesh = mesh_of(&drawing, 100.0, 100.0)?;
    check_layout(&mesh)?;
    for number in 0..mesh.indices().len() / 3 {
        let items = corners(&mesh, number).map(|v| v.item);
        assert!(items[0] == items[1] && items[1] == items[2], "triangle {number}");
    }
    // the first four vertices are the rectangle's, and no later triangle reaches back to them
    assert!(mesh.indices()[6..].iter().all(|&index| index >= 4));
    assert!(mesh.indices()[..6].iter().all(|&index| index < 4));
    Ok(())
}

#[test]
fn all_triangles_wind_the_same_way() -> Outcome {
    let drawing = Drawing::new([
        Drawing::from(Path::star((50.0, 50.0), 40.0, 15.0, 5)?).fill(Color::hex(0x00ff_0000)),
        Drawing::from(Path::circle((50.0, 50.0), 10.0)?)
            .fill(Color::hex(0x0000_00ff))
            .stroke(Stroke::new(Color::hex(0x0000_ff00)).width(3.0)),
    ]);
    let mesh = mesh_of(&drawing, 100.0, 100.0)?;
    let turns: Vec<f64> = (0..mesh.indices().len() / 3)
        .map(|number| {
            let [a, b, c] = corners(&mesh, number);
            (b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y)
        })
        .collect();
    assert!(turns.len() > 30);
    let positive = turns.iter().filter(|&&turn| turn > 0.0).count();
    let negative = turns.iter().filter(|&&turn| turn < 0.0).count();
    assert!(
        positive == 0 || negative == 0,
        "{positive} one way, {negative} the other"
    );
    Ok(())
}
