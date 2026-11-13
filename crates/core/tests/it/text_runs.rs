//! `text` made of runs: each run its own font, size, axes and tracking, shaped apart on shared lines. Oracles are the
//! library's own plain text (a run of one font alone, drawn by the code the runs must agree with) and advances from
//! `HarfBuzz` over the fonts of `common/fonts.rs`: `fraunces-subset.ttf` (2000 to the em; `H` 1656, `i` 601, space 449;
//! `H` at wght 300 to 700 is 1608, 1656, 1680, 1704, 1752 wide) and `narrowing.ttf` (1000 to the em; `A` 1000 at
//! nrrw 0, a 400 square; space 300).

use crate::common::fonts::{fraunces, narrowing};
use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Content, Drawing, Font, Param, Path, Run, Shape, Text, Values, text};

const EM: f64 = 2000.0;

fn shapes(content: impl Into<Content>, options: Text) -> Result<Vec<Shape>, bullterrier_vector::Error> {
    text(content, options).shapes(&Values::new())
}

/// The one glyph `content` is in `font` at `size`, its pen at `x`, on the first baseline.
fn glyph(content: &str, font: &Font, size: f64, x: f64) -> Result<Path, bullterrier_vector::Error> {
    let shapes = shapes(content, Text::new(font).size(size))?;
    assert_eq!(shapes.len(), 1, "{content:?}");
    shapes[0].path().translate(x, 0.0)
}

fn attribute(shapes: &[Shape], read: impl Fn(&Shape) -> Option<u32>) -> Vec<Option<u32>> {
    shapes.iter().map(read).collect()
}

#[test]
fn runs_of_one_font_split_at_a_space_draw_what_the_plain_text_draws() -> Outcome {
    let font = fraunces()?;
    let plain = shapes("Hi Hi To", Text::new(&font).size(EM))?;
    let runs = shapes(
        [Run::new("Hi", &font), Run::new(" Hi", &font), Run::new(" To", &font)],
        Text::default().size(EM),
    )?;
    assert_eq!(plain.len(), runs.len());
    for (one, other) in plain.iter().zip(&runs) {
        assert_eq!(one.path(), other.path());
        assert_eq!(one.attributes().glyph(), other.attributes().glyph());
        assert_eq!(one.attributes().word(), other.attributes().word());
        assert_eq!(one.attributes().origin(), other.attributes().origin());
    }
    // the run each glyph is in: the plain text is one run
    assert_eq!(attribute(&plain, |s| s.attributes().run()), vec![Some(0); 6]);
    assert_eq!(attribute(&runs, |s| s.attributes().run()), [0, 0, 1, 1, 2, 2].map(Some));
    Ok(())
}

#[test]
fn a_run_stands_where_the_advances_of_the_runs_before_it_end_in_its_own_font_and_size() -> Outcome {
    let (serif, narrow) = (fraunces()?, narrowing()?);
    let drawn = shapes(
        [
            Run::new("H", &serif),
            Run::new("A", &narrow).size(1000.0),
            Run::new("H", &serif).size(1000.0),
        ],
        Text::default().size(EM),
    )?;
    assert_eq!(drawn.len(), 3);
    // H is 1656 at 2000 to the em; A of the narrow font is 1000 at 1000 to the em; the second H is at half the size
    assert_eq!(drawn[0].path(), &glyph("H", &serif, EM, 0.0)?);
    assert_eq!(drawn[1].path(), &glyph("A", &narrow, 1000.0, 1656.0)?);
    assert_eq!(drawn[2].path(), &glyph("H", &serif, 1000.0, 2656.0)?);
    Ok(())
}

#[test]
fn no_kerning_reaches_across_the_boundary_between_two_runs() -> Outcome {
    let font = fraunces()?;
    // fraunces-subset.ttf, HarfBuzz: A is 1424 wide and V follows it at 1334 when kerned. One run kerns; two do not
    let kerned = shapes("AV", Text::new(&font).size(EM))?;
    let apart = shapes([Run::new("A", &font), Run::new("V", &font)], Text::default().size(EM))?;
    assert_eq!(kerned[1].path(), &glyph("V", &font, EM, 1334.0)?);
    assert_eq!(apart[1].path(), &glyph("V", &font, EM, 1424.0)?);
    Ok(())
}

