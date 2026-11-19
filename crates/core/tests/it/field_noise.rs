//! `Field::noise` and the arithmetic of fields (`add`, `mul`, `min`, `max`, `abs`) through the public API. Oracles:
//! the plain kernel is the one `Path::wobble` moves a lone point at `x = 0` across by (amplitude 1, so the shift is the
//! value itself); octaves are the sum the docs state, read from the one-octave field at doubled points; arithmetic is
//! checked against the operands' own readings combined in the test; a small tree against a hand computation.

#![expect(
    clippy::float_cmp,
    reason = "the laws hold exactly: a sum or product is compared with the same sum or product"
)]

use crate::common::{Outcome, lines, parameter_refusal};
use bullterrier_vector::{Drawing, Error, Field, Flow, Hatch, Inputs, Param, Path, Point, Rect, Values, by_index};
use proptest::prelude::*;

fn noise(wavelength: f64, phase: f64, seed: f64, octaves: usize) -> Result<Field, Error> {
    Field::noise(wavelength, phase, seed, octaves)
}

fn number(value: f64) -> Result<Field, Error> {
    Field::constant(value)
}

/// The pool the laws are checked over: noise, a ramp, a constant and a signed distance.
fn pool() -> Result<Vec<Field>, Error> {
    Ok(vec![
        noise(40.0, 0.25, 3.0, 3)?,
        noise(17.0, 0.0, -8.0, 1)?,
        Field::ramp((0.0, 0.0), (200.0, 50.0))?,
        number(-2.5)?,
        Field::distance(&Path::circle((50.0, 60.0), 30.0)?, false)?,
    ])
}

fn fail(error: impl std::fmt::Display) -> TestCaseError {
    TestCaseError::fail(error.to_string())
}

// Noise ----------------------------------------------------------------------------------------------------------------

#[test]
fn one_octave_is_the_kernel_wobble_displaces_by() -> Outcome {
    for (wavelength, phase, seed) in [(40.0, 0.0, 3.0), (17.5, 0.25, -11.0), (300.0, 0.7, 0.0)] {
        let field = noise(wavelength, phase, seed, 1)?;
        let ys: Vec<f64> = (0..40).map(|i| f64::from(i) * 7.3 - 50.0).collect();
        let data: Vec<String> = ys.iter().map(|y| format!("M0 {y}")).collect();
        let moved = lines(&Path::from_svg_path(&data.join(" "))?.wobble(1.0, wavelength, phase, seed)?)?;
        for (y, (points, _)) in ys.iter().zip(moved) {
            // the shift across, at x = 0, is 0 + 1 * the kernel's value
            assert_eq!(field.at((0.0, *y)), points[0].0, "at y {y}");
        }
    }
    Ok(())
}

proptest! {
    /// Each octave halves the wavelength and the amplitude, and the sum is divided by the sum of the amplitudes.
    #[test]
    fn octaves_sum_the_kernel_at_halved_wavelengths_and_amplitudes(
        (x, y) in (-2000.0..2000.0_f64, -2000.0..2000.0_f64),
        wavelength in 5.0..400.0_f64,
        phase in -3.0..3.0_f64,
        seed in -1000_i32..1000,
        octaves in 1_usize..=8,
    ) {
        let one = noise(wavelength, phase, f64::from(seed), 1).map_err(fail)?;
        let many = noise(wavelength, phase, f64::from(seed), octaves).map_err(fail)?;
        let (mut total, mut weights, mut weight, mut scale) = (0.0, 0.0, 1.0, 1.0);
        for _ in 0..octaves {
            total += weight * one.at((x * scale, y * scale));
            weights += weight;
            weight /= 2.0;
            scale *= 2.0;
        }
        let want = total / weights;
        let got = many.at((x, y));
        prop_assert!((got - want).abs() < 1e-12, "{got} against {want}");
    }

    /// Within -1 and 1 wherever it is read, whatever the octaves.
    #[test]
    fn noise_stays_within_one(
        (x, y) in (-1e6..1e6_f64, -1e6..1e6_f64),
        wavelength in 0.5..1000.0_f64,
        phase in -10.0..10.0_f64,
        seed in -1_000_000_i32..1_000_000,
        octaves in 1_usize..=8,
    ) {
        let value = noise(wavelength, phase, f64::from(seed), octaves).map_err(fail)?.at((x, y));
        prop_assert!((-1.0..=1.0).contains(&value), "{value}");
    }
}

