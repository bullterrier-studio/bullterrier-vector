//! The cache of evaluated steps, through the public API: a draw computes only the steps that depend on a Param whose
//! value changed, a drawing used twice is computed once, equal drawings built apart share their steps, and what a
//! draw did not use is dropped. Expected counts are worked out by hand from the chain, not from the library.

use crate::common::Outcome;
use crate::common::ops::{chain, edits, extend, materialise, values_of};
use bullterrier_vector::{
    Blend, Color, Deg, Drawing, Error, Expr, Gradient, Halftone, Join, Param, Path, Select, Shadow, Shape, Stats,
    Stroke, Values,
};
use proptest::prelude::*;

fn square() -> Result<Drawing, Error> {
    Ok(Drawing::from(Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)?))
}

fn slider(name: &str, default: f64) -> Result<Param, Error> {
    Param::new(name, default, -100.0, 100.0)
}

/// What the draw of `drawing` at `values` did.
fn stats_of(drawing: &Drawing, values: &Values) -> Result<Stats, Error> {
    drawing.shapes(values)?;
    Ok(drawing.stats())
}

fn counts(computed: usize, reused: usize, entries: usize) -> Stats {
    Stats {
        computed,
        reused,
        entries,
    }
}

/// Six calls, numbered as a person counts them: 1 the path, 2 reads `a`, 3 reads nothing, 4 reads `a`, 5 reads `b`,
/// 6 reads nothing. A step depends on every Param read at or before it.
fn six() -> Result<Drawing, Error> {
    let (a, b) = (slider("a", 0.0)?, slider("b", 0.0)?);
    Ok(square()?
        .translate(&a, 0.0)
        .translate(1.0, 1.0)
        .translate(0.0, &a)
        .translate(&b, 0.0)
        .opacity(0.5))
}

// Only the dependents are computed -----------------------------------------------------------------------------------

#[test]
fn a_changed_param_recomputes_the_steps_from_its_first_reader_on() -> Outcome {
    let chain = six()?;
    let rows = [
        // (case, values, computed, reused): the entries are the three results a later draw can start from each time:
        // step 1 (step 2 reads `a`), step 4 (step 5 reads `b`) and step 6 (the draw itself)
        ("cold: all six", Values::new(), 6, 0),
        ("a changed: steps 2 to 6", Values::new().set("a", 1.0), 5, 1),
        (
            "b changed too: steps 5 and 6",
            Values::new().set("a", 1.0).set("b", 1.0),
            2,
            1,
        ),
        ("the same again: none", Values::new().set("a", 1.0).set("b", 1.0), 0, 1),
        ("b changed back: steps 5 and 6", Values::new().set("a", 1.0), 2, 1),
        ("a changed again: steps 2 to 6", Values::new().set("a", 2.0), 5, 1),
    ];
    for (case, values, computed, reused) in rows {
        assert_eq!(stats_of(&chain, &values)?, counts(computed, reused, 3), "{case}");
    }
    Ok(())
}

// A drawing used twice is computed once -----------------------------------------------------------------------------

#[test]
fn a_drawing_stacked_with_itself_twelve_times_computes_each_distinct_step_once() -> Outcome {
    // the reviewer's repro: without a cache this is 2^12 evaluations of 201 steps
    let mut drawing = square()?;
    for _ in 0..200 {
        drawing = drawing.translate(1.0, 0.0);
    }
    for _ in 0..12 {
        drawing = Drawing::new([drawing.clone(), drawing]);
    }
    let shapes = drawing.shapes(&Values::new())?;
    assert_eq!(shapes.len(), 4096);
    // distinct steps: the path, 200 translates and 12 stacks; each stack finds its second item already computed.
    // Kept: the 12 results used twice (the 201-step chain and 11 stacks) and the last stack; the 200 translates and the
    // path feed one step each that reads the same Params, so they are made again with it and not kept
    assert_eq!(drawing.stats(), counts(213, 12, 13));
    // drawn again, the last step is served as it is
    drawing.shapes(&Values::new())?;
    assert_eq!(drawing.stats(), counts(0, 1, 13));
    Ok(())
}