#[test]
fn tracking_is_the_texts_unless_a_run_gives_its_own_and_counts_after_the_glyphs_of_its_own_run() -> Outcome {
    let font = fraunces()?;
    // H of 1000 to the em is 828 wide and 100 thousandths of an em is 100 pixels; no tracking after the last glyph
    let pen = |runs: [Run; 3], options: Text| -> Result<Vec<f64>, Box<dyn std::error::Error>> {
        shapes(runs, options)?
            .iter()
            .map(|shape| Ok(shape.attributes().origin().ok_or("no origin")?.x - 414.0))
            .collect()
    };
    let three = || [Run::new("H", &font), Run::new("H", &font), Run::new("H", &font)];
    let own = pen(
        [
            Run::new("H", &font).tracking(0.0),
            Run::new("H", &font).tracking(200.0),
            Run::new("H", &font),
        ],
        Text::default().size(1000.0),
    )?;
    assert_eq!(own, [0.0, 828.0, 1856.0]);
    let inherited = pen(three(), Text::default().size(1000.0).tracking(100.0))?;
    assert_eq!(inherited, [0.0, 928.0, 1856.0]);
    Ok(())
}

#[test]
fn the_line_height_is_that_of_the_tallest_run_on_the_line() -> Outcome {
    let font = fraunces()?;
    // line 1 holds runs of 1000 and 2000: it stands 1.2 * 2000 below line 0; line 2 holds only 1000: 1.2 * 1000 below
    let drawn = shapes(
        [
            Run::new("H\n", &font).size(1000.0),
            Run::new("H", &font).size(1000.0),
            Run::new("H\n", &font).size(2000.0),
            Run::new("H", &font).size(1000.0),
        ],
        Text::default(),
    )?;
    let baselines: Vec<f64> = drawn
        .iter()
        .map(|shape| shape.path().bounds().map(|bounds| bounds.y + bounds.height))
        .collect::<Result<_, _>>()?;
    // H stands on the baseline, 1000 to the em: its outline is the same height whatever the line, so the bottoms of
    // the first glyph of each line differ by the baselines' distance (scaled glyphs differ in height: compare to alone)
    let alone = |size: f64, at: f64| -> Result<f64, bullterrier_vector::Error> {
        let bounds = glyph("H", &font, size, 0.0)?.translate(0.0, at)?.bounds()?;
        Ok(bounds.y + bounds.height)
    };
    assert_eq!(attribute(&drawn, |s| s.attributes().line()), [0, 1, 1, 2].map(Some));
    assert!((baselines[0] - alone(1000.0, 0.0)?).abs() < 1e-9);
    assert!((baselines[1] - alone(1000.0, 2400.0)?).abs() < 1e-9);
    assert!((baselines[3] - alone(1000.0, 2400.0 + 1200.0)?).abs() < 1e-9);
    Ok(())
}

#[test]
fn a_word_that_spans_runs_is_never_broken_inside_and_counts_as_one_word() -> Outcome {
    let (serif, narrow) = (fraunces()?, narrowing()?);
    // "HiHi" is wider than 3000 and has no whitespace: it stands alone on a line, whichever runs it is made of
    let drawn = shapes(
        [
            Run::new("H", &serif),
            Run::new("i", &serif),
            Run::new("H", &narrow).size(EM),
            Run::new("i H", &serif),
        ],
        Text::default().size(EM).width(3000.0),
    )?;
    // narrowing.ttf has no H: the run draws .notdef (no outline), so the word has three outlines and "H" follows
    assert_eq!(
        attribute(&drawn, |s| s.attributes().line()),
        [0, 0, 0, 1].map(Some),
        "the word of three runs is on line 0, the next word on line 1"
    );
    assert_eq!(attribute(&drawn, |s| s.attributes().word()), [0, 0, 0, 1].map(Some));
    assert_eq!(attribute(&drawn, |s| s.attributes().run()), [0, 1, 3, 3].map(Some));
    Ok(())
}

#[test]
fn fit_size_scales_every_run_by_one_factor_until_the_line_is_the_width() -> Outcome {
    let font = fraunces()?;
    // H of 1000 is 828 wide and of 2000 is 1656: the line is 2484, so a width of 4968 takes the factor 2
    let drawn = shapes(
        [Run::new("H", &font).size(1000.0), Run::new("H", &font).size(2000.0)],
        Text::default().width(4968.0).fit("size"),
    )?;
    assert_eq!(drawn[0].path(), &glyph("H", &font, 2000.0, 0.0)?);
    assert_eq!(drawn[1].path(), &glyph("H", &font, 4000.0, 1656.0)?);
    Ok(())
}

