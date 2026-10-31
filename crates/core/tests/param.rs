//! `Param` and `Values` through the public API: what a Param accepts when it is made, which values it takes when it is
//! given one (a bounded Param refuses the ones outside its range, a cyclic one wraps them into it), and the values a
//! caller gives. Expected numbers are written by hand; the wrap property checks the result against the definition of a
//! turn (the result lies in one turn from the minimum and differs from the value by whole turns).

mod common;

use bullterrier_vector::{Param, Unit, Value, Values};
use common::{Outcome, log_uniform, parameter_refusal};
use proptest::prelude::*;

// Making a Param ----------------------------------------------------------------------------------------------------

#[test]
fn a_param_keeps_what_it_was_made_with() -> Outcome {
    let param = Param::new("wght", 400.0, 100.0, 900.0)?
        .unit(Unit::Px)?
        .meaning("how heavy the letters are");
    assert_eq!(param.name(), "wght");
    assert_eq!(param.default(), 400.0);
    assert_eq!((param.min(), param.max()), (100.0, 900.0));
    assert_eq!(param.unit_of(), Unit::Px);
    assert_eq!(param.meaning_of(), "how heavy the letters are");
    // unit and meaning are optional: a plain number with no description
    let plain = Param::new("seed", 0.0, 0.0, 99.0)?;
    assert_eq!((plain.unit_of(), plain.meaning_of()), (Unit::None, ""));
    Ok(())
}

#[test]
fn a_param_is_made_with_a_good_name_and_range() -> Outcome {
    // (name, default, min, max)
    let rows = [
        ("rise", 0.0, 0.0, 1.0),
        ("A", 0.5, 0.0, 1.0),
        ("pulse_2", 3.0, -5.0, 5.0),
        ("x9_", -5.0, -5.0, 5.0), // the default may sit on either bound
        ("y", 5.0, -5.0, 5.0),
        ("tiny", 1e-300, 0.0, 1e-299),
        ("huge", 0.0, -1e300, 1e300),
    ];
    for (name, default, min, max) in rows {
        Param::new(name, default, min, max).map_err(|error| format!("{name}: {error}"))?;
    }
    Ok(())
}

#[test]
fn a_param_is_refused_naming_the_argument_and_the_value() -> Outcome {
    // (case, outcome, the argument the error names, a text the message holds)
    let rows = [
        ("empty name", Param::new("", 0.0, 0.0, 1.0), "name", "\"\""),
        ("starts with a digit", Param::new("2x", 0.0, 0.0, 1.0), "name", "2x"),
        ("starts with _", Param::new("_x", 0.0, 0.0, 1.0), "name", "_x"),
        ("has a space", Param::new("a b", 0.0, 0.0, 1.0), "name", "a b"),
        ("has a dash", Param::new("a-b", 0.0, 0.0, 1.0), "name", "a-b"),
        ("not ASCII", Param::new("größe", 0.0, 0.0, 1.0), "name", "größe"),
        ("an output option", Param::new("into", 0.0, 0.0, 1.0), "name", "into"),
        (
            "an output option too",
            Param::new("pen_mm", 0.0, 0.0, 1.0),
            "name",
            "pen_mm",
        ),
        ("min equals max", Param::new("a", 1.0, 1.0, 1.0), "min", "1"),
        ("min above max", Param::new("a", 0.0, 2.0, 1.0), "min", "2"),
        ("default below min", Param::new("a", -0.5, 0.0, 1.0), "default", "-0.5"),
        (
            "default above max",
            Param::new("a", 1.0001, 0.0, 1.0),
            "default",
            "1.0001",
        ),
        ("default NaN", Param::new("a", f64::NAN, 0.0, 1.0), "default", "NaN"),
        (
            "min infinite",
            Param::new("a", 0.0, f64::NEG_INFINITY, 1.0),
            "min",
            "-inf",
        ),
        ("max infinite", Param::new("a", 0.0, 0.0, f64::INFINITY), "max", "inf"),
        ("max NaN", Param::new("a", 0.0, 0.0, f64::NAN), "max", "NaN"),
    ];
    for (case, outcome, argument, text) in rows {
        let (name, message) = parameter_refusal(outcome).map_err(|error| format!("{case}: {error}"))?;
        assert!(
            name == argument && message.contains(text),
            "{case}: named {name}, said {message}"
        );
    }
    Ok(())
}