#[test]
fn equal_drawings_built_apart_share_their_steps() -> Outcome {
    let apart = || -> Result<Drawing, Error> { Ok(square()?.translate(&slider("a", 0.0)?, 0.0).rotate(Deg(30.0))) };
    let stacked = Drawing::new([apart()?, apart()?]);
    stacked.shapes(&Values::new())?;
    // the path, the translate and the rotate once, and the stack: the second item is the first again
    assert_eq!(stacked.stats(), counts(4, 1, 4));
    // and a value for the Param moves both: the path is served, the translate, the rotate and the stack are computed,
    // and the second item is served again
    let moved = stacked.shapes(&Values::new().set("a", 5.0))?;
    assert_eq!(stacked.stats(), counts(3, 2, 4));
    assert_eq!(moved[0].path(), moved[1].path());
    Ok(())
}

// What a draw drops -------------------------------------------------------------------------------------------------

#[test]
fn entries_of_the_values_before_are_gone_after_a_draw() -> Outcome {
    let chain = six()?;
    for step in 0..20 {
        // each draw is a new `a`; the cache holds the same three entries, not three more per value tried
        let values = Values::new().set("a", f64::from(step));
        assert_eq!(stats_of(&chain, &values)?.entries, 3, "draw {step}");
    }
    // gone: asking for an earlier value computes again
    stats_of(&chain, &Values::new().set("a", 3.0))?;
    assert_eq!(stats_of(&chain, &Values::new().set("a", 4.0))?.computed, 5);
    assert_eq!(stats_of(&chain, &Values::new().set("a", 3.0))?.computed, 5);
    Ok(())
}

#[test]
fn a_long_chain_that_reads_no_param_keeps_only_what_it_drew() -> Outcome {
    // every step feeds a step that reads what it reads (nothing), so none is a place a later draw starts from
    let mut chain = square()?;
    for _ in 0..300 {
        chain = chain.translate(1.0, 0.0);
    }
    assert_eq!(stats_of(&chain, &Values::new())?, counts(301, 0, 1));
    assert_eq!(stats_of(&chain, &Values::new())?, counts(0, 1, 1));
    // a Param read by the last step makes the step before it a place to start from: two entries, and 300 steps served
    let a = slider("a", 0.0)?;
    let read = chain.translate(&a, 0.0);
    assert_eq!(stats_of(&read, &Values::new())?, counts(1, 1, 2));
    assert_eq!(stats_of(&read, &Values::new().set("a", 5.0))?, counts(1, 1, 2));
    Ok(())
}

#[test]
fn failed_draws_do_not_pile_up_entries() -> Outcome {
    // `a` is read by step 2 and `b` by step 3, which refuses 0: each failed draw computes step 2 at a new `a`
    let (a, b) = (slider("a", 0.0)?, slider("b", 1.0)?);
    let chain = square()?.translate(&a, 0.0).scale(&b);
    for step in 0..200 {
        let values = Values::new().set("a", f64::from(step)).set("b", 0.0);
        assert!(chain.shapes(&values).is_err(), "draw {step}");
        assert!(
            chain.stats().entries <= 3,
            "draw {step}: {} entries",
            chain.stats().entries
        );
    }
    // and the drawing still draws what a cold one does
    let good = Values::new().set("a", 7.0);
    assert_eq!(
        chain.shapes(&good)?,
        square()?
            .translate(&slider("a", 0.0)?, 0.0)
            .scale(&slider("b", 1.0)?)
            .shapes(&good)?
    );
    Ok(())
}

#[test]
fn steps_before_a_hit_stay_for_the_next_draw() -> Outcome {
    let chain = six()?;
    stats_of(&chain, &Values::new().set("a", 1.0).set("b", 1.0))?;
    // `b` changes: step 4 is served, step 1 stays behind it, and step 6 is a new entry (step 5 is not kept): three
    assert_eq!(
        stats_of(&chain, &Values::new().set("a", 1.0).set("b", 2.0))?,
        counts(2, 1, 3)
    );
    // so `b` changing again still finds step 4
    assert_eq!(
        stats_of(&chain, &Values::new().set("a", 1.0).set("b", 3.0))?,
        counts(2, 1, 3)
    );
    Ok(())
}

#[test]
fn the_drawings_a_served_step_was_made_of_stay_for_the_next_draw() -> Outcome {
    let a = slider("a", 0.0)?;
    // the path (one for both items), the left translate, the right rotate by `a`, the stack: four steps
    let left = square()?.translate(20.0, 0.0);
    let right = square()?.rotate(Deg(&a));
    let stacked = Drawing::new([left, right]);
    // kept: the path (used twice), the left translate (the stack reads `a`, it does not) and the stack; the rotate
    // reads what the stack reads, so it is made again with it
    assert_eq!(stats_of(&stacked, &Values::new())?, counts(4, 1, 3));
    // served as a whole: nothing below the stack is looked at
    assert_eq!(stats_of(&stacked, &Values::new())?, counts(0, 1, 3));
    // `a` changes: the left translate and the path are still there, and served; the rotate and the stack are computed
    assert_eq!(stats_of(&stacked, &Values::new().set("a", 45.0))?, counts(2, 2, 3));
    Ok(())
}

