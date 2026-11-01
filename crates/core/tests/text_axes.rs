//! `text` with the font's variable axes and OpenType features, through the public API: what `Font::axes` and
//! `Font::features` report, a text drawn at the minimum, default and maximum of `wght`, a number per character, the
//! features switched off, the errors and the cache. The expected numbers are literals from fontTools (the outline of a
//! glyph at a location, drawn by `varLib.instancer` and a `BasePen`) and from `HarfBuzz` (uharfbuzz, an independent
//! shaper, with `font.set_variations`); the commands that produced them are written beside them. Every drawing is made
//! at 2000 pixels to the em, the font's own, so a coordinate is a coordinate of the font file, y pointing down.
//!
//! Literals of `fraunces-subset.ttf` (`wght` 300 to 700, default 400), from
//! `uvx --from fonttools --with uharfbuzz python literals.py fraunces-subset.ttf`, where the script prints per string
//! `hb.shape` as `(name, cluster, x_advance, x_offset, y_offset)` at `wght` 300, 400, 700 (and 350, 500, 600), with
//! `features={"liga": False}` and `{"kern": False}`, and the outline of `hyphen` after `instancer.instantiateVariableFont`:
//!
//! ```text
//! advance of      wght 300   350   400   500   600   700
//!   H                 1608  1632  1656  1680  1704  1752
//!   -  (hyphen)        844   838   832   826   820   808
//!   A (before V)      1280          1334                1441     (hmtx advance of A at 400: 1424, kerned by -90)
//!   T (before o)      1203          1249                1341
//!   f (before i)       674           732                 849
//!   i (after f)        568           601                 667
//! 'fi' at 400: one glyph of 1333 (liga); with liga off: f 732, i 601;   'AV' at 400 with kern off: A 1424, V 1401
//! ```

mod common;

use bullterrier_vector::{Drawing, Error, Font, Param, Path, Shape, Text, Unit, Values, text};
use common::fonts::{fraunces, marks, synthetic};
use common::{Outcome, parameter_refusal};
use proptest::prelude::*;

/// 2000 pixels to the em: the font's own.
const EM: f64 = 2000.0;

fn at_em(font: &Font) -> Text {
    Text::new(font).size(EM)
}

fn at_weight(font: &Font, weight: f64) -> Text {
    at_em(font).axes([("wght", weight)])
}

fn shapes_of(content: &str, options: Text) -> Result<Vec<Shape>, Error> {
    text(content, options).shapes(&Values::new())
}

/// The outline of the one glyph `content` is at `wght` `weight`, as the library draws it at the origin.
fn alone_at(content: &str, weight: f64) -> Result<Path, Error> {
    let shapes = shapes_of(content, at_weight(&fraunces()?, weight))?;
    assert_eq!(shapes.len(), 1, "{content:?}");
    Ok(shapes[0].path().clone())
}

/// The x of the middle of the advance of shape `at`: the pen in front of it plus half its advance.
fn middle(shapes: &[Shape], at: usize) -> Result<f64, Box<dyn std::error::Error>> {
    Ok(shapes[at].attributes().origin().ok_or("no origin")?.x)
}

/// The error a call left the drawing with, as its parameter and message.
fn refusal(drawing: &Drawing) -> Result<(String, String), Box<dyn std::error::Error>> {
    parameter_refusal(Err::<(), _>(drawing.error().cloned().ok_or("no error")?))
}

// What the font reports ---------------------------------------------------------------------------------------------

#[test]
fn a_variable_font_reports_its_axes_as_params_with_its_own_range_default_and_name() -> Outcome {
    // fontTools: `fvar` axes [('wght', 300.0, 400.0, 700.0, 257)], name 257 is "Weight"
    let axes = fraunces()?.axes();
    assert_eq!(axes.len(), 1);
    let wght = &axes[0];
    assert_eq!(
        (wght.name(), wght.min(), wght.default(), wght.max()),
        ("wght", 300.0, 400.0, 700.0)
    );
    assert_eq!((wght.meaning_of(), wght.unit_of()), ("Weight", Unit::None));
    // a static font has none
    assert_eq!(synthetic()?.axes().len(), 0);
    Ok(())
}

