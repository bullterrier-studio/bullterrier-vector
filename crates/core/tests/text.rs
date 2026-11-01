//! `text` through the public API: the shapes a string makes in a font, their places, attributes and outlines, and the
//! errors and limits. The expected numbers are literals read from the font file with fontTools and from `HarfBuzz`
//! (uharfbuzz, an independent shaper), not computed by the library; the commands that produced them are written beside
//! them. Every drawing here is made at 2000 pixels to the em, the font's own, so a coordinate in the drawing is a
//! coordinate in the font file, y pointing down.
//!
//! Literals of `fraunces-subset.ttf` (`common/fonts.rs` says how it was made), from
//! `uvx --from fonttools --with uharfbuzz python literals.py fraunces-subset.ttf "HH" "AV" ...`, where the script
//! shapes the string with `HarfBuzz` (`hb.shape`) and prints, per glyph, `(name, cluster, x_advance, x_offset, y_offset)`,
//! and draws a glyph with a `fontTools.pens.basePen.BasePen` that writes `M`, `L`, `Q`, `Z` of `Path::to_svg_path`
//! with y negated (a closed sub-path loses a last line back to its start, as the library's does):
//!
//! ```text
//! 'H' [('H', 0, 1656, 0, 0)]
//! 'AV' [('A', 0, 1334, 0, 0), ('V', 1, 1401, 0, 0)]       (hmtx advance of A: 1424; kerned by -90)
//! 'To' [('T', 0, 1249, 0, 0), ('o', 1, 1166, 0, 0)]       (hmtx advance of T: 1350)
//! 'fi' [('glyph00018', 0, 1333, 0, 0)]                    (one glyph, a ligature)
//! 'f i' [('f', 0, 732, 0, 0), ('space', 1, 449, 0, 0), ('i', 2, 601, 0, 0)]
//! 'Hi To' [('H', 0, 1656), ('i', 1, 601), ('space', 2, 449), ('T', 3, 1249), ('o', 4, 1166)]
//! 'Fe' [('F', 0, 1134, 0, 0), ('e', 1, 1041, 0, 0)]
//! unitsPerEm 2000, OS/2 sCapHeight 1400
//! ```

mod common;

use bullterrier_vector::{Align, Drawing, Error, Font, Param, Path, Shape, Size, Text, Values, text};
use common::fonts::{fraunces, marks, synthetic};
use common::{Outcome, parameter_refusal};
use proptest::prelude::*;

/// 2000 pixels to the em: the font's own.
const EM: f64 = 2000.0;

fn at_em(font: &Font) -> Text {
    Text::new(font).size(EM)
}

fn shapes_of(content: &str, options: Text) -> Result<Vec<Shape>, Error> {
    text(content, options).shapes(&Values::new())
}

/// The outline of the one glyph `content` is, as the library draws it at the origin.
fn alone(content: &str) -> Result<Path, Error> {
    let shapes = shapes_of(content, at_em(&fraunces()?))?;
    assert_eq!(shapes.len(), 1, "{content:?}");
    Ok(shapes[0].path().clone())
}

// Advances and kerning ----------------------------------------------------------------------------------------------

#[test]
fn a_glyph_follows_the_one_before_by_the_advance_the_shaper_gives() -> Outcome {
    // (text, advance of the first glyph): from HarfBuzz, so that the kerned pairs are narrower than the font's hmtx
    let rows = [
        ("HH", "H", "H", 1656.0),
        ("AV", "A", "V", 1334.0), // hmtx says 1424: the pair kerns by -90
        ("To", "T", "o", 1249.0), // hmtx says 1350: -101
        ("Fe", "F", "e", 1134.0),
    ];
    let font = fraunces()?;
    for (content, first, second, advance) in rows {
        let shapes = shapes_of(content, at_em(&font))?;
        assert_eq!(shapes.len(), 2, "{content}");
        assert_eq!(
            shapes[0].path(),
            &alone(first)?,
            "{content}: the first glyph stays at the origin"
        );
        assert_eq!(shapes[1].path(), &alone(second)?.translate(advance, 0.0)?, "{content}");
    }
    Ok(())
}

