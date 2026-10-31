//! `Drawing::to_mesh` through the public API: the layout of the buffers, what is drawn for nothing, the sizes it takes
//! and refuses, the limits, and that a refused draw leaves the mesh alone. Expected values are written by hand.

mod common;

use bullterrier_vector::{Cap, Color, Drawing, Join, Mesh, Path, Size, Stroke, Values};
use common::mesh::{check_layout, mesh_of};
use common::pixels::SIZE_REFUSALS;
use common::{Outcome, parameter_refusal};

fn rect(x: f64, y: f64, width: f64, height: f64) -> Result<Drawing, bullterrier_vector::Error> {
    Ok(Drawing::from(Path::rect(x, y, width, height, 0.0)?).fill(Color::hex(0x00ff_0000)))
}

fn to_mesh(drawing: &Drawing, width: f64, height: f64, into: &mut Mesh) -> Result<(), bullterrier_vector::Error> {
    drawing.to_mesh(Size::px(width, height), &Values::new(), into)
}

#[test]
fn a_new_mesh_is_empty_and_generation_0() {
    let mesh = Mesh::new();
    assert!(mesh.vertices().is_empty() && mesh.indices().is_empty() && mesh.ranges().is_empty());
    assert_eq!(mesh.generation(), 0);
}

#[test]
fn a_rectangle_is_four_vertices_two_triangles_and_one_range() -> Outcome {
    let mesh = mesh_of(&rect(10.0, 20.0, 30.0, 40.0)?, 100.0, 100.0)?;
    check_layout(&mesh)?;
    assert_eq!(mesh.vertices().len(), 4 * 8);
    assert_eq!(mesh.indices().len(), 6);
    assert_eq!(mesh.ranges(), [0, 6, 0]);
    Ok(())
}

#[test]
fn nothing_visible_is_drawn_as_empty_buffers() -> Outcome {
    let red = Color::hex(0x00ff_0000);
    let line = Path::line((0.0, 0.0), (9.0, 9.0))?;
    let rows = [
        ("no shapes", Drawing::new(Vec::<Drawing>::new())),
        ("an empty path", Drawing::from(Path::new()).fill(red)),
        (
            "a lone move",
            Drawing::from(Path::start((5.0, 5.0))?)
                .fill(red)
                .stroke(Stroke::new(red)),
        ),
        (
            "no fill and no stroke",
            Drawing::from(Path::circle((5.0, 5.0), 4.0)?).fill(Color::NONE),
        ),
        (
            "opacity 0",
            Drawing::from(Path::circle((5.0, 5.0), 4.0)?).fill(red).opacity(0.0),
        ),
        (
            "a fill of alpha 0",
            Drawing::from(Path::circle((5.0, 5.0), 4.0)?).fill(Color::parse("#ff000000")?),
        ),
        ("a line with a fill", Drawing::from(line).fill(red)),
    ];
    for (case, drawing) in rows {
        let mesh = mesh_of(&drawing, 10.0, 10.0)?;
        assert!(
            mesh.vertices().is_empty() && mesh.indices().is_empty() && mesh.ranges().is_empty(),
            "{case}"
        );
        assert_eq!(mesh.generation(), 0, "{case}");
    }
    Ok(())
}

#[test]
fn a_size_outside_whole_pixels_from_1_to_100000_is_refused_naming_the_side() -> Outcome {
    let past_the_limit = [
        ("one past the limit wide", 100_001.0, 10.0, "width", "100001"),
        ("one past the limit high", 10.0, 100_001.0, "height", "100001"),
    ];
    for (case, width, height, side, shown) in SIZE_REFUSALS.into_iter().chain(past_the_limit) {
        let (parameter, message) = parameter_refusal(mesh_of(&rect(0.0, 0.0, 5.0, 5.0)?, width, height))?;
        assert_eq!(parameter, side, "{case}");
        assert!(message.contains(shown), "{case}: {message}");
    }
    Ok(())
}

#[test]
fn the_largest_sides_are_accepted_and_the_pixel_count_is_not_limited() -> Outcome {
    // no canvas is allocated, so the 2^27 pixels of the pixel outputs do not apply: 100,000 by 100,000 is a mesh
    for (width, height) in [(100_000.0, 100_000.0), (100_000.0, 1.0), (1.0, 100_000.0)] {
        let mesh = mesh_of(&rect(0.0, 0.0, 5.0, 5.0)?, width, height)?;
        check_layout(&mesh)?;
        assert_eq!(mesh.indices().len(), 6, "{width} by {height}");
    }
    Ok(())
}

#[test]
fn a_bad_size_is_refused_before_the_drawing_is_evaluated() -> Outcome {
    let broken = Drawing::from(Path::circle((0.0, 0.0), 5.0)?).opacity(2.0);
    assert!(broken.error().is_some());
    let (parameter, _) = parameter_refusal(mesh_of(&broken, 0.0, 10.0))?;
    assert_eq!(parameter, "width");
    Ok(())
}

