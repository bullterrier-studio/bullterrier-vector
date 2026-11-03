//! Web fonts through `Font::new`: a WOFF2 file is read like the TrueType file it packs, a WOFF file is refused with the
//! way out, and a damaged or hostile WOFF2 is refused by kind and never panics. The WOFF2 and WOFF files are made from
//! `fraunces-subset.ttf` by fontTools (`common/fonts.rs` says how), an encoder that shares no code with the decoder the
//! library uses; the digest is a literal from `shasum -a 256`.

use crate::common::fonts::{FRAUNCES, FRAUNCES_WOFF, FRAUNCES_WOFF2, fraunces};
use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Drawing, Error, Font, Inputs, Text, Values, text};
use proptest::prelude::*;

/// Path data of every shape of `content` in `font`, at the font's own 2000 pixels to the em and at `wght` `weight`.
fn outlines(font: &Font, content: &str, weight: f64) -> Result<Vec<String>, Error> {
    let options = Text::new(font).size(2000.0).axes([("wght", weight)]);
    Ok(text(content, options)
        .shapes(&Values::new())?
        .iter()
        .map(|shape| shape.path().to_svg_path())
        .collect())
}

#[test]
fn a_woff2_font_draws_what_the_truetype_font_it_packs_draws() -> Outcome {
    let packed = Font::new(FRAUNCES_WOFF2)?;
    let plain = fraunces()?;
    for weight in [300.0, 400.0, 550.0, 700.0] {
        let found = outlines(&packed, "Hi To AV fi é.", weight)?;
        assert_eq!(found.len(), 9, "{weight}");
        assert_eq!(found, outlines(&plain, "Hi To AV fi é.", weight)?, "wght {weight}");
    }
    assert_eq!(packed.axes(), plain.axes());
    assert_eq!(packed.features(), plain.features());
    Ok(())
}

#[test]
fn a_woff2_font_has_the_advance_the_font_file_gives() -> Outcome {
    // `H` is 1656 units wide in fraunces-subset.ttf (hmtx, read with fontTools): the origin of a shape is the middle
    // of its advance, so the second `H` is 1656 further on than the first
    let shapes = text("HH", Text::new(&Font::new(FRAUNCES_WOFF2)?).size(2000.0)).shapes(&Values::new())?;
    let origins: Vec<f64> = shapes
        .iter()
        .map(|shape| shape.attributes().origin().map(|point| point.x).ok_or("no origin"))
        .collect::<Result<_, _>>()?;
    assert!((origins[1] - origins[0] - 1656.0).abs() < 1e-9, "{origins:?}");
    Ok(())
}

#[test]
fn a_saved_document_names_the_bytes_that_were_supplied_and_loads_back_from_them() -> Outcome {
    // the SHA-256 of the .woff2 file (`shasum -a 256`), not of the TrueType font inside it
    const DIGEST: &str = "8bab72ed1d2ff1780bfa3b195c9df388f9c176e7b08394f2ff4f81002889176b";
    let font = Font::new(FRAUNCES_WOFF2)?.named("fraunces")?;
    let json = text("Hi", Text::new(&font)).to_json()?;
    assert!(json.contains(DIGEST), "{json}");
    let inputs = Inputs::new().set("fraunces", FRAUNCES_WOFF2);
    let loaded = Drawing::from_json(&json, &inputs)?;
    assert_eq!(loaded.to_json()?, json);
    assert_eq!(loaded.shapes(&Values::new())?.len(), 2);
    // the TrueType bytes are another input as far as the document goes
    let wrong = Inputs::new().set("fraunces", FRAUNCES);
    let (parameter, message) = parameter_refusal(Drawing::from_json(&json, &wrong))?;
    assert_eq!(parameter, "inputs.fraunces");
    assert!(message.contains(DIGEST), "{message}");
    Ok(())
}

#[test]
fn a_woff_font_is_refused_and_told_to_be_converted() -> Outcome {
    let (parameter, message) = parameter_refusal(Font::new(FRAUNCES_WOFF))?;
    assert_eq!(parameter, "data");
    assert!(message.contains("WOFF is not read"), "{message}");
    assert!(message.contains("WOFF2"), "{message}");
    Ok(())
}

/// A WOFF2 header (48 bytes) with the given `flavor`, number of tables and `total_sfnt_size`, then `tail`.
fn header(flavor: [u8; 4], tables: u16, total_sfnt_size: u32, tail: &[u8]) -> Vec<u8> {
    let mut bytes = b"wOF2".to_vec();
    bytes.extend_from_slice(&flavor);
    bytes.extend_from_slice(&u32::try_from(48 + tail.len()).unwrap_or(0).to_be_bytes());
    bytes.extend_from_slice(&tables.to_be_bytes());
    bytes.extend_from_slice(&0_u16.to_be_bytes());
    bytes.extend_from_slice(&total_sfnt_size.to_be_bytes());
    bytes.extend_from_slice(&0_u32.to_be_bytes()); // totalCompressedSize
    bytes.extend_from_slice(&[0; 24]); // version, metadata, private data
    bytes.extend_from_slice(tail);
    bytes
}

#[test]
fn a_woff2_that_is_damaged_is_refused_naming_data_and_the_format() -> Outcome {
    let rows: [(&str, Vec<u8>); 6] = [
        ("the signature only", b"wOF2".to_vec()),
        ("a header with no tables", header(*b"\0\x01\0\0", 0, 0, &[])),
        (
            "a header that promises tables it lacks",
            header(*b"\0\x01\0\0", 9, 1000, &[]),
        ),
        ("a collection of fonts", header(*b"ttcf", 1, 1000, &[0x0a, 0, 1, 0])),
        (
            "a table that claims 4 GB",
            header(
                *b"\0\x01\0\0",
                1,
                u32::MAX,
                &[0x3f, 0x80, 0x80, 0x80, 0xc0, 0x00, 0x80, 0x00],
            ),
        ),
        ("the end cut off", FRAUNCES_WOFF2[..FRAUNCES_WOFF2.len() - 40].to_vec()),
    ];
    for (name, bytes) in rows {
        let (parameter, message) = parameter_refusal(Font::new(bytes))?;
        assert_eq!(parameter, "data", "{name}");
        assert!(message.contains("WOFF2"), "{name}: {message}");
    }
    Ok(())
}

proptest! {
    /// A WOFF2 file cut anywhere or with bytes changed never panics: it is refused by kind, or read as a font that
    /// draws or is refused by kind.
    #[test]
    fn a_broken_woff2_is_refused_or_read_and_never_panics(
        cut in 0_usize..=FRAUNCES_WOFF2.len(),
        edits in prop::collection::vec((0..FRAUNCES_WOFF2.len(), any::<u8>()), 0..8),
    ) {
        let mut bytes = FRAUNCES_WOFF2[..cut].to_vec();
        for (at, byte) in edits {
            if let Some(slot) = bytes.get_mut(at) {
                *slot = byte;
            }
        }
        match Font::new(bytes) {
            Err(error) => prop_assert!(matches!(error, Error::Parameter { .. }), "{error}"),
            Ok(font) => match text("Hi Toé fi\nAV", Text::new(&font)).shapes(&Values::new()) {
                Ok(shapes) => prop_assert!(shapes.len() <= 12),
                Err(error) => prop_assert!(matches!(error, Error::Parameter { .. }), "{error}"),
            },
        }
    }
}