#[test]
fn a_glyph_the_font_moves_without_moving_the_pen_is_drawn_moved() -> Outcome {
    // synthetic.ttf, 1000 to the em: the square C (0 0 to 100 100) is moved by (50, 20) with an advance of 500
    // (HarfBuzz: `'AC' [('g64', 0, 500, 0, 0), ('C', 1, 500, 50, 20)]`); y points down
    let font = synthetic()?;
    let one = shapes_of("C", Text::new(&font).size(1000.0))?;
    assert_eq!(one[0].path().to_svg_path(), "M50 -20 L50 -120 L150 -120 L150 -20 Z");
    let after = shapes_of("CC", Text::new(&font).size(1000.0))?;
    assert_eq!(after[1].path().to_svg_path(), "M550 -20 L550 -120 L650 -120 L650 -20 Z");
    Ok(())
}

#[test]
fn the_fi_ligature_is_one_shape_and_a_space_keeps_it_apart() -> Outcome {
    let font = fraunces()?;
    let ligature = shapes_of("fi", at_em(&font))?;
    assert_eq!(ligature.len(), 1);
    assert_eq!(
        ligature[0].attributes().char(),
        Some('f'),
        "a ligature stands for its first character"
    );
    // fontTools: BasePen over glyph00018 of fraunces-subset.ttf
    let expected = "M457 -138 Q457 -112 469 -97 Q481 -82 505 -80 L607 -69 Q627 -67 637 -57.5 Q647 -48 647 -33 Q647 0 602 0 \
        L150 0 Q130 0 118.5 -8.5 Q107 -17 107 -32 Q107 -47 117 -55.5 Q127 -64 146 -69 L196 -79 Q222 -85 234.5 -99 \
        Q247 -113 247 -136 L247 -776 Q247 -796 237 -809.5 Q227 -823 200 -831 L144 -847 Q112 -857 101.5 -867 Q91 -877 \
        91 -890 Q91 -903 101 -911 Q111 -919 132 -919 L296 -919 L248 -884 L248 -918 Q248 -950 233 -979.5 Q218 -1009 \
        198 -1040 Q178 -1071 163 -1109 Q148 -1147 148 -1198 Q148 -1313 228 -1384 Q308 -1455 447 -1455 Q536 -1455 \
        598.5 -1426 Q661 -1397 693.5 -1348.5 Q726 -1300 726 -1243 Q726 -1189 699 -1160.5 Q672 -1132 628 -1132 \
        Q581 -1132 553.5 -1162 Q526 -1192 526 -1242 L526 -1290 Q526 -1341 498.5 -1368 Q471 -1395 421 -1395 \
        Q360 -1394 328 -1356 Q296 -1318 296 -1249 Q296 -1206 310.5 -1170.5 Q325 -1135 347.5 -1105 Q370 -1075 392 \
        -1046.5 Q414 -1018 429 -988.5 Q444 -959 444 -924 L444 -883 L416 -919 L652 -919 Q672 -919 682.5 -911.5 \
        Q693 -904 693 -889 Q693 -868 672 -854.5 Q651 -841 606 -841 L457 -841 Z M1154 -900 L1154 -137 Q1154 -110 \
        1164 -96.5 Q1174 -83 1194 -78 L1239 -68 Q1257 -63 1265.5 -54.5 Q1274 -46 1274 -33 Q1274 -18 1263 -9 \
        Q1252 0 1228 0 L866 0 Q843 0 832.5 -9 Q822 -18 822 -33 Q822 -45 830 -53.5 Q838 -62 856 -67 L903 -78 \
        Q923 -84 933 -97 Q943 -110 943 -137 L943 -759 Q943 -781 935.5 -791 Q928 -801 911 -803 L843 -806 \
        Q826 -809 818.5 -816 Q811 -823 811 -834 Q811 -847 819.5 -855.5 Q828 -864 850 -872 L1017 -931 \
        Q1049 -942 1068.5 -947 Q1088 -952 1102 -952 Q1128 -952 1141 -938 Q1154 -924 1154 -900 Z M1026 -1138 \
        Q962 -1138 921.5 -1173.5 Q881 -1209 881 -1265 Q881 -1322 921.5 -1357 Q962 -1392 1026 -1392 \
        Q1092 -1392 1132.5 -1356.5 Q1173 -1321 1173 -1265 Q1173 -1209 1132.5 -1173.5 Q1092 -1138 1026 -1138 Z";
    assert_eq!(ligature[0].path().to_svg_path(), expected);
    let apart = shapes_of("f i", at_em(&font))?;
    assert_eq!(apart.len(), 2, "no ligature across a space");
    assert_eq!(apart[1].path(), &alone("i")?.translate(732.0 + 449.0, 0.0)?);
    Ok(())
}