/// `10^exponent`, read as the number it writes.
fn power_of_ten(exponent: i32) -> f64 {
    format!("1e{exponent}").parse().unwrap_or(f64::NAN)
}

proptest! {
    /// From the shortest wavelength the docs allow (1e-200) up, anywhere within the coordinate limits (±1e100), with
    /// every octave: never NaN, within -1 and 1.
    #[test]
    fn noise_stays_within_one_at_any_allowed_wavelength_and_coordinate(
        wavelength in (-200_i32..=3).prop_map(power_of_ten),
        (x, y) in (-1.0..1.0_f64, -1.0..1.0_f64),
        reach in (0_i32..=100).prop_map(power_of_ten),
        octaves in 1_usize..=8,
    ) {
        let field = noise(wavelength, 0.3, 5.0, octaves).map_err(fail)?;
        let value = field.at((x * reach, y * reach));
        prop_assert!((-1.0..=1.0).contains(&value), "{value} at wavelength {wavelength}");
    }
}

#[test]
fn the_shortest_wavelength_reads_at_the_coordinate_limits_and_a_shorter_one_is_refused() -> Outcome {
    let shortest = noise(1e-200, 0.0, 1.0, 8)?;
    for point in [(1e100, 1e100), (-1e100, 3.0), (1e9, -1e100)] {
        let value = shortest.at(point);
        assert!((-1.0..=1.0).contains(&value), "{value} at {point:?}");
    }
    for wavelength in [1e-200 * (1.0 - f64::EPSILON), 1e-300, 1e-307] {
        let (named, message) = parameter_refusal(noise(wavelength, 0.0, 1.0, 8))?;
        assert_eq!(named, "wavelength");
        assert!(message.contains("1e-200"), "{message}");
        assert_eq!(parameter_refusal(Flow::noise(wavelength, 0.0, 1.0))?.0, "wavelength");
        let line = Path::from_svg_path("M0 0 L10 0")?;
        assert_eq!(
            parameter_refusal(line.wobble(1.0, wavelength, 0.0, 1.0))?.0,
            "wavelength"
        );
    }
    Ok(())
}

#[test]
fn equal_seeds_give_equal_fields_and_another_seed_another() -> Outcome {
    let points: Vec<Point> = (0..200)
        .map(|i| Point::new(f64::from(i) * 13.7, f64::from(i % 17) * 21.1))
        .collect();
    let read = |field: &Field| points.iter().map(|p| field.at(*p)).collect::<Vec<_>>();
    let (first, again) = (noise(40.0, 0.3, 7.0, 3)?, noise(40.0, 0.3, 7.0, 3)?);
    assert_eq!(first, again);
    assert_eq!(read(&first), read(&again));
    let other = noise(40.0, 0.3, 8.0, 3)?;
    assert_ne!(first, other);
    let differ = read(&first).iter().zip(read(&other)).filter(|(a, b)| a != &b).count();
    assert!(differ > 190, "only {differ} of 200 readings differ");
    Ok(())
}

#[test]
fn a_whole_cycle_of_phase_is_the_same_field_to_the_bit() -> Outcome {
    for phase in [0.0, 0.25, 0.5, 0.75] {
        let (base, on, back) = (
            noise(30.0, phase, 2.0, 4)?,
            noise(30.0, phase + 1.0, 2.0, 4)?,
            noise(30.0, phase - 1.0, 2.0, 4)?,
        );
        for i in 0..100 {
            let p = (f64::from(i) * 9.1, f64::from(i) * -4.7);
            assert_eq!(on.at(p).to_bits(), base.at(p).to_bits(), "phase {phase} at {p:?}");
            assert_eq!(back.at(p).to_bits(), base.at(p).to_bits(), "phase {phase} at {p:?}");
        }
    }
    Ok(())
}