#[test]
fn fit_on_an_axis_sets_the_runs_whose_font_has_it_and_leaves_the_others_as_they_look() -> Outcome {
    let (serif, narrow) = (fraunces()?, narrowing()?);
    // A of the narrow font is 2000 wide at 2000 to the em, whatever the fit; H at wght 500 is 1680: 3680 in all
    let drawn = shapes(
        [Run::new("H", &serif), Run::new("A", &narrow).size(2000.0)],
        Text::default().size(EM).width(3680.0).fit("wght"),
    )?;
    assert_eq!(drawn.len(), 2);
    let centre = |at: usize| drawn[at].attributes().origin().map(|origin| origin.x);
    assert!((centre(0).ok_or("no origin")? - 840.0).abs() <= 1.0);
    assert!((centre(1).ok_or("no origin")? - (1680.0 + 1000.0)).abs() <= 1.0);
    assert_eq!(
        drawn[1].path().bounds()?.width,
        glyph("A", &narrow, 2000.0, 0.0)?.bounds()?.width
    );
    Ok(())
}

#[test]
fn fit_on_an_axis_sets_every_run_that_has_it() -> Outcome {
    let font = fraunces()?;
    // H at wght 500 is 1680 wide, and two of them are 3360: both runs are set to 500, not the first alone
    let drawn = shapes(
        [Run::new("H", &font), Run::new("H", &font)],
        Text::default().size(EM).width(3360.0).fit("wght"),
    )?;
    let origins: Vec<f64> = drawn
        .iter()
        .map(|shape| Ok(shape.attributes().origin().ok_or("no origin")?.x))
        .collect::<Result<_, Box<dyn std::error::Error>>>()?;
    assert!(
        (origins[0] - 840.0).abs() <= 1.0 && (origins[1] - (1680.0 + 840.0)).abs() <= 1.0,
        "{origins:?}"
    );
    Ok(())
}

#[test]
fn a_wrapped_line_is_fitted_with_the_axis_values_of_its_own_characters() -> Outcome {
    // bricolage-grotesque has opsz, wght and wdth: wdth runs from 75 to 80 along the five characters of "HH HH", so the
    // characters of its two lines are at 75, 76.25 and at 78.75, 80. Each line is the text of its own two characters
    // with those values, fitted alike: 82 wide on wght, the second one a leading (120) below the first
    let font = Font::bundled("bricolage_grotesque")?;
    let line = |wdth: (f64, f64), y: f64| {
        shapes(
            "HH",
            Text::new(&font)
                .size(100.0)
                .axes([("wdth", bullterrier_vector::by_index(wdth.0, wdth.1))])
                .width(82.0)
                .fit("wght")
                .anchor((0.0, y)),
        )
    };
    let wrapped = shapes(
        "HH HH",
        Text::new(&font)
            .size(100.0)
            .axes([("wdth", bullterrier_vector::by_index(75.0, 80.0))])
            .width(82.0)
            .fit("wght"),
    )?;
    let (first, second) = (line((75.0, 76.25), 0.0)?, line((78.75, 80.0), 120.0)?);
    assert_eq!(wrapped.len(), 4);
    for (found, wanted) in wrapped.iter().zip(first.iter().chain(&second)) {
        assert_eq!(found.path(), wanted.path());
    }
    Ok(())
}

#[test]
fn a_fit_on_an_axis_no_run_has_is_refused_naming_fit() -> Outcome {
    let (serif, narrow) = (fraunces()?, narrowing()?);
    let drawing = text(
        [Run::new("H", &serif), Run::new("A", &narrow)],
        Text::default().width(100.0).fit("opsz"),
    );
    let (parameter, message) = parameter_refusal(Err::<(), _>(drawing.error().cloned().ok_or("no error")?))?;
    assert_eq!(parameter, "source (text).fit");
    assert!(message.contains("opsz"), "{message}");
    // and a run that gives the fitted axis itself is refused as a plain text is
    let weight = Param::new("w", 400.0, 300.0, 700.0)?;
    let drawing = text(
        [Run::new("H", &serif).axes([("wght", &weight)]), Run::new("A", &narrow)],
        Text::default().width(100.0).fit("wght"),
    );
    let (_, message) = parameter_refusal(Err::<(), _>(drawing.error().cloned().ok_or("no error")?))?;
    assert!(message.contains("wght") && message.contains("axes"), "{message}");
    Ok(())
}