#[test]
fn a_space_and_a_line_break_make_no_shape() -> Outcome {
    let font = fraunces()?;
    for (content, drawn) in [("A A\nA", 3), (" ", 0), ("\n\n", 0), ("", 0), ("A\n", 1)] {
        assert_eq!(shapes_of(content, at_em(&font))?.len(), drawn, "{content:?}");
    }
    Ok(())
}

// Outlines ----------------------------------------------------------------------------------------------------------

#[test]
fn a_glyph_is_the_outline_the_font_file_holds() -> Outcome {
    // fontTools: BasePen over each glyph, as in the header
    let rows = [
        (
            "-",
            "M682 -637 Q739 -637 718 -584 L696 -525 Q687 -500 673.5 -491 Q660 -482 636 -482 L150 -482 \
             Q93 -482 114 -536 L136 -594 Q146 -620 159 -628.5 Q172 -637 195 -637 Z",
        ),
        (
            "I",
            "M497 -142 Q497 -120 510 -107 Q523 -94 547 -88 L613 -76 Q651 -66 651 -36 Q651 -20 639.5 -10 Q628 0 602 0 \
             L160 0 Q135 0 123 -10 Q111 -20 111 -36 Q111 -66 149 -76 L215 -88 Q239 -94 252 -107 Q265 -120 265 -142 \
             L265 -1258 Q265 -1280 252 -1293 Q239 -1306 215 -1312 L149 -1324 Q111 -1334 111 -1364 Q111 -1381 123 \
             -1390.5 Q135 -1400 160 -1400 L602 -1400 Q628 -1400 639.5 -1390.5 Q651 -1381 651 -1364 Q651 -1334 613 \
             -1324 L547 -1312 Q523 -1306 510 -1293 Q497 -1280 497 -1258 Z",
        ),
        // a composite: e, and an accent placed above it
        (
            "é",
            "M961 -584 Q961 -539 934 -514.5 Q907 -490 855 -490 L216 -490 L216 -554 L696 -554 Q746 -554 746 -598 \
             Q746 -732 688 -806.5 Q630 -881 533 -881 Q457 -881 399.5 -838 Q342 -795 309 -715.5 Q276 -636 276 -528 \
             Q276 -329 371.5 -226 Q467 -123 621 -123 Q720 -123 793 -168 Q866 -213 896 -287 Q909 -303 917.5 -309.5 \
             Q926 -316 936 -316 Q949 -316 955.5 -304 Q962 -292 961 -276 Q955 -194 901.5 -126 Q848 -58 759 -17.5 \
             Q670 23 556 23 Q416 23 308.5 -36 Q201 -95 141.5 -201.5 Q82 -308 82 -452 Q82 -598 140 -712.5 \
             Q198 -827 305.5 -893.5 Q413 -960 561 -960 Q684 -960 773.5 -912 Q863 -864 912 -779.5 Q961 -695 961 -584 Z \
             M619 -1278 Q647 -1333 678 -1364 Q709 -1395 755 -1389 Q795 -1385 815 -1356.5 Q835 -1328 830 -1296 \
             Q826 -1260 801 -1232.5 Q776 -1205 732 -1178 L556 -1045 Q545 -1039 532.5 -1038.5 Q520 -1038 512 -1046 \
             Q502 -1054 504.5 -1065 Q507 -1076 513 -1087 Z",
        ),
    ];
    for (content, expected) in rows {
        assert_eq!(alone(content)?.to_svg_path(), expected, "{content}");
    }
    Ok(())
}

// Alignment, anchor, tracking, leading ------------------------------------------------------------------------------