#[test]
fn a_font_reports_the_feature_tags_of_its_layout_tables_sorted() -> Outcome {
    // fontTools: the FeatureList tags of GSUB and GPOS
    for (font, tags) in [
        (fraunces()?, vec!["kern", "liga"]),
        (synthetic()?, vec!["kern"]),
        (marks()?, vec!["mark"]),
    ] {
        assert_eq!(font.features(), tags);
    }
    Ok(())
}

// Axis values -------------------------------------------------------------------------------------------------------

#[test]
fn the_second_glyph_follows_the_first_by_its_advance_at_that_weight() -> Outcome {
    // (first, second, wght, advance of the first at that wght): HarfBuzz, with the kern pair applied
    let rows = [
        ("A", "V", 300.0, 1280.0),
        ("A", "V", 400.0, 1334.0),
        ("A", "V", 700.0, 1441.0),
        ("T", "o", 300.0, 1203.0),
        ("T", "o", 700.0, 1341.0),
        ("H", "H", 300.0, 1608.0),
        ("H", "H", 400.0, 1656.0),
        ("H", "H", 700.0, 1752.0),
        ("-", "H", 300.0, 844.0),
        ("-", "H", 700.0, 808.0), // the hyphen gets narrower as it gets bold
    ];
    let font = fraunces()?;
    for (first, second, weight, advance) in rows {
        let shapes = shapes_of(&format!("{first}{second}"), at_weight(&font, weight))?;
        assert_eq!(shapes.len(), 2);
        assert_eq!(
            shapes[1].path(),
            &alone_at(second, weight)?.translate(advance, 0.0)?,
            "{first}{second} at {weight}"
        );
    }
    Ok(())
}

#[test]
fn a_glyph_is_the_outline_the_font_has_at_the_minimum_default_and_maximum() -> Outcome {
    // fontTools: `instancer.instantiateVariableFont(font, {"wght": w})`, a BasePen over `hyphen`, y negated
    let rows = [
        (
            300.0,
            "M683 -621 Q727 -621 712 -584 L692 -535 Q686 -518 675.5 -511.5 Q665 -505 646 -505 L161 -505 \
             Q117 -505 132 -543 L151 -591 Q159 -610 168.5 -615.5 Q178 -621 197 -621 Z",
        ),
        (
            400.0,
            "M682 -637 Q739 -637 718 -584 L696 -525 Q687 -500 673.5 -491 Q660 -482 636 -482 L150 -482 \
             Q93 -482 114 -536 L136 -594 Q146 -620 159 -628.5 Q172 -637 195 -637 Z",
        ),
        (
            700.0,
            "M680 -669 Q762 -669 731 -584 L703 -505 Q689 -465 669 -450.5 Q649 -436 616 -436 L129 -436 \
             Q46 -436 78 -522 L105 -600 Q120 -640 139.5 -654.5 Q159 -669 191 -669 Z",
        ),
    ];
    for (weight, expected) in rows {
        assert_eq!(alone_at("-", weight)?.to_svg_path(), expected, "wght {weight}");
    }
    Ok(())
}

#[test]
fn the_default_of_an_axis_is_what_a_text_without_axes_draws() -> Outcome {
    let font = fraunces()?;
    let plain = shapes_of("Hi To", at_em(&font))?;
    assert_eq!(shapes_of("Hi To", at_weight(&font, 400.0))?, plain);
    assert_ne!(shapes_of("Hi To", at_weight(&font, 300.0))?, plain);
    Ok(())
}

#[test]
fn a_font_axis_param_feeds_the_axis_and_its_value_is_the_weight() -> Outcome {
    let font = fraunces()?;
    let wght = font.axes().remove(0);
    let drawing = text("Hi To", at_em(&font).axes([("wght", &wght)]));
    assert_eq!(drawing.error(), None);
    // the Param's default is the font's default; a value is the weight
    assert_eq!(
        drawing.shapes(&Values::new())?,
        shapes_of("Hi To", at_weight(&font, 400.0))?
    );
    assert_eq!(
        drawing.shapes(&Values::new().set("wght", 700.0))?,
        shapes_of("Hi To", at_weight(&font, 700.0))?
    );
    Ok(())
}