#[test]
fn an_axis_list_belongs_to_its_run_and_counts_that_runs_characters() -> Outcome {
    let (serif, narrow) = (fraunces()?, narrowing()?);
    let weight = Param::new("weight", 400.0, 300.0, 700.0)?;
    let drawing = text(
        [Run::new("HH", &serif).axes([("wght", &weight)]), Run::new("A", &narrow)],
        Text::default().size(EM),
    );
    // two characters in the first run: a list of two is right, a list of three names the run
    let ok = drawing.shapes(&Values::new().set("weight", vec![300.0, 700.0]))?;
    assert_eq!(ok.len(), 3);
    let centre = |at: usize| ok[at].attributes().origin().map(|origin| origin.x);
    assert!((centre(0).ok_or("no origin")? - 804.0).abs() <= 1.0); // H at 300: 1608 wide
    let (parameter, message) =
        parameter_refusal(drawing.shapes(&Values::new().set("weight", vec![300.0, 700.0, 400.0])))?;
    assert!(
        parameter.contains("weight") || parameter.contains("axes"),
        "{parameter}"
    );
    assert!(message.contains("run 0"), "{message}");
    Ok(())
}

#[test]
fn only_a_text_of_several_runs_names_the_run_in_an_axis_error() -> Outcome {
    let font = fraunces()?;
    let weight = Param::new("weight", 400.0, 300.0, 700.0)?;
    let wrong = Values::new().set("weight", vec![300.0, 700.0, 400.0]);
    let plain = text("HH", Text::new(&font).axes([("wght", &weight)]));
    let (_, message) = parameter_refusal(plain.shapes(&wrong))?;
    assert!(!message.contains("run"), "{message}");
    let one = text([Run::new("HH", &font).axes([("wght", &weight)])], Text::default());
    let (_, message) = parameter_refusal(one.shapes(&wrong))?;
    assert!(!message.contains("run 0"), "{message}");
    Ok(())
}

#[test]
fn what_cannot_be_made_of_runs_is_refused() -> Outcome {
    let font = fraunces()?;
    let many: Vec<Run> = (0..1001).map(|_| Run::new("H", &font)).collect();
    let rows: [(&str, Drawing, &str); 4] = [
        ("no runs", text(Vec::<Run>::new(), Text::default()), "runs"),
        ("too many runs", text(many, Text::default()), "runs"),
        (
            "a font on the text of runs",
            text([Run::new("H", &font)], Text::new(&font)),
            "font",
        ),
        (
            "axes on the text of runs",
            text([Run::new("H", &font)], Text::default().axes([("wght", 500.0)])),
            "axes",
        ),
    ];
    for (rule, drawing, argument) in rows {
        let (parameter, _) = parameter_refusal(Err::<(), _>(drawing.error().cloned().ok_or("no error")?))?;
        assert!(parameter.contains(argument), "{rule}: {parameter}");
    }
    // a plain text with no font
    let drawing = text("H", Text::default());
    assert!(drawing.error().is_some());
    Ok(())
}

#[test]
fn inside_and_alignment_work_on_the_whole_text_of_runs() -> Outcome {
    let (serif, narrow) = (fraunces()?, narrowing()?);
    let centred = shapes(
        [Run::new("H", &serif), Run::new("A", &narrow).size(EM)],
        Text::default().size(EM).align(bullterrier_vector::Align::Center),
    )?;
    // 1656 + 2000 = 3656 wide, centred on x = 0: the first glyph's pen is at -1828
    let first = centred[0].attributes().origin().ok_or("no origin")?.x;
    assert!((first - (-1828.0 + 828.0)).abs() <= 1e-9);
    let region = Path::rect(0.0, 0.0, 6000.0, 6000.0, 0.0)?;
    let inside = shapes(
        [Run::new("H H", &serif), Run::new(" H", &serif)],
        Text::default().size(EM).inside(&region),
    )?;
    assert_eq!(inside.len(), 3);
    for shape in &inside {
        assert!(region.contains(shape.attributes().origin().ok_or("no origin")?)?);
    }
    Ok(())
}