#[test]
fn a_whole_cycle_of_any_phase_is_the_same_field_up_to_the_rounding_of_the_sum() -> Outcome {
    // 0.1 + 1 is not 1.1 exactly: the phases the two fields read differ by the rounding of the caller's sum, and the
    // field by at most the documented 15 per cycle of phase times that
    for phase in [0.1, 0.3, 0.7, -2.9, 1e6 + 0.1] {
        let shift = ((phase + 1.0_f64).rem_euclid(1.0) - phase.rem_euclid(1.0)).abs();
        let (base, on) = (noise(17.0, phase, 2.0, 4)?, noise(17.0, phase + 1.0, 2.0, 4)?);
        for i in 0..100 {
            let p = (f64::from(i) * 5.5, f64::from(i) * 9.1);
            let apart = (on.at(p) - base.at(p)).abs();
            assert!(
                apart <= 15.0 * shift + 1e-15,
                "phase {phase} at {p:?}: {apart} for a shift of {shift}"
            );
        }
    }
    Ok(())
}

#[test]
fn noise_refuses_a_bad_wavelength_octaves_seed_or_per_item_number() -> Outcome {
    for wavelength in [0.0, -3.0, f64::NAN, f64::INFINITY] {
        assert_eq!(parameter_refusal(noise(wavelength, 0.0, 0.0, 1))?.0, "wavelength");
    }
    for octaves in [0, 9] {
        assert_eq!(parameter_refusal(noise(40.0, 0.0, 0.0, octaves))?.0, "octaves");
    }
    assert!(noise(40.0, 0.0, 0.0, 8).is_ok(), "eight octaves are allowed");
    for seed in [0.5, f64::NAN, 1e16] {
        assert_eq!(parameter_refusal(noise(40.0, 0.0, seed, 1))?.0, "seed");
    }
    assert_eq!(parameter_refusal(noise(40.0, f64::NAN, 0.0, 1))?.0, "phase");
    let by = by_index(10.0, 20.0);
    assert_eq!(
        parameter_refusal(Field::noise(by.clone(), 0.0, 0.0, 1))?.0,
        "wavelength"
    );
    assert_eq!(parameter_refusal(Field::noise(40.0, by, 0.0, 1))?.0, "phase");
    Ok(())
}

// Arithmetic -----------------------------------------------------------------------------------------------------------