#[test]
fn an_expression_of_a_param_feeds_the_axis() -> Outcome {
    let font = fraunces()?;
    let boost = Param::new("boost", 0.0, 0.0, 300.0)?;
    let drawing = text("Hi", at_em(&font).axes([("wght", 400.0 + &boost)]));
    assert_eq!(
        drawing.shapes(&Values::new().set("boost", 300.0))?,
        shapes_of("Hi", at_weight(&font, 700.0))?
    );
    Ok(())
}

proptest! {
    /// A wider axis makes the word wider: the middle of the last of three H moves right as `wght` rises (every
    /// advance of H grows from 1608 at 300 to 1752 at 700, in HarfBuzz).
    #[test]
    fn a_heavier_weight_never_makes_a_word_narrower(a in 300.0_f64..=700.0, b in 300.0_f64..=700.0) {
        let font = fraunces()?;
        let (light, heavy) = (a.min(b), a.max(b));
        let last = |weight: f64| -> Result<f64, TestCaseError> {
            middle(&shapes_of("HHH", at_weight(&font, weight))?, 2).map_err(|error| TestCaseError::fail(error.to_string()))
        };
        let (at_light, at_heavy) = (last(light)?, last(heavy)?);
        prop_assert!(at_light <= at_heavy, "{} at {} against {} at {}", at_light, light, at_heavy, heavy);
    }
}

// A number per character --------------------------------------------------------------------------------------------

#[test]
fn a_list_gives_each_character_its_own_weight_and_the_advances_follow() -> Outcome {
    // fontTools/HarfBuzz: the hyphen at 300, 400 and 700 is the outline of the test above, and its advances are 844, 832, 808
    let font = fraunces()?;
    let wght = font.axes().remove(0);
    let drawing = text("---", at_em(&font).axes([("wght", &wght)]));
    let shapes = drawing.shapes(&Values::new().set("wght", vec![300.0, 400.0, 700.0]))?;
    let hyphen = |weight: f64, dx: f64| alone_at("-", weight)?.translate(dx, 0.0);
    assert_eq!(shapes[0].path(), &hyphen(300.0, 0.0)?);
    assert_eq!(shapes[1].path(), &hyphen(400.0, 844.0)?);
    assert_eq!(shapes[2].path(), &hyphen(700.0, 844.0 + 832.0)?);
    Ok(())
}

#[test]
fn a_rising_ramp_makes_the_advances_rise() -> Outcome {
    // H at 300, 350, 400, 500, 700 advances 1608, 1632, 1656, 1680, 1752 (HarfBuzz): the pens are 0, 1608, 3240, 4896, 6576
    let font = fraunces()?;
    let wght = font.axes().remove(0);
    let drawing = text("HHHHH", at_em(&font).axes([("wght", &wght)]));
    let shapes = drawing.shapes(&Values::new().set("wght", vec![300.0, 350.0, 400.0, 500.0, 700.0]))?;
    let middles: Vec<f64> = (0..5).map(|at| middle(&shapes, at)).collect::<Result<_, _>>()?;
    assert_eq!(middles, vec![804.0, 2424.0, 4068.0, 5736.0, 7452.0]);
    Ok(())
}

