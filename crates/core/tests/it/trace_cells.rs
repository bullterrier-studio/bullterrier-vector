//! `Drawing::trace_cells` through the public API. Oracles: the cells counted by hand (a colour's shape has exactly its
//! cells' area, the shapes together the map's), `Path::contains` at every cell's middle against the cell's colour, and
//! the corners of small maps written out.

#![expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::unwrap_used,
    reason = "map sizes and tones are small, a corner of a cell map is an exact multiple, and a proptest body has no Result to return"
)]

use crate::common::region::{net_area, sampled};
use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Drawing, Error, Field, Rect, Shape, Values};
use proptest::prelude::*;

fn map(width: usize, height: usize, bytes: &[u8]) -> Result<Field, Error> {
    Field::map(width, height, bytes, None, None)
}

fn traced(field: &Field, cell: usize, max_colors: usize) -> Result<Vec<Shape>, Error> {
    Drawing::trace_cells(field, cell, max_colors).shapes(&Values::new())
}

fn level(shape: &Shape) -> Option<f64> {
    shape.attributes().label().map(f64::from)
}

#[test]
fn a_checkerboard_is_isolated_squares_with_exact_areas() -> Outcome {
    // 4 by 4 cells of 2 pixels
    let bytes: Vec<u8> = (0..64)
        .map(|i| u8::from(((i % 8) / 2 + (i / 8) / 2) % 2 == 1))
        .collect();
    let shapes = traced(&map(8, 8, &bytes)?, 2, 32)?;
    assert_eq!(shapes.len(), 2);
    let mut total = 0.0;
    for (shape, colour) in shapes.iter().zip([0.0, 1.0]) {
        assert_eq!(level(shape), Some(colour));
        let rings = sampled(shape.path(), 1)?;
        assert_eq!(rings.len(), 8, "eight separate squares");
        assert!(rings.iter().all(|ring| ring.len() == 4), "a square has four corners");
        let area = net_area(shape.path(), 1)?;
        assert!((area - 32.0).abs() < 1e-9, "{area}");
        total += area;
    }
    assert!((total - 64.0).abs() < 1e-9);
    Ok(())
}

#[test]
fn a_ring_of_cells_is_an_outline_and_a_hole_with_the_corners_only() -> Outcome {
    let mut bytes = vec![0_u8; 25];
    bytes[12] = 1;
    let shapes = traced(&map(5, 5, &bytes)?, 1, 32)?;
    let outer = &shapes[0];
    let mut rings = sampled(outer.path(), 1)?;
    rings.sort_by_key(Vec::len);
    assert_eq!(rings.len(), 2);
    assert_eq!(rings[0].len(), 4, "the hole");
    assert_eq!(
        rings[1].len(),
        4,
        "the outline: merged along straight sides, no collinear points"
    );
    assert!((net_area(outer.path(), 1)? - 24.0).abs() < 1e-9);
    assert!((net_area(shapes[1].path(), 1)? - 1.0).abs() < 1e-9);
    assert!(!outer.path().contains((2.5, 2.5))? && outer.path().contains((0.5, 0.5))?);
    Ok(())
}

#[test]
fn cells_that_meet_only_at_a_corner_are_not_merged() -> Outcome {
    let shapes = traced(&map(2, 2, &[1, 0, 0, 1])?, 1, 32)?;
    assert_eq!(shapes.len(), 2);
    for shape in &shapes {
        let rings = sampled(shape.path(), 1)?;
        assert_eq!(rings.len(), 2);
        assert!(rings.iter().all(|ring| ring.len() == 4));
    }
    Ok(())
}

#[test]
fn the_map_box_places_and_scales_the_corners() -> Outcome {
    let frame = Rect {
        x: 10.0,
        y: 20.0,
        width: 40.0,
        height: 80.0,
    };
    let field = Field::map(2, 2, &[0, 0, 1, 1], Some(frame), None)?;
    let shapes = traced(&field, 1, 32)?;
    let rings = sampled(shapes[0].path(), 1)?;
    let mut corners = rings[0].clone();
    corners.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    assert_eq!(corners, vec![(10.0, 20.0), (10.0, 60.0), (50.0, 20.0), (50.0, 60.0)]);
    Ok(())
}

proptest! {
    #[test]
    fn every_cell_is_inside_exactly_its_colours_shape(
        cell in 1_usize..4, columns in 1_usize..7, rows in 1_usize..7,
        seed in proptest::collection::vec(0_u8..4, 36),
    ) {
        let (width, height) = (columns * cell, rows * cell);
        let colour = |c: usize, r: usize| seed[r * 6 + c];
        let bytes: Vec<u8> = (0..width * height).map(|i| colour((i % width) / cell, (i / width) / cell)).collect();
        let shapes = traced(&map(width, height, &bytes).unwrap(), cell, 32).unwrap();
        let mut total = 0.0;
        for shape in &shapes {
            let wanted = level(shape).unwrap() as u8;
            let count = (0..columns * rows).filter(|i| colour(i % columns, i / columns) == wanted).count();
            let area = net_area(shape.path(), 1).unwrap();
            prop_assert!((area - (count * cell * cell) as f64).abs() < 1e-9, "{area} for {count} cells");
            total += area;
        }
        prop_assert!((total - (width * height) as f64).abs() < 1e-9);
        for r in 0..rows {
            for c in 0..columns {
                let middle = ((c * cell) as f64 + cell as f64 / 2.0, (r * cell) as f64 + cell as f64 / 2.0);
                for shape in &shapes {
                    let inside = shape.path().contains(middle).unwrap();
                    prop_assert_eq!(inside, level(shape) == Some(f64::from(colour(c, r))));
                }
            }
        }
    }
}

#[test]
fn a_map_that_is_not_cells_is_refused_naming_the_argument() -> Outcome {
    let named = |outcome: Result<Vec<Shape>, Error>| parameter_refusal(outcome).map(|(name, _)| name);
    assert!(
        named(traced(&map(5, 4, &[0; 20])?, 2, 32))?.contains("cell_size"),
        "5 is not a multiple of 2"
    );
    let mut uneven = vec![0_u8; 16];
    uneven[1] = 1;
    assert!(
        named(traced(&map(4, 4, &uneven)?, 2, 32))?.contains("cell_size"),
        "a cell of two colours"
    );
    let many: Vec<u8> = (0..16).collect();
    assert!(named(traced(&map(4, 4, &many)?, 1, 8))?.contains("max_colors"));
    assert!(
        traced(&map(4, 4, &many)?, 1, 16).is_ok(),
        "16 colours are within a cap of 16"
    );
    assert!(named(Drawing::trace_cells(&Field::constant(0.5)?, 1, 32).shapes(&Values::new()))?.contains("map"));
    assert!(named(traced(&map(4, 4, &[0; 16])?, 0, 32))?.contains("cell_size"));
    Ok(())
}

#[test]
fn a_huge_checkerboard_is_refused_before_anything_is_made() -> Outcome {
    let side = 4096;
    let bytes: Vec<u8> = (0..side * side)
        .map(|i| u8::from((i % side + i / side) % 2 == 1))
        .collect();
    let refusal = parameter_refusal(traced(&map(side, side, &bytes)?, 1, 32))?;
    assert!(
        refusal.0.contains("trace_cells") && refusal.1.contains("MAX_SEGMENTS"),
        "{refusal:?}"
    );
    Ok(())
}