#[test]
fn a_refused_draw_leaves_the_mesh_as_it_was() -> Outcome {
    let mut mesh = Mesh::new();
    to_mesh(&rect(0.0, 0.0, 5.0, 5.0)?, 10.0, 10.0, &mut mesh)?;
    let (vertices, indices, ranges, generation) = (
        mesh.vertices().to_vec(),
        mesh.indices().to_vec(),
        mesh.ranges().to_vec(),
        mesh.generation(),
    );
    let broken = Drawing::from(Path::circle((0.0, 0.0), 5.0)?).opacity(2.0);
    let far = Drawing::new([rect(0.0, 0.0, 5.0, 5.0)?, rect(0.0, 0.0, 2.0e7, 5.0)?]);
    for (case, drawing, width) in [
        ("a bad size", rect(0.0, 0.0, 1.0, 1.0)?, 0.5),
        ("a bad drawing", broken, 10.0),
    ]
    .into_iter()
    .chain([("a shape too far, after a good one", far, 10.0)])
    {
        assert!(to_mesh(&drawing, width, 10.0, &mut mesh).is_err(), "{case}");
        assert_eq!(mesh.vertices(), vertices, "{case}");
        assert_eq!(mesh.indices(), indices, "{case}");
        assert_eq!(mesh.ranges(), ranges, "{case}");
        assert_eq!(mesh.generation(), generation, "{case}");
    }
    Ok(())
}

#[test]
fn a_coordinate_past_ten_million_pixels_is_refused_naming_the_shape() -> Outcome {
    // (case, the shape that is too far, the design frame the drawing has, the output side); it is drawn second, over a
    // small rectangle, and it is the one named
    let line = || Path::line((0.0, 0.0), (1.0, 1.0));
    let red = Color::hex(0x00ff_0000);
    let rows = [
        ("past on the right", rect(0.0, 0.0, 1.0e7 + 1.0, 4.0)?, None, 10.0),
        ("past on the left", rect(-1.0e7 - 1.0, 0.0, 4.0, 4.0)?, None, 10.0),
        ("past above", rect(0.0, -1.0e7 - 1.0, 4.0, 4.0)?, None, 10.0),
        (
            "a coordinate the view scales past it: 2e5 in a frame of 1 drawn 100 across",
            rect(0.0, 0.0, 2.0e5, 1.0)?,
            Some((1.0, 1.0)),
            100.0,
        ),
        (
            "a coordinate past it in design pixels that the view shrinks: 2e7 in a frame of 1e5 drawn 100 across",
            rect(0.0, 0.0, 2.0e7, 1.0)?,
            Some((1.0e5, 1.0e5)),
            100.0,
        ),
        (
            "a stroke the view scales past it",
            Drawing::from(line()?)
                .fill(Color::NONE)
                .stroke(Stroke::new(red).width(2.0e5)),
            Some((1.0, 1.0)),
            100.0,
        ),
    ];
    for (case, far, frame, side) in rows {
        let mut drawing = Drawing::new([rect(0.0, 0.0, 2.0, 2.0)?, far]);
        if let Some((width, height)) = frame {
            drawing = drawing.with_size(width, height);
        }
        let (parameter, message) = parameter_refusal(mesh_of(&drawing, side, side))?;
        assert_eq!(parameter, "shape 1", "{case}");
        assert!(message.contains("10000000"), "{case}: {message}");
    }
    Ok(())
}

#[test]
fn coordinates_up_to_ten_million_are_drawn() -> Outcome {
    let mesh = mesh_of(&rect(-1.0e7, -1.0e7, 2.0e7, 2.0e7)?, 8.0, 8.0)?;
    check_layout(&mesh)?;
    assert_eq!(mesh.indices().len(), 6);
    Ok(())
}

#[test]
fn flattening_more_than_ten_million_edges_in_a_draw_is_refused_naming_the_shape_and_the_counter() -> Outcome {
    // a curve whose control points are a billion pixels away is halved to the depth cap, 65,536 edges; a path of 160 of
    // them is over the 10,000,000 that one draw may flatten (the same count the pixels charge)
    let mut path = Path::start((0.0, 0.0))?;
    for index in 0..160 {
        let end = if index % 2 == 0 { (100.0, 0.0) } else { (0.0, 0.0) };
        path = path.curve_to((1.0e9, 1.0e9), (-1.0e9, 1.0e9), end)?;
    }
    let drawing = Drawing::new([
        rect(0.0, 0.0, 2.0, 2.0)?,
        Drawing::from(path).fill(Color::hex(0x00ff_0000)),
    ]);
    let (parameter, message) = parameter_refusal(mesh_of(&drawing, 10.0, 10.0))?;
    assert_eq!(parameter, "shape 1");
    assert!(message.contains("MAX_SEGMENTS 10000000"), "{message}");
    Ok(())
}