proptest! {
    #[test]
    fn add_and_mul_commute_and_associate(
        (x, y) in (-300.0..300.0_f64, -300.0..300.0_f64),
        (a, b, c) in (0_usize..5, 0_usize..5, 0_usize..5),
    ) {
        let fields = pool().map_err(fail)?;
        let (a, b, c) = (&fields[a], &fields[b], &fields[c]);
        let p = (x, y);
        let (va, vb, vc) = (a.at(p), b.at(p), c.at(p));
        let sum = a.add(b).map_err(fail)?.at(p);
        prop_assert_eq!(sum, va + vb);
        prop_assert_eq!(sum, b.add(a).map_err(fail)?.at(p));
        let product = a.mul(b).map_err(fail)?.at(p);
        prop_assert_eq!(product, va * vb);
        prop_assert_eq!(product, b.mul(a).map_err(fail)?.at(p));
        let left = a.add(b).map_err(fail)?.add(c).map_err(fail)?.at(p);
        let right = a.add(&b.add(c).map_err(fail)?).map_err(fail)?.at(p);
        prop_assert!((left - right).abs() <= 1e-12 * (1.0 + va.abs() + vb.abs() + vc.abs()));
        let left = a.mul(b).map_err(fail)?.mul(c).map_err(fail)?.at(p);
        let right = a.mul(&b.mul(c).map_err(fail)?).map_err(fail)?.at(p);
        prop_assert!((left - right).abs() <= 1e-12 * (1.0 + (va * vb * vc).abs()));
    }

    #[test]
    fn min_max_and_abs_obey_their_laws(
        (x, y) in (-300.0..300.0_f64, -300.0..300.0_f64),
        (a, b) in (0_usize..5, 0_usize..5),
    ) {
        let fields = pool().map_err(fail)?;
        let (a, b) = (&fields[a], &fields[b]);
        let p = (x, y);
        let (va, vb) = (a.at(p), b.at(p));
        let (low, high) = (a.min(b).map_err(fail)?.at(p), a.max(b).map_err(fail)?.at(p));
        prop_assert_eq!(low, va.min(vb));
        prop_assert_eq!(high, va.max(vb));
        prop_assert!(low <= high);
        prop_assert_eq!(a.min(a).map_err(fail)?.at(p), va);
        prop_assert_eq!(a.max(a).map_err(fail)?.at(p), va);
        let size = a.abs().map_err(fail)?;
        prop_assert_eq!(size.at(p), va.abs());
        prop_assert!(size.at(p) >= 0.0);
        prop_assert_eq!(size.abs().map_err(fail)?.at(p), size.at(p));
    }

    #[test]
    fn one_is_the_identity_of_mul_and_minus_one_twice_is_the_field(
        (x, y) in (-300.0..300.0_f64, -300.0..300.0_f64),
        a in 0_usize..5,
    ) {
        let fields = pool().map_err(fail)?;
        let a = &fields[a];
        let p = (x, y);
        prop_assert_eq!(a.mul(&number(1.0).map_err(fail)?).map_err(fail)?.at(p), a.at(p));
        let minus = number(-1.0).map_err(fail)?;
        let negated = a.mul(&minus).map_err(fail)?;
        prop_assert_eq!(negated.at(p), -a.at(p));
        prop_assert_eq!(negated.mul(&minus).map_err(fail)?.at(p), a.at(p));
    }
}

#[test]
fn a_small_tree_reads_as_the_hand_computation() -> Outcome {
    // |min(4 * ramp - 1, 2)| + 0.5, the ramp 0 at x = 0 rising to 1 at x = 100 (held beyond)
    let ramp = Field::ramp((0.0, 0.0), (100.0, 0.0))?;
    let tree = ramp
        .mul(&number(4.0)?)?
        .add(&number(-1.0)?)?
        .min(&number(2.0)?)?
        .abs()?
        .add(&number(0.5)?)?;
    for (x, want) in [
        (0.0, 1.5),
        (10.0, 1.1),
        (25.0, 0.5),
        (50.0, 1.5),
        (90.0, 2.5),
        (500.0, 2.5),
    ] {
        assert!(
            (tree.at((x, 7.0)) - want).abs() < 1e-12,
            "at x {x}: {}",
            tree.at((x, 7.0))
        );
    }
    Ok(())
}

#[test]
fn numbers_fold_and_a_number_operand_comes_first() -> Outcome {
    let folded = number(3.0)?.add(&number(4.0)?)?.mul(&number(2.0)?)?;
    assert_eq!(folded, number(14.0)?);
    // every operation on two numbers is the number it makes, written as one
    for (made, want) in [
        (number(2.0)?.mul(&number(-1.0)?)?, "-2"),
        (number(3.0)?.add(&number(4.0)?)?, "7"),
        (number(1.0)?.min(&number(2.0)?)?, "1"),
        (number(1.0)?.max(&number(2.0)?)?, "2"),
        (number(-3.0)?.abs()?, "3"),
    ] {
        assert_eq!(made.to_string(), want);
    }
    let ramp = Field::ramp((0.0, 0.0), (10.0, 0.0))?;
    assert_eq!(ramp.mul(&number(14.0)?)?, number(14.0)?.mul(&ramp)?);
    assert_eq!(
        ramp.mul(&number(14.0)?)?.to_string(),
        "14 * Field.ramp((0, 0), (10, 0))"
    );
    Ok(())
}