// Every input of a step is in its key -------------------------------------------------------------------------------

/// Two drawings of one family (built on one base), with the values each is drawn at.
type Pair = (Drawing, Values, Drawing, Values);

/// Builds a pair on a base.
type Make = fn(&Drawing) -> Result<Pair, Error>;

/// Rows that differ in exactly one input of a step; each is built from a base of its own, so building one again gives
/// a drawing with a cache of its own to compare with.
fn pairs() -> Vec<(&'static str, Make)> {
    vec![
        ("an argument of the step", |base| {
            Ok((
                base.translate(1.0, 0.0),
                Values::new(),
                base.translate(2.0, 0.0),
                Values::new(),
            ))
        }),
        ("the step before", |base| {
            Ok((
                base.translate(1.0, 0.0).rotate(Deg(30.0)),
                Values::new(),
                base.translate(2.0, 0.0).rotate(Deg(30.0)),
                Values::new(),
            ))
        }),
        ("a drawing the step takes", |base| {
            Ok((
                Drawing::new([base.clone(), base.translate(1.0, 0.0)]),
                Values::new(),
                Drawing::new([base.clone(), base.translate(2.0, 0.0)]),
                Values::new(),
            ))
        }),
        ("the values of a with_values", |base| {
            let x = slider("x", 0.0)?;
            let read = base.translate(&x, 0.0);
            Ok((
                read.with_values(&Values::new().set("x", 1.0)),
                Values::new(),
                read.with_values(&Values::new().set("x", 2.0)),
                Values::new(),
            ))
        }),
        ("the default of a Param", |base| {
            Ok((
                base.translate(&slider("x", 1.0)?, 0.0),
                Values::new(),
                base.translate(&slider("x", 2.0)?, 0.0),
                Values::new(),
            ))
        }),
        ("a selection", |base| {
            let two = Drawing::new([base.clone(), base.translate(20.0, 0.0)]);
            Ok((
                two.only(Select::new().attribute("index", 0_u32)),
                Values::new(),
                two.only(Select::new().attribute("index", 1_u32)),
                Values::new(),
            ))
        }),
        ("the unit of a Param", |base| {
            // 400 wraps to 40 in a cyclic Param and is refused in a bounded one
            let cyclic = Param::new("t", 0.0, 0.0, 360.0)?.unit(bullterrier_vector::Unit::Deg)?;
            let bounded = Param::new("t", 0.0, 0.0, 360.0)?;
            let at = Values::new().set("t", 400.0);
            Ok((base.rotate(Deg(&cyclic)), at.clone(), base.rotate(Deg(&bounded)), at))
        }),
        ("an operator of an expression", |base| {
            let x = slider("x", 0.0)?;
            let at = Values::new().set("x", 5.0);
            Ok((
                base.translate(Expr::from(&x) + 1.0, 0.0),
                at.clone(),
                base.translate(Expr::from(&x) - 1.0, 0.0),
                at,
            ))
        }),
        ("another operator of an expression", |base| {
            let x = slider("x", 0.0)?;
            let at = Values::new().set("x", 5.0);
            Ok((
                base.translate(Expr::from(&x) * 2.0, 0.0),
                at.clone(),
                base.translate(Expr::from(&x) / 2.0, 0.0),
                at,
            ))
        }),
        ("the range of a Param, its top", |base| {
            let at = Values::new().set("x", 50.0);
            Ok((
                base.translate(&Param::new("x", 0.0, 0.0, 100.0)?, 0.0),
                at.clone(),
                base.translate(&Param::new("x", 0.0, 0.0, 10.0)?, 0.0),
                at,
            ))
        }),
        ("the range of a Param, its bottom", |base| {
            let at = Values::new().set("x", -50.0);
            Ok((
                base.translate(&slider("x", 0.0)?, 0.0),
                at.clone(),
                base.translate(&Param::new("x", 0.0, 0.0, 100.0)?, 0.0),
                at,
            ))
        }),
    ]
}