#[test]
fn a_line_stands_at_its_left_end_middle_or_right_end_against_the_anchor() -> Outcome {
    // "HH" is 2 x 1656 = 3312 wide (HarfBuzz)
    let font = fraunces()?;
    let rows = [(Align::Left, 0.0), (Align::Center, -1656.0), (Align::Right, -3312.0)];
    for (align, shift) in rows {
        let shapes = shapes_of("HH", at_em(&font).align(align))?;
        assert_eq!(shapes[0].path(), &alone("H")?.translate(shift, 0.0)?, "{align:?}");
        assert_eq!(
            shapes[1].path(),
            &alone("H")?.translate(shift + 1656.0, 0.0)?,
            "{align:?}"
        );
    }
    Ok(())
}

#[test]
fn each_line_is_aligned_by_its_own_width() -> Outcome {
    // "H" is 1656 wide and "HH" 3312: right-aligned, the second line starts 1656 to the left of the first
    let shapes = shapes_of("H\nHH", at_em(&fraunces()?).align(Align::Right))?;
    assert_eq!(shapes[0].path(), &alone("H")?.translate(-1656.0, 0.0)?);
    assert_eq!(shapes[1].path(), &alone("H")?.translate(-3312.0, 2400.0)?);
    Ok(())
}

#[test]
fn the_anchor_is_where_the_first_baseline_meets_the_alignment() -> Outcome {
    let font = fraunces()?;
    let shapes = shapes_of("H", at_em(&font).anchor((100.0, 200.0)))?;
    assert_eq!(shapes[0].path(), &alone("H")?.translate(100.0, 200.0)?);
    let centred = shapes_of("H", at_em(&font).anchor((100.0, 200.0)).align(Align::Center))?;
    assert_eq!(centred[0].path(), &alone("H")?.translate(100.0 - 828.0, 200.0)?);
    Ok(())
}

#[test]
fn tracking_adds_thousandths_of_an_em_between_glyphs_and_not_after_the_last() -> Outcome {
    // at 2000 pixels to the em, a thousandth is 2 pixels; "HH" is 3312 wide
    let font = fraunces()?;
    for (tracking, second_at) in [(0.0, 1656.0), (100.0, 1856.0), (-50.0, 1556.0)] {
        let shapes = shapes_of("HH", at_em(&font).tracking(tracking))?;
        assert_eq!(shapes[0].path(), &alone("H")?, "tracking {tracking}");
        assert_eq!(
            shapes[1].path(),
            &alone("H")?.translate(second_at, 0.0)?,
            "tracking {tracking}"
        );
    }
    // right-aligned, the width is 3312 + 200: one gap, between the two
    let right = shapes_of("HH", at_em(&font).tracking(100.0).align(Align::Right))?;
    assert_eq!(right[1].path(), &alone("H")?.translate(-1656.0, 0.0)?);
    Ok(())
}

#[test]
fn tracking_is_added_between_clusters_so_a_mark_stays_over_its_base() -> Outcome {
    // marks.ttf, 1000 to the em: `B` (400 square, advance 500) and U+0301 (100 by 50, no advance) put over it at
    // (150, 450) by the font (HarfBuzz: both in cluster 0); tracking 300 is 300 pixels. y points down.
    let font = marks()?;
    let options = Text::new(&font).size(1000.0).tracking(300.0);
    let svg = |content: &str, options: Text| -> Result<Vec<String>, Error> {
        Ok(shapes_of(content, options)?
            .iter()
            .map(|shape| shape.path().to_svg_path())
            .collect())
    };
    let mark = |x: f64| format!("M{x} -450 L{x} -500 L{} -500 L{} -450 Z", x + 100.0, x + 100.0);
    let square = |x: f64| format!("M{x} 0 L{x} -400 L{} -400 L{} 0 Z", x + 400.0, x + 400.0);
    // B and its mark, then B: one gap, between the clusters
    assert_eq!(
        svg("B\u{301}B", options.clone())?,
        [square(0.0), mark(150.0), square(800.0)],
        "the mark stays at 150 over its base"
    );
    // the gap inside a cluster is not part of the width: the line is 500 + 300 + 500 wide
    assert_eq!(
        svg("B\u{301}B", options.align(Align::Right))?,
        [square(-1300.0), mark(-1150.0), square(-500.0)]
    );
    Ok(())
}

