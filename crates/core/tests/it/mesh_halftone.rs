//! A halftone in the mesh `Drawing::to_mesh` fills: the dots as triangles that cover each point of them once, none
//! outside the shape, the dots that cross the outline cut to it. The oracles are the lattice worked out here from the
//! pitch and the angle, the shape's own geometry, the pixels, and `Path::contains`, never the screen's own classification.

use crate::common::mesh::{area, check_layout, corners, covering, mesh_of, same_triangles, triangles};
use crate::common::pixels::render;
use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Color, Deg, Drawing, Error, Halftone, Mesh, Path, Shadow, Size, Stroke, Values};
use std::f64::consts::PI;

const RED: Color = Color::hex(0xff_00_00);
const BLACK: Color = Color::hex(0x00_00_00);
const SIDE: f64 = 100.0;

fn rect(left: f64, top: f64, width: f64, height: f64) -> Result<Path, Error> {
    Path::rect(left, top, width, height, 0.0)
}

fn disc() -> Result<Path, Error> {
    Path::circle((50.0, 50.0), 40.0)
}

/// How far `point` is from the nearest point of the lattice `pitch` apart turned `degrees` about the origin.
fn from_lattice((x, y): (f64, f64), pitch: f64, degrees: f64) -> f64 {
    let (sin, cos) = (libm::sin(degrees.to_radians()), libm::cos(degrees.to_radians()));
    // the point in lattice units: turned back
    let (u, v) = ((x * cos + y * sin) / pitch, (y * cos - x * sin) / pitch);
    pitch * libm::hypot(u - libm::round(u), v - libm::round(v))
}

/// An error unless the mesh covers each point of a lattice of every third pixel of the 100 by 100 canvas (offset by a
/// third of a pixel from the pixel centres) that `wanted` says (`Some(true)` once, `Some(false)` not at all) as it says,
/// and no point more than once; points too near an edge to say, and those `wanted` leaves open (`None`), are not
/// checked.
fn covers_as(mesh: &Mesh, wanted: impl Fn((f64, f64)) -> Option<bool>) -> Outcome {
    let all = triangles(mesh);
    let (mut decided, mut covered) = (0, 0);
    for column in 0..33 {
        for row in 0..33 {
            let point = (f64::from(column) * 3.0 + 0.7, f64::from(row) * 3.0 + 0.7);
            let Some(count) = covering(&all, point) else { continue };
            if count > 1 {
                return Err(format!("{point:?} is covered {count} times").into());
            }
            if let Some(should) = wanted(point) {
                decided += 1;
                covered += usize::from(should);
                if (count == 1) != should {
                    return Err(format!(
                        "{point:?} is covered {count} times, but should{} be",
                        if should { "" } else { " not" }
                    )
                    .into());
                }
            }
        }
    }
    if decided < 500 || covered < 20 {
        return Err(format!("only {decided} points were decided, {covered} of them covered").into());
    }
    Ok(())
}

/// The mesh covers a point if it is clearly inside the shape (by `margin`) and clearly inside a dot of radius `radius`,
/// not if clearly outside either.
fn dots_in_disc(pitch: f64, degrees: f64, radius: f64) -> impl Fn((f64, f64)) -> Option<bool> {
    move |point| {
        let from_centre = libm::hypot(point.0 - 50.0, point.1 - 50.0);
        let near = from_lattice(point, pitch, degrees);
        if from_centre > 40.0 + 0.05 || near > radius + 0.001 {
            Some(false)
        } else if from_centre < 40.0 - 0.05 && near < radius - 0.05 {
            Some(true)
        } else {
            None
        }
    }
}

#[test]
fn the_triangles_cover_the_dots_in_the_shape_once_and_nothing_else() -> Outcome {
    // (pitch, angle, tone): dots wholly inside, dots cut by the circle, and dots that overlap (the tone below a fifth)
    let rows = [(8.0, 30.0, 0.5), (6.0, 0.0, 0.7), (10.0, 45.0, 0.1), (12.0, -17.0, 0.9)];
    for (pitch, angle, tone) in rows {
        let drawing = Drawing::from(disc()?).halftone(Halftone::new(BLACK, tone).pitch(pitch).angle(Deg(angle)));
        let mesh = mesh_of(&drawing, SIDE, SIDE)?;
        check_layout(&mesh)?;
        let radius = pitch * libm::sqrt((1.0 - tone) / PI);
        covers_as(&mesh, dots_in_disc(pitch, angle, radius))
            .map_err(|e| format!("pitch {pitch}, angle {angle}, tone {tone}: {e}"))?;
    }
    Ok(())
}