/// Rows that differ in one argument of a shadow.
fn shadow_pairs() -> Vec<(&'static str, Make)> {
    vec![
        ("the x offset of a shadow", |base| {
            shadow_pair(base, Shadow::new().dx(1.0), Shadow::new().dx(2.0))
        }),
        ("the y offset of a shadow", |base| {
            shadow_pair(base, Shadow::new().dy(1.0), Shadow::new().dy(2.0))
        }),
        ("the blur of a shadow", |base| {
            shadow_pair(base, Shadow::new().blur(1.0), Shadow::new().blur(2.0))
        }),
        ("whether a shape knocks out", knockout_pair),
        ("the colour of a shadow", |base| {
            let color = |text| Color::parse(text).map(|color| Shadow::new().color(color));
            shadow_pair(base, color("#000000")?, color("#00000080")?)
        }),
    ]
}

/// The colour of the halftones below.
const BLACK: Color = Color::hex(0);

/// Rows that differ in one argument of a halftone, and a halftone against the fill of the same colour.
fn halftone_pairs() -> Vec<(&'static str, Make)> {
    vec![
        ("the tone of a halftone", |base| {
            halftone_pair(base, Halftone::new(BLACK, 0.25), Halftone::new(BLACK, 0.5))
        }),
        ("the spacing of a halftone", |base| {
            halftone_pair(
                base,
                Halftone::new(BLACK, 0.5).spacing(4.0),
                Halftone::new(BLACK, 0.5).spacing(5.0),
            )
        }),
        ("the angle of a halftone", |base| {
            halftone_pair(
                base,
                Halftone::new(BLACK, 0.5).angle(Deg(10.0)),
                Halftone::new(BLACK, 0.5).angle(Deg(20.0)),
            )
        }),
        ("the colour of a halftone", |base| {
            halftone_pair(
                base,
                Halftone::new(BLACK, 0.5),
                Halftone::new(Color::hex(0xff_00_00), 0.5),
            )
        }),
        ("a halftone against the fill of its colour", |base| {
            Ok((
                base.halftone(Halftone::new(BLACK, 0.5)),
                Values::new(),
                base.fill(BLACK),
                Values::new(),
            ))
        }),
    ]
}

/// The pair of `base` with the halftone `left` and with the halftone `right`.
#[expect(clippy::unnecessary_wraps, reason = "a row of a table of `Make`")]
fn halftone_pair(base: &Drawing, left: Halftone, right: Halftone) -> Result<Pair, Error> {
    Ok((base.halftone(left), Values::new(), base.halftone(right), Values::new()))
}

/// The pair of `base` as it is and as a knockout.
#[expect(clippy::unnecessary_wraps, reason = "a row of a table of `Make`")]
fn knockout_pair(base: &Drawing) -> Result<Pair, Error> {
    Ok((base.clone(), Values::new(), base.knockout(), Values::new()))
}

/// The pair of `base` with the shadow `left` and with the shadow `right`.
#[expect(clippy::unnecessary_wraps, reason = "a row of a table of `Make`")]
fn shadow_pair(base: &Drawing, left: Shadow, right: Shadow) -> Result<Pair, Error> {
    Ok((base.shadow(left), Values::new(), base.shadow(right), Values::new()))
}