#[test]
fn the_lines_stand_leading_pixels_apart_which_is_one_and_a_fifth_ems_unless_given() -> Outcome {
    let font = fraunces()?;
    for (options, dy) in [
        (at_em(&font), 2400.0),
        (at_em(&font).leading(500.0), 500.0),
        (at_em(&font).leading(0.0), 0.0),
    ] {
        let shapes = shapes_of("H\nH\nH", options)?;
        assert_eq!(shapes[1].path(), &alone("H")?.translate(0.0, dy)?);
        assert_eq!(shapes[2].path(), &alone("H")?.translate(0.0, 2.0 * dy)?);
    }
    Ok(())
}

// Attributes --------------------------------------------------------------------------------------------------------

#[test]
fn each_shape_carries_glyph_char_word_line_order_and_origin() -> Outcome {
    // "Hi To" / "Fe": the pens are 0, 1656, 2706 (after the space), 3955 on line 0 (HarfBuzz) and 0, 1134 on line 1;
    // the origin is the middle of the advance, half of the cap height (1400, OS/2) above the baseline
    let shapes = shapes_of("Hi To\nFe", at_em(&fraunces()?))?;
    // (char, glyph, word, line, order, origin)
    let expected = [
        ('H', 0, 0, 0, 0.0, (828.0, -700.0)),
        ('i', 1, 0, 0, 0.2, (1956.5, -700.0)),
        ('T', 2, 1, 0, 0.4, (3330.5, -700.0)),
        ('o', 3, 1, 0, 0.6, (4538.0, -700.0)),
        ('F', 4, 2, 1, 0.8, (567.0, 1700.0)),
        ('e', 5, 2, 1, 1.0, (1654.5, 1700.0)),
    ];
    assert_eq!(shapes.len(), expected.len());
    for (shape, (character, glyph, word, line, order, origin)) in shapes.iter().zip(expected) {
        let attributes = shape.attributes();
        assert_eq!(attributes.char(), Some(character));
        assert_eq!(attributes.glyph(), Some(glyph), "{character}");
        assert_eq!(attributes.word(), Some(word), "{character}");
        assert_eq!(attributes.line(), Some(line), "{character}");
        assert!(
            (attributes.order().ok_or("no order")? - order).abs() < 1e-12,
            "{character}"
        );
        let at = attributes.origin().ok_or("no origin")?;
        assert_eq!((at.x, at.y), origin, "{character}");
    }
    Ok(())
}

#[test]
fn a_font_that_states_a_cap_height_of_zero_gets_the_default_for_the_origin() -> Outcome {
    // synthetic.ttf has OS/2 version 4 with sCapHeight 0, which states no height: the origin is 0.7 x 1000 / 2 above
    // the baseline, and the middle of the advance of 500
    let shapes = shapes_of("C", Text::new(&synthetic()?).size(1000.0))?;
    let at = shapes[0].attributes().origin().ok_or("no origin")?;
    assert_eq!((at.x, at.y), (250.0, -350.0));
    Ok(())
}

#[test]
fn a_single_glyph_has_order_zero() -> Outcome {
    let shapes = shapes_of("H", at_em(&fraunces()?))?;
    assert_eq!(shapes[0].attributes().order(), Some(0.0));
    Ok(())
}

#[test]
fn the_glyph_and_word_numbers_survive_a_selection() -> Outcome {
    let drawing = text("Hi To", at_em(&fraunces()?)).only(bullterrier_vector::Select::new().attribute("word", 1.0));
    let shapes = drawing.shapes(&Values::new())?;
    let glyphs: Vec<_> = shapes.iter().map(|shape| shape.attributes().glyph()).collect();
    assert_eq!(glyphs, [Some(2), Some(3)]);
    Ok(())
}

// Size --------------------------------------------------------------------------------------------------------------

