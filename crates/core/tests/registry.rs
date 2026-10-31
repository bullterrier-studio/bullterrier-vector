//! The registry of verbs through the public API: every declaration is consistent. The checks run over `verbs()`, so a
//! verb added later is checked without a new test. Each check is a function here that is also run on a declaration
//! built in the test that breaks the rule, so a check that could never fail is caught. The domains are written out
//! by hand below, not read from the library.

use bullterrier_vector::{Arg, Domain, Kind, Literal, Reads, verbs};
use std::collections::BTreeSet;

/// The names that appear more than once.
fn duplicates<'a>(names: impl IntoIterator<Item = &'a str>) -> Vec<&'a str> {
    let mut seen = BTreeSet::new();
    let mut repeated = BTreeSet::new();
    for name in names {
        if !seen.insert(name) {
            repeated.insert(name);
        }
    }
    repeated.into_iter().collect()
}

/// Whether `default` is a value `domain` accepts, by the rules api.md states for each domain.
fn fits(domain: &Domain, default: &Literal) -> bool {
    match (domain, default) {
        (Domain::Any, _) => true,
        (Domain::Choice(options), Literal::Text(text)) => options.contains(text),
        (_, Literal::Text(_)) | (Domain::Choice(_), Literal::Number(_)) => false,
        (_, Literal::Number(number)) => {
            let number = *number;
            number.is_finite()
                && match domain {
                    Domain::Finite => true,
                    Domain::Coordinate => number.abs() <= 1e100,
                    Domain::NonZero => number != 0.0,
                    Domain::Positive => number > 0.0,
                    Domain::AtLeast(least) => number >= *least,
                    Domain::Fraction => (0.0..=1.0).contains(&number),
                    Domain::UpTo(most) => number <= *most,
                    _ => false,
                }
        }
    }
}

/// The argument declarations whose default is outside their domain.
fn bad_defaults(args: &[Arg]) -> Vec<&str> {
    args.iter()
        .filter(|arg| arg.default.as_ref().is_some_and(|default| !fits(&arg.domain, default)))
        .map(|arg| arg.name)
        .collect()
}

// The registry ------------------------------------------------------------------------------------------------------

#[test]
fn verb_names_are_unique() {
    assert_eq!(duplicates(verbs().map(|verb| verb.name)), Vec::<&str>::new());
}

#[test]
fn the_argument_names_of_each_verb_are_unique() {
    for verb in verbs() {
        assert_eq!(
            duplicates(verb.args.iter().map(|arg| arg.name)),
            Vec::<&str>::new(),
            "verb {}",
            verb.name
        );
    }
}

#[test]
fn every_default_satisfies_the_domain_of_its_argument() {
    for verb in verbs() {
        assert_eq!(bad_defaults(verb.args), Vec::<&str>::new(), "verb {}", verb.name);
    }
}

// The checks can fail -----------------------------------------------------------------------------------------------

#[test]
fn the_duplicate_check_names_a_repeated_name_and_passes_distinct_names() {
    assert_eq!(duplicates(["a", "b", "c"]), Vec::<&str>::new());
    assert_eq!(duplicates(["dx", "dy", "dx", "dy", "dx"]), vec!["dx", "dy"]);
}

/// An argument with the given default and domain.
fn arg(name: &'static str, default: Option<Literal>, domain: Domain) -> Arg {
    Arg {
        name,
        kind: Kind::Number,
        default,
        reads: Reads::PerItem,
        domain,
    }
}

#[test]
fn the_default_check_names_an_argument_whose_default_is_outside_its_domain() {
    let args = [
        arg("alpha", Some(Literal::Number(2.0)), Domain::Fraction),
        arg("ok", Some(Literal::Number(0.5)), Domain::Fraction),
        arg("required", None, Domain::Fraction),
        arg(
            "join",
            Some(Literal::Text("sharp")),
            Domain::Choice(&["round", "miter", "bevel"]),
        ),
    ];
    assert_eq!(bad_defaults(&args), vec!["alpha", "join"]);
}

#[test]
fn the_domain_oracle_agrees_with_api_md_at_its_boundaries() {
    // (domain, value, accepted)
    let rows: [(Domain, Literal, bool); 30] = [
        (Domain::Any, Literal::Number(f64::NAN), true),
        (Domain::Any, Literal::Text("x"), true),
        (Domain::Finite, Literal::Number(-3.5), true),
        (Domain::Finite, Literal::Number(f64::INFINITY), false),
        (Domain::Finite, Literal::Text("1"), false),
        (Domain::Coordinate, Literal::Number(1e100), true),
        (Domain::Coordinate, Literal::Number(-1e100), true),
        (Domain::Coordinate, Literal::Number(1.000_000_1e100), false),
        (Domain::NonZero, Literal::Number(-0.5), true),
        (Domain::NonZero, Literal::Number(0.0), false),
        (Domain::Positive, Literal::Number(f64::MIN_POSITIVE), true),
        (Domain::Positive, Literal::Number(0.0), false),
        (Domain::Positive, Literal::Number(-1.0), false),
        (Domain::AtLeast(1.0), Literal::Number(1.0), true),
        (Domain::AtLeast(1.0), Literal::Number(0.99), false),
        (Domain::AtLeast(1.0), Literal::Number(f64::NAN), false),
        (Domain::Fraction, Literal::Number(0.0), true),
        (Domain::Fraction, Literal::Number(1.0), true),
        (Domain::Fraction, Literal::Number(1.0001), false),
        (Domain::Fraction, Literal::Number(-0.0001), false),
        (Domain::UpTo(100_000.0), Literal::Number(100_000.0), true),
        (Domain::UpTo(100_000.0), Literal::Number(100_000.5), false),
        (Domain::UpTo(100_000.0), Literal::Number(f64::NEG_INFINITY), false),
        (Domain::Choice(&["butt", "round"]), Literal::Text("round"), true),
        (Domain::Choice(&["butt", "round"]), Literal::Text("Round"), false),
        (Domain::Choice(&["butt", "round"]), Literal::Text(""), false),
        (Domain::Choice(&["butt", "round"]), Literal::Number(1.0), false),
        (Domain::Choice(&[]), Literal::Text("round"), false),
        (Domain::Positive, Literal::Text("2"), false),
        (Domain::Fraction, Literal::Number(f64::NAN), false),
    ];
    for (domain, value, accepted) in rows {
        assert_eq!(fits(&domain, &value), accepted, "{domain:?} with {value:?}");
    }
}