/// More rows: the words a step is called with, and the sources a drawing is made of.
fn source_pairs() -> Vec<(&'static str, Make)> {
    vec![
        ("a colour", |base| {
            Ok((
                base.fill(Color::hex(0xff_00_00)),
                Values::new(),
                base.fill(Color::hex(0x00_00_ff)),
                Values::new(),
            ))
        }),
        ("a word", |base| {
            Ok((
                base.blend(Blend::Multiply),
                Values::new(),
                base.blend(Blend::ColorBurn),
                Values::new(),
            ))
        }),
        ("a join", |base| {
            let stroke = |join| Stroke::new(Color::hex(0)).join(join);
            Ok((
                base.stroke(stroke(Join::Round)),
                Values::new(),
                base.stroke(stroke(Join::Miter)),
                Values::new(),
            ))
        }),
        ("the name of a selected attribute", |base| {
            // orders 0, 0.5 and 1: `index` 1 is the middle one, `order` 1 the last
            let three = Drawing::new([base.clone(), base.translate(20.0, 0.0), base.translate(40.0, 0.0)]);
            Ok((
                three.only(Select::new().attribute("index", 1_u32)),
                Values::new(),
                three.only(Select::new().attribute("order", 1.0)),
                Values::new(),
            ))
        }),
        ("a path source, open or closed", |base| {
            let corners = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)];
            let on = |closed| -> Result<Drawing, Error> {
                Ok(Drawing::new([
                    base.clone(),
                    Drawing::from(Path::polyline(corners, closed)?),
                ]))
            };
            Ok((on(false)?, Values::new(), on(true)?, Values::new()))
        }),
        ("the style of a shape source", |base| {
            let shape = |color| -> Result<Shape, Error> {
                Ok(square()?.fill(Color::hex(color)).shapes(&Values::new())?.remove(0))
            };
            Ok((
                Drawing::new([base.clone(), Drawing::from(shape(0xff_00_00)?)]),
                Values::new(),
                Drawing::new([base.clone(), Drawing::from(shape(0x00_00_ff)?)]),
                Values::new(),
            ))
        }),
        ("the attributes of a shape source", |base| {
            // both shapes are at index 0: one carries order 0.5 (the first of three, dropped), the other none
            let ordered = Drawing::new([base.clone(), base.clone(), base.clone()])
                .without(Select::new().attribute("index", 0_u32))
                .shapes(&Values::new())?
                .remove(0);
            let plain = base.shapes(&Values::new())?.remove(0);
            Ok((
                Drawing::new([base.clone(), Drawing::from(ordered)]),
                Values::new(),
                Drawing::new([base.clone(), Drawing::from(plain)]),
                Values::new(),
            ))
        }),
        ("a Param given a value", |base| {
            let x = slider("x", 0.0)?;
            Ok((
                base.translate(&x, 0.0),
                Values::new().set("x", 1.0),
                base.translate(&x, 0.0),
                Values::new().set("x", 2.0),
            ))
        }),
        ("a list given to a Param", |base| {
            let x = slider("x", 0.0)?;
            let two = Drawing::new([base.clone(), base.translate(0.0, 20.0)]).translate(&x, 0.0);
            Ok((
                two.clone(),
                Values::new().set("x", vec![1.0, 2.0]),
                two,
                Values::new().set("x", vec![1.0, 3.0]),
            ))
        }),
        ("a number against a list of it", |base| {
            let x = slider("x", 0.0)?;
            let two = Drawing::new([base.clone(), base.translate(0.0, 20.0)]).translate(&x, 0.0);
            Ok((
                two.clone(),
                Values::new().set("x", 1.0),
                two,
                Values::new().set("x", vec![1.0, 2.0]),
            ))
        }),
    ]
}

/// Rows that differ in one part of a gradient, or in a gradient against a colour or another kind.
fn gradient_pairs() -> Vec<(&'static str, Make)> {
    vec![
        ("the start of a gradient", |base| {
            let stops = [(0.0, Color::hex(0xff_00_00)), (1.0, Color::hex(0x00_00_ff))];
            Ok((
                base.fill(Gradient::linear((0.0, 0.0), (10.0, 0.0), stops)?),
                Values::new(),
                base.fill(Gradient::linear((1.0, 0.0), (10.0, 0.0), stops)?),
                Values::new(),
            ))
        }),
        ("the end of a gradient", |base| {
            let stops = [(0.0, Color::hex(0xff_00_00)), (1.0, Color::hex(0x00_00_ff))];
            Ok((
                base.fill(Gradient::linear((0.0, 0.0), (10.0, 0.0), stops)?),
                Values::new(),
                base.fill(Gradient::linear((0.0, 0.0), (10.0, 1.0), stops)?),
                Values::new(),
            ))
        }),
        ("an offset of a gradient", |base| {
            let stops = |at| [(0.0, Color::hex(0xff_00_00)), (at, Color::hex(0x00_00_ff))];
            Ok((
                base.fill(Gradient::linear((0.0, 0.0), (10.0, 0.0), stops(0.5))?),
                Values::new(),
                base.fill(Gradient::linear((0.0, 0.0), (10.0, 0.0), stops(0.75))?),
                Values::new(),
            ))
        }),
        ("a colour of a gradient", |base| {
            let stops = |color| [(0.0, Color::hex(0xff_00_00)), (1.0, color)];
            Ok((
                base.fill(Gradient::linear(
                    (0.0, 0.0),
                    (10.0, 0.0),
                    stops(Color::hex(0x00_00_ff)),
                )?),
                Values::new(),
                base.fill(Gradient::linear(
                    (0.0, 0.0),
                    (10.0, 0.0),
                    stops(Color::hex(0x00_ff_00)),
                )?),
                Values::new(),
            ))
        }),
        ("the radius of a gradient", |base| {
            let stops = [(0.0, Color::hex(0xff_00_00)), (1.0, Color::hex(0x00_00_ff))];
            Ok((
                base.fill(Gradient::radial((5.0, 5.0), 4.0, stops)?),
                Values::new(),
                base.fill(Gradient::radial((5.0, 5.0), 5.0, stops)?),
                Values::new(),
            ))
        }),
        ("a gradient of two kinds", |base| {
            let stops = [(0.0, Color::hex(0xff_00_00)), (1.0, Color::hex(0x00_00_ff))];
            Ok((
                base.fill(Gradient::linear((0.0, 0.0), (5.0, 0.0), stops)?),
                Values::new(),
                base.fill(Gradient::radial((0.0, 0.0), 5.0, stops)?),
                Values::new(),
            ))
        }),
        ("a gradient against a colour", |base| {
            let stops = [(0.0, Color::hex(0xff_00_00)), (1.0, Color::hex(0xff_00_00))];
            Ok((
                base.fill(Gradient::linear((0.0, 0.0), (5.0, 0.0), stops)?),
                Values::new(),
                base.fill(Color::hex(0xff_00_00)),
                Values::new(),
            ))
        }),
        ("a gradient fill against a gradient stroke", |base| {
            let stops = [(0.0, Color::hex(0xff_00_00)), (1.0, Color::hex(0x00_00_ff))];
            let gradient = Gradient::linear((0.0, 0.0), (5.0, 0.0), stops)?;
            Ok((
                base.fill(gradient.clone()),
                Values::new(),
                base.stroke(gradient),
                Values::new(),
            ))
        }),
    ]
}

