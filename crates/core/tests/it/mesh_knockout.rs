//! A knockout in the mesh `Drawing::to_mesh` fills: the triangles below it are cut where it covers, as a boolean of the
//! shapes' regions. The oracles are the region written in the tests (a rectangle, a distance from a centre), the
//! `Path::subtract` of the same shapes built by hand, and the pixels, never the cutting itself.

use crate::common::mesh::{area, check_layout, corners, covering, mesh_of, same_triangles, triangles};
use crate::common::pixels::render;
use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Cap, Color, Drawing, Error, Mesh, Path, Shadow, Stroke};

const RED: Color = Color::hex(0xff_00_00);
const GREEN: Color = Color::hex(0x00_ff_00);
const BLUE: Color = Color::hex(0x00_00_ff);
const SIDE: f64 = 100.0;

fn rect(left: f64, top: f64, width: f64, height: f64) -> Result<Path, Error> {
    Path::rect(left, top, width, height, 0.0)
}

fn filled(left: f64, top: f64, width: f64, height: f64, color: Color) -> Result<Drawing, Error> {
    Ok(Drawing::from(rect(left, top, width, height)?).fill(color))
}

fn hole(left: f64, top: f64, width: f64, height: f64) -> Result<Drawing, Error> {
    Ok(filled(left, top, width, height, BLUE)?.knockout())
}

/// An error unless the mesh covers every point of the lattice of half pixels that `wanted` names, except those too near
/// an edge to say.
fn covers_where(mesh: &Mesh, wanted: impl Fn((f64, f64)) -> bool) -> Outcome {
    let all = triangles(mesh);
    let mut decided = 0;
    for column in 0..100 {
        for row in 0..100 {
            let point = (f64::from(column) + 0.5, f64::from(row) + 0.5);
            let Some(count) = covering(&all, point) else { continue };
            decided += 1;
            if (count > 0) != wanted(point) {
                return Err(format!(
                    "{point:?} is covered {count} times, but should{} be",
                    if wanted(point) { "" } else { " not" }
                )
                .into());
            }
        }
    }
    if decided < 9_000 {
        return Err(format!("only {decided} of 10,000 points were decided").into());
    }
    Ok(())
}

fn inside(left: f64, top: f64, right: f64, bottom: f64) -> impl Fn((f64, f64)) -> bool {
    move |(x, y)| x > left && x < right && y > top && y < bottom
}

#[test]
fn the_triangles_of_what_is_below_cover_it_less_the_hole_and_the_knockout_adds_none() -> Outcome {
    let mesh = mesh_of(
        &Drawing::new([filled(0.0, 0.0, 100.0, 100.0, RED)?, hole(30.0, 30.0, 40.0, 40.0)?]),
        SIDE,
        SIDE,
    )?;
    check_layout(&mesh)?;
    let hole = inside(30.0, 30.0, 70.0, 70.0);
    covers_where(&mesh, |point| !hole(point))?;
    assert!((triangles(&mesh).iter().map(area).sum::<f64>() - (10_000.0 - 1_600.0)).abs() < 1e-3);
    // every triangle is the red square's: the knockout paints nothing and is not blue
    for number in 0..mesh.indices().len() / 3 {
        assert!(
            corners(&mesh, number)
                .iter()
                .all(|vertex| vertex.paint() == ([1.0_f32.to_bits(), 0, 0, 1.0_f32.to_bits()], 0))
        );
    }
    Ok(())
}

#[test]
fn the_cut_is_the_subtraction_of_the_regions_built_by_hand() -> Outcome {
    let cut = rect(0.0, 0.0, 100.0, 100.0)?.subtract(&rect(30.0, 30.0, 40.0, 40.0)?, 0.01)?;
    let knocked = mesh_of(
        &Drawing::new([filled(0.0, 0.0, 100.0, 100.0, RED)?, hole(30.0, 30.0, 40.0, 40.0)?]),
        SIDE,
        SIDE,
    )?;
    let by_hand = mesh_of(&Drawing::from(cut).fill(RED), SIDE, SIDE)?;
    same_triangles(&knocked, &by_hand)
}

