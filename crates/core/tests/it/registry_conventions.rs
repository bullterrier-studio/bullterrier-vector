//! The naming conventions of api.md, checked over the registry: one argument name is one kind of value read one way,
//! no argument is called by a name the conventions ban, and the layout words of `halftone` and `bands` are one list.
//! An exception names the heading of docs/design-decisions.md that gives its reason; the heading must exist.

use bullterrier_vector::{Arg, Domain, verbs};
use std::collections::BTreeMap;

const API: &str = include_str!("../../../../docs/api.md");
const DECISIONS: &str = include_str!("../../../../docs/design-decisions.md");

/// Argument names that may differ in kind, read rule or words between verbs, each with the heading that says why.
const ALLOWED: &[(&str, &str)] = &[
    ("amplitude", RULE),
    ("angle", RULE),
    ("by", RULE),
    ("cell", RULE),
    ("center", RULE),
    ("color", RULE),
    ("frame", RULE),
    ("near", RULE),
    ("radius", RULE),
    ("seed", RULE),
    ("shape", RULE),
    ("size", RULE),
    ("spacing", RULE),
    ("start", RULE),
    ("tolerance", RULE),
    ("tone", RULE),
    ("wavelength", RULE),
    ("width", RULE),
];

/// The heading of docs/design-decisions.md whose bullets give the reason for each name of [`ALLOWED`].
const RULE: &str = "One name, one kind: the registry rule";

/// The names the conventions ban, read from the sentence in api.md that bans them ("never `strength`, ... or `k`").
fn banned_names() -> Vec<String> {
    // a missing sentence gives no names, which `the_banned_names_are_the_ones_the_conventions_list` reports
    let Some(sentence) = API.split_once("never `").and_then(|(_, rest)| rest.split_once(". ")) else {
        return Vec::new();
    };
    // after the opening backtick, so the names are the even pieces
    sentence.0.split('`').step_by(2).map(str::to_owned).collect()
}

/// Every way the arguments break the conventions, one line each; empty when they keep them.
fn offences(verbs: &[(&str, &[Arg])], banned: &[String], allowed: &[(&str, &str)]) -> Vec<String> {
    let mut found = Vec::new();
    let mut by_name: BTreeMap<&str, Vec<(&str, &Arg)>> = BTreeMap::new();
    for (verb, args) in verbs {
        for arg in *args {
            if banned.iter().any(|name| name == arg.name) {
                found.push(format!("{verb}.{} is a banned name", arg.name));
            }
            by_name.entry(arg.name).or_default().push((verb, arg));
        }
    }
    for (name, uses) in by_name {
        if allowed.iter().any(|(allowed, _)| *allowed == name) {
            continue;
        }
        let (first_verb, first) = uses[0];
        for (verb, arg) in &uses[1..] {
            if arg.kind != first.kind {
                found.push(format!(
                    "{name}: {first_verb} is {:?}, {verb} is {:?}",
                    first.kind, arg.kind
                ));
            }
            if arg.reads != first.reads {
                found.push(format!(
                    "{name}: {first_verb} reads {:?}, {verb} reads {:?}",
                    first.reads, arg.reads
                ));
            }
            if let (Domain::Choice(a), Domain::Choice(b)) = (first.domain, arg.domain)
                && a != b
            {
                found.push(format!("{name}: {first_verb} takes {a:?}, {verb} takes {b:?}"));
            }
        }
    }
    found
}

/// The exceptions whose heading is missing from `decisions` or whose section never names the argument in backticks.
fn unexplained(allowed: &[(&str, &str)], decisions: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (name, heading) in allowed {
        let Some((_, rest)) = decisions.split_once(&format!("\n## {heading}\n")) else {
            found.push(format!("{name}: no heading `{heading}`"));
            continue;
        };
        let section = rest.split_once("\n## ").map_or(rest, |(section, _)| section);
        if !section.contains(&format!("`{name}`")) {
            found.push(format!("{name}: `{heading}` never names it"));
        }
    }
    found
}

fn registered() -> Vec<(&'static str, &'static [Arg])> {
    verbs().map(|verb| (verb.name, verb.args)).collect()
}

#[test]
fn the_registered_verbs_keep_the_conventions() {
    let found = offences(&registered(), &banned_names(), ALLOWED);
    assert!(found.is_empty(), "{}", found.join("\n"));
}

#[test]
fn halftone_and_bands_take_their_layout_as_one_argument() {
    let layout = |verb: &str| {
        verbs()
            .find(|registered| registered.name == verb)
            .and_then(|verb| verb.args.iter().find(|arg| arg.name == "family"))
            .map(|arg| arg.domain)
    };
    assert!(layout("bands").is_some(), "bands has no `family`");
    assert_eq!(
        layout("halftone"),
        layout("bands"),
        "halftone's layout words are not bands'"
    );
}