#[test]
fn a_line_break_and_a_space_take_an_entry_of_the_list_too() -> Outcome {
    // "H\nH": the entries are H, the line break, H: the second H is the third entry
    let font = fraunces()?;
    let wght = font.axes().remove(0);
    let drawing = text("H\nH", at_em(&font).axes([("wght", &wght)]));
    let shapes = drawing.shapes(&Values::new().set("wght", vec![300.0, 400.0, 700.0]))?;
    assert_eq!(shapes[1].path(), &alone_at("H", 700.0)?.translate(0.0, 1.2 * EM)?);
    // a space is a character too: "H H" has three entries
    let spaced = text("H H", at_em(&font).axes([("wght", &wght)]));
    assert_eq!(
        spaced
            .shapes(&Values::new().set("wght", vec![300.0, 400.0, 700.0]))?
            .len(),
        2
    );
    parameter_refusal(spaced.shapes(&Values::new().set("wght", vec![300.0, 700.0])))?;
    let (parameter, message) = parameter_refusal(drawing.shapes(&Values::new().set("wght", vec![300.0, 700.0])))?;
    assert_eq!(parameter, "source (text).axes");
    assert!(
        message.contains("wght") && message.contains("2 numbers") && message.contains("3 items"),
        "{message}"
    );
    Ok(())
}

#[test]
fn kerning_and_ligatures_do_not_cross_a_change_of_weight() -> Outcome {
    let font = fraunces()?;
    let wght = font.axes().remove(0);
    let drawing = text("fi", at_em(&font).axes([("wght", &wght)]));
    // one weight: the ligature; two: f and i apart (f 674 at 300, then i at 700, 667 wide)
    let one = drawing.shapes(&Values::new().set("wght", vec![700.0, 700.0]))?;
    assert_eq!(one.len(), 1);
    let two = drawing.shapes(&Values::new().set("wght", vec![300.0, 700.0]))?;
    assert_eq!(two.len(), 2);
    assert_eq!(middle(&two, 1)?, 674.0 + 667.0 / 2.0);
    // the same pair kerned in one run, and apart when the weight changes: A 1280 then V at 300; at 400 the kern is -90
    let kerned = text("AV", at_em(&font).axes([("wght", &wght)]));
    let together = kerned.shapes(&Values::new().set("wght", vec![400.0, 400.0]))?;
    let apart = kerned.shapes(&Values::new().set("wght", vec![400.0, 400.000_1]))?;
    assert_eq!(middle(&together, 1)?, 1334.0 + 1401.0 / 2.0);
    assert!(
        (middle(&apart, 1)? - (1424.0 + 1401.0 / 2.0)).abs() < 0.5,
        "no kerning across the change, got {}",
        middle(&apart, 1)?
    );
    Ok(())
}

#[test]
fn a_ramp_over_thousands_of_characters_is_drawn() -> Outcome {
    let font = fraunces()?;
    let wght = font.axes().remove(0);
    let count = 5000_u32;
    let drawing = text("H".repeat(count as usize), at_em(&font).axes([("wght", &wght)]));
    let ramp: Vec<f64> = (0..count)
        .map(|at| 300.0 + 400.0 * f64::from(at) / f64::from(count - 1))
        .collect();
    let shapes = drawing.shapes(&Values::new().set("wght", ramp))?;
    assert_eq!(shapes.len(), count as usize);
    let (first, last) = (middle(&shapes, 0)?, middle(&shapes, count as usize - 1)?);
    // each H is between 1608 and 1752 wide, and the ramp is increasing: the last middle is between the two bounds
    assert!(
        last > first + 1608.0 * f64::from(count - 2) && last < first + 1752.0 * f64::from(count),
        "{last}"
    );
    Ok(())
}

// Features ----------------------------------------------------------------------------------------------------------

#[test]
fn a_feature_switched_off_changes_what_the_shaper_makes() -> Outcome {
    let font = fraunces()?;
    // liga on is the default: one glyph for "fi"; off: f (732) and i (601) at 400, from HarfBuzz
    assert_eq!(shapes_of("fi", at_em(&font))?.len(), 1);
    assert_eq!(shapes_of("fi", at_em(&font).features([("liga", 1)]))?.len(), 1);
    let apart = shapes_of("fi", at_em(&font).features([("liga", 0)]))?;
    assert_eq!(apart.len(), 2);
    assert_eq!(middle(&apart, 1)?, 732.0 + 601.0 / 2.0);
    // the same at a weight: 849 and 667 at 700
    let bold = shapes_of("fi", at_weight(&font, 700.0).features([("liga", 0)]))?;
    assert_eq!(middle(&bold, 1)?, 849.0 + 667.0 / 2.0);
    // kerning off: A is 1424 wide, not 1334
    let unkerned = shapes_of("AV", at_em(&font).features([("kern", 0)]))?;
    assert_eq!(middle(&unkerned, 1)?, 1424.0 + 1401.0 / 2.0);
    Ok(())
}