#[test]
fn a_shape_after_the_knockout_is_whole_and_keeps_its_item() -> Outcome {
    let mesh = mesh_of(
        &Drawing::new([
            filled(0.0, 0.0, 100.0, 100.0, RED)?,
            hole(30.0, 30.0, 40.0, 40.0)?,
            filled(40.0, 40.0, 20.0, 20.0, GREEN)?,
        ]),
        SIDE,
        SIDE,
    )?;
    let green: f64 = (0..mesh.indices().len() / 3)
        .filter(|&number| corners(&mesh, number)[0].item == 2)
        .map(|number| area(&corners(&mesh, number).map(|vertex| (vertex.x, vertex.y))))
        .sum();
    assert!((green - 400.0).abs() < 1e-3, "{green}");
    let (inner, outer) = (inside(40.0, 40.0, 60.0, 60.0), inside(30.0, 30.0, 70.0, 70.0));
    covers_where(&mesh, |point| inner(point) || !outer(point))
}

#[test]
fn a_hard_shadow_below_is_cut_with_its_shape() -> Outcome {
    // a square 20..60 across and 20..60 down, its shadow 20 lower: 20..60 across, 40..80 down, and a hole 50..70 down
    let below =
        filled(20.0, 20.0, 40.0, 40.0, RED)?.shadow(Shadow::new().dy(20.0).blur(0.0).color(Color::parse("#000000")?));
    let mesh = mesh_of(&Drawing::new([below, hole(0.0, 50.0, 100.0, 20.0)?]), SIDE, SIDE)?;
    let (square, shadow, cut) = (
        inside(20.0, 20.0, 60.0, 60.0),
        inside(20.0, 40.0, 60.0, 80.0),
        inside(0.0, 50.0, 100.0, 70.0),
    );
    covers_where(&mesh, |point| (square(point) || shadow(point)) && !cut(point))
}

#[test]
fn a_stroke_is_cut_out_with_its_round_joins() -> Outcome {
    // a stroke 10 wide round the square 30..70, and no fill: the band is 25..35 and 65..75, rounded at the corners with
    // radius 5 about each corner
    let ring = Drawing::from(rect(30.0, 30.0, 40.0, 40.0)?)
        .fill(Color::NONE)
        .stroke(Stroke::new(BLUE).width(10.0))
        .knockout();
    let mesh = mesh_of(&Drawing::new([filled(0.0, 0.0, 100.0, 100.0, RED)?, ring]), SIDE, SIDE)?;
    let outer = inside(25.0, 25.0, 75.0, 75.0);
    let inner = inside(35.0, 35.0, 65.0, 65.0);
    let corner = |(x, y): (f64, f64)| {
        let (near_x, near_y) = (x < 30.0 || x > 70.0, y < 30.0 || y > 70.0);
        // beyond a corner of the square, the band ends where a disc of radius 5 about it does
        near_x
            && near_y
            && libm::hypot(
                x - if x < 30.0 { 30.0 } else { 70.0 },
                y - if y < 30.0 { 30.0 } else { 70.0 },
            ) > 5.0
    };
    covers_where(&mesh, |point| !(outer(point) && !inner(point) && !corner(point)))
}

#[test]
fn a_stroke_that_is_cut_ends_flat_at_the_cut() -> Outcome {
    // a line 10 wide along y = 50 with round ends, cut by a hole 40..60 across: the band stops at the cut with no cap
    let line = Drawing::from(Path::polyline([(10.0, 50.0), (90.0, 50.0)], false)?)
        .stroke(Stroke::new(RED).width(10.0).cap(Cap::Round));
    let mesh = mesh_of(&Drawing::new([line, hole(40.0, 0.0, 20.0, 100.0)?]), SIDE, SIDE)?;
    let band = |(x, y): (f64, f64)| {
        let across = y > 45.0 && y < 55.0;
        // the round ends beyond 10 and 90 are discs of radius 5
        let along =
            (x > 10.0 && x < 90.0) || libm::hypot(x - 10.0, y - 50.0) < 5.0 || libm::hypot(x - 90.0, y - 50.0) < 5.0;
        (across && along || libm::hypot(x - 10.0, y - 50.0) < 5.0 || libm::hypot(x - 90.0, y - 50.0) < 5.0)
            && !(x > 40.0 && x < 60.0)
    };
    covers_where(&mesh, band)
}

