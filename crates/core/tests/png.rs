//! `Drawing::to_png` through the public API: the header of the file it returns, its size, its determinism and the
//! errors it shares with `to_rgba`. That the pixels inside are exactly `to_rgba`'s is proven in Python
//! (`tests/test_output.py`), where a decoder written from the PNG specification with the standard library reads them
//! back; the expected values here are written by hand.

mod common;

use bullterrier_vector::{Color, Drawing, Error, Path, Size, Values};
use common::pixels::pixel_size_refusals;
use common::{Outcome, parameter_refusal};

fn empty() -> Drawing {
    Drawing::new(Vec::<Drawing>::new())
}

fn png(drawing: &Drawing, width: f64, height: f64) -> Result<Vec<u8>, Error> {
    drawing.to_png(Size::px(width, height), &Values::new())
}

/// The big-endian number in `bytes[at..at + 4]`.
fn be32(bytes: &[u8], at: usize) -> Result<u32, Box<dyn std::error::Error>> {
    let four: [u8; 4] = bytes.get(at..at + 4).ok_or("file too short")?.try_into()?;
    Ok(u32::from_be_bytes(four))
}

const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

#[test]
fn the_file_opens_with_the_signature_and_a_header_naming_the_size_as_8_bit_rgba() -> Outcome {
    // (width, height): square, wide, tall and the smallest
    for (width, height) in [(7, 7), (7, 5), (5, 7), (1, 1)] {
        let bytes = png(&empty(), f64::from(width), f64::from(height))?;
        assert_eq!(bytes[..8], SIGNATURE, "{width} by {height}");
        // the first chunk is IHDR: 13 data bytes after the length and the name
        assert_eq!(be32(&bytes, 8)?, 13, "{width} by {height}");
        assert_eq!(&bytes[12..16], b"IHDR", "{width} by {height}");
        assert_eq!(be32(&bytes, 16)?, width, "width of {width} by {height}");
        assert_eq!(be32(&bytes, 20)?, height, "height of {width} by {height}");
        // bit depth 8, colour type 6 (red, green, blue, alpha), deflate, adaptive filtering, no interlace
        assert_eq!(bytes[24..29], [8, 6, 0, 0, 0], "{width} by {height}");
    }
    Ok(())
}

#[test]
fn the_file_ends_with_an_empty_iend_chunk() -> Outcome {
    let bytes = png(&empty(), 9.0, 4.0)?;
    // length 0, the name, and its CRC-32 (a known constant of the format)
    assert_eq!(
        bytes[bytes.len() - 12..],
        [0, 0, 0, 0, b'I', b'E', b'N', b'D', 0xAE, 0x42, 0x60, 0x82]
    );
    Ok(())
}

#[test]
fn a_flat_full_hd_image_is_under_one_percent_of_its_raw_size() -> Outcome {
    let wash = Drawing::from(Path::rect(-10.0, -10.0, 2000.0, 1200.0, 0.0)?).fill(Color::parse("#336699")?);
    let bytes = png(&wash, 1920.0, 1080.0)?;
    let raw = 1920 * 1080 * 4;
    assert!(bytes.len() * 100 < raw, "{} bytes of {raw}", bytes.len());
    Ok(())
}

#[test]
fn the_same_call_returns_the_same_bytes() -> Outcome {
    let drawing = Drawing::from(Path::circle((30.0, 20.0), 15.0)?).fill(Color::parse("#ff880080")?);
    assert_eq!(png(&drawing, 64.0, 48.0)?, png(&drawing, 64.0, 48.0)?);
    Ok(())
}

#[test]
fn a_size_outside_whole_pixels_from_1_to_65000_is_refused_naming_the_side() -> Outcome {
    for (case, width, height, side, shown) in pixel_size_refusals() {
        let (parameter, message) = parameter_refusal(png(&empty(), width, height))?;
        assert_eq!(parameter, side, "{case}");
        assert!(message.contains(shown), "{case}: {message}");
    }
    Ok(())
}

#[test]
fn a_shape_past_ten_million_device_pixels_is_refused_as_to_rgba_refuses_it() -> Outcome {
    let far = Drawing::from(Path::rect(0.0, 0.0, 2e7, 4.0, 0.0)?).fill(Color::parse("#000000")?);
    let (parameter, message) = parameter_refusal(png(&far, 10.0, 10.0))?;
    let (rgba_parameter, rgba_message) = parameter_refusal(far.to_rgba(Size::px(10.0, 10.0), &Values::new()))?;
    assert_eq!(parameter, "shape 0");
    assert_eq!((parameter, message), (rgba_parameter, rgba_message));
    Ok(())
}

#[test]
fn the_largest_side_is_encoded() -> Outcome {
    let bytes = png(&empty(), 65_000.0, 1.0)?;
    assert_eq!(be32(&bytes, 16)?, 65_000);
    assert_eq!(be32(&bytes, 20)?, 1);
    Ok(())
}
