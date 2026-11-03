//! `Drawing::from_json`: each check of formats.md "Checked by the loader, not the schema" that a registered call can
//! reach, with a test named after it, and the order the checks run in. A failing check is a `FormatError` with the JSON
//! Pointer of the offending value. The documents are written by hand.

use crate::common::{Outcome, format_refusal};
use bullterrier_vector::{Drawing, Inputs};

const TRIANGLE: &str = "M0 0 L10 0 L10 10 Z";

/// The pointer and message `from_json` refuses `text` with.
fn refused(text: &str) -> Result<(String, String), Box<dyn std::error::Error>> {
    format_refusal(Drawing::from_json(text, &Inputs::new()))
}

/// A Param description.
fn param(name: &str, default: f64, min: f64, max: f64, unit: &str) -> String {
    format!(r#"{{"name":"{name}","default":{default},"min":{min},"max":{max},"unit":"{unit}","meaning":""}}"#)
}

/// A plain Param `name` from 0 to 10.
fn plain(name: &str) -> String {
    param(name, 0.0, 0.0, 10.0, "")
}

/// A call that reads the Param `name`.
fn reading(name: &str) -> String {
    format!(r#"{{"translate":{{"dx":{{"param":"{name}"}},"dy":0}}}}"#)
}

/// A document with `params`, `drawings` (a JSON object body) and `root` (JSON text); a part left empty is left out.
fn assemble(params: &[String], drawings: &str, root: &str) -> String {
    let mut members = vec![String::from(r#""version":1"#)];
    if !params.is_empty() {
        members.push(format!(r#""params":[{}]"#, params.join(",")));
    }
    if !drawings.is_empty() {
        members.push(format!(r#""drawings":{{{drawings}}}"#));
    }
    members.push(format!(r#""root":{root}"#));
    format!("{{{}}}", members.join(","))
}

/// A document whose root is a triangle followed by `steps` (JSON text of each call).
fn document(params: &[String], drawings: &str, steps: &[String]) -> String {
    let steps: Vec<String> = steps.iter().map(|step| format!(",{step}")).collect();
    assemble(
        params,
        drawings,
        &format!(r#"[{{"path":"{TRIANGLE}"}}{}]"#, steps.concat()),
    )
}

fn triangle_entry(name: &str) -> String {
    format!(r#""{name}":[{{"path":"{TRIANGLE}"}}]"#)
}

/// A document whose root is a stack of the shared drawings `names`, one item each.
fn drawing_using(params: &[String], drawings: &str, names: &[&str]) -> String {
    let items: Vec<String> = names
        .iter()
        .map(|name| format!(r#"[{{"drawing":"{name}"}}]"#))
        .collect();
    assemble(
        params,
        drawings,
        &format!(r#"[{{"stack":{{"items":[{}]}}}}]"#, items.join(",")),
    )
}

// json-text ---------------------------------------------------------------------------------------------------------

/// A text, the pointer it is refused at and what the message says.
type TextCase = (&'static str, &'static str, &'static str, &'static str);

const NOT_I_JSON: [TextCase; 17] = [
    ("nothing", "", "", "not JSON"),
    (
        "a duplicate key at the top",
        r#"{"version":1,"root":[{"path":""}],"root":[]}"#,
        "/root",
        "twice",
    ),
    (
        "a duplicate key inside a call",
        r#"{"version":1,"root":[{"path":"","path":""}]}"#,
        "/root/0/path",
        "twice",
    ),
    (
        "a duplicate key with an escaped twin",
        r#"{"version":1,"a":1,"\u0061":2,"root":[]}"#,
        "/a",
        "twice",
    ),
    (
        "a lone high surrogate",
        r#"{"version":1,"root":[{"path":"\ud800"}]}"#,
        "/root/0/path",
        "surrogate",
    ),
    (
        "a lone low surrogate",
        r#"{"version":1,"root":[{"path":"\udc00"}]}"#,
        "/root/0/path",
        "surrogate",
    ),
    (
        "a high surrogate with no low one",
        r#"{"version":1,"root":[{"path":"\ud800A"}]}"#,
        "/root/0/path",
        "surrogate",
    ),
    (
        "a number beyond a float",
        r#"{"version":1,"root":[{"translate":{"dx":1e999,"dy":0}}]}"#,
        "/root/0/translate/dx",
        "64-bit float",
    ),
    (
        "a negative number beyond a float",
        r#"{"version":1,"root":[{"translate":{"dx":-1e999,"dy":0}}]}"#,
        "/root/0/translate/dx",
        "64-bit float",
    ),
    (
        "a leading zero",
        r#"{"version":1,"root":[{"translate":{"dx":01,"dy":0}}]}"#,
        "/root/0/translate/dx",
        "leading zero",
    ),
    (
        "NaN",
        r#"{"version":1,"root":[{"translate":{"dx":NaN,"dy":0}}]}"#,
        "/root/0/translate/dx",
        "expected a value",
    ),
    (
        "Infinity",
        r#"{"version":1,"root":[{"translate":{"dx":Infinity,"dy":0}}]}"#,
        "/root/0/translate/dx",
        "expected a value",
    ),
    (
        "a trailing comma",
        r#"{"version":1,"root":[{"path":""},]}"#,
        "/root/1",
        "expected a value",
    ),
    (
        "text after the value",
        r#"{"version":1,"root":[{"path":""}]} x"#,
        "",
        "after the value",
    ),
    (
        "a raw control character in a string",
        "{\"version\":1,\"root\":[{\"path\":\"a\u{1}b\"}]}",
        "/root/0/path",
        "control character",
    ),
    (
        "an escape JSON does not have",
        r#"{"version":1,"root":[{"path":"\x"}]}"#,
        "/root/0/path",
        "escape",
    ),
    (
        "a byte order mark",
        "\u{feff}{\"version\":1,\"root\":[]}",
        "",
        "expected a value",
    ),
];

#[test]
fn json_text_refuses_text_that_is_not_i_json() -> Outcome {
    for (case, text, pointer, fragment) in NOT_I_JSON {
        let (found, message) = refused(text)?;
        assert_eq!(found, pointer, "{case}: {message}");
        assert!(message.contains(fragment), "{case}: {message}");
    }
    Ok(())
}

#[test]
fn json_text_reads_a_surrogate_pair_and_escapes_as_the_characters_they_stand_for() -> Outcome {
    let text = r#"{"version":1,"params":[{"name":"x","default":0,"min":0,"max":1,"unit":"","meaning":"😀 é \n \/ \"q\""}],"root":[{"path":"M0 0"},{"translate":{"dx":{"param":"x"},"dy":0}}]}"#;
    let drawing = Drawing::from_json(text, &Inputs::new())?;
    assert_eq!(drawing.parameters()?[0].meaning_of(), "\u{1f600} \u{e9} \n / \"q\"");
    Ok(())
}

// limits ------------------------------------------------------------------------------------------------------------

#[test]
fn limits_refuses_a_document_past_the_byte_limit_and_reads_one_at_it() -> Outcome {
    let head = r#"{"version":1,"root":[{"path":"M0 0"}]}"#;
    let at_limit = format!("{head}{}", " ".repeat(16 * 1024 * 1024 - head.len()));
    assert!(Drawing::from_json(&at_limit, &Inputs::new()).is_ok());
    let (pointer, message) = refused(&format!("{at_limit} "))?;
    assert_eq!(pointer, "");
    assert!(message.contains("MAX_DOCUMENT_BYTES 16777216"), "{message}");
    Ok(())
}

#[test]
fn limits_refuses_a_document_past_the_call_limit_and_reads_one_at_it() -> Outcome {
    let steps = |count: usize| document(&[], "", &vec![r#"{"translate":{"dx":1,"dy":1}}"#.to_owned(); count]);
    // the source is a call too
    assert!(Drawing::from_json(&steps(99_999), &Inputs::new()).is_ok());
    let (pointer, message) = refused(&steps(100_000))?;
    assert_eq!(pointer, "");
    assert!(message.contains("MAX_CALLS 100000"), "{message}");
    Ok(())
}

#[test]
fn limits_refuses_the_first_param_past_the_param_limit_and_reads_a_thousand() -> Outcome {
    let reads = |count: usize| -> String {
        let params: Vec<String> = (0..count).map(|number| plain(&format!("p{number}"))).collect();
        let steps: Vec<String> = (0..count).map(|number| reading(&format!("p{number}"))).collect();
        document(&params, "", &steps)
    };
    assert_eq!(
        Drawing::from_json(&reads(1_000), &Inputs::new())?.parameters()?.len(),
        1_000
    );
    let (pointer, message) = refused(&reads(1_001))?;
    assert_eq!(pointer, "/params/1000");
    assert!(message.contains("MAX_PARAMS 1000"), "{message}");
    Ok(())
}

#[test]
fn limits_counts_expressions_as_calls() -> Outcome {
    // 50,000 steps whose angle is an expression of one call each, and the steps themselves: 100,001 in all
    let step = r#"{"rotate":{"angle":{"neg":{"param":"x"}}}}"#.to_owned();
    let text = document(&[plain("x")], "", &vec![step; 33_333]);
    // 1 source + 33,333 steps + 2 expressions each (neg, param) = 100,000: at the limit
    assert!(Drawing::from_json(&text, &Inputs::new()).is_ok());
    let step = r#"{"rotate":{"angle":{"neg":{"param":"x"}}}}"#.to_owned();
    let (_, message) = refused(&document(&[plain("x")], "", &vec![step; 33_334]))?;
    assert!(message.contains("MAX_CALLS"), "{message}");
    Ok(())
}

#[test]
fn limits_refuses_text_nested_past_max_depth_without_recursing_past_it() -> Outcome {
    let nested = |levels: usize| format!(r#"{{"version":1,"x":{}{}}}"#, "[".repeat(levels), "]".repeat(levels));
    // the document object is level 1; the parser stops at the first array past MAX_DEPTH and names where it stands
    let (pointer, message) = refused(&nested(200))?;
    assert!(message.contains("MAX_DEPTH 128"), "{message}");
    assert!(pointer.starts_with("/x/0/0/0"), "{pointer}");
    let (pointer, _) = refused(&nested(1_000_000))?;
    assert!(pointer.starts_with("/x/0/0/0"), "{pointer}");
    // 127 arrays inside the document object make 128 levels: read, and then `x` is no key of a document
    let (pointer, message) = refused(&nested(127))?;
    assert_eq!(pointer, "/x");
    assert!(message.contains("not a key of a document"), "{message}");
    // one more is past it
    let (_, message) = refused(&nested(128))?;
    assert!(message.contains("MAX_DEPTH 128"), "{message}");
    Ok(())
}

// drawing-declared, drawing-acyclic, depth --------------------------------------------------------------------------

#[test]
fn drawing_declared_refuses_a_source_that_names_no_entry() -> Outcome {
    let text = drawing_using(&[], &triangle_entry("a"), &["b"]);
    let (pointer, message) = refused(&text)?;
    assert_eq!(pointer, "/root/0/stack/items/0/0/drawing");
    assert!(message.contains(r#""b" is not an entry of `drawings`"#), "{message}");
    Ok(())
}

#[test]
fn drawing_acyclic_refuses_a_shared_drawing_that_reaches_itself() -> Outcome {
    let selfish = r#""a":[{"drawing":"a"}]"#;
    let (pointer, message) = refused(&drawing_using(&[], selfish, &["a"]))?;
    assert_eq!(pointer, "/drawings/a/0/drawing");
    assert!(message.contains("a -> a"), "{message}");
    // through another, and through a stack item
    let round = r#""a":[{"stack":{"items":[[{"drawing":"b"}]]}}],"b":[{"stack":{"items":[[{"drawing":"a"}]]}}]"#;
    let (pointer, message) = refused(&drawing_using(&[], round, &["a"]))?;
    assert_eq!(pointer, "/drawings/b/0/stack/items/0/0/drawing");
    assert!(message.contains("a -> b -> a"), "{message}");
    Ok(())
}

#[test]
fn drawing_acyclic_is_reported_for_a_cycle_that_would_also_be_too_deep() -> Outcome {
    let round = r#""a":[{"drawing":"b"}],"b":[{"drawing":"a"}]"#;
    let (_, message) = refused(&drawing_using(&[], round, &["a"]))?;
    assert!(message.contains("cycle"), "{message}");
    Ok(())
}

/// A document with `levels` shared drawings, each a stack of the one before it.
fn tower(levels: usize) -> String {
    let mut entries = vec![triangle_entry("e0")];
    for level in 1..levels {
        entries.push(format!(
            r#""e{level}":[{{"stack":{{"items":[[{{"drawing":"e{}"}}]]}}}}]"#,
            level - 1
        ));
    }
    drawing_using(&[], &entries.join(","), &[&format!("e{}", levels - 1)])
}

#[test]
fn depth_counts_the_nesting_through_references() -> Outcome {
    // By hand, counting a `{"drawing": name}` call as the array it names: the array of `e0` is 2 deep (it and its
    // path call); each stack over it adds the array, the stack call, its arguments, its items and the item: 5 levels.
    // So `e` of level i is 2 + 5i deep, the root array over `e(L-1)` is 5 more, and the document object one more
    // than the deeper of that and the `drawings` object: 3 + 5L, at most 128 for 25 levels.
    assert!(Drawing::from_json(&tower(25), &Inputs::new()).is_ok());
    let (pointer, message) = refused(&tower(26))?;
    assert_eq!(pointer, "");
    assert!(
        message.contains("MAX_DEPTH 128") && message.contains("133"),
        "{message}"
    );
    Ok(())
}

#[test]
fn depth_reads_a_long_chain_of_references_without_running_out_of_stack() -> Outcome {
    // 20,000 entries each referring to the one before: far past the depth, refused, and not by overflowing the stack
    let mut entries = vec![triangle_entry("e0")];
    for level in 1..20_000 {
        entries.push(format!(r#""e{level}":[{{"drawing":"e{}"}}]"#, level - 1));
    }
    let (_, message) = refused(&drawing_using(&[], &entries.join(","), &["e19999"]))?;
    assert!(message.contains("MAX_DEPTH 128"), "{message}");
    Ok(())
}

// param-range, param-unique, param-declared, param-order, param-used -----------------------------------------------

#[test]
fn param_range_refuses_a_range_that_is_not_one() -> Outcome {
    let cases = [
        (
            "min equals max",
            param("x", 1.0, 1.0, 1.0, ""),
            "/params/0/min",
            "below max",
        ),
        (
            "min above max",
            param("x", 1.0, 2.0, 1.0, ""),
            "/params/0/min",
            "below max",
        ),
        (
            "default below the range",
            param("x", -1.0, 0.0, 10.0, ""),
            "/params/0/default",
            "within 0 to 10",
        ),
        (
            "default above the range",
            param("x", 11.0, 0.0, 10.0, ""),
            "/params/0/default",
            "within 0 to 10",
        ),
        (
            "a cyclic range that is not a turn",
            param("x", 0.0, 0.0, 180.0, "deg"),
            "/params/0/unit",
            "one turn",
        ),
        (
            "a cycle range that is not a turn",
            param("x", 0.0, 0.0, 2.0, "cycle"),
            "/params/0/unit",
            "one turn",
        ),
        (
            "a reserved name",
            param("into", 0.0, 0.0, 1.0, ""),
            "/params/0/name",
            "output option",
        ),
    ];
    for (case, description, pointer, fragment) in cases {
        let name = if case == "a reserved name" { "into" } else { "x" };
        let (found, message) = refused(&document(&[description], "", &[reading(name)]))?;
        assert_eq!(found, pointer, "{case}: {message}");
        assert!(message.contains(fragment), "{case}: {message}");
    }
    Ok(())
}

#[test]
fn param_range_reads_a_range_that_is_one() -> Outcome {
    for description in [
        param("x", 5.0, 0.0, 10.0, ""),
        param("x", 0.0, 0.0, 1.0, "cycle"),
        param("x", 0.0, 0.0, 360.0, "deg"),
        param("x", 0.0, -180.0, 180.0, "deg"),
        param("x", 10.0, 0.0, 10.0, "px"),
    ] {
        Drawing::from_json(&document(&[description], "", &[reading("x")]), &Inputs::new())?;
    }
    Ok(())
}

#[test]
fn param_unique_refuses_two_entries_of_one_name() -> Outcome {
    let (pointer, message) = refused(&document(&[plain("x"), plain("x")], "", &[reading("x")]))?;
    assert_eq!(pointer, "/params/1/name");
    assert!(message.contains(r#""x""#), "{message}");
    Ok(())
}

#[test]
fn param_declared_refuses_a_param_expression_that_names_no_entry() -> Outcome {
    let (pointer, message) = refused(&document(&[plain("x")], "", &[reading("x"), reading("y")]))?;
    assert_eq!(pointer, "/root/2/translate/dx/param");
    assert!(message.contains(r#""y" is not an entry of `params`"#), "{message}");
    // inside an operand
    let step = r#"{"rotate":{"angle":{"mul":[2,{"neg":{"param":"z"}}]}}}"#.to_owned();
    let (pointer, _) = refused(&document(&[], "", &[step]))?;
    assert_eq!(pointer, "/root/1/rotate/angle/mul/1/neg/param");
    Ok(())
}

#[test]
fn param_declared_refuses_a_with_values_key_that_names_no_entry() -> Outcome {
    let steps = [reading("x"), r#"{"with_values":{"y":1}}"#.to_owned()];
    let (pointer, message) = refused(&document(&[plain("x")], "", &steps))?;
    assert_eq!(pointer, "/root/2/with_values/y");
    assert!(message.contains(r#""y" is not an entry of `params`"#), "{message}");
    Ok(())
}

#[test]
fn param_order_refuses_params_in_another_order_than_the_walk_reads_them() -> Outcome {
    let steps = [reading("b"), reading("a")];
    let (pointer, message) = refused(&document(&[plain("a"), plain("b")], "", &steps))?;
    assert_eq!(pointer, "/params");
    assert!(message.contains("that order is b, a"), "{message}");
    Drawing::from_json(&document(&[plain("b"), plain("a")], "", &steps), &Inputs::new())?;
    Ok(())
}

#[test]
fn param_order_follows_the_arguments_of_a_call_then_the_shared_drawings_when_first_reached() -> Outcome {
    // rotate reads `angle` before `about`, whatever order the text gives them in
    let step = r#"{"rotate":{"about":[{"param":"b"},0],"angle":{"param":"a"}}}"#.to_owned();
    Drawing::from_json(
        &document(&[plain("a"), plain("b")], "", std::slice::from_ref(&step)),
        &Inputs::new(),
    )?;
    let (_, message) = refused(&document(&[plain("b"), plain("a")], "", &[step]))?;
    assert!(message.contains("that order is a, b"), "{message}");
    // a shared drawing is read where it is first reached, not where the text lists it
    let entries = format!(r#""s":[{{"path":"{TRIANGLE}"}},{}]"#, reading("late"));
    let root = |text: &str| {
        text.replace(
            r#""root":[{"path":"M0 0 L10 0 L10 10 Z"}]"#,
            &stack_of_early_and_shared(),
        )
    };
    let ordered = root(&document(&[plain("early"), plain("late")], &entries, &[]));
    Drawing::from_json(&ordered, &Inputs::new())?;
    let (pointer, _) = refused(&root(&document(&[plain("late"), plain("early")], &entries, &[])))?;
    assert_eq!(pointer, "/params");
    Ok(())
}

/// A root that reads `early` in its first item and then uses the shared drawing `s`, which reads `late`.
fn stack_of_early_and_shared() -> String {
    format!(
        r#""root":[{{"stack":{{"items":[[{{"path":"{TRIANGLE}"}},{}],[{{"drawing":"s"}}]]}}}}]"#,
        reading("early")
    )
}

#[test]
fn param_used_refuses_a_param_nothing_reads() -> Outcome {
    let (pointer, message) = refused(&document(&[plain("x"), plain("unused")], "", &[reading("x")]))?;
    assert_eq!(pointer, "/params/1");
    assert!(message.contains("read by no call"), "{message}");
    Ok(())
}

// input-declared, input-used ----------------------------------------------------------------------------------------

#[test]
fn input_used_refuses_an_input_nothing_reads() -> Outcome {
    let text = format!(r#"{{"version":1,"inputs":{{"anybody":{{"kind":"font"}}}},"root":[{{"path":"{TRIANGLE}"}}]}}"#);
    let (pointer, message) = refused(&text)?;
    assert_eq!(pointer, "/inputs/anybody");
    assert!(message.contains("read by no call"), "{message}");
    Ok(())
}

// drawing-used ------------------------------------------------------------------------------------------------------

#[test]
fn drawing_used_refuses_an_entry_nothing_uses() -> Outcome {
    let entries = format!("{},{}", triangle_entry("a"), triangle_entry("b"));
    let (pointer, message) = refused(&drawing_using(&[], &entries, &["a"]))?;
    assert_eq!(pointer, "/drawings/b");
    assert!(message.contains("used by no call"), "{message}");
    Ok(())
}

#[test]
fn drawing_used_reads_an_entry_used_once_and_one_used_only_by_another_entry() -> Outcome {
    let entries = format!(
        r#"{},"b":[{{"stack":{{"items":[[{{"drawing":"a"}}]]}}}}]"#,
        triangle_entry("a")
    );
    Drawing::from_json(&drawing_using(&[], &entries, &["b"]), &Inputs::new())?;
    Ok(())
}

// path-data ---------------------------------------------------------------------------------------------------------

#[test]
fn path_data_refuses_data_that_does_not_parse_naming_the_first_bad_command() -> Outcome {
    let text = r#"{"version":1,"root":[{"path":"M0 0 L10 10 L5"}]}"#;
    let (pointer, message) = refused(text)?;
    assert_eq!(pointer, "/root/0/path");
    assert!(message.contains('L'), "{message}");
    // a shape's path
    let text = r#"{"version":1,"root":[{"shape":{"path":"M0 0 X"}}]}"#;
    let (pointer, _) = refused(text)?;
    assert_eq!(pointer, "/root/0/shape/path");
    Ok(())
}

// with-values -------------------------------------------------------------------------------------------------------

#[test]
fn with_values_refuses_a_key_the_drawing_before_it_does_not_read() -> Outcome {
    let steps = [reading("x"), r#"{"with_values":{"y":1}}"#.to_owned(), reading("y")];
    let (pointer, message) = refused(&document(&[plain("x"), plain("y")], "", &steps))?;
    assert_eq!(pointer, "/root/2/with_values/y");
    assert!(message.contains("reads no Param of this name"), "{message}");
    // a Param read only by a step after it
    let steps = [r#"{"with_values":{"x":1}}"#.to_owned(), reading("x")];
    let (pointer, _) = refused(&document(&[plain("x")], "", &steps))?;
    assert_eq!(pointer, "/root/1/with_values/x");
    Ok(())
}

#[test]
fn with_values_refuses_a_key_an_earlier_with_values_fixed() -> Outcome {
    let steps = [
        reading("x"),
        r#"{"with_values":{"x":1}}"#.to_owned(),
        r#"{"with_values":{"x":2}}"#.to_owned(),
    ];
    let (pointer, message) = refused(&document(&[plain("x")], "", &steps))?;
    assert_eq!(pointer, "/root/3/with_values/x");
    assert!(message.contains("already fixed"), "{message}");
    Ok(())
}

#[test]
fn with_values_refuses_a_value_outside_the_range_of_its_param_and_wraps_a_cyclic_one() -> Outcome {
    let steps = [reading("x"), r#"{"with_values":{"x":11}}"#.to_owned()];
    let (pointer, message) = refused(&document(&[plain("x")], "", &steps))?;
    assert_eq!(pointer, "/root/2/with_values/x");
    assert!(message.contains("within 0 to 10"), "{message}");
    let wrapped = [reading("a"), r#"{"with_values":{"a":370}}"#.to_owned()];
    let drawing = Drawing::from_json(
        &document(&[param("a", 0.0, 0.0, 360.0, "deg")], "", &wrapped),
        &Inputs::new(),
    )?;
    assert_eq!(drawing.parameters()?, vec![]);
    Ok(())
}

#[test]
fn with_values_refuses_a_list_value_with_a_bad_entry() -> Outcome {
    let steps = [reading("x"), r#"{"with_values":{"x":[1,11]}}"#.to_owned()];
    let (pointer, _) = refused(&document(&[plain("x")], "", &steps))?;
    assert_eq!(pointer, "/root/2/with_values/x");
    Ok(())
}

// call-checks -------------------------------------------------------------------------------------------------------

#[test]
fn call_checks_refuses_what_a_call_refuses_when_it_is_made() -> Outcome {
    let sized = |width: u32| format!(r#"[{{"stack":{{"items":[[{{"path":"{TRIANGLE}"}}]],"size":[{width},10]}}}}]"#);
    let cases = [
        (
            "items whose sizes differ",
            format!(
                r#"{{"version":1,"root":[{{"stack":{{"items":[{},{}]}}}}]}}"#,
                sized(10),
                sized(20)
            ),
            "/root/0/stack/items",
            "different sizes",
        ),
        (
            "a stroke of none",
            format!(r#"{{"version":1,"root":[{{"path":"{TRIANGLE}"}},{{"stroke":{{"color":"none"}}}}]}}"#),
            "/root/1/stroke/color",
            "paints nothing",
        ),
        (
            "a selection that asks nothing",
            format!(r#"{{"version":1,"root":[{{"path":"{TRIANGLE}"}},{{"only":{{}}}}]}}"#),
            "/root/1/only",
            "at least one attribute",
        ),
        (
            "a whole attribute that is not whole",
            format!(r#"{{"version":1,"root":[{{"path":"{TRIANGLE}"}},{{"without":{{"index":1.5}}}}]}}"#),
            "/root/1/without/index",
            "whole number",
        ),
        (
            "a negative index",
            format!(r#"{{"version":1,"root":[{{"path":"{TRIANGLE}"}},{{"without":{{"index":-1}}}}]}}"#),
            "/root/1/without/index",
            "whole number",
        ),
    ];
    for (case, text, pointer, fragment) in cases {
        let (found, message) = refused(&text)?;
        assert_eq!(found, pointer, "{case}: {message}");
        assert!(message.contains(fragment), "{case}: {message}");
    }
    Ok(())
}

// The order the checks run in ---------------------------------------------------------------------------------------

#[test]
fn the_first_check_to_fail_is_the_one_reported() -> Outcome {
    // each document breaks the check named first and every check named after it
    let cycle = r#""a":[{"drawing":"b"}],"b":[{"drawing":"a"}]"#;
    let table: Vec<(&str, String, &str)> = vec![
        (
            "json-text before drawing-declared",
            r#"{"version":1,"root":[{"drawing":"nowhere"}],"root":[]}"#.to_owned(),
            "twice",
        ),
        (
            "drawing-declared before drawing-acyclic",
            drawing_using(&[], &format!(r#"{cycle},"c":[{{"drawing":"missing"}}]"#), &["a", "c"]),
            "is not an entry of `drawings`",
        ),
        (
            "drawing-acyclic before depth",
            drawing_using(&[], cycle, &["a"]),
            "cycle",
        ),
        (
            "depth before param-range",
            tower(40).replacen(
                r#""version":1"#,
                &format!(r#""version":1,"params":[{}]"#, param("x", 5.0, 1.0, 1.0, "")),
                1,
            ),
            "MAX_DEPTH",
        ),
        (
            "param-range before param-unique",
            document(&[param("x", 0.0, 1.0, 1.0, ""), plain("x")], "", &[reading("x")]),
            "below max",
        ),
        (
            "param-unique before param-declared",
            document(&[plain("x"), plain("x")], "", &[reading("x"), reading("y")]),
            "names an earlier Param",
        ),
        (
            "param-declared before param-order",
            document(
                &[plain("b"), plain("a")],
                "",
                &[reading("a"), reading("b"), reading("c")],
            ),
            "is not an entry of `params`",
        ),
        (
            "param-order before param-used",
            document(
                &[plain("b"), plain("a"), plain("unused")],
                "",
                &[reading("a"), reading("b")],
            ),
            "that order is a, b",
        ),
        (
            "param-used before input-used",
            format!(
                r#"{{"version":1,"params":[{}],"inputs":{{"f":{{"kind":"font"}}}},"root":[{{"path":"{TRIANGLE}"}}]}}"#,
                plain("unused")
            ),
            "read by no call",
        ),
    ];
    for (case, text, fragment) in &table {
        let (pointer, message) = refused(text)?;
        assert!(message.contains(fragment), "{case}: {pointer}: {message}");
    }
    Ok(())
}

#[test]
fn the_pointer_says_which_of_two_failing_checks_was_reported() -> Outcome {
    let unused_entry = triangle_entry("unused");
    let (pointer, _) = refused(&format!(
        r#"{{"version":1,"params":[{}],"inputs":{{"f":{{"kind":"font"}}}},"root":[{{"path":"{TRIANGLE}"}}]}}"#,
        plain("unused")
    ))?;
    assert_eq!(pointer, "/params/0", "param-used before input-used");
    let text = format!(
        r#"{{"version":1,"inputs":{{"f":{{"kind":"font"}}}},"drawings":{{{unused_entry}}},"root":[{{"path":"{TRIANGLE}"}}]}}"#
    );
    assert_eq!(refused(&text)?.0, "/inputs/f", "input-used before drawing-used");
    let text = format!(r#"{{"version":1,"drawings":{{{unused_entry}}},"root":[{{"path":"M0 0 X"}}]}}"#);
    assert_eq!(refused(&text)?.0, "/drawings/unused", "drawing-used before path-data");
    let text = document(
        &[plain("x")],
        "",
        &[reading("x"), r#"{"with_values":{"x":11}}"#.to_owned()],
    )
    .replace(TRIANGLE, "M0 0 X");
    assert_eq!(refused(&text)?.0, "/root/0/path", "path-data before with-values");
    let text = document(
        &[plain("x")],
        "",
        &[
            reading("x"),
            r#"{"with_values":{"x":11}}"#.to_owned(),
            r#"{"stroke":{"color":"none"}}"#.to_owned(),
        ],
    );
    assert_eq!(
        refused(&text)?.0,
        "/root/2/with_values/x",
        "with-values before call-checks"
    );
    // call-checks of an early call come after the with-values failure of a later one
    let text = document(
        &[plain("x")],
        "",
        &[
            r#"{"stroke":{"color":"none"}}"#.to_owned(),
            reading("x"),
            r#"{"with_values":{"x":11}}"#.to_owned(),
        ],
    );
    assert_eq!(
        refused(&text)?.0,
        "/root/3/with_values/x",
        "with-values anywhere before call-checks anywhere"
    );
    Ok(())
}