#[test]
fn a_cyclic_unit_needs_a_range_of_exactly_one_turn() -> Outcome {
    // (unit, min, max, accepted): a degree turn is 360, a cycle is 1
    let rows = [
        (Unit::Deg, 0.0, 360.0, true),
        (Unit::Deg, -180.0, 180.0, true),
        (Unit::Deg, 0.0, 180.0, false),
        (Unit::Deg, 0.0, 720.0, false),
        (Unit::Deg, 0.0, 1.0, false),
        (Unit::Cycle, 0.0, 1.0, true),
        (Unit::Cycle, -0.5, 0.5, true),
        (Unit::Cycle, 0.0, 360.0, false),
        (Unit::Cycle, 0.0, 2.0, false),
        // the other units take any range
        (Unit::Px, 0.0, 180.0, true),
        (Unit::Fraction, -3.0, 700.0, true),
        (Unit::None, 0.0, 0.5, true),
    ];
    for (unit, min, max, accepted) in rows {
        let outcome = Param::new("a", min, min, max)?.unit(unit);
        if accepted {
            outcome.map_err(|error| format!("{unit:?} {min}..{max}: {error}"))?;
        } else {
            let (name, message) = parameter_refusal(outcome)?;
            assert!(
                name == "unit" && message.contains(&format!("{max}")),
                "{unit:?} {min}..{max}: named {name}, said {message}"
            );
        }
    }
    Ok(())
}

#[test]
fn params_are_equal_when_all_six_fields_are_equal() -> Outcome {
    let base = || -> Result<Param, bullterrier_vector::Error> {
        Param::new("a", 0.5, 0.0, 1.0)?
            .unit(Unit::Fraction)
            .map(|p| p.meaning("m"))
    };
    assert_eq!(base()?, base()?);
    let others = [
        (
            "name",
            Param::new("b", 0.5, 0.0, 1.0)?.unit(Unit::Fraction)?.meaning("m"),
        ),
        (
            "default",
            Param::new("a", 0.25, 0.0, 1.0)?.unit(Unit::Fraction)?.meaning("m"),
        ),
        (
            "min",
            Param::new("a", 0.5, -1.0, 1.0)?.unit(Unit::Fraction)?.meaning("m"),
        ),
        (
            "max",
            Param::new("a", 0.5, 0.0, 2.0)?.unit(Unit::Fraction)?.meaning("m"),
        ),
        ("unit", Param::new("a", 0.5, 0.0, 1.0)?.unit(Unit::Px)?.meaning("m")),
        (
            "meaning",
            Param::new("a", 0.5, 0.0, 1.0)?.unit(Unit::Fraction)?.meaning("n"),
        ),
    ];
    for (field, other) in others {
        assert_ne!(base()?, other, "differs in {field}");
    }
    Ok(())
}

#[test]
fn a_negative_zero_is_stored_as_zero() -> Outcome {
    let param = Param::new("a", -0.0, -0.0, 1.0)?;
    assert!(param.default().is_sign_positive() && param.min().is_sign_positive());
    let param = Param::new("a", 0.0, -1.0, -0.0)?;
    assert!(param.max().is_sign_positive());
    Ok(())
}

// Taking a value ----------------------------------------------------------------------------------------------------

#[test]
fn a_bounded_param_takes_values_within_its_range_and_refuses_the_rest() -> Outcome {
    let rise = Param::new("rise", 0.5, 0.0, 1.0)?.unit(Unit::Fraction)?;
    for inside in [0.0, 1.0, 0.25, 1e-300] {
        assert_eq!(rise.accept(inside)?, inside);
    }
    for outside in [1.0001, -0.0001, 2.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let (name, message) = parameter_refusal(rise.accept(outside))?;
        assert!(
            name == "rise" && message.contains(&format!("{outside}")),
            "{outside}: named {name}, said {message}"
        );
    }
    Ok(())
}

#[test]
fn a_cyclic_param_wraps_a_value_into_one_turn_from_its_minimum() -> Outcome {
    let deg = Param::new("angle", 0.0, 0.0, 360.0)?.unit(Unit::Deg)?;
    let signed = Param::new("lean", 0.0, -180.0, 180.0)?.unit(Unit::Deg)?;
    let cycle = Param::new("phase", 0.0, 0.0, 1.0)?.unit(Unit::Cycle)?;
    let centred = Param::new("swing", 0.0, -0.5, 0.5)?.unit(Unit::Cycle)?;
    // (param, value, wrapped)
    let rows = [
        (&deg, 370.0, 10.0),
        (&deg, -10.0, 350.0),
        (&deg, 360.0, 0.0),
        (&deg, 0.0, 0.0),
        (&deg, 359.5, 359.5),
        (&deg, 720.0, 0.0),
        (&deg, -360.0, 0.0),
        (&deg, -725.0, 355.0),
        (&signed, 190.0, -170.0),
        (&signed, -190.0, 170.0),
        (&signed, 180.0, -180.0),
        (&signed, -180.0, -180.0),
        (&signed, 0.0, 0.0),
        (&cycle, 1.25, 0.25),
        (&cycle, -0.25, 0.75),
        (&cycle, 1.0, 0.0),
        (&cycle, 3.0, 0.0),
        (&centred, 0.75, -0.25),
        (&centred, 0.5, -0.5),
        (&centred, -0.5, -0.5),
    ];
    for (param, value, wrapped) in rows {
        assert_eq!(param.accept(value)?, wrapped, "{} given {value}", param.name());
    }
    Ok(())
}