/// Rows that differ in the region of a clip or the shapes of a mask, as a step and as a shape put back into a drawing.
fn cut_pairs() -> Vec<(&'static str, Make)> {
    vec![
        ("the region of a clip", |base| {
            Ok((
                base.clip(Path::rect(0.0, 0.0, 5.0, 10.0, 0.0)?),
                Values::new(),
                base.clip(Path::rect(5.0, 0.0, 5.0, 10.0, 0.0)?),
                Values::new(),
            ))
        }),
        ("a clip against none", |base| {
            Ok((
                base.clone(),
                Values::new(),
                base.clip(Path::rect(0.0, 0.0, 5.0, 10.0, 0.0)?),
                Values::new(),
            ))
        }),
        ("the shapes of a mask", |base| {
            let mask = |level: u8| -> Result<Drawing, Error> {
                Ok(Drawing::from(Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)?)
                    .fill(Color::hex(u32::from(level) * 0x01_01_01)))
            };
            Ok((
                base.mask(mask(60)?),
                Values::new(),
                base.mask(mask(200)?),
                Values::new(),
            ))
        }),
        ("a mask against none", |base| {
            let mask = Drawing::from(Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)?).fill(Color::hex(0xff_ff_ff));
            Ok((base.clone(), Values::new(), base.mask(mask), Values::new()))
        }),
        ("the region of a clip on a shape put back", |base| {
            let clipped = |left| -> Result<Drawing, Error> {
                Ok(Drawing::from(
                    base.clip(Path::rect(left, 0.0, 5.0, 10.0, 0.0)?)
                        .shapes(&Values::new())?[0]
                        .clone(),
                ))
            };
            Ok((clipped(0.0)?, Values::new(), clipped(5.0)?, Values::new()))
        }),
        ("the shapes of a mask on a shape put back", |base| {
            let masked = |level: u8| -> Result<Drawing, Error> {
                let mask = Drawing::from(Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)?)
                    .fill(Color::hex(u32::from(level) * 0x01_01_01));
                Ok(Drawing::from(base.mask(mask).shapes(&Values::new())?[0].clone()))
            };
            Ok((masked(60)?, Values::new(), masked(200)?, Values::new()))
        }),
    ]
}

fn outcome(drawing: &Drawing, values: &Values) -> Result<Vec<Shape>, String> {
    drawing.shapes(values).map_err(|error| error.to_string())
}