proptest! {
    /// The text at `size` is the text at the font's size scaled about the origin by `size / 2000`.
    #[test]
    fn size_scales_the_text_linearly(size in 1.0_f64..4000.0, content in "[HiToFe .\\-]{1,6}") {
        let font = fraunces()?;
        let reference = shapes_of(&content, at_em(&font))?;
        let scaled = shapes_of(&content, Text::new(&font).size(size))?;
        prop_assert_eq!(scaled.len(), reference.len());
        let factor = size / EM;
        for (one, two) in reference.iter().zip(&scaled) {
            let (a, b) = (one.path().bounds()?, two.path().bounds()?);
            let tolerance = 1e-9 * (1.0 + a.width.abs().max(a.height.abs()) * factor + a.x.abs() * factor);
            for (expected, found) in [(a.x * factor, b.x), (a.y * factor, b.y), (a.width * factor, b.width), (a.height * factor, b.height)] {
                prop_assert!((expected - found).abs() <= tolerance, "{expected} against {found} at {size}");
            }
        }
    }

    /// Every outline is finite and closed, and sits inside the box the font's own glyphs allow around its line:
    /// between 0.74 em above and 0.02 em below its baseline (the highest and lowest ink of the glyphs drawn here,
    /// 1475 and -36 of 2000 in the font file, from fontTools), and along the line within the widest advance (1.04 em)
    /// and the tracking of the characters before it.
    #[test]
    fn outlines_are_finite_and_closed(size in 1.0_f64..1e6, tracking in -500.0_f64..500.0, content in "[HiToFeAVC .\\-lfé\n]{0,12}") {
        let font = fraunces()?;
        for shape in shapes_of(&content, Text::new(&font).size(size).tracking(tracking))? {
            let data = shape.path().to_svg_path();
            prop_assert_eq!(data.matches('M').count(), data.matches('Z').count(), "{}", data);
            let bounds = shape.path().bounds()?;
            prop_assert!([bounds.x, bounds.y, bounds.width, bounds.height].iter().all(|number| number.is_finite()));
            let line = f64::from(shape.attributes().line().ok_or_else(|| TestCaseError::fail("no line"))?);
            let baseline = line * 1.2 * size;
            let (top, bottom) = (bounds.y, bounds.y + bounds.height);
            prop_assert!(top >= baseline - 0.74 * size && bottom <= baseline + 0.02 * size, "{:?} on baseline {}", bounds, baseline);
            let characters = content.chars().count() as f64;
            let reach = characters * (1.04 + tracking.abs() / 1000.0) * size;
            prop_assert!(bounds.x >= -reach && bounds.x + bounds.width <= reach, "{:?} within ±{}", bounds, reach);
        }
    }
}

// Errors and limits -------------------------------------------------------------------------------------------------

#[test]
fn bytes_that_are_no_font_are_refused_naming_data() -> Outcome {
    let truncated = common::fonts::FRAUNCES[..200].to_vec();
    // a collection: the header of two fonts, the first at offset 20
    let mut collection = b"ttcf\0\x01\0\0\0\0\0\x02\0\0\0\x14\0\0\0\x14".to_vec();
    collection.extend_from_slice(common::fonts::FRAUNCES);
    for (bytes, named) in [
        (b"not a font".to_vec(), "not a font"),
        (Vec::new(), "not a font"),
        (truncated, "no head table"),
        (collection, "collection of 2 fonts"),
        (b"%!PS-AdobeFont-1.0: Name 001.000\n".to_vec(), "TrueType and OpenType"),
    ] {
        let (parameter, message) = parameter_refusal(Font::new(bytes))?;
        assert_eq!(parameter, "data");
        assert!(message.contains(named), "{message}");
    }
    Ok(())
}

/// The position in `font` of the record of the table `tag` in the table directory (12 bytes in, 16 to a record).
fn record_of(font: &[u8], tag: [u8; 4]) -> Result<usize, Box<dyn std::error::Error>> {
    let tables = usize::from(u16::from_be_bytes([font[4], font[5]]));
    (0..tables)
        .map(|at| 12 + 16 * at)
        .find(|record| font[*record..*record + 4] == tag)
        .ok_or_else(|| format!("no {} table", String::from_utf8_lossy(&tag)).into())
}

