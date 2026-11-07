//! Per-item numbers (`by_index`, `by_order`, `Random`, `Stagger`) through the public API: each shape of a list gets its
//! own value from its attributes, a drawing refuses what api.md says it refuses, and a per-item number survives being
//! saved, loaded, printed and combined. Expected values are written by hand; `random` is also checked against a
//! `SplitMix64` written here from the published algorithm.

use crate::common::fonts::fraunces;
use crate::common::{Outcome, format_refusal, parameter_refusal};
use bullterrier_vector::{
    Drawing, Error, Expr, Inputs, Param, Path, Random, Select, Stagger, Text, Values, by_index, by_order, text,
};
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;

/// `count` squares stacked, each at x = 0; the stack writes their `order`: 0, 1/(count-1), ..., 1.
fn row(count: u32) -> Result<Drawing, Error> {
    let squares = (0..count)
        .map(|_| Path::rect(0.0, 0.0, 10.0, 10.0, 0.0))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Drawing::new(squares))
}

/// The left edges of the squares `drawing` draws with `values`, bottom first.
fn lefts(drawing: &Drawing, values: &Values) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
    drawing
        .shapes(values)?
        .iter()
        .map(|shape| Ok(shape.path().bounds()?.x))
        .collect()
}

/// Where each of `count` squares lands when moved right by `by`.
fn moved_by(count: u32, by: impl Into<Expr>) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
    lefts(&row(count)?.translate(by, 0.0), &Values::new())
}

fn held(drawing: &Drawing) -> Result<Error, Box<dyn std::error::Error>> {
    drawing
        .error()
        .cloned()
        .ok_or_else(|| "the drawing holds no error".into())
}

fn assert_near(found: &[f64], expected: &[f64], what: &str) {
    assert_eq!(found.len(), expected.len(), "{what}: {found:?}");
    for (found, expected) in found.iter().zip(expected) {
        assert!((found - expected).abs() < 1e-9, "{what}: {found:?} is not {expected:?}");
    }
}

// By index and by order ----------------------------------------------------------------------------------------------

#[test]
fn by_index_runs_linearly_from_first_on_the_first_shape_to_last_on_the_last() -> Outcome {
    // (shapes, first, last, where each lands)
    let rows: [(u32, f64, f64, &[f64]); 5] = [
        (5, 10.0, 50.0, &[10.0, 20.0, 30.0, 40.0, 50.0]),
        (3, 8.0, -8.0, &[8.0, 0.0, -8.0]),
        (2, 1.0, 2.0, &[1.0, 2.0]),
        (4, 0.0, 3.0, &[0.0, 1.0, 2.0, 3.0]),
        (1, 3.0, 9.0, &[3.0]),
    ];
    for (count, first, last, expected) in rows {
        assert_eq!(moved_by(count, by_index(first, last))?, expected, "{count} shapes");
    }
    Ok(())
}

#[test]
fn by_order_reads_the_order_each_shape_carries() -> Outcome {
    // a stack of 5 writes order 0, 1/4, 1/2, 3/4, 1
    assert_eq!(moved_by(5, by_order(10.0, 50.0))?, [10.0, 20.0, 30.0, 40.0, 50.0]);
    assert_eq!(moved_by(3, by_order(8.0, -8.0))?, [8.0, 0.0, -8.0]);
    // a lone shape has order 0, even one that carries none
    let alone = Drawing::from(Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)?).translate(by_order(7.0, 9.0), 0.0);
    assert_eq!(lefts(&alone, &Values::new())?, [7.0]);
    Ok(())
}

#[test]
fn by_index_counts_the_current_list_and_by_order_keeps_the_order_a_generator_wrote() -> Outcome {
    // drop the first of three: the two left are indexes 0 and 1 of 2, but keep orders 1/2 and 1
    let kept = row(3)?.without(Select::new().attribute("order", 0.0));
    assert_eq!(
        lefts(&kept.translate(by_index(0.0, 10.0), 0.0), &Values::new())?,
        [0.0, 10.0]
    );
    assert_eq!(
        lefts(&kept.translate(by_order(0.0, 10.0), 0.0), &Values::new())?,
        [5.0, 10.0]
    );
    Ok(())
}

// Stagger -----------------------------------------------------------------------------------------------------------

