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