#[test]
fn the_outline_of_one_shapes_stroke_is_refused_past_max_segments() -> Outcome {
    // 1,000 points that turn back on themselves, joined round at the widest stroke the bound allows: each join is an arc
    // of about 25,000 edges, 25 million in all, though the path itself is 1,000 edges
    let zigzag: Vec<(f64, f64)> = (0..1000u32)
        .map(|index| (500.0 + f64::from(index % 2) * 3.0, 500.0 + f64::from(index) * 0.001))
        .collect();
    let stroke = Stroke::new(Color::hex(0x00ff_0000))
        .width(1.0e7)
        .join(Join::Round)
        .cap(Cap::Butt);
    let drawing = Drawing::from(Path::polyline(zigzag, false)?)
        .fill(Color::NONE)
        .stroke(stroke);
    let (parameter, message) = parameter_refusal(mesh_of(&drawing, 1000.0, 1000.0))?;
    assert_eq!(parameter, "shape 0");
    assert!(message.contains("MAX_SEGMENTS 10000000"), "{message}");
    Ok(())
}

/// The star polygon {n/k} of `n` corners: every corner joined to the one `k` along, so that each edge crosses most of
/// the others.
fn star_polygon(n: usize, k: usize) -> Result<Path, bullterrier_vector::Error> {
    #[expect(clippy::cast_precision_loss, reason = "test sizes are small")]
    let corner = |index: usize| {
        let turn = 2.0 * std::f64::consts::PI * ((index * k) % n) as f64 / n as f64;
        (500.0 + 450.0 * libm::cos(turn), 500.0 + 450.0 * libm::sin(turn))
    };
    Path::polyline((0..n).map(corner), true)
}

#[test]
fn a_tessellation_that_would_take_seconds_is_refused_naming_the_shape_and_the_counter() -> Outcome {
    // Tessellating costs about (vertices made) x (edges crossing a sweep line): a star polygon of 2,001 corners makes
    // 750,000 vertices with 2,000 edges alive at once, which is seconds, though far below MAX_SEGMENTS. The
    // refused rows are drawn second, over a small rectangle, and are the shape named
    let red = Color::hex(0x00ff_0000);
    let stroke = Stroke::new(red).width(5.0);
    let refused = [
        (
            "a star polygon filled",
            Drawing::from(star_polygon(2001, 1000)?).fill(red),
        ),
        (
            "a star polygon stroked",
            Drawing::from(star_polygon(801, 400)?).fill(Color::NONE).stroke(stroke),
        ),
    ];
    for (case, far) in refused {
        let drawing = Drawing::new([rect(0.0, 0.0, 2.0, 2.0)?, far]).with_size(1000.0, 1000.0);
        let (parameter, message) = parameter_refusal(mesh_of(&drawing, 1000.0, 1000.0))?;
        assert_eq!(parameter, "shape 1", "{case}");
        assert!(message.contains("MAX_TESSELLATION_WORK 250000000"), "{case}: {message}");
    }
    Ok(())
}

#[test]
fn shapes_with_many_edges_but_little_crossing_are_drawn() -> Outcome {
    let red = Color::hex(0x00ff_0000);
    // 3,000 thin teeth side by side: 12,001 edges, none crossing, 6,000 alive at the widest sweep line
    let mut comb = vec![(0.0, 900.0)];
    for tooth in 0..3000_u32 {
        let x = 10.0 + f64::from(tooth) * 0.2;
        comb.extend([(x, 100.0), (x + 0.1, 100.0), (x + 0.1, 900.0), (x + 0.2, 900.0)]);
    }
    // (case, path, the stroke it has); a stroke of width 5 around teeth 0.1 apart would be a hostile comb of its own
    let stroke = || Some(Stroke::new(red).width(5.0));
    let rows = [
        ("a comb of 3,000 teeth", Path::polyline(comb, true)?, None),
        ("a star polygon of 201 corners", star_polygon(201, 100)?, stroke()),
        (
            "a circle across the frame",
            Path::circle((500.0, 500.0), 450.0)?,
            stroke(),
        ),
    ];
    for (case, path, stroke) in rows {
        let mut drawing = Drawing::from(path).fill(red);
        if let Some(stroke) = stroke {
            drawing = drawing.stroke(stroke);
        }
        let drawing = drawing.with_size(1000.0, 1000.0);
        let mesh = mesh_of(&drawing, 1000.0, 1000.0)?;
        check_layout(&mesh)?;
        assert!(!mesh.indices().is_empty(), "{case}");
    }
    Ok(())
}