// Errors ------------------------------------------------------------------------------------------------------------

#[test]
fn an_axis_or_feature_the_font_lacks_is_refused_listing_the_ones_it_has() -> Outcome {
    let font = fraunces()?;
    let rows = [
        (
            at_em(&font).axes([("wdth", 100.0)]),
            "axes",
            vec!["\"wdth\"", "wght (300 to 700, default 400)"],
        ),
        (
            at_em(&font).features([("ss01", 1)]),
            "features",
            vec!["\"ss01\"", "kern, liga"],
        ),
        (
            Text::new(&synthetic()?).axes([("wght", 400.0)]),
            "axes",
            vec!["\"wght\"", "no axes"],
        ),
        (
            Text::new(&marks()?).features([("liga", 1)]),
            "features",
            vec!["\"liga\"", "mark"],
        ),
    ];
    for (options, argument, named) in rows {
        let (parameter, message) = refusal(&text("H", options))?;
        assert_eq!(parameter, format!("source (text).{argument}"));
        for part in named {
            assert!(message.contains(part), "{part} not in {message}");
        }
    }
    Ok(())
}

#[test]
fn an_axis_value_outside_the_fonts_range_is_refused_and_the_ends_are_not() -> Outcome {
    let font = fraunces()?;
    // (value, accepted): the range is 300 to 700, inclusive
    let rows = [
        (300.0, true),
        (400.0, true),
        (700.0, true),
        (299.9, false),
        (700.000_1, false),
        (-400.0, false),
        (1e300, false),
        (f64::NAN, false),
        (f64::INFINITY, false),
        (f64::NEG_INFINITY, false),
    ];
    for (value, accepted) in rows {
        let drawing = text("H", at_weight(&font, value));
        match (accepted, drawing.error()) {
            (true, None) => {
                drawing.shapes(&Values::new())?;
            }
            (false, Some(_)) => {
                let (parameter, message) = refusal(&drawing)?;
                assert_eq!(parameter, "source (text).axes");
                assert!(
                    message.contains(&value.to_string()) || message.contains(&format!("{value:e}")),
                    "{message}"
                );
            }
            (accepted, error) => return Err(format!("{value}: accepted {accepted}, error {error:?}").into()),
        }
    }
    Ok(())
}

#[test]
fn a_tag_that_is_not_four_printable_characters_is_refused_naming_the_argument() -> Outcome {
    let font = fraunces()?;
    for tag in ["wgt", "weight", "", "wgh\u{e9}", "wg\nt"] {
        let (parameter, message) = refusal(&text("H", at_em(&font).axes([(tag, 400.0)])))?;
        assert_eq!(parameter, "source (text).axes", "{tag:?}");
        assert!(message.contains("four"), "{message}");
        let (parameter, _) = refusal(&text("H", at_em(&font).features([(tag, 1)])))?;
        assert_eq!(parameter, "source (text).features", "{tag:?}");
    }
    Ok(())
}

#[test]
fn a_param_wider_than_the_fonts_range_is_refused_when_drawn() -> Outcome {
    let font = fraunces()?;
    let wide = Param::new("wide", 400.0, 0.0, 1000.0)?;
    let drawing = text("H", at_em(&font).axes([("wght", &wide)]));
    assert_eq!(drawing.error(), None);
    drawing.shapes(&Values::new().set("wide", 700.0))?;
    let (parameter, message) = parameter_refusal(drawing.shapes(&Values::new().set("wide", 1000.0)))?;
    assert_eq!(parameter, "source (text).axes");
    assert!(message.contains("1000") && message.contains("300 to 700"), "{message}");
    // an entry of a list is checked the same
    let (parameter, _) = parameter_refusal(
        text("HH", at_em(&font).axes([("wght", &wide)])).shapes(&Values::new().set("wide", vec![400.0, 1000.0])),
    )?;
    assert_eq!(parameter, "source (text).axes");
    Ok(())
}