#[test]
fn every_exception_is_explained_under_its_heading() {
    let found = unexplained(ALLOWED, DECISIONS);
    assert!(found.is_empty(), "{}", found.join("\n"));
}

#[test]
fn the_banned_names_are_the_ones_the_conventions_list() {
    assert_eq!(
        banned_names(),
        ["strength", "intensity", "factor", "amount", "val", "k"]
    );
}

mod the_rule_itself {
    use super::*;
    use bullterrier_vector::{Kind, Reads};

    const fn arg(name: &'static str, kind: Kind, reads: Reads, domain: Domain) -> Arg {
        Arg {
            name,
            kind,
            default: None,
            reads,
            domain,
        }
    }

    const ANGLE: Arg = arg("angle", Kind::Number, Reads::PerItem, Domain::Finite);
    const SEED_ONCE: Arg = arg("seed", Kind::Number, Reads::Once, Domain::Finite);
    const SEED_PER_ITEM: Arg = arg("seed", Kind::Number, Reads::PerItem, Domain::Finite);
    const AMOUNT: Arg = arg("amount", Kind::Number, Reads::PerItem, Domain::Finite);
    const SPACING_PAIR: Arg = arg("spacing", Kind::Pair, Reads::Once, Domain::Positive);
    const SPACING_NUMBER: Arg = arg("spacing", Kind::Number, Reads::Once, Domain::Positive);
    const FAMILY_A: Arg = arg(
        "family",
        Kind::Text,
        Reads::Literal,
        Domain::Choice(&["lines", "waves"]),
    );
    const FAMILY_B: Arg = arg("family", Kind::Text, Reads::Literal, Domain::Choice(&["grid", "wave"]));

    fn banned() -> Vec<String> {
        vec!["amount".to_owned()]
    }

    #[test]
    fn a_name_with_one_kind_and_one_read_rule_passes() {
        let verbs: [(&str, &[Arg]); 2] = [("rotate", &[ANGLE]), ("wave", &[ANGLE, SEED_ONCE])];
        assert_eq!(offences(&verbs, &banned(), &[]), Vec::<String>::new());
    }

    #[test]
    fn a_banned_name_is_found() {
        let verbs: [(&str, &[Arg]); 1] = [("bulge", &[AMOUNT])];
        assert_eq!(offences(&verbs, &banned(), &[]), ["bulge.amount is a banned name"]);
    }

    #[test]
    fn one_name_with_two_kinds_is_found() {
        let verbs: [(&str, &[Arg]); 2] = [("bands", &[SPACING_NUMBER]), ("hatch", &[SPACING_PAIR])];
        assert_eq!(offences(&verbs, &[], &[]), ["spacing: bands is Number, hatch is Pair"]);
    }

    #[test]
    fn one_name_with_two_read_rules_is_found() {
        let verbs: [(&str, &[Arg]); 2] = [("wobble", &[SEED_PER_ITEM]), ("stipple", &[SEED_ONCE])];
        assert_eq!(
            offences(&verbs, &[], &[]),
            ["seed: wobble reads PerItem, stipple reads Once"]
        );
    }

    #[test]
    fn one_name_with_two_word_lists_is_found() {
        let verbs: [(&str, &[Arg]); 2] = [("bands", &[FAMILY_A]), ("halftone", &[FAMILY_B])];
        assert_eq!(offences(&verbs, &[], &[]).len(), 1);
    }

    const DECISIONS: &str = "## Other\n\nx\n\n## Why\n\n- `seed` is read once.\n\n## After\n\n- `phase`.\n";

    #[test]
    fn an_exception_named_under_its_heading_is_explained() {
        assert_eq!(unexplained(&[("seed", "Why")], DECISIONS), Vec::<String>::new());
    }

    #[test]
    fn an_exception_missing_from_its_section_or_its_heading_is_found() {
        // `phase` is named, but under another heading
        assert_eq!(
            unexplained(&[("phase", "Why")], DECISIONS),
            ["phase: `Why` never names it"]
        );
        assert_eq!(unexplained(&[("seed", "Gone")], DECISIONS), ["seed: no heading `Gone`"]);
    }

    #[test]
    fn an_allowed_name_is_not_reported() {
        let verbs: [(&str, &[Arg]); 2] = [("wobble", &[SEED_PER_ITEM]), ("stipple", &[SEED_ONCE])];
        assert_eq!(offences(&verbs, &[], &[("seed", "Some heading")]), Vec::<String>::new());
    }
}