#[test]
fn equal_runs_share_a_cache_entry_and_a_run_that_differs_in_anything_does_not() -> Outcome {
    let (serif, narrow) = (fraunces()?, narrowing()?);
    let base = || Run::new("H", &serif);
    // a stack of two texts: the text and the stack are computed, and the second text is the first again if equal
    let rows: [(&str, Run, usize); 6] = [
        ("equal runs", base(), 1),
        ("another font", Run::new("H", &narrow), 0),
        ("another content", Run::new("i", &serif), 0),
        ("another size", base().size(10.0), 0),
        ("another tracking", base().tracking(10.0), 0),
        ("other axes", base().axes([("wght", 700.0)]), 0),
    ];
    for (rule, other, reused) in rows {
        let stacked = Drawing::new([text([base()], Text::default()), text([other], Text::default())]);
        stacked.shapes(&Values::new())?;
        assert_eq!(stacked.stats().reused, reused, "{rule}");
        assert_eq!(stacked.stats().computed, 3 - reused, "{rule}");
    }
    let features = Drawing::new([
        text([base()], Text::default()),
        text([base().features([("liga", 0)])], Text::default()),
    ]);
    features.shapes(&Values::new())?;
    assert_eq!(features.stats().reused, 0, "other features");
    Ok(())
}

// In a document ------------------------------------------------------------------------------------------------------

mod document {
    use super::*;
    use crate::common::fonts::{FRAUNCES, NARROWING};
    use bullterrier_vector::{Error, Inputs};

    fn inputs() -> Inputs {
        Inputs::new().set("serif", FRAUNCES).set("narrow", NARROWING)
    }

    /// A document of a text of runs whose runs are `runs` (JSON), with the two fonts declared.
    fn document(runs: &str, extra: &str) -> String {
        format!(
            r#"{{"version":1,"inputs":{{"serif":{{"kind":"font"}},"narrow":{{"kind":"font"}}}},"root":[{{"text":{{"runs":{runs}{extra}}}}}]}}"#
        )
    }

    /// The pointer and message of the `Error::Format` loading `text` gave.
    fn refusal(text: &str) -> Result<(String, String), Box<dyn std::error::Error>> {
        // a document that declares no input is given none
        let given = if text.contains("inputs") {
            inputs()
        } else {
            Inputs::new()
        };
        match Drawing::from_json(text, &given) {
            Err(Error::Format { pointer, message }) => Ok((pointer, message)),
            Err(other) => Err(format!("expected a format error, got {other}").into()),
            Ok(_) => Err("expected a format error, but it loaded".into()),
        }
    }