#[test]
fn knockouts_with_a_shape_between_cut_what_is_below_each() -> Outcome {
    let mesh = mesh_of(
        &Drawing::new([
            filled(0.0, 0.0, 100.0, 100.0, RED)?,
            hole(20.0, 20.0, 20.0, 20.0)?,
            filled(0.0, 0.0, 100.0, 50.0, BLUE)?,
            hole(30.0, 30.0, 20.0, 20.0)?,
        ]),
        SIDE,
        SIDE,
    )?;
    let (first, second, top) = (
        inside(20.0, 20.0, 40.0, 40.0),
        inside(30.0, 30.0, 50.0, 50.0),
        inside(0.0, 0.0, 100.0, 50.0),
    );
    // the blue is cut by the second hole, the red by both; blue lies over the first hole
    covers_where(&mesh, |point| {
        let red = !first(point) && !second(point);
        let blue = top(point) && !second(point);
        red || blue
    })
}

#[test]
fn a_ring_shaped_knockout_leaves_what_its_hole_encloses() -> Outcome {
    let ring = Drawing::from(Path::from_svg_path(
        "M20 20 L80 20 L80 80 L20 80 Z M40 40 L40 60 L60 60 L60 40 Z",
    )?)
    .fill(BLUE)
    .knockout();
    let mesh = mesh_of(&Drawing::new([filled(0.0, 0.0, 100.0, 100.0, RED)?, ring]), SIDE, SIDE)?;
    let (outer, enclosed) = (inside(20.0, 20.0, 80.0, 80.0), inside(40.0, 40.0, 60.0, 60.0));
    covers_where(&mesh, |point| !outer(point) || enclosed(point))
}

#[test]
fn a_knockout_that_removes_nothing_leaves_the_mesh_as_it_was() -> Outcome {
    let base = Drawing::new([
        filled(0.0, 0.0, 100.0, 100.0, RED)?,
        filled(20.0, 20.0, 30.0, 30.0, GREEN)?,
    ]);
    let plain = mesh_of(&base, SIDE, SIDE)?;
    let nothing_painted = Drawing::from(rect(30.0, 30.0, 40.0, 40.0)?)
        .fill(Color::NONE)
        .knockout();
    let no_path = Drawing::from(Path::new()).fill(BLUE).knockout();
    let far = hole(500.0, 500.0, 10.0, 10.0)?;
    for (case, knocker) in [
        ("nothing painted", nothing_painted),
        ("an empty path", no_path),
        ("far away", far),
    ] {
        let mesh = mesh_of(&Drawing::new([base.clone(), knocker]), SIDE, SIDE)?;
        assert!(
            mesh.vertices() == plain.vertices() && mesh.indices() == plain.indices() && mesh.ranges() == plain.ranges(),
            "{case}"
        );
    }
    Ok(())
}