#[test]
fn the_dots_that_cross_the_outline_never_leave_the_shape() -> Outcome {
    // a thin shape, so that most dots cross: a triangle and a ring, every triangle's middle inside the shape
    let shapes = [
        Path::from_svg_path("M10 10 L90 30 L20 70 Z")?,
        Path::from_svg_path("M10 10 L90 10 L90 90 L10 90 Z M25 25 L25 75 L75 75 L75 25 Z")?,
        disc()?,
    ];
    for shape in shapes {
        let drawing = Drawing::from(shape.clone()).halftone(Halftone::new(BLACK, 0.4).pitch(9.0).angle(Deg(15.0)));
        let mesh = mesh_of(&drawing, SIDE, SIDE)?;
        let mut checked = 0;
        for triangle in triangles(&mesh) {
            let middle = (
                (triangle[0].0 + triangle[1].0 + triangle[2].0) / 3.0,
                (triangle[0].1 + triangle[1].1 + triangle[2].1) / 3.0,
            );
            assert!(
                shape.contains(middle)?,
                "a triangle with its middle at {middle:?} is outside the shape"
            );
            checked += 1;
        }
        assert!(checked > 100, "{checked} triangles");
    }
    Ok(())
}

#[test]
fn the_area_of_the_triangles_is_the_area_the_pixels_cover() -> Outcome {
    for (pitch, angle, tone) in [(8.0, 30.0, 0.5), (5.0, 45.0, 0.8)] {
        let drawing = Drawing::from(disc()?).halftone(Halftone::new(BLACK, tone).pitch(pitch).angle(Deg(angle)));
        let from_triangles: f64 = triangles(&mesh_of(&drawing, SIDE, SIDE)?).iter().map(area).sum();
        let from_pixels = render(&drawing, 100, 100)?.covered_area();
        assert!(
            (from_triangles - from_pixels).abs() < 0.015 * from_pixels,
            "pitch {pitch}: {from_triangles} against {from_pixels}"
        );
    }
    Ok(())
}

#[test]
fn a_tone_of_zero_is_the_solid_fill_and_a_tone_of_one_has_no_triangles() -> Outcome {
    let solid = mesh_of(&Drawing::from(disc()?).fill(RED), SIDE, SIDE)?;
    let zero = mesh_of(&Drawing::from(disc()?).halftone(Halftone::new(RED, 0.0)), SIDE, SIDE)?;
    same_triangles(&zero, &solid)?;
    let bare = mesh_of(&Drawing::from(disc()?).halftone(Halftone::new(RED, 1.0)), SIDE, SIDE)?;
    assert_eq!(bare.indices(), []);
    Ok(())
}

#[test]
fn a_stroke_is_a_layer_over_the_dots_and_the_colour_and_opacity_are_the_vertices() -> Outcome {
    let drawing = Drawing::from(rect(10.0, 10.0, 80.0, 80.0)?)
        .halftone(Halftone::new(color("#ff0000ff")?, 0.5).pitch(20.0).angle(Deg(0.0)))
        .stroke(Stroke::new(BLACK).width(4.0))
        .opacity(0.5);
    let mesh = mesh_of(&drawing, SIDE, SIDE)?;
    check_layout(&mesh)?;
    let colours: Vec<[u32; 4]> = (0..mesh.indices().len() / 3)
        .map(|number| corners(&mesh, number)[0].rgba.map(f32::to_bits))
        .collect();
    let half = |channel: f32| (channel * 0.5).to_bits();
    let (red, black) = ([half(1.0), 0, 0, half(1.0)], [0, 0, 0, half(1.0)]);
    // the dots come first, and the stroke last
    let first_black = colours.iter().position(|colour| *colour == black).ok_or("no stroke")?;
    assert!(colours[..first_black].iter().all(|colour| *colour == red));
    assert!(colours[first_black..].iter().all(|colour| *colour == black));
    assert!(first_black > 20);
    Ok(())
}