#[test]
fn a_font_the_library_cannot_draw_is_refused_when_it_is_made() -> Outcome {
    // a real font with one field changed: the units per em in `head` (offset 18 of the table), and the `glyf` table
    // renamed, so that the font has no outlines
    let mut small_em = common::fonts::FRAUNCES.to_vec();
    let head = u32::from_be_bytes(small_em[record_of(&small_em, *b"head")? + 8..][..4].try_into()?) as usize;
    small_em[head + 18..head + 20].copy_from_slice(&8_u16.to_be_bytes());
    let mut no_outlines = common::fonts::FRAUNCES.to_vec();
    let glyf = record_of(&no_outlines, *b"glyf")?;
    no_outlines[glyf..glyf + 4].copy_from_slice(b"glyg");
    for (bytes, named) in [(small_em, "8 units per em"), (no_outlines, "no outlines")] {
        let (parameter, message) = parameter_refusal(Font::new(bytes))?;
        assert_eq!(parameter, "data");
        assert!(message.contains(named), "{message}");
    }
    Ok(())
}

#[test]
fn a_font_longer_than_the_limit_is_refused_before_it_is_read() -> Outcome {
    let (parameter, message) = parameter_refusal(Font::new(vec![0_u8; 32 * 1024 * 1024 + 1]))?;
    assert_eq!(parameter, "data");
    assert!(message.contains("MAX_FONT_BYTES 33554432"), "{message}");
    // exactly at the limit it is read, and found to be no font, not too long
    let (_, message) = parameter_refusal(Font::new(vec![0_u8; 32 * 1024 * 1024]))?;
    assert!(!message.contains("MAX_FONT_BYTES"), "{message}");
    Ok(())
}

#[test]
fn a_size_not_above_zero_is_refused_naming_the_step_and_the_value() -> Outcome {
    let font = fraunces()?;
    for size in [0.0, -3.5, f64::NAN, f64::INFINITY] {
        let drawing = text("H", Text::new(&font).size(size));
        let (parameter, message) = parameter_refusal(Err::<(), _>(drawing.error().cloned().ok_or("no error")?))?;
        assert_eq!(parameter, "source (text).size");
        assert!(message.contains(&size.to_string()), "{message}");
    }
    Ok(())
}

#[test]
fn a_size_tracking_or_leading_that_pushes_the_text_past_the_coordinates_names_that_argument() -> Outcome {
    let font = fraunces()?;
    // each row is the argument that did it, the text and options that put its glyphs beyond ±1e100, and what the message
    // shows: the value of the argument, or for a size the place its glyph came to
    let rows = [
        ("size", "H", Text::new(&font).size(1e300), "e299"),
        ("tracking", "HH", Text::new(&font).size(100.0).tracking(1e300), "1e300"),
        ("leading", "H\nH", Text::new(&font).size(100.0).leading(1e300), "1e300"),
    ];
    for (argument, content, options, value) in rows {
        let (parameter, message) = parameter_refusal(shapes_of(content, options))?;
        assert_eq!(parameter, format!("source (text).{argument}"), "{message}");
        assert!(
            message.contains(value) && message.contains("1e100"),
            "{argument}: {message}"
        );
    }
    Ok(())
}

#[test]
fn a_number_read_once_takes_a_param_but_never_a_list() -> Outcome {
    let size = Param::new("size", 100.0, 1.0, 1000.0)?;
    let drawing = text("Hi", Text::new(&fraunces()?).size(&size));
    assert_eq!(drawing.error(), None);
    let wide = drawing.shapes(&Values::new().set("size", 200.0))?;
    let narrow = drawing.shapes(&Values::new().set("size", 100.0))?;
    assert!(
        wide[1].path().bounds()?.x > narrow[1].path().bounds()?.x,
        "a Param feeds the size"
    );
    // even a list of one number: a text has no shapes to give an entry to
    let (parameter, message) = parameter_refusal(drawing.shapes(&Values::new().set("size", vec![150.0])))?;
    assert_eq!(parameter, "source (text).size");
    assert!(message.contains("list"), "{message}");
    Ok(())
}

#[test]
fn a_text_of_more_than_a_hundred_thousand_characters_is_refused_at_the_call() -> Outcome {
    let font = fraunces()?;
    assert_eq!(text("H".repeat(100_000), &font).error(), None);
    let drawing = text("H".repeat(100_001), &font);
    let (parameter, message) = parameter_refusal(Err::<(), _>(drawing.error().cloned().ok_or("no error")?))?;
    assert_eq!(parameter, "source (text).content");
    assert!(
        message.contains("MAX_GLYPHS 100000") && message.contains("100001"),
        "{message}"
    );
    Ok(())
}