#[test]
fn stagger_gives_each_shape_its_own_progress_as_the_documented_formula_says() -> Outcome {
    // five shapes at order 0, 1/4, 1/2, 3/4, 1; the shape at order p runs while progress goes from p * spread to
    // p * spread + 1 - spread; (spread, progress, each shape's value)
    #[rustfmt::skip]
    let rows: [(f64, f64, [f64; 5]); 15] = [
        (0.5, 0.0, [0.0, 0.0, 0.0, 0.0, 0.0]),
        (0.5, 0.25, [0.5, 0.25, 0.0, 0.0, 0.0]),
        (0.5, 0.5, [1.0, 0.75, 0.5, 0.25, 0.0]),
        (0.5, 0.75, [1.0, 1.0, 1.0, 0.75, 0.5]),
        (0.5, 1.0, [1.0, 1.0, 1.0, 1.0, 1.0]),
        // spread 0: all together
        (0.0, 0.3, [0.3, 0.3, 0.3, 0.3, 0.3]),
        (0.0, 0.0, [0.0, 0.0, 0.0, 0.0, 0.0]),
        // towards 1: one after another; the last shape starts at 0.75 and runs for 0.25
        (0.75, 0.5, [1.0, 1.0, 0.5, 0.0, 0.0]),
        (0.75, 1.0, [1.0, 1.0, 1.0, 1.0, 1.0]),
        (0.999, 0.0, [0.0, 0.0, 0.0, 0.0, 0.0]),
        (0.999, 0.999, [1.0, 1.0, 1.0, 1.0, 0.0]),
        (0.999, 0.9995, [1.0, 1.0, 1.0, 1.0, 0.5]),
        (0.999, 1.0, [1.0, 1.0, 1.0, 1.0, 1.0]),
        // a progress beyond 0..1 holds the end values
        (0.5, -1.0, [0.0, 0.0, 0.0, 0.0, 0.0]),
        (0.5, 2.0, [1.0, 1.0, 1.0, 1.0, 1.0]),
    ];
    let progress = Param::new("progress", 0.0, -1.0, 2.0)?;
    for (spread, at, expected) in rows {
        let drawing = row(5)?.translate(Stagger::new(&progress).spread(spread), 0.0);
        let found = lefts(&drawing, &Values::new().set("progress", at))?;
        assert_near(&found, &expected, &format!("spread {spread}, progress {at}"));
    }
    Ok(())
}

#[test]
fn stagger_takes_its_progress_and_spread_from_expressions() -> Outcome {
    let progress = Param::new("progress", 0.5, 0.0, 1.0)?;
    let spread = Param::new("spread", 0.0, 0.0, 0.99)?;
    // progress 0.5 doubled is 1: everything has finished; spread 0.75 and progress 0.5 as above
    let drawing = row(5)?.translate(Stagger::new(&progress * 2.0).spread(&spread), 0.0);
    assert_near(&lefts(&drawing, &Values::new())?, &[1.0; 5], "doubled progress");
    let values = Values::new().set("spread", 0.75);
    assert_near(
        &lefts(
            &row(5)?.translate(Stagger::new(&progress).spread(&spread), 0.0),
            &values,
        )?,
        &[1.0, 1.0, 0.5, 0.0, 0.0],
        "spread from a Param",
    );
    Ok(())
}

// Random ------------------------------------------------------------------------------------------------------------

/// `SplitMix64` as Vigna published it (public domain, prng.di.unimi.it/splitmix64.c): the state steps by 0x9e3779b97f4a7c15
/// and the output is the state scrambled by two xor-shift-multiply rounds.
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// The number from 0 up to 1 that the top 53 bits of `output` make.
#[expect(clippy::cast_precision_loss, reason = "53 bits fit a float exactly")]
fn unit_of(output: u64) -> f64 {
    (output >> 11) as f64 / 9_007_199_254_740_992.0
}

#[test]
fn the_reference_splitmix64_gives_the_published_outputs_for_seed_0() {
    // the first three outputs of the generator seeded 0, from the published test vectors
    let mut state = 0_u64;
    let outputs: Vec<u64> = (0..3).map(|_| splitmix64(&mut state)).collect();
    assert_eq!(
        outputs,
        [0xe220_a839_7b1d_cdaf, 0x6e78_9e6a_a1b9_65f4, 0x06c4_5d18_8009_454f]
    );
}