#[test]
fn drawings_that_differ_in_one_input_never_share_a_result() -> Outcome {
    for (case, make) in pairs()
        .into_iter()
        .chain(source_pairs())
        .chain(shadow_pairs())
        .chain(halftone_pairs())
        .chain(gradient_pairs())
        .chain(cut_pairs())
    {
        let (left, left_at, right, right_at) = make(&square()?)?;
        // a twin of the right drawing, built on a base of its own: its cache holds nothing of the left
        let (_, _, twin, twin_at) = make(&square()?)?;
        let first = outcome(&left, &left_at);
        let second = outcome(&right, &right_at);
        assert_ne!(first, second, "{case}: the row must tell the two apart");
        assert_eq!(second, outcome(&twin, &twin_at), "{case}: drawn after its sibling");
    }
    Ok(())
}

// Cached equals fresh -----------------------------------------------------------------------------------------------

/// A failed property case.
fn failed(error: &Error) -> TestCaseError {
    TestCaseError::fail(error.to_string())
}

proptest! {
    #[test]
    fn a_long_lived_drawing_draws_what_a_cold_one_does_at_every_edit(ops in chain(), edits in edits()) {
        let long = materialise(&ops).map_err(|error| failed(&error))?;
        let free = long.parameters().map_err(|error| failed(&error))?;
        for edit in &edits {
            let values = values_of(edit, &free);
            let cold = materialise(&ops).map_err(|error| failed(&error))?;
            prop_assert_eq!(outcome(&long, &values), outcome(&cold, &values));
        }
    }

    #[test]
    fn drawings_that_share_a_start_draw_what_a_cold_one_does_when_drawn_by_turns(
        start in chain(),
        left in chain(),
        right in chain(),
        edits in edits(),
    ) {
        // two drawings derived from one, so in one cache, drawn by turns at the same edits
        let family = materialise(&start).map_err(|error| failed(&error))?;
        let siblings = [
            (extend(family.clone(), &left).map_err(|error| failed(&error))?, [&start[..], &left[..]].concat()),
            (extend(family, &right).map_err(|error| failed(&error))?, [&start[..], &right[..]].concat()),
        ];
        for edit in &edits {
            for (drawing, ops) in &siblings {
                let free = drawing.parameters().map_err(|error| failed(&error))?;
                let values = values_of(edit, &free);
                let cold = materialise(ops).map_err(|error| failed(&error))?;
                prop_assert_eq!(outcome(drawing, &values), outcome(&cold, &values));
            }
        }
    }
}

// The sampled grid of a distance leaf, kept between draws ------------------------------------------------------------

mod grids {
    use super::slider;
    use crate::common::{Outcome, parameter_refusal};
    use bullterrier_vector::{Drawing, Error, Field, Param, Path, Rect, Shape, Trace, Values};
    use sha2::{Digest, Sha256};

    const BOX: Rect = Rect {
        x: 0.0,
        y: 0.0,
        width: 200.0,
        height: 200.0,
    };

    /// The outlines drawn, as the digest of their path data: the frames are compared by it, never as buffers.
    fn digest(shapes: &[Shape]) -> Vec<u8> {
        let mut hash = Sha256::new();
        for shape in shapes {
            hash.update(shape.path().to_svg_path().as_bytes());
        }
        hash.finalize().to_vec()
    }

    /// What a read of the circle's distance is made of: normalized or not, the frame, the cell.
    type Read = (bool, Rect, f64);

    /// Level 10 of the distance to a circle (about its middle ring) at `frame` and `cell`; `shift` moves the circle.
    fn traced(
        shift: impl Into<bullterrier_vector::Expr>,
        normalized: bool,
        frame: Rect,
        cell: f64,
    ) -> Result<Drawing, Error> {
        let circle = Drawing::from(Path::circle((80.0, 100.0), 40.0)?).translate(shift, 0.0);
        let level = if normalized { 0.5 } else { 10.0 };
        Ok(Drawing::trace_levels(
            &Field::distance(circle, normalized)?,
            [level],
            &Trace::new().tolerance(0.0).frame(frame).cell(cell),
        ))
    }