    #[test]
    fn a_text_of_two_fonts_writes_both_inputs_with_their_digest_and_reads_back_the_same() -> Outcome {
        let (serif, narrow) = (fraunces()?.named("serif")?, narrowing()?.named("narrow")?);
        let weight = serif.axes().remove(0);
        let drawing = text(
            [
                Run::new("Hi", &serif)
                    .size(120.0)
                    .axes([("wght", &weight)])
                    .features([("liga", 0)]),
                Run::new("A", &narrow).tracking(20.0),
            ],
            Text::default().size(100.0).width(900.0).fit("size"),
        );
        let written = drawing.to_json()?;
        for expected in [
            r#""inputs":{"narrow":{"kind":"font","sha256":""#,
            r#""serif":{"kind":"font","sha256":""#,
            r#""runs":[{"axes":{"wght":{"param":"wght"}},"content":"Hi","features":{"liga":0},"font":"serif","size":120},"#,
            r#"{"content":"A","font":"narrow","tracking":20}]"#,
        ] {
            assert!(written.contains(expected), "{expected} is not in {written}");
        }
        assert_eq!(written.matches("sha256").count(), 2);
        let loaded = Drawing::from_json(&written, &inputs())?;
        assert_eq!(loaded.to_json()?, written);
        let values = Values::new().set("wght", 600.0);
        let (made, read) = (drawing.shapes(&values)?, loaded.shapes(&values)?);
        assert_eq!(made.len(), read.len());
        assert!(made.iter().zip(&read).all(|(a, b)| a.path() == b.path()));
        // the plain text still writes content and font, as before runs
        let plain = text("Hi", Text::new(&serif).size(120.0)).to_json()?;
        assert!(
            plain.contains(r#""content":"Hi","font":"serif""#) && !plain.contains("runs"),
            "{plain}"
        );
        Ok(())
    }

    #[test]
    fn the_run_attribute_survives_a_saved_shape_and_a_selection_by_it() -> Outcome {
        let font = fraunces()?.named("serif")?;
        let narrow = narrowing()?.named("narrow")?;
        let runs = shapes(
            [Run::new("H", &font), Run::new("A", &narrow), Run::new("H", &font)],
            Text::default().size(EM),
        )?;
        let saved = Drawing::new(runs.iter().cloned().map(Drawing::from)).to_json()?;
        let loaded = Drawing::from_json(&saved, &Inputs::new())?.shapes(&Values::new())?;
        assert_eq!(
            loaded.iter().map(|shape| shape.attributes().run()).collect::<Vec<_>>(),
            [0, 1, 2].map(Some)
        );
        let chosen = text(
            [Run::new("H", &font), Run::new("A", &narrow), Run::new("H", &font)],
            Text::default().size(EM),
        )
        .only(bullterrier_vector::Select::new().attribute("run", 2u32));
        assert_eq!(chosen.shapes(&Values::new())?.len(), 1);
        Ok(())
    }

    #[test]
    fn the_loader_refuses_a_run_it_cannot_read_naming_its_pointer() -> Outcome {
        let rows = [
            (
                "an unknown key of a run",
                document(
                    r#"[{"content":"H","font":"serif"},{"content":"H","font":"serif","colour":1}]"#,
                    "",
                ),
                "/root/0/text/runs/1/colour",
            ),
            (
                "a run without a font",
                document(r#"[{"content":"H"}]"#, ""),
                "/root/0/text/runs/0",
            ),
            (
                "runs that are not an array",
                document(r#"{"content":"H","font":"serif"}"#, ""),
                "/root/0/text/runs",
            ),
            (
                "a size that is not above 0",
                document(r#"[{"content":"H","font":"serif","size":0}]"#, ""),
                "/root/0/text/runs/0/size",
            ),
            (
                "an input a run names that is not declared",
                document(r#"[{"content":"H","font":"nowhere"}]"#, ""),
                "/root/0/text/runs/0/font",
            ),
            (
                "a tag that is not four characters",
                document(r#"[{"content":"H","font":"serif","axes":{"wg":500}}]"#, ""),
                "/root/0/text/runs/0/axes",
            ),
        ];
        for (rule, document, pointer) in rows {
            let (found, _) = refusal(&document)?;
            assert!(found.starts_with(pointer), "{rule}: {found}");
        }
        Ok(())
    }

    #[test]
    fn what_a_text_of_runs_may_not_carry_is_a_call_check_naming_the_call() -> Outcome {
        let both = r#"[{"content":"H","font":"serif"},{"content":"A","font":"narrow"}]"#;
        let rows = [
            ("content with runs", document(both, r#","content":"H""#)),
            ("a font with runs", document(both, r#","font":"serif""#)),
            ("no runs", r#"{"version":1,"root":[{"text":{"runs":[]}}]}"#.to_owned()),
            (
                "a fit on an axis no run's font has",
                document(both, r#","width":100,"fit":"opsz""#),
            ),
        ];
        for (rule, document) in rows {
            let (pointer, _) = refusal(&document).map_err(|error| format!("{rule}: {error}"))?;
            assert!(pointer.starts_with("/root/0"), "{rule}: {pointer}");
        }
        Ok(())
    }

    #[test]
    fn the_axes_and_features_of_a_run_are_checked_against_its_own_font() -> Outcome {
        // wght is the serif's, nrrw the narrow's: each run's font is the one that must have what the run names
        let good = document(
            r#"[{"content":"H","font":"serif","axes":{"wght":500}},{"content":"A","font":"narrow","axes":{"nrrw":500}}]"#,
            "",
        );
        assert_eq!(Drawing::from_json(&good, &inputs())?.error(), None);
        let swapped = document(
            r#"[{"content":"H","font":"serif","axes":{"wght":500}},{"content":"A","font":"narrow","axes":{"wght":500}}]"#,
            "",
        );
        match Drawing::from_json(&swapped, &inputs()) {
            Err(Error::Parameter { parameter, message }) => {
                assert_eq!(parameter, "inputs.narrow");
                assert!(message.contains("/root/0/text/runs/1/axes/wght"), "{message}");
            }
            other => return Err(format!("expected the font-checks refusal, got {other:?}").into()),
        }
        Ok(())
    }
}