#[test]
fn random_is_the_published_generator_started_at_the_seed_and_read_at_the_shapes_index() -> Outcome {
    for seed in [
        0_i64,
        1,
        7,
        123_456_789,
        -1,
        -42,
        9_007_199_254_740_991,
        -9_007_199_254_740_991,
    ] {
        let mut state = seed.cast_unsigned();
        let expected: Vec<f64> = (0..6).map(|_| unit_of(splitmix64(&mut state))).collect();
        #[expect(clippy::cast_precision_loss, reason = "the seeds are within 2^53")]
        let found = moved_by(6, Random::new(0.0, 1.0).seed(seed as f64))?;
        assert_eq!(found, expected, "seed {seed}");
    }
    Ok(())
}

#[test]
fn random_scales_the_generator_to_low_and_high() -> Outcome {
    let unit = moved_by(4, Random::new(0.0, 1.0).seed(3.0))?;
    let found = moved_by(4, Random::new(-10.0, 30.0).seed(3.0))?;
    for (unit, found) in unit.iter().zip(&found) {
        assert!((found - (-10.0 + 40.0 * unit)).abs() < 1e-12, "{found} from {unit}");
    }
    assert_eq!(moved_by(3, Random::new(5.0, 5.0))?, [5.0; 3], "low equal to high");
    Ok(())
}

#[test]
fn random_follows_the_shapes_current_index_not_the_shape() -> Outcome {
    // the same three squares, with the first dropped: the seed is read at index 0 and 1 now, as api.md says
    let all = lefts(
        &row(3)?.translate(Random::new(0.0, 100.0).seed(9.0), 0.0),
        &Values::new(),
    )?;
    let kept = row(3)?.without(Select::new().attribute("index", 0));
    let after = lefts(&kept.translate(Random::new(0.0, 100.0).seed(9.0), 0.0), &Values::new())?;
    assert_eq!(after, all[..2]);
    assert_ne!(after, all[1..]);
    Ok(())
}

#[test]
fn random_takes_every_operand_from_expressions() -> Outcome {
    let high = Param::new("high", 10.0, 0.0, 100.0)?;
    let seed = Param::new("seed", 4.0, 0.0, 1000.0)?;
    let drawing = row(5)?.translate(Random::new(0.0, &high).seed(&seed), 0.0);
    let by_literals = |high: f64, seed: f64| moved_by(5, Random::new(0.0, high).seed(seed));
    assert_eq!(lefts(&drawing, &Values::new())?, by_literals(10.0, 4.0)?);
    let values = Values::new().set("high", 20.0).set("seed", 5.0);
    assert_eq!(lefts(&drawing, &values)?, by_literals(20.0, 5.0)?);
    Ok(())
}

proptest! {
    #[test]
    fn random_stays_between_low_and_high_for_any_seed(
        low in -1e6..1e6_f64,
        width in 0.0..1e6_f64,
        seed in -9_007_199_254_740_991_i64..=9_007_199_254_740_991,
    ) {
        #[expect(clippy::cast_precision_loss, reason = "the seeds are within 2^53")]
        let found = moved_by(16, Random::new(low, low + width).seed(seed as f64)).map_err(|error| TestCaseError::fail(error.to_string()))?;
        for value in found {
            prop_assert!(low <= value && value <= low + width, "{value} outside {low}..{}", low + width);
        }
    }

    #[test]
    fn random_is_the_same_for_the_same_seed_and_different_for_another(
        seed in 0_i64..1_000_000,
        other in 1_i64..1_000_000,
    ) {
        #[expect(clippy::cast_precision_loss, reason = "the seeds are within 2^53")]
        let (first, again, another) = (
            moved_by(8, Random::new(0.0, 1.0).seed(seed as f64)).map_err(|error| TestCaseError::fail(error.to_string()))?,
            moved_by(8, Random::new(0.0, 1.0).seed(seed as f64)).map_err(|error| TestCaseError::fail(error.to_string()))?,
            moved_by(8, Random::new(0.0, 1.0).seed((seed + other) as f64)).map_err(|error| TestCaseError::fail(error.to_string()))?,
        );
        prop_assert_eq!(&first, &again);
        prop_assert_ne!(&first, &another);
        // eight shapes, eight different numbers: the value depends on the index
        let mut sorted = first.clone();
        sorted.sort_by(f64::total_cmp);
        sorted.dedup();
        prop_assert_eq!(sorted.len(), 8);
    }
}

// What a drawing refuses --------------------------------------------------------------------------------------------