#[test]
fn more_than_sixty_four_axes_or_features_are_refused_at_the_call() -> Outcome {
    let font = fraunces()?;
    let tags: Vec<String> = (0..65).map(|at| format!("a{at:03}")).collect();
    let (parameter, message) = refusal(&text(
        "H",
        at_em(&font).axes(tags.iter().map(|tag| (tag.as_str(), 1.0))),
    ))?;
    assert_eq!(parameter, "source (text).axes");
    assert!(
        message.contains("MAX_SETTINGS 64") && message.contains("65"),
        "{message}"
    );
    let (parameter, _) = refusal(&text(
        "H",
        at_em(&font).features(tags.iter().map(|tag| (tag.as_str(), 1))),
    ))?;
    assert_eq!(parameter, "source (text).features");
    Ok(())
}

#[test]
fn a_text_over_the_glyph_limit_is_refused_with_axes_too() -> Outcome {
    let font = fraunces()?;
    let (parameter, message) = refusal(&text("H".repeat(100_001), at_weight(&font, 700.0)))?;
    assert_eq!(parameter, "source (text).content");
    assert!(message.contains("MAX_GLYPHS"), "{message}");
    Ok(())
}

// The cache ---------------------------------------------------------------------------------------------------------

#[test]
fn different_axis_values_never_share_a_result() -> Outcome {
    let font = fraunces()?;
    let weights = Drawing::new([text("H", at_weight(&font, 300.0)), text("H", at_weight(&font, 700.0))]);
    let shapes = weights.shapes(&Values::new())?;
    assert_ne!(shapes[0].path(), shapes[1].path());
    assert_eq!(weights.stats().computed, 3, "two texts and the stack");
    // the same value twice is one text
    let same = Drawing::new([text("H", at_weight(&font, 700.0)), text("H", at_weight(&font, 700.0))]);
    same.shapes(&Values::new())?;
    assert_eq!(same.stats().computed, 2);
    // a feature on or off
    let features = Drawing::new([
        text("fi", at_em(&font).features([("liga", 0)])),
        text("fi", at_em(&font).features([("liga", 1)])),
    ]);
    assert_eq!(features.shapes(&Values::new())?.len(), 3);
    assert_eq!(features.stats().computed, 3);
    // an axis that is not given against one given at its default is another call, with the same shapes
    let omitted = Drawing::new([text("H", at_em(&font)), text("H", at_weight(&font, 400.0))]);
    omitted.shapes(&Values::new())?;
    assert_eq!(omitted.stats().computed, 3);
    Ok(())
}

#[test]
fn a_param_axis_drawn_at_two_values_gives_two_results_and_the_first_again() -> Outcome {
    let font = fraunces()?;
    let drawing = text("Hi", at_em(&font).axes([("wght", &font.axes().remove(0))]));
    let light = drawing.shapes(&Values::new().set("wght", 300.0))?;
    let heavy = drawing.shapes(&Values::new().set("wght", 700.0))?;
    assert_ne!(light, heavy);
    assert_eq!(drawing.shapes(&Values::new().set("wght", 300.0))?, light);
    Ok(())
}

// Another font --------------------------------------------------------------------------------------------------

#[test]
fn a_feature_switched_off_leaves_a_mark_at_the_pen_of_its_base() -> Outcome {
    // marks.ttf: `mark` puts the acute over B; switched off, it stays at the pen (HarfBuzz: offsets 0, 0)
    let font = marks()?;
    let on = shapes_of("B\u{301}", Text::new(&font).size(1000.0))?;
    let off = shapes_of("B\u{301}", Text::new(&font).size(1000.0).features([("mark", 0)]))?;
    assert_eq!(on.len(), 2);
    assert_ne!(on[1].path(), off[1].path());
    Ok(())
}