#[test]
fn a_cyclic_param_refuses_a_value_that_is_not_a_number() -> Outcome {
    let deg = Param::new("angle", 0.0, 0.0, 360.0)?.unit(Unit::Deg)?;
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let (name, message) = parameter_refusal(deg.accept(value))?;
        assert!(
            name == "angle" && message.contains(&format!("{value}")),
            "{value}: named {name}, said {message}"
        );
    }
    Ok(())
}

#[test]
fn a_tiny_negative_turn_wraps_to_the_start_not_to_the_end() -> Outcome {
    // 360 - 1e-20 is not a number: the nearest two are 359.99999999999994 and 360, and 360 is not in the range
    let deg = Param::new("angle", 0.0, 0.0, 360.0)?.unit(Unit::Deg)?;
    let wrapped = deg.accept(-1e-20)?;
    assert!((0.0..360.0).contains(&wrapped), "{wrapped}");
    let signed = Param::new("lean", 0.0, -180.0, 180.0)?.unit(Unit::Deg)?;
    let wrapped = signed.accept(179.999_999_999_999_999_9)?;
    assert!((-180.0..180.0).contains(&wrapped), "{wrapped}");
    Ok(())
}

proptest! {
    // The wrapped value lies in [min, min + span) and is the given value shifted by whole turns, at any size and sign.
    #[test]
    fn wrapping_gives_a_value_in_the_turn_that_differs_by_whole_turns(
        degrees in any::<bool>(),
        start in -1000i32..1000,
        magnitude in log_uniform(-18.0, 6.0),
        negative in any::<bool>(),
    ) {
        let (unit, span) = if degrees { (Unit::Deg, 360.0) } else { (Unit::Cycle, 1.0) };
        let min = f64::from(start) / if degrees { 1.0 } else { 8.0 };
        let value = if negative { -magnitude } else { magnitude };
        let param = Param::new("a", min, min, min + span)
            .and_then(|param| param.unit(unit))
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        let wrapped = param.accept(value).map_err(|error| TestCaseError::fail(error.to_string()))?;
        prop_assert!(min <= wrapped && wrapped < min + span, "{wrapped} is outside {min}..{}", min + span);
        let turns = (wrapped - value) / span;
        prop_assert!((turns - libm::round(turns)).abs() <= 1e-8, "{value} became {wrapped}: {turns} turns");
    }

    // A value already inside the range is not moved.
    #[test]
    fn a_value_inside_the_turn_is_kept(start in -1000i32..1000, share in 0.0f64..1.0) {
        let param = Param::new("a", f64::from(start), f64::from(start), f64::from(start) + 360.0)
            .and_then(|param| param.unit(Unit::Deg))
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        let inside = f64::from(start) + share * 360.0;
        prop_assume!(inside < f64::from(start) + 360.0);
        let kept = param.accept(inside).map_err(|error| TestCaseError::fail(error.to_string()))?;
        prop_assert_eq!(kept.to_bits(), inside.to_bits());
    }
}

// Values ------------------------------------------------------------------------------------------------------------

#[test]
fn values_hold_a_number_or_a_list_by_name() {
    let values = Values::new().set("rise", 0.25).set("pulse", vec![0.9, 0.6, 0.3]);
    assert_eq!(values.get("rise"), Some(&Value::Number(0.25)));
    assert_eq!(values.get("pulse"), Some(&Value::List(vec![0.9, 0.6, 0.3])));
    assert_eq!(values.get("weight"), None);
    assert_eq!(Values::new().get("rise"), None);
}

#[test]
fn setting_a_name_again_replaces_its_value() {
    let values = Values::new()
        .set("rise", 0.25)
        .set("rise", vec![1.0, 2.0])
        .set("rise", 0.75);
    assert_eq!(values.get("rise"), Some(&Value::Number(0.75)));
}

#[test]
fn values_are_equal_whatever_the_order_they_were_set_in() {
    let one = Values::new().set("b", 2.0).set("a", 1.0).set("c", vec![3.0]);
    let other = Values::new().set("c", vec![3.0]).set("a", 1.0).set("b", 2.0);
    assert_eq!(one, other);
    assert_ne!(one, Values::new().set("b", 2.0).set("a", 1.0));
    assert_ne!(one, Values::new().set("b", 2.0).set("a", 1.5).set("c", vec![3.0]));
    assert_eq!(Values::new(), Values::default());
}

#[test]
fn a_negative_zero_is_stored_as_zero_in_values() -> Outcome {
    let values = Values::new().set("a", -0.0).set("b", vec![1.0, -0.0]);
    let Some(Value::Number(a)) = values.get("a") else {
        return Err("a is not a number".into());
    };
    assert!(a.is_sign_positive());
    let Some(Value::List(b)) = values.get("b") else {
        return Err("b is not a list".into());
    };
    assert!(b[1].is_sign_positive());
    Ok(())
}