#[test]
fn a_call_refuses_the_operands_a_per_item_number_cannot_take_and_names_the_argument() -> Outcome {
    let two_53 = 9_007_199_254_740_992.0;
    // (case, the number, whether the call refuses it, a word the message holds)
    let rows: Vec<(&str, Expr, Option<&str>)> = vec![
        ("low above high", Random::new(5.0, 1.0).into(), Some("low")),
        ("low equal to high", Random::new(1.0, 1.0).into(), None),
        (
            "a seed with a fraction",
            Random::new(0.0, 1.0).seed(1.5).into(),
            Some("seed"),
        ),
        (
            "a seed beyond 2^53 - 1",
            Random::new(0.0, 1.0).seed(two_53).into(),
            Some("seed"),
        ),
        (
            "a seed of 2^53 - 1",
            Random::new(0.0, 1.0).seed(two_53 - 1.0).into(),
            None,
        ),
        (
            "a seed of -(2^53 - 1)",
            Random::new(0.0, 1.0).seed(1.0 - two_53).into(),
            None,
        ),
        ("a spread of 1", Stagger::new(0.5).spread(1.0).into(), Some("spread")),
        (
            "a spread below 0",
            Stagger::new(0.5).spread(-0.1).into(),
            Some("spread"),
        ),
        ("a spread of 0", Stagger::new(0.5).spread(0.0).into(), None),
        ("a spread of 0.999", Stagger::new(0.5).spread(0.999).into(), None),
        (
            "a refusal inside a sum",
            1.0 + by_index(0.0, Random::new(2.0, 1.0)),
            Some("low"),
        ),
        (
            "a refusal under a negation",
            -Stagger::new(0.5).spread(2.0),
            Some("spread"),
        ),
    ];
    for (case, number, refused) in rows {
        let drawing = row(2)?.translate(number, 0.0);
        match refused {
            None => assert_eq!(drawing.error(), None, "{case}"),
            Some(word) => {
                let (parameter, message) = parameter_refusal(Err::<(), _>(held(&drawing)?))?;
                assert!(parameter.ends_with("(translate).dx"), "{case}: {parameter}");
                assert!(message.contains(word), "{case}: {message}");
            }
        }
    }
    Ok(())
}

#[test]
fn operands_known_only_when_drawn_are_refused_when_drawn() -> Outcome {
    let low = Param::new("low", 1.0, 0.0, 10.0)?;
    let spread = Param::new("spread", 0.5, 0.0, 2.0)?;
    let seed = Param::new("seed", 0.0, 0.0, 10.0)?;
    let random = row(2)?.translate(Random::new(&low, 2.0).seed(&seed), 0.0);
    let stagger = row(2)?.translate(Stagger::new(0.5).spread(&spread), 0.0);
    assert_eq!(random.error(), None);
    assert_eq!(stagger.error(), None);
    // (drawing, values, a word of the message)
    let rows = [
        (&random, Values::new().set("low", 3.0), "low"),
        (&random, Values::new().set("seed", 2.5), "seed"),
        (&stagger, Values::new().set("spread", 1.0), "spread"),
        (&stagger, Values::new().set("spread", 1.5), "spread"),
    ];
    for (drawing, values, word) in rows {
        let (_, message) = parameter_refusal(drawing.shapes(&values))?;
        assert!(message.contains(word), "{message}");
    }
    assert_eq!(lefts(&stagger, &Values::new().set("spread", 0.25))?.len(), 2);
    Ok(())
}

#[test]
fn a_per_item_number_has_no_value_without_shapes() -> Outcome {
    for number in [
        by_index(0.0, 1.0),
        by_order(0.0, 1.0),
        Random::new(0.0, 1.0).into(),
        Stagger::new(0.5).into(),
        -by_index(0.0, 1.0) * 2.0,
    ] {
        assert_eq!(number.constant(), None, "{number}");
        let (_, message) = parameter_refusal(number.value(&Values::new()))?;
        assert!(message.contains("per shape"), "{message}");
    }
    Ok(())
}