    #[test]
    fn frames_across_a_sweep_of_a_param_the_distance_ignores_equal_fresh_drawings() -> Outcome {
        // the distance to the circle does not read `growth`; the noise added to it does
        let build = || -> Result<Drawing, Error> {
            let growth = Param::new("growth", 40.0, 10.0, 80.0)?;
            let wobble = Field::noise(&growth, 0.0, 5.0, 2)?.mul(&Field::constant(6.0)?)?;
            let field = Field::distance(Path::circle((100.0, 100.0), 40.0)?, false)?.add(&wobble)?;
            Ok(Drawing::trace_levels(
                &field,
                [8.0, 14.0],
                &Trace::new().tolerance(0.0).frame(BOX).cell(4.0),
            ))
        };
        let one = build()?;
        for growth in [40.0, 25.0, 60.0, 40.0, 12.0] {
            let values = Values::new().set("growth", growth);
            let (kept, fresh) = (one.shapes(&values)?, build()?.shapes(&values)?);
            assert!(!kept.is_empty(), "growth {growth}: the control draws nothing");
            assert_eq!(digest(&kept), digest(&fresh), "growth {growth}");
        }
        Ok(())
    }

    #[test]
    fn a_moved_outline_is_never_read_from_the_grid_of_the_old_one() -> Outcome {
        // one drawing, the circle moved by a Param: a grid kept for the circle at 0 is not the grid at 30
        let shift = slider("shift", 0.0)?;
        let one = traced(&shift, false, BOX, 4.0)?;
        for value in [0.0, 30.0, 0.0, 30.0, -20.0] {
            let values = Values::new().set("shift", value);
            let (kept, fresh) = (
                one.shapes(&values)?,
                traced(value, false, BOX, 4.0)?.shapes(&Values::new())?,
            );
            assert!(!kept.is_empty(), "shift {value}: the control draws nothing");
            assert_eq!(digest(&kept), digest(&fresh), "shift {value}");
        }
        Ok(())
    }

    #[test]
    fn two_reads_of_one_outline_that_differ_in_flag_frame_or_grid_never_share_a_grid() -> Outcome {
        let moved = Rect { x: 7.0, ..BOX };
        let taller = Rect { height: 150.0, ..BOX };
        // (case, one read, the other): drawn in one stack, so in one draw of one cache
        let rows: [(&str, Read, Read); 4] = [
            ("normalized or not", (false, BOX, 4.0), (true, BOX, 4.0)),
            ("the frame's position", (false, BOX, 4.0), (false, moved, 4.0)),
            ("the frame's size, same cell", (false, BOX, 4.0), (false, taller, 4.0)),
            ("the cell", (false, BOX, 4.0), (false, BOX, 5.0)),
        ];
        for (case, (flag_a, frame_a, cell_a), (flag_b, frame_b, cell_b)) in rows {
            let (a, b) = (
                traced(0.0, flag_a, frame_a, cell_a)?,
                traced(0.0, flag_b, frame_b, cell_b)?,
            );
            let together = Drawing::new([a, b]).shapes(&Values::new())?;
            let alone = [
                traced(0.0, flag_a, frame_a, cell_a)?.shapes(&Values::new())?,
                traced(0.0, flag_b, frame_b, cell_b)?.shapes(&Values::new())?,
            ]
            .concat();
            assert_eq!(together.len(), 2, "{case}: each read draws a shape");
            assert_eq!(digest(&together), digest(&alone), "{case}");
        }
        Ok(())
    }

    #[test]
    fn a_read_too_slow_for_the_budget_is_refused_though_its_distance_grid_is_kept() -> Outcome {
        // the same distance leaf: the plain field draws and keeps its grid, the same field plus a very deep noise is
        // refused for the work of its reads, whether or not that grid is already held
        let leaf = || -> Result<Field, Error> { Field::distance(Path::circle((100.0, 100.0), 40.0)?, false) };
        let noise = Field::noise(40.0, 0.0, 1.0, 8)?;
        let mut deep = noise.clone();
        for _ in 0..11 {
            deep = deep.add(&deep.clone())?;
        }
        let slow = leaf()?.add(&deep)?;
        let trace = Trace::new().frame(BOX).cell(1.0);
        let cheap = || -> Result<Drawing, Error> { Ok(Drawing::trace_levels(&leaf()?, [10.0], &trace)) };
        let refused = |drawing: &Drawing| parameter_refusal(drawing.shapes(&Values::new()));
        let alone = refused(&Drawing::trace_levels(&slow, [10.0], &trace))?;
        let stacked = Drawing::new([cheap()?, Drawing::trace_levels(&slow, [10.0], &trace)]);
        let (named, message) = refused(&stacked)?;
        assert!(named.contains("cell"), "{named}");
        assert_eq!(message, alone.1, "the refusal is the request's, not the cache's");
        // and the cheap one alone draws, so the grid was there to be reused
        assert_ne!(cheap()?.shapes(&Values::new())?.len(), 0);
        Ok(())
    }
}