#[test]
fn a_tree_deeper_than_sixteen_is_refused_where_it_is_built() -> Outcome {
    let leaf = noise(40.0, 0.0, 1.0, 1)?;
    let mut tree = leaf.clone();
    for _ in 0..16 {
        tree = tree.add(&leaf)?;
    }
    assert_eq!(parameter_refusal(tree.add(&leaf))?.0, "other");
    assert_eq!(parameter_refusal(leaf.add(&tree))?.0, "other");
    assert_eq!(parameter_refusal(tree.abs())?.0, "field");
    Ok(())
}

#[test]
fn reading_a_deep_tree_of_noise_over_a_fine_grid_is_refused_before_it_runs() -> Outcome {
    let leaf = noise(40.0, 0.0, 1.0, 8)?;
    let mut tree = leaf.clone();
    for _ in 0..16 {
        tree = tree.add(&leaf)?;
    }
    let frame = Rect {
        x: 0.0,
        y: 0.0,
        width: 3000.0,
        height: 3000.0,
    };
    assert_eq!(parameter_refusal(tree.contours(&[0.0], frame, 2.0))?.0, "cell");
    // the same grid over one noise field is affordable
    assert!(noise(40.0, 0.0, 1.0, 1)?.contours(&[0.0], frame, 30.0).is_ok());
    Ok(())
}

// Documents and steps --------------------------------------------------------------------------------------------------

fn eaten(wavelength: &Param) -> Result<Field, Error> {
    let edge = Field::distance(&Path::circle((100.0, 60.0), 40.0)?, false)?;
    let bite = Field::noise(wavelength, 0.25, 3.0, 3)?;
    edge.add(&bite.mul(&number(14.0)?)?)?.max(&number(-30.0)?)?.abs()
}