#[test]
fn an_argument_read_once_refuses_a_per_item_number_anywhere_in_it() -> Outcome {
    let font = fraunces()?;
    let at = |options: Text| held(&text("H", options)).and_then(|error| parameter_refusal(Err::<(), _>(error)));
    let per_item = || 1.0 + by_index(10.0, 20.0) * 2.0;
    let rows: [(&str, Text); 4] = [
        ("size", Text::new(&font).size(per_item())),
        ("tracking", Text::new(&font).size(100.0).tracking(per_item())),
        ("leading", Text::new(&font).size(100.0).leading(per_item())),
        ("width", Text::new(&font).size(100.0).width(per_item())),
    ];
    for (argument, options) in rows {
        let (parameter, message) = at(options)?;
        assert!(parameter.ends_with(argument), "{argument}: {parameter}");
        assert!(message.contains("per-item"), "{argument}: {message}");
    }
    // the same arguments take a Param or a plain expression
    let size = Param::new("size", 100.0, 1.0, 500.0)?;
    assert_eq!(
        text("H", Text::new(&font).size(&size * 2.0).tracking(5.0)).error(),
        None
    );
    Ok(())
}

#[test]
fn font_axes_take_a_per_item_number_for_each_character() -> Outcome {
    let font = fraunces()?;
    let width_of = |shapes: &[bullterrier_vector::Shape], at: usize| -> Result<f64, Error> {
        Ok(shapes[at].path().bounds()?.width)
    };
    let draw = |weight: Expr| text("HH", Text::new(&font).size(2000.0).axes([("wght", weight)])).shapes(&Values::new());
    let (light, bold) = (draw(300.0.into())?, draw(700.0.into())?);
    assert!(
        (width_of(&light, 0)? - width_of(&bold, 0)?).abs() > 1.0,
        "the weights differ"
    );
    let mixed = draw(by_index(300.0, 700.0))?;
    assert_eq!(mixed.len(), 2);
    assert_eq!(
        width_of(&mixed, 0)?,
        width_of(&light, 0)?,
        "the first character is light"
    );
    assert_eq!(width_of(&mixed, 1)?, width_of(&bold, 1)?, "the last character is bold");
    Ok(())
}

// Printing and operators -------------------------------------------------------------------------------------------

#[test]
fn a_per_item_number_prints_as_a_call_with_its_operands_named() -> Outcome {
    let burst = Param::new("burst", 0.0, 0.0, 1.0)?;
    let rows: [(Expr, &str); 5] = [
        (by_index(0.0, 1.0), "by_index(first=0, last=1)"),
        (by_order(&burst, 5.0), "by_order(first=burst, last=5)"),
        (
            Random::new(-10.0, 10.0).seed(7.0).into(),
            "random(low=-10, high=10, seed=7)",
        ),
        (Stagger::new(&burst).into(), "stagger(progress=burst, spread=0.5)"),
        (
            Random::new(-10.0, 10.0).seed(7.0) * &burst,
            "random(low=-10, high=10, seed=7) * burst",
        ),
    ];
    for (number, printed) in rows {
        assert_eq!(number.to_string(), printed);
    }
    Ok(())
}

#[test]
fn random_and_stagger_combine_with_the_operators_on_either_side() -> Outcome {
    let burst = Param::new("burst", 2.0, 0.0, 10.0)?;
    let random = || Random::new(0.0, 1.0).seed(2.0);
    let alone = moved_by(3, random())?;
    // the number written, and what each shape's plain random number `r` becomes by hand (the Param is 2)
    let rows = [
        (random() * &burst, alone.iter().map(|r| r * 2.0).collect::<Vec<_>>()),
        (&burst * random(), alone.iter().map(|r| 2.0 * r).collect()),
        (random() + 1.0, alone.iter().map(|r| r + 1.0).collect()),
        (1.0 - random(), alone.iter().map(|r| 1.0 - r).collect()),
        (-random(), alone.iter().map(|r| -r).collect()),
        (random() / &burst, alone.iter().map(|r| r / 2.0).collect()),
    ];
    for (number, expected) in rows {
        let text = number.to_string();
        assert_near(&moved_by(3, number)?, &expected, &text);
    }
    let staggered = moved_by(2, Stagger::new(0.5) + Stagger::new(1.0))?;
    // progress 0.5 finishes the first of two shapes (1) and starts the last (0); progress 1 finishes both
    assert_near(&staggered, &[2.0, 1.0], "stagger plus stagger");
    Ok(())
}

// The three places that fold a plain number to its value ------------------------------------------------------------

#[test]
fn a_per_item_number_with_literal_operands_is_not_folded_to_a_constant_when_the_call_is_made() -> Outcome {
    // `Args::validate` works a plain expression out with no shapes; folding this one would refuse the call or give
    // every shape the value of the first
    let drawing = row(3)?.translate(by_index(0.0, 20.0), 0.0);
    assert_eq!(drawing.error(), None);
    assert_eq!(lefts(&drawing, &Values::new())?, [0.0, 10.0, 20.0]);
    Ok(())
}