#[test]
fn glyph_outlines_are_charged_to_the_draw_before_any_is_built() -> Outcome {
    // B has 4001 path elements: 2500 of them are past the 10,000,000 a draw may make
    let drawing = text("B".repeat(2500), &synthetic()?);
    let (parameter, message) = parameter_refusal(drawing.shapes(&Values::new()))?;
    assert_eq!(parameter, "source (text)");
    assert!(message.contains("MAX_OUTLINE_SEGMENTS 10000000"), "{message}");
    assert_eq!(shapes_of(&"B".repeat(100), Text::new(&synthetic()?))?.len(), 100);
    Ok(())
}

// Hostile fonts -----------------------------------------------------------------------------------------------------

proptest! {
    /// Bytes cut from or changed in a real font never panic; a font that is still read draws or is refused by kind.
    #[test]
    fn a_broken_font_is_refused_or_read_and_never_panics(
        cut in 0_usize..=common::fonts::FRAUNCES.len(),
        edits in prop::collection::vec((0..common::fonts::FRAUNCES.len(), any::<u8>()), 0..8),
    ) {
        let mut bytes = common::fonts::FRAUNCES[..cut].to_vec();
        for (at, byte) in edits {
            if let Some(slot) = bytes.get_mut(at) {
                *slot = byte;
            }
        }
        match Font::new(bytes) {
            Err(error) => prop_assert!(matches!(error, Error::Parameter { .. }), "{error}"),
            Ok(font) => match text("Hi Toé fi\nAV", at_em(&font)).shapes(&Values::new()) {
                Ok(shapes) => prop_assert!(shapes.len() <= 12),
                Err(error) => prop_assert!(matches!(error, Error::Parameter { .. }), "{error}"),
            },
        }
    }
}

// The cache ---------------------------------------------------------------------------------------------------------

#[test]
fn a_font_is_part_of_what_a_step_is_by_its_bytes_and_not_its_name() -> Outcome {
    let named = fraunces()?.named("anybody")?;
    // the same bytes, built apart (and named): one text, computed once
    let same = Drawing::new([text("H", &fraunces()?), text("H", &named)]);
    same.shapes(&Values::new())?;
    assert_eq!(same.stats().computed, 2, "one text and the stack");
    // other bytes: two texts
    let other = Drawing::new([text("A", &fraunces()?), text("A", &synthetic()?)]);
    other.shapes(&Values::new())?;
    assert_eq!(other.stats().computed, 3, "two texts and the stack");
    Ok(())
}

#[test]
fn a_text_drawn_again_is_not_computed_again() -> Outcome {
    let drawing = text("Hi", at_em(&fraunces()?));
    drawing.shapes(&Values::new())?;
    drawing.shapes(&Values::new())?;
    assert_eq!(drawing.stats().computed, 0);
    assert_eq!(drawing.stats().reused, 1);
    Ok(())
}

// The smallest stack ------------------------------------------------------------------------------------------------

/// The smallest thread stack the library supports (docs/design-decisions.md, "Bounded depth"), in bytes.
const SMALLEST_STACK: usize = 256 * 1024;

#[test]
fn a_glyph_of_the_deepest_composites_the_reader_draws_is_shaped_and_drawn_on_the_smallest_stack() -> Outcome {
    let handle = std::thread::Builder::new().stack_size(SMALLEST_STACK).spawn(|| {
        let run = || -> Outcome {
            let drawing =
                text("AAA", Text::new(&synthetic()?).size(100.0)).fill(bullterrier_vector::Color::hex(0x00_00_00));
            let shapes = drawing.shapes(&Values::new())?;
            assert_eq!(shapes.len(), 3);
            assert!(
                drawing
                    .to_svg(Size::px(400.0, 200.0), &Values::new())?
                    .contains("<path")
            );
            drop(drawing);
            Ok(())
        };
        run().map_err(|error| error.to_string())
    })?;
    handle.join().map_err(|_| "the thread panicked")??;
    Ok(())
}
