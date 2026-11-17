//! `Field::map` and `Field::to_map` through the public API. Oracles: values worked out by hand from the bytes
//! (bilinear between pixel centres), and the closed-form distance to a circle.

use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Field, Path, Point, Rect};

/// 3 by 2: row 0 is 0, 100, 200; row 1 is 255, 50, 0.
const BYTES: [u8; 6] = [0, 100, 200, 255, 50, 0];

fn grid() -> Result<Field, bullterrier_vector::Error> {
    Field::map(3, 2, &BYTES, None)
}

fn near(got: f64, want: f64) {
    assert!((got - want).abs() < 1e-9, "{got} against {want}");
}

#[test]
fn a_pixel_centre_reads_its_byte() -> Outcome {
    let field = grid()?;
    for (i, byte) in (0..6_u32).zip(BYTES) {
        let (x, y) = (f64::from(i % 3) + 0.5, f64::from(i / 3) + 0.5);
        near(field.at(Point::new(x, y)), f64::from(byte) / 255.0);
    }
    Ok(())
}

#[test]
fn between_centres_it_is_bilinear() -> Outcome {
    let field = grid()?;
    // half way between 0 and 100 along row 0
    near(field.at(Point::new(1.0, 0.5)), 50.0 / 255.0);
    // the middle of the four centres 0, 100, 255, 50
    near(
        field.at(Point::new(1.0, 1.0)),
        (0.0 + 100.0 + 255.0 + 50.0) / 4.0 / 255.0,
    );
    // a quarter across, three quarters down: weights (3/4)(1/4), (1/4)(1/4), (3/4)(3/4), (1/4)(3/4)
    let want = (0.0 * 0.1875 + 100.0 * 0.0625 + 255.0 * 0.5625 + 50.0 * 0.1875) / 255.0;
    near(field.at(Point::new(0.5 + 0.25, 0.5 + 0.75)), want);
    Ok(())
}

#[test]
fn beyond_the_box_the_nearest_edge_value_holds() -> Outcome {
    let field = grid()?;
    near(field.at(Point::new(-1000.0, -1000.0)), 0.0);
    near(field.at(Point::new(1000.0, 1000.0)), 0.0);
    near(field.at(Point::new(-5.0, 1.5)), 255.0 / 255.0);
    near(field.at(Point::new(2.5, -9.0)), 200.0 / 255.0);
    near(field.at(Point::new(0.0, 0.5)), 0.0);
    Ok(())
}

#[test]
fn a_box_places_the_map_in_design_units() -> Outcome {
    let frame = Rect {
        x: 100.0,
        y: 50.0,
        width: 30.0,
        height: 20.0,
    };
    let field = Field::map(3, 2, &BYTES, Some(frame))?;
    // centre of pixel (2, 1): x = 100 + 2.5 * 10, y = 50 + 1.5 * 10
    near(field.at(Point::new(125.0, 65.0)), 0.0);
    near(field.at(Point::new(105.0, 65.0)), 1.0);
    near(field.at(Point::new(115.0, 65.0)), 50.0 / 255.0);
    Ok(())
}

#[test]
fn equal_bytes_and_box_make_equal_fields() -> Outcome {
    assert_eq!(grid()?, grid()?);
    let mut other = BYTES;
    other[3] = 254;
    assert_ne!(grid()?, Field::map(3, 2, &other, None)?);
    let moved = Rect {
        x: 1.0,
        y: 0.0,
        width: 3.0,
        height: 2.0,
    };
    assert_ne!(grid()?, Field::map(3, 2, &BYTES, Some(moved))?);
    Ok(())
}

#[test]
fn a_map_the_library_cannot_read_is_refused_naming_the_argument() -> Outcome {
    let nan = Rect {
        x: f64::NAN,
        y: 0.0,
        width: 1.0,
        height: 1.0,
    };
    let flat = Rect {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 1.0,
    };
    let far = Rect {
        x: 1e300,
        y: 0.0,
        width: 1.0,
        height: 1.0,
    };
    for (outcome, parameter) in [
        (Field::map(3, 2, &BYTES[..5], None), "data"),
        (Field::map(0, 2, &[], None), "width"),
        (Field::map(2, 0, &[], None), "width"),
        (Field::map(1 << 20, 1 << 20, &[], None), "width"),
        (Field::map(3, 2, &BYTES, Some(nan)), "box"),
        (Field::map(3, 2, &BYTES, Some(flat)), "box"),
        (Field::map(3, 2, &BYTES, Some(far)), "box.x"),
    ] {
        assert_eq!(parameter_refusal(outcome)?.0, parameter);
    }
    Ok(())
}

#[test]
fn a_distance_field_written_out_reads_back_within_half_a_level() -> Outcome {
    let circle = Path::circle((100.0, 100.0), 50.0)?;
    let distance = Field::distance(&circle, false)?;
    let frame = Rect {
        x: 20.0,
        y: 20.0,
        width: 160.0,
        height: 160.0,
    };
    let (low, high) = (-50.0, 30.0);
    let bytes = distance.to_map(64, 64, Some(frame), low, high)?;
    assert_eq!(bytes.len(), 64 * 64);
    let back = Field::map(64, 64, &bytes, Some(frame))?;
    for row in [0, 10, 31, 40, 63] {
        for column in [0, 5, 31, 50, 63] {
            let (x, y) = (
                20.0 + (f64::from(column) + 0.5) * 2.5,
                20.0 + (f64::from(row) + 0.5) * 2.5,
            );
            // the closed form: |q - c| - r, written as a level
            let exact = ((libm::hypot(x - 100.0, y - 100.0) - 50.0 - low) / (high - low)).clamp(0.0, 1.0);
            assert!(
                (back.at(Point::new(x, y)) - exact).abs() < 0.5 / 255.0 + 0.002,
                "at ({x}, {y})"
            );
        }
    }
    Ok(())
}

#[test]
fn to_map_by_hand_on_a_ramp() -> Outcome {
    let ramp = Field::ramp((0.0, 0.0), (4.0, 0.0))?;
    // 4 pixels over 0..4: centres read 0.125, 0.375, 0.625, 0.875; low 0.25 and high 0.75 stretch them to
    // 0 (clamped), 0.25, 0.75 and 1 (clamped): 63.75 and 191.25 round to 64 and 191
    let bytes = ramp.to_map(4, 1, None, 0.25, 0.75)?;
    assert_eq!(bytes, [0_u8, 64, 191, 255]);
    Ok(())
}

#[test]
fn to_map_refuses_what_it_cannot_write() -> Outcome {
    let ramp = Field::ramp((0.0, 0.0), (4.0, 0.0))?;
    for (outcome, parameter) in [
        (ramp.to_map(0, 1, None, 0.0, 1.0), "width"),
        (ramp.to_map(1 << 20, 1 << 20, None, 0.0, 1.0), "width"),
        (ramp.to_map(2, 2, None, 1.0, 1.0), "high"),
        (ramp.to_map(2, 2, None, 0.0, f64::INFINITY), "high"),
        (ramp.to_map(2, 2, None, f64::NAN, 1.0), "high"),
    ] {
        assert_eq!(parameter_refusal(outcome)?.0, parameter);
    }
    Ok(())
}