#[test]
fn a_hatch_toned_by_arithmetic_on_fields_reads_its_params_and_round_trips() -> Outcome {
    let wavelength = Param::new("wavelength", 40.0, 10.0, 80.0)?;
    let square = Drawing::from(Path::rect(0.0, 0.0, 200.0, 120.0, 0.0)?);
    let drawing = square.hatch(&Hatch::between(3.0, 9.0, eaten(&wavelength)?), None);
    assert_eq!(drawing.parameters()?.len(), 1, "the Param inside an operand is read");
    let document = drawing.to_json()?;
    assert!(document.contains(r#"{"Field.mul":[14,"#), "{document}");
    let back = Drawing::from_json(&document, &Inputs::new())?;
    assert_eq!(back.to_json()?, document);
    for value in [40.0, 25.0] {
        let values = Values::new().set("wavelength", value);
        let (a, b) = (drawing.shapes(&values)?, back.shapes(&values)?);
        assert_eq!(a.len(), b.len());
        for (a, b) in a.iter().zip(&b) {
            assert_eq!(a.path().to_svg_path(), b.path().to_svg_path());
        }
    }
    let at = |value: f64| -> Result<String, Error> {
        Ok(drawing.shapes(&Values::new().set("wavelength", value))?[0]
            .path()
            .to_svg_path())
    };
    assert_ne!(
        at(40.0)?,
        at(25.0)?,
        "the wavelength is read with the values of the step"
    );
    Ok(())
}

#[test]
fn a_per_item_number_operand_is_refused() -> Outcome {
    assert_eq!(parameter_refusal(Field::constant(by_index(0.0, 1.0)))?.0, "value");
    Ok(())
}

fn hatched(tone: &str) -> String {
    format!(
        r#"{{"root":[{{"path":"M0 0 L200 0 L200 120 L0 120 Z"}},{{"hatch":{{"spacing":[3,9],"tone":{tone}}}}}],"version":1}}"#
    )
}

#[test]
fn a_document_reads_each_operation_and_writes_it_canonically() -> Outcome {
    let noise = r#"{"Field.noise":{"wavelength":40,"seed":3,"octaves":2}}"#;
    for (written, canonical) in [
        (format!(r#"{{"Field.add":[{noise},2]}}"#), r#"{"Field.add":[2,"#),
        (format!(r#"{{"Field.mul":[{noise},-1]}}"#), r#"{"Field.mul":[-1,"#),
        (
            format!(r#"{{"Field.min":[{noise},0.5]}}"#),
            r#"{"Field.min":[{"Field.noise""#,
        ),
        (format!(r#"{{"Field.max":[-0.5,{noise}]}}"#), r#"{"Field.max":[-0.5,"#),
        (format!(r#"{{"Field.abs":{noise}}}"#), r#"{"Field.abs":{"Field.noise""#),
    ] {
        let drawing = Drawing::from_json(&hatched(&written), &Inputs::new())?;
        let text = drawing.to_json()?;
        assert!(text.contains(canonical), "{written}: {text}");
        assert!(
            text.contains(r#""phase":0"#),
            "every argument of noise is written: {text}"
        );
        assert_eq!(Drawing::from_json(&text, &Inputs::new())?.to_json()?, text);
    }
    Ok(())
}

#[test]
fn a_document_noise_without_a_wavelength_takes_the_default_of_flow_noise() -> Outcome {
    let field = Drawing::from_json(&hatched(r#"{"Field.noise":{"seed":1}}"#), &Inputs::new())?.to_json()?;
    assert!(
        field.contains(r#"{"Field.noise":{"octaves":1,"phase":0,"seed":1,"wavelength":400}"#),
        "{field}"
    );
    Ok(())
}

/// [`hatched`], with a Param `p` declared when the tone reads it.
fn hatched_with_p(tone: &str) -> String {
    if !tone.contains("param") {
        return hatched(tone);
    }
    hatched(tone).replace(
        r#"{"root""#,
        r#"{"params":[{"name":"p","default":0.5,"min":0,"max":1,"unit":"","meaning":""}],"root""#,
    )
}

#[test]
fn a_document_writes_no_operation_the_constructor_folds() -> Outcome {
    // as an expression that reads no Param is refused (write its value), an operation the constructor would fold is
    // refused: abs of a number, min or max of two numbers, add or mul of two numbers or expressions
    for tone in [
        r#"{"Field.abs":3}"#,
        r#"{"Field.min":[1,2]}"#,
        r#"{"Field.add":[1,{"param":"p"}]}"#,
        r#"{"Field.mul":[{"param":"p"},2]}"#,
    ] {
        match Drawing::from_json(&hatched_with_p(tone), &Inputs::new()) {
            Err(error @ Error::Format { .. }) => assert!(error.to_string().contains("write"), "{tone}: {error}"),
            other => return Err(format!("{tone}: {other:?}").into()),
        }
    }
    // what the constructor keeps as a node reads back as written
    for tone in [r#"{"Field.abs":{"param":"p"}}"#, r#"{"Field.min":[1,{"param":"p"}]}"#] {
        let text = Drawing::from_json(&hatched_with_p(tone), &Inputs::new())?.to_json()?;
        assert!(text.contains(tone), "{tone}: {text}");
    }
    Ok(())
}

#[test]
fn a_document_with_two_numbers_a_bad_octave_or_one_operand_is_refused() {
    for tone in [
        r#"{"Field.add":[1,2]}"#,
        r#"{"Field.mul":[{"Field.noise":{"wavelength":40}}]}"#,
        r#"{"Field.noise":{"wavelength":40,"octaves":2.5}}"#,
        r#"{"Field.noise":{"wavelength":40,"octaves":9}}"#,
    ] {
        assert!(
            matches!(
                Drawing::from_json(&hatched(tone), &Inputs::new()),
                Err(Error::Format { .. })
            ),
            "{tone}"
        );
    }
}
