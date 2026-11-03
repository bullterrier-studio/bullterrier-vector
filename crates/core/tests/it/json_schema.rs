//! `drawing_schema()`: the schema generated from the registrations. What it says about documents is checked against
//! the draft schema from Python (`tests/test_drawing_schema.py`), which has a JSON Schema validator; these tests check
//! what needs none: the text is stable and every registered verb, argument and default is in it.

use bullterrier_vector::{Kind, Literal, drawing_schema, verbs};

#[test]
fn the_schema_is_the_same_text_every_time() {
    assert_eq!(drawing_schema(), drawing_schema());
}

#[test]
fn every_registered_verb_and_argument_is_in_the_schema() {
    let schema = drawing_schema();
    for verb in verbs() {
        assert!(
            schema.contains(&format!("\"{}\":", verb.name)),
            "verb {} is not in the schema",
            verb.name
        );
        // the one argument of a call such as `path` is the call's value, with no key of its own
        let bare = matches!(verb.args, [only] if matches!(only.kind, Kind::Path | Kind::Shape | Kind::Attributes | Kind::Values));
        for arg in verb.args.iter().filter(|_| !bare) {
            assert!(
                schema.contains(&format!("\"{}\":", arg.name)),
                "argument {} of {} is not in the schema",
                arg.name,
                verb.name
            );
        }
    }
}

#[test]
fn the_schema_states_the_defaults_and_the_limit_of_params() {
    let schema = drawing_schema();
    assert!(schema.contains(r#""maxItems":1000"#), "MAX_PARAMS is not in the schema");
    for arg in verbs().flat_map(|verb| verb.args) {
        if let Some(Literal::Text(default)) = arg.default {
            assert!(
                schema.contains(&format!(r#""default":"{default}""#)),
                "default {default} of {} is not in the schema",
                arg.name
            );
        }
    }
}

#[test]
fn the_schema_is_one_line_of_json_that_is_an_object() {
    let schema = drawing_schema();
    assert!(schema.starts_with('{') && schema.ends_with('}'));
    assert!(!schema.contains('\n'));
}
