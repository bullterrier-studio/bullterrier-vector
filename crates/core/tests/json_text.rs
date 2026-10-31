//! The JSON text itself: what the parser accepts and refuses at the character level, and how the writer escapes. The
//! documents are written by hand; each row names the character rule it holds.

mod common;

use bullterrier_vector::Drawing;
use common::{Outcome, format_refusal};

/// A document whose one Param carries `meaning` (JSON text, quotes included), read by a translate.
fn with_meaning(meaning: &str) -> String {
    format!(
        r#"{{"version":1,"params":[{{"name":"x","default":0,"min":0,"max":1,"unit":"","meaning":{meaning}}}],"root":[{{"path":"M0 0"}},{{"translate":{{"dx":{{"param":"x"}},"dy":0}}}}]}}"#
    )
}

/// The meaning the document's Param reads back as.
fn meaning_of(text: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(Drawing::from_json(text)?.parameters()?[0].meaning_of().to_owned())
}

#[test]
fn an_escaped_surrogate_pair_is_the_one_character_it_stands_for() -> Outcome {
    // the first, a middle one and the last pair of the supplementary planes
    for (escaped, character) in [
        (r#""\ud800\udc00""#, '\u{10000}'),
        (r#""\ud83d\ude00""#, '\u{1f600}'),
        (r#""\udbff\udfff""#, '\u{10ffff}'),
    ] {
        assert_eq!(meaning_of(&with_meaning(escaped))?, character.to_string(), "{escaped}");
    }
    Ok(())
}

#[test]
fn a_unicode_escape_takes_four_hex_digits_and_nothing_else() -> Outcome {
    assert_eq!(meaning_of(&with_meaning(r#""\u0041\u00e9\u00E9""#))?, "A\u{e9}\u{e9}");
    for bad in [r#""\u+041""#, r#""\u-041""#, r#""\u 041""#, r#""\u004""#, r#""\u004g""#] {
        let (pointer, _) = format_refusal(Drawing::from_json(&with_meaning(bad)))?;
        assert_eq!(pointer, "/params/0/meaning", "{bad}");
    }
    Ok(())
}

#[test]
fn a_raw_control_character_in_a_string_is_refused_and_an_escaped_one_is_read() -> Outcome {
    for code in [0x00_u32, 0x09, 0x0a, 0x1f] {
        let raw = format!("\"{}\"", char::from_u32(code).ok_or("a character")?);
        let (pointer, _) = format_refusal(Drawing::from_json(&with_meaning(&raw)))?;
        assert_eq!(pointer, "/params/0/meaning", "raw 0x{code:02x}");
        let escaped = format!("\"\\u{code:04x}\"");
        assert_eq!(
            meaning_of(&with_meaning(&escaped))?,
            char::from_u32(code).ok_or("a character")?.to_string()
        );
    }
    // 0x20 is the first character a string may hold raw
    assert_eq!(meaning_of(&with_meaning("\" \""))?, " ");
    Ok(())
}

#[test]
fn the_writer_escapes_a_control_character_in_lower_case_and_reads_it_back() -> Outcome {
    let drawing = Drawing::from_json(&with_meaning(r#""a\u001fb\u0000c\n\t""#))?;
    let text = drawing.to_json()?;
    assert!(text.contains(r#""meaning":"a\u001fb\u0000c\n\t""#), "{text}");
    assert_eq!(Drawing::from_json(&text)?.to_json()?, text);
    Ok(())
}

#[test]
fn only_a_space_tab_line_feed_and_carriage_return_are_whitespace() -> Outcome {
    let body = r#"{"version":1,"root":[{"path":"M0 0"}]}"#;
    assert!(Drawing::from_json(&format!(" \t\r\n{body} \t\r\n")).is_ok());
    for stray in ['\u{0b}', '\u{0c}', '\u{a0}', '\u{feff}'] {
        let (pointer, _) = format_refusal(Drawing::from_json(&format!("{stray}{body}")))?;
        assert_eq!(pointer, "", "U+{:04x} before the document", stray as u32);
        let (pointer, _) = format_refusal(Drawing::from_json(&format!("{body}{stray}")))?;
        assert_eq!(pointer, "", "U+{:04x} after the document", stray as u32);
    }
    Ok(())
}

#[test]
fn a_divisor_that_is_a_zero_literal_is_refused_whichever_way_the_zero_is_written() -> Outcome {
    for zero in ["0", "-0", "0.0", "-0.0", "0e5", "-0e-3"] {
        let text = format!(
            r#"{{"version":1,"root":[{{"path":"M0 0"}},{{"translate":{{"dx":{{"div":[1,{zero}]}},"dy":0}}}}]}}"#
        );
        let (pointer, _) = format_refusal(Drawing::from_json(&text))?;
        assert!(pointer.starts_with("/root/1/translate/dx"), "{zero}: {pointer}");
    }
    Ok(())
}

#[test]
fn an_attribute_to_select_by_goes_up_to_the_largest_u32() -> Outcome {
    let select = |value: &str| {
        format!(r#"{{"version":1,"root":[{{"path":"M0 0 L1 0 L1 1 Z"}},{{"only":{{"glyph":{value}}}}}]}}"#)
    };
    // nothing carries `glyph` here, so a value in range loads and fails only when drawn; one past u32 is refused now
    assert!(Drawing::from_json(&select("4294967295")).is_ok());
    let (pointer, _) = format_refusal(Drawing::from_json(&select("4294967296")))?;
    assert!(pointer.starts_with("/root/1/only"), "{pointer}");
    Ok(())
}