#[test]
fn the_mesh_covers_the_pixels_that_are_wholly_inside_a_picture_with_knockouts() -> Outcome {
    let disc = |x: f64, y: f64, r: f64| Path::circle((x, y), r);
    let rows = [
        (
            "a disc out of a square",
            Drawing::new([
                filled(10.0, 10.0, 80.0, 80.0, RED)?,
                Drawing::from(disc(50.0, 50.0, 25.0)?).fill(BLUE).knockout(),
            ]),
        ),
        (
            "a fat stroke out of a stroked and filled shape",
            Drawing::new([
                Drawing::from(rect(15.0, 15.0, 70.0, 70.0)?)
                    .fill(RED)
                    .stroke(Stroke::new(GREEN).width(12.0)),
                Drawing::from(disc(50.0, 50.0, 28.0)?)
                    .fill(Color::NONE)
                    .stroke(Stroke::new(BLUE).width(14.0))
                    .knockout(),
            ]),
        ),
        (
            "a fill and a stroke, and a shape after",
            Drawing::new([
                filled(10.0, 10.0, 80.0, 80.0, RED)?,
                Drawing::from(disc(40.0, 50.0, 22.0)?)
                    .fill(BLUE)
                    .stroke(Stroke::new(BLUE).width(8.0))
                    .knockout(),
                Drawing::from(disc(60.0, 50.0, 18.0)?).fill(GREEN),
            ]),
        ),
        (
            "two knockouts and a shape between",
            Drawing::new([
                filled(5.0, 5.0, 90.0, 90.0, RED)?,
                hole(20.0, 20.0, 30.0, 30.0)?,
                filled(0.0, 0.0, 100.0, 40.0, GREEN)?,
                hole(35.0, 35.0, 30.0, 30.0)?,
            ]),
        ),
        (
            "a hard shadow cut",
            Drawing::new([
                filled(15.0, 15.0, 40.0, 40.0, RED)?.shadow(
                    Shadow::new()
                        .dx(15.0)
                        .dy(20.0)
                        .blur(0.0)
                        .color(Color::parse("#000000")?),
                ),
                Drawing::from(disc(55.0, 55.0, 20.0)?).fill(BLUE).knockout(),
            ]),
        ),
    ];
    for (case, drawing) in rows {
        let mesh = mesh_of(&drawing, SIDE, SIDE)?;
        let pixels = render(&drawing, 100, 100)?;
        let all = triangles(&mesh);
        let mut compared = 0;
        for y in 0..100 {
            for x in 0..100 {
                // a pixel with partial alpha is on an edge of the picture; one the mesh cannot place is on an edge of a triangle
                let alpha = pixels.alpha(x, y);
                let centre = (f64::from(u32::try_from(x)?) + 0.5, f64::from(u32::try_from(y)?) + 0.5);
                let (true, Some(count)) = (alpha == 0 || alpha == 255, covering(&all, centre)) else {
                    continue;
                };
                compared += 1;
                assert_eq!(
                    count > 0,
                    alpha == 255,
                    "{case}: pixel ({x}, {y}) has alpha {alpha}, the mesh covers it {count} times"
                );
            }
        }
        assert!(compared > 8_000, "{case}: only {compared} pixels compared");
    }
    Ok(())
}

/// A closed polygon through `count` points scattered over a 1000 square by a fixed xorshift: every edge is long and
/// crosses about a fifth of the others.
fn scribble(count: usize) -> Result<Path, Error> {
    let mut state = 0x1234_5678_9abc_def1_u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        #[expect(clippy::cast_precision_loss, reason = "the top 53 bits")]
        let share = (state >> 11) as f64 / (1_u64 << 53) as f64;
        share * 1000.0
    };
    Path::polyline((0..count).map(|_| (next(), next())).collect::<Vec<_>>(), true)
}

#[test]
fn a_cut_is_charged_for_the_edges_that_cross_and_refused_past_ten_million_segments() -> Outcome {
    // 16,000 long edges cross about 30 million times: the boolean that cuts them is refused before it runs, naming the
    // shape that was to be cut; 400 of them are cut
    let cut = |count| {
        mesh_of(
            &Drawing::new([Drawing::from(scribble(count)?).fill(RED), hole(0.0, 0.0, 500.0, 500.0)?]),
            1000.0,
            1000.0,
        )
    };
    let (name, message) = parameter_refusal(cut(16_000))?;
    assert!(
        name == "shape 0" && message.contains("MAX_SEGMENTS 10000000"),
        "{name}: {message}"
    );
    let mesh = cut(400)?;
    check_layout(&mesh)?;
    assert_ne!(mesh.indices().len(), 0);
    Ok(())
}

#[test]
fn the_shadow_of_a_knockout_is_not_drawn_so_a_blur_on_it_is_no_error() -> Outcome {
    // a blurred shadow has no triangle form and is refused on a shape that paints; on one that knocks out it is not drawn
    let blurred = hole(30.0, 30.0, 40.0, 40.0)?.shadow(Shadow::new().blur(12.0));
    let mesh = mesh_of(
        &Drawing::new([filled(0.0, 0.0, 100.0, 100.0, RED)?, blurred]),
        SIDE,
        SIDE,
    )?;
    let hole = inside(30.0, 30.0, 70.0, 70.0);
    covers_where(&mesh, |point| !hole(point))
}