fn color(text: &str) -> Result<Color, Error> {
    Color::parse(text)
}

#[test]
fn a_hard_shadow_is_a_copy_of_the_dots_moved() -> Outcome {
    // dots of radius 15.96 at 40 and 80 (and 0, cut); the copy moved 20 right and down has its dots at 60 and 100
    let drawing = Drawing::from(rect(5.0, 5.0, 90.0, 90.0)?)
        .halftone(Halftone::new(RED, 0.5).pitch(40.0).angle(Deg(0.0)))
        .shadow(Shadow::new().dx(20.0).dy(20.0).blur(0.0).color(color("#0000ffff")?));
    let mesh = mesh_of(&drawing, SIDE, SIDE)?;
    let all = triangles(&mesh);
    // a dot, a dot of the copy, and the gap between: each point is covered by what the dots and the copy cover there
    for (point, count) in [
        ((40.5, 40.5), Some(1)),
        ((60.5, 60.5), Some(1)),
        ((60.5, 40.5), Some(0)),
    ] {
        assert_eq!(covering(&all, point), count, "{point:?}");
    }
    // the blur has no triangles
    let blurred = Drawing::from(rect(5.0, 5.0, 90.0, 90.0)?)
        .halftone(Halftone::new(RED, 0.5).pitch(40.0))
        .shadow(Shadow::new().blur(8.0));
    let (name, message) = parameter_refusal(mesh_of(&blurred, SIDE, SIDE))?;
    assert_eq!(name, "shadow", "{message}");
    Ok(())
}

#[test]
fn a_knockout_of_dots_takes_the_dots_out_of_what_is_below() -> Outcome {
    let drawing = Drawing::new([
        Drawing::from(rect(0.0, 0.0, 100.0, 100.0)?).fill(RED),
        Drawing::from(rect(0.0, 0.0, 100.0, 100.0)?)
            .halftone(Halftone::new(BLACK, 0.5).pitch(20.0).angle(Deg(0.0)))
            .knockout(),
    ]);
    let mesh = mesh_of(&drawing, SIDE, SIDE)?;
    let radius = 20.0 * libm::sqrt(0.5 / PI);
    covers_as(&mesh, |point| {
        let near = from_lattice(point, 20.0, 0.0);
        if near < radius - 0.05 {
            Some(false)
        } else if near > radius + 0.05 {
            Some(true)
        } else {
            None
        }
    })
}

#[test]
fn a_pitch_that_asks_for_millions_of_dots_is_refused_naming_the_pitch() -> Outcome {
    let drawing = Drawing::from(Path::circle((500.0, 500.0), 500.0)?).halftone(Halftone::new(BLACK, 0.5).pitch(0.01));
    let (name, message) = parameter_refusal(drawing.to_mesh(Size::px(100.0, 100.0), &Values::new(), &mut Mesh::new()))?;
    assert!(
        name.contains("pitch") && message.contains("MAX_SEGMENTS"),
        "{name}: {message}"
    );
    Ok(())
}

#[test]
fn a_pitch_too_small_for_the_numbers_of_its_lattice_is_refused_naming_the_pitch() -> Outcome {
    // (pitch, where the shape is): the lattice coordinates, the position over the pitch, are 10^301 in the first row,
    // and beyond what a float holds in the others, which leaves no count of dots to charge
    for (pitch, at) in [(1e-300, 10.0), (1e-320, 10.0), (1e-310, 50.0)] {
        let drawing =
            Drawing::from(rect(at, at, 10.0, 10.0)?).halftone(Halftone::new(BLACK, 0.5).pitch(pitch).angle(Deg(0.0)));
        let (name, message) =
            parameter_refusal(drawing.to_mesh(Size::px(100.0, 100.0), &Values::new(), &mut Mesh::new()))?;
        assert!(
            name.contains("pitch") && message.contains("MAX_SEGMENTS"),
            "pitch {pitch}: {name}: {message}"
        );
    }
    Ok(())
}