#[test]
fn a_per_item_number_is_saved_with_all_its_operands_and_loads_back_to_the_same_shapes() -> Outcome {
    let burst = Param::new("burst", 0.5, 0.0, 1.0)?;
    let progress = Param::new("progress", 0.6, 0.0, 1.0)?;
    let numbers: [(Expr, &str); 5] = [
        (by_index(0.0, 20.0), r#"{"by_index":{"first":0,"last":20}}"#),
        // a per-item number is no plain number even when it equals the default (0) of the argument it stands in
        (by_index(0.0, 0.0), r#"{"by_index":{"first":0,"last":0}}"#),
        (
            by_order(&burst, 5.0),
            r#"{"by_order":{"first":{"param":"burst"},"last":5}}"#,
        ),
        (
            Random::new(-10.0, 10.0).seed(7.0) * &burst,
            r#"{"mul":[{"random":{"high":10,"low":-10,"seed":7}},{"param":"burst"}]}"#,
        ),
        (
            Stagger::new(&progress).spread(0.25).into(),
            r#"{"stagger":{"progress":{"param":"progress"},"spread":0.25}}"#,
        ),
    ];
    for (number, written) in numbers {
        let drawing = row(4)?.translate(number, 0.0);
        let json = drawing.to_json()?;
        assert!(json.contains(written), "{written} is not in {json}");
        let loaded = Drawing::from_json(&json, &Inputs::new())?;
        assert_eq!(loaded.to_json()?, json);
        let values = Values::new();
        assert_eq!(loaded.shapes(&values)?, drawing.shapes(&values)?, "{written}");
    }
    Ok(())
}

/// A document that moves four squares right by `dx`.
fn moving_by(dx: &str) -> String {
    // a document that declares a Param no call reads is refused, so `p` is declared only when `dx` reads it
    let params = if dx.contains(r#""param""#) {
        r#""params":[{"name":"p","default":0.5,"min":0,"max":1,"unit":"","meaning":""}],"#
    } else {
        ""
    };
    format!(
        r#"{{"version":1,{params}"root":[{{"stack":{{"items":[[{{"path":"M0 0 L1 0"}}],[{{"path":"M0 0 L1 0"}}],[{{"path":"M0 0 L1 0"}}],[{{"path":"M0 0 L1 0"}}]]}}}},{{"translate":{{"dx":{dx},"dy":0}}}}]}}"#
    )
}

#[test]
fn a_document_reads_a_per_item_number_where_a_plain_expression_is_refused() -> Outcome {
    let moved = |dx: &str| -> Result<Vec<f64>, Box<dyn std::error::Error>> {
        let drawing = Drawing::from_json(&moving_by(dx), &Inputs::new())?;
        lefts(&drawing, &Values::new())
    };
    assert_eq!(moved(r#"{"by_index":{"first":0,"last":30}}"#)?, [0.0, 10.0, 20.0, 30.0]);
    assert_eq!(moved(r#"{"by_order":{"first":3,"last":6}}"#)?, [3.0, 4.0, 5.0, 6.0]);
    assert_eq!(
        moved(r#"{"add":[{"by_index":{"first":0,"last":3}},1]}"#)?,
        [1.0, 2.0, 3.0, 4.0]
    );
    // seed and spread left out take their defaults
    assert_eq!(
        moved(r#"{"random":{"low":0,"high":1}}"#)?,
        moved(r#"{"random":{"low":0,"high":1,"seed":0}}"#)?
    );
    // four shapes at order 0, 1/3, 2/3, 1 and spread 0.5: they run from 0, 1/6, 1/3, 1/2 for 0.5, at progress 0.5
    assert_near(
        &moved(r#"{"stagger":{"progress":{"param":"p"}}}"#)?,
        &[1.0, 2.0 / 3.0, 1.0 / 3.0, 0.0],
        "default spread",
    );
    // a plain expression stays refused, inside a per-item number as outside
    for plain in [r#"{"add":[1,2]}"#, r#"{"by_index":{"first":{"neg":3},"last":1}}"#] {
        let (_, message) = format_refusal(Drawing::from_json(&moving_by(plain), &Inputs::new()))?;
        assert!(message.contains("plain number"), "{plain}: {message}");
    }
    Ok(())
}

#[test]
fn a_document_refuses_a_per_item_number_with_a_wrong_field_or_a_missing_one() -> Outcome {
    let rows = [
        (r#"{"by_index":{"first":0}}"#, "/root/1/translate/dx/by_index", "last"),
        (
            r#"{"by_index":{"first":0,"last":1,"step":2}}"#,
            "/root/1/translate/dx/by_index/step",
            "not a field",
        ),
        (r#"{"random":{"high":1}}"#, "/root/1/translate/dx/random", "low"),
        (
            r#"{"stagger":{"spread":0.5}}"#,
            "/root/1/translate/dx/stagger",
            "progress",
        ),
        (r#"{"by_order":[0,1]}"#, "/root/1/translate/dx/by_order", "object"),
    ];
    for (dx, pointer, word) in rows {
        let (found, message) = format_refusal(Drawing::from_json(&moving_by(dx), &Inputs::new()))?;
        assert_eq!(found, pointer, "{dx}");
        assert!(message.contains(word), "{dx}: {message}");
    }
    Ok(())
}

#[test]
fn a_document_applies_the_call_checks_to_the_per_item_numbers_it_holds() -> Outcome {
    let rows = [
        (r#"{"random":{"low":3,"high":1}}"#, "low"),
        (r#"{"random":{"low":0,"high":1,"seed":1.5}}"#, "seed"),
        (r#"{"stagger":{"progress":{"param":"p"},"spread":1}}"#, "spread"),
        (r#"{"stagger":{"progress":{"param":"p"},"spread":-0.1}}"#, "spread"),
    ];
    for (dx, word) in rows {
        let (_, message) = format_refusal(Drawing::from_json(&moving_by(dx), &Inputs::new()))?;
        assert!(message.contains(word), "{dx}: {message}");
    }
    Ok(())
}

#[test]
fn a_document_refuses_a_per_item_number_in_an_argument_read_once() -> Outcome {
    let document = |size: &str| {
        format!(
            r#"{{"version":1,"inputs":{{"a":{{"kind":"font"}}}},"root":[{{"text":{{"content":"H","font":"a","size":{size}}}}}]}}"#
        )
    };
    for number in [
        r#"{"by_index":{"first":9,"last":90}}"#,
        r#"{"mul":[{"random":{"low":1,"high":2}},{"param":"p"}]}"#,
        r#"{"neg":{"stagger":{"progress":{"param":"p"}}}}"#,
    ] {
        let (pointer, message) = format_refusal(Drawing::from_json(&document(number), &Inputs::new()))?;
        assert!(pointer.starts_with("/root/0/text/size"), "{number}: {pointer}");
        assert!(message.contains("per-item"), "{number}: {message}");
    }
    Ok(())
}

// The cache tells calls apart by every operand ---------------------------------------------------------------------

#[test]
fn drawings_that_differ_in_one_operand_of_a_per_item_number_draw_differently() -> Outcome {
    // the steps share the cache of the drawing they are built on, so two calls with equal keys would share a result
    // one shape dropped, so that the index (0, 1, 2) and the order (1/3, 2/3, 1) of the three left differ
    let base = row(4)?.without(Select::new().attribute("order", 0.0));
    let drawn = |number: Expr| lefts(&base.translate(number, 0.0), &Values::new());
    let progress = Param::new("progress", 0.6, 0.0, 1.0)?;
    let pairs: [(&str, Expr, Expr); 8] = [
        (
            "seed",
            Random::new(0.0, 9.0).seed(1.0).into(),
            Random::new(0.0, 9.0).seed(2.0).into(),
        ),
        ("low", Random::new(0.0, 9.0).into(), Random::new(1.0, 9.0).into()),
        ("high", Random::new(0.0, 9.0).into(), Random::new(0.0, 8.0).into()),
        (
            "spread",
            Stagger::new(&progress).into(),
            Stagger::new(&progress).spread(0.1).into(),
        ),
        ("first", by_index(0.0, 9.0), by_index(1.0, 9.0)),
        ("last", by_index(0.0, 9.0), by_index(0.0, 8.0)),
        ("index against order", by_index(0.0, 9.0), by_order(0.0, 9.0)),
        ("function", by_order(2.0, 9.0), Random::new(2.0, 9.0).into()),
    ];
    for (operand, left, right) in pairs {
        assert_ne!(drawn(left)?, drawn(right)?, "{operand}");
    }
    Ok(())
}
