//! `Drawing::from_json`: what the schema checks value by value (the version, names, types, arguments, ranges of
//! literals, expressions), the error each gives with its JSON Pointer, and that every spelling the format allows reads
//! as the same drawing. The documents are written by hand.

use crate::common::{Outcome, format_refusal};
use bullterrier_vector::{Drawing, Inputs};

const TRIANGLE: &str = "M0 0 L10 0 L10 10 Z";
const LIBRARY: &str = env!("CARGO_PKG_VERSION");

fn refused(text: &str) -> Result<(String, String), Box<dyn std::error::Error>> {
    format_refusal(Drawing::from_json(text, &Inputs::new()))
}

/// A document whose root is the triangle followed by `steps` (JSON text).
fn after_triangle(steps: &str) -> String {
    format!(r#"{{"version":1,"root":[{{"path":"{TRIANGLE}"}}{steps}]}}"#)
}

fn canonical(text: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(Drawing::from_json(text, &Inputs::new())?.to_json()?)
}

// The envelope ------------------------------------------------------------------------------------------------------

#[test]
fn a_document_without_a_version_or_with_another_one_is_refused_naming_the_versions_read() -> Outcome {
    let (pointer, message) = refused(r#"{"root":[{"path":""}]}"#)?;
    assert_eq!(pointer, "");
    assert!(
        message.contains("no `version`") && message.contains("reads version 1"),
        "{message}"
    );
    for (found, text) in [
        ("2", r#"{"version":2,"root":[{"path":""}]}"#),
        ("1.5", r#"{"version":1.5,"root":[{"path":""}]}"#),
        ("a string", r#"{"version":"1","root":[{"path":""}]}"#),
        ("null", r#"{"version":null,"root":[{"path":""}]}"#),
    ] {
        let (pointer, message) = refused(text)?;
        assert_eq!(pointer, "/version", "{found}");
        assert!(
            message.contains(found) && message.contains("reads version 1"),
            "{found}: {message}"
        );
    }
    Ok(())
}

#[test]
fn a_document_that_is_not_an_object_or_has_no_root_or_an_unknown_key_is_refused() -> Outcome {
    for text in ["[]", "1", "null", r#""x""#] {
        assert_eq!(refused(text)?.0, "", "{text}");
    }
    let (pointer, message) = refused(r#"{"version":1}"#)?;
    assert_eq!((pointer.as_str(), message.contains("no `root`")), ("", true));
    let (pointer, message) = refused(r#"{"version":1,"root":[{"path":""}],"fonts":{}}"#)?;
    assert_eq!(pointer, "/fonts");
    assert!(
        message.contains(LIBRARY),
        "a newer document names the library's version: {message}"
    );
    Ok(())
}

#[test]
fn a_part_that_is_empty_is_refused_because_the_format_leaves_it_out() -> Outcome {
    for (part, text) in [
        ("params", r#"{"version":1,"params":[],"root":[{"path":""}]}"#),
        ("inputs", r#"{"version":1,"inputs":{},"root":[{"path":""}]}"#),
        ("drawings", r#"{"version":1,"drawings":{},"root":[{"path":""}]}"#),
    ] {
        assert_eq!(refused(text)?.0, format!("/{part}"));
    }
    assert_eq!(refused(r#"{"version":1,"root":[]}"#)?.0, "/root");
    Ok(())
}

#[test]
fn a_param_description_has_all_six_fields_with_the_right_types() -> Outcome {
    let good = r#"{"name":"x","default":0,"min":0,"max":1,"unit":"","meaning":""}"#;
    let use_it = r#"{"translate":{"dx":{"param":"x"},"dy":0}}"#;
    let doc = |param: &str| format!(r#"{{"version":1,"params":[{param}],"root":[{{"path":"M0 0"}},{use_it}]}}"#);
    Drawing::from_json(&doc(good), &Inputs::new())?;
    for (case, param, pointer) in [
        (
            "a missing field",
            r#"{"name":"x","default":0,"min":0,"max":1,"unit":""}"#,
            "/params/0",
        ),
        (
            "an extra field",
            r#"{"name":"x","default":0,"min":0,"max":1,"unit":"","meaning":"","step":1}"#,
            "/params/0/step",
        ),
        (
            "a name that starts with a digit",
            r#"{"name":"1x","default":0,"min":0,"max":1,"unit":"","meaning":""}"#,
            "/params/0/name",
        ),
        (
            "a name with a dash",
            r#"{"name":"a-b","default":0,"min":0,"max":1,"unit":"","meaning":""}"#,
            "/params/0/name",
        ),
        (
            "an empty name",
            r#"{"name":"","default":0,"min":0,"max":1,"unit":"","meaning":""}"#,
            "/params/0/name",
        ),
        (
            "a default that is a string",
            r#"{"name":"x","default":"0","min":0,"max":1,"unit":"","meaning":""}"#,
            "/params/0/default",
        ),
        (
            "a unit that is not one",
            r#"{"name":"x","default":0,"min":0,"max":1,"unit":"km","meaning":""}"#,
            "/params/0/unit",
        ),
        (
            "a meaning that is a number",
            r#"{"name":"x","default":0,"min":0,"max":1,"unit":"","meaning":3}"#,
            "/params/0/meaning",
        ),
        ("an entry that is not an object", "[]", "/params/0"),
    ] {
        let (found, message) = refused(&doc(param))?;
        assert_eq!(found, pointer, "{case}: {message}");
    }
    Ok(())
}

#[test]
fn an_input_has_a_kind_and_an_optional_sha256() -> Outcome {
    let doc = |input: &str| format!(r#"{{"version":1,"inputs":{{"f":{input}}},"root":[{{"path":""}}]}}"#);
    for (case, input, pointer) in [
        ("no kind", "{}", "/inputs/f"),
        ("an unknown kind", r#"{"kind":"image"}"#, "/inputs/f/kind"),
        ("an extra field", r#"{"kind":"font","uri":"x"}"#, "/inputs/f/uri"),
        ("a short hash", r#"{"kind":"map","sha256":"abc"}"#, "/inputs/f/sha256"),
        (
            "an upper case hash",
            &format!(r#"{{"kind":"map","sha256":"{}"}}"#, "A".repeat(64)),
            "/inputs/f/sha256",
        ),
    ] {
        assert_eq!(refused(&doc(input))?.0, pointer, "{case}");
    }
    // a good input reaches the next check, which finds nothing reads it
    let good = format!(r#"{{"kind":"map","sha256":"{}"}}"#, "ab".repeat(32));
    assert_eq!(refused(&doc(&good))?.0, "/inputs/f");
    Ok(())
}

#[test]
fn a_name_of_a_shared_drawing_or_an_input_is_letters_digits_and_underscores() -> Outcome {
    let text = format!(r#"{{"version":1,"drawings":{{"a b":[{{"path":""}}]}},"root":[{{"path":"{TRIANGLE}"}}]}}"#);
    assert_eq!(refused(&text)?.0, "/drawings/a b");
    let text = format!(r#"{{"version":1,"drawings":{{"a/b":[{{"path":""}}]}},"root":[{{"path":"{TRIANGLE}"}}]}}"#);
    assert_eq!(
        refused(&text)?.0,
        "/drawings/a~1b",
        "a slash in a name is escaped in the pointer"
    );
    Ok(())
}

// Calls -------------------------------------------------------------------------------------------------------------

#[test]
fn a_call_is_an_object_of_one_key() -> Outcome {
    let cases = [
        ("a string", r#"{"version":1,"root":["path"]}"#, "/root/0"),
        ("no key", r#"{"version":1,"root":[{}]}"#, "/root/0"),
        (
            "two keys",
            &after_triangle(r##",{"fill":{"color":"#ff0000"},"opacity":{"alpha":1}}"##),
            "/root/1",
        ),
    ];
    for (case, text, pointer) in cases {
        assert_eq!(refused(text)?.0, pointer, "{case}");
    }
    Ok(())
}

#[test]
fn a_name_the_library_does_not_know_is_refused_naming_it_and_the_library_version() -> Outcome {
    let (pointer, message) = refused(&after_triangle(r#",{"wibble":{}}"#))?;
    assert_eq!(pointer, "/root/1/wibble");
    assert!(message.contains("`wibble`") && message.contains(LIBRARY), "{message}");
    // a call from a later batch of the roadmap reads the same way
    let (pointer, message) = refused(&after_triangle(r#",{"offset":{"distance":4}}"#))?;
    assert_eq!(pointer, "/root/1/offset");
    assert!(message.contains("`offset`"), "{message}");
    Ok(())
}

#[test]
fn a_drawing_starts_with_a_source_and_continues_with_steps() -> Outcome {
    let (pointer, message) = refused(r##"{"version":1,"root":[{"fill":{"color":"#ff0000"}}]}"##)?;
    assert_eq!(pointer, "/root/0/fill");
    assert!(message.contains("starts with a source"), "{message}");
    let (pointer, message) = refused(&after_triangle(&format!(r#",{{"path":"{TRIANGLE}"}}"#)))?;
    assert_eq!(pointer, "/root/1/path");
    assert!(message.contains("source"), "{message}");
    let text = format!(
        r#"{{"version":1,"drawings":{{"a":[{{"path":"{TRIANGLE}"}}]}},"root":[{{"stack":{{"items":[[{{"drawing":"a"}},{{"drawing":"a"}}]]}}}}]}}"#
    );
    assert_eq!(refused(&text)?.0, "/root/0/stack/items/0/1/drawing");
    Ok(())
}

#[test]
fn the_arguments_of_a_call_are_the_ones_its_verb_declares() -> Outcome {
    let cases = [
        (
            "an argument the verb lacks",
            after_triangle(r#",{"translate":{"dx":1,"dy":2,"dz":3}}"#),
            "/root/1/translate/dz",
        ),
        (
            "a required argument missing",
            after_triangle(r#",{"translate":{"dx":1}}"#),
            "/root/1/translate",
        ),
        (
            "a body that is not an object",
            after_triangle(r#",{"translate":[1,2]}"#),
            "/root/1/translate",
        ),
        (
            "a path that is not a string",
            r#"{"version":1,"root":[{"path":3}]}"#.to_owned(),
            "/root/0/path",
        ),
        (
            "a bare body of the wrong type",
            after_triangle(r#",{"only":[1]}"#),
            "/root/1/only",
        ),
        (
            "a stack with no items",
            r#"{"version":1,"root":[{"stack":{"size":[1,1]}}]}"#.to_owned(),
            "/root/0/stack",
        ),
        (
            "items that are not drawings",
            r#"{"version":1,"root":[{"stack":{"items":[1]}}]}"#.to_owned(),
            "/root/0/stack/items/0",
        ),
        (
            "an item that is empty",
            r#"{"version":1,"root":[{"stack":{"items":[[]]}}]}"#.to_owned(),
            "/root/0/stack/items/0",
        ),
        (
            "a point of three numbers",
            after_triangle(r#",{"rotate":{"angle":1,"about":[1,2,3]}}"#),
            "/root/1/rotate/about",
        ),
        (
            "a point that is a number",
            after_triangle(r#",{"rotate":{"angle":1,"about":1}}"#),
            "/root/1/rotate/about",
        ),
        (
            "a number that is a string",
            after_triangle(r#",{"translate":{"dx":"1","dy":2}}"#),
            "/root/1/translate/dx",
        ),
    ];
    for (case, text, pointer) in cases {
        let (found, message) = refused(&text)?;
        assert_eq!(found, pointer, "{case}: {message}");
    }
    Ok(())
}

#[test]
fn a_literal_outside_the_range_of_its_argument_is_refused_with_the_message_of_the_call() -> Outcome {
    let cases = [
        (
            "a stroke width of 0",
            after_triangle(r##",{"stroke":{"color":"#ff0000","width":0}}"##),
            "/root/1/stroke/width",
            "above 0",
        ),
        (
            "a miter limit below 1",
            after_triangle(r##",{"stroke":{"color":"#ff0000","miter_limit":0.5}}"##),
            "/root/1/stroke/miter_limit",
            "at least 1",
        ),
        (
            "an alpha above 1",
            after_triangle(r#",{"opacity":{"alpha":1.5}}"#),
            "/root/1/opacity/alpha",
            "from 0 to 1",
        ),
        (
            "a scale of 0",
            after_triangle(r#",{"scale":{"by":0}}"#),
            "/root/1/scale/by",
            "other than 0",
        ),
        (
            "a scale pair with a 0",
            after_triangle(r#",{"scale":{"by":[2,0]}}"#),
            "/root/1/scale/by/1",
            "other than 0",
        ),
        (
            "a translation beyond 1e100",
            after_triangle(r#",{"translate":{"dx":1e101,"dy":0}}"#),
            "/root/1/translate/dx",
            "within",
        ),
        (
            "a stack size of 0",
            r#"{"version":1,"root":[{"stack":{"items":[],"size":[0,10]}}]}"#.to_owned(),
            "/root/0/stack/size/0",
            "above 0",
        ),
        (
            "a stack size past 100000",
            r#"{"version":1,"root":[{"stack":{"items":[],"size":[10,100001]}}]}"#.to_owned(),
            "/root/0/stack/size/1",
            "at most 100000",
        ),
        (
            "a join that is not a word of the set",
            after_triangle(r##",{"stroke":{"color":"#ff0000","join":"sharp"}}"##),
            "/root/1/stroke/join",
            "\"round\", \"miter\", \"bevel\"",
        ),
        (
            "a blend mode that is not one",
            after_triangle(r#",{"blend":{"mode":"screen"}}"#),
            "/root/1/blend/mode",
            "\"normal\", \"multiply\"",
        ),
        (
            "a colour that is three digits",
            after_triangle(r##",{"fill":{"color":"#f00"}}"##),
            "/root/1/fill/color",
            "#rrggbb",
        ),
        (
            "a colour name",
            after_triangle(r#",{"fill":{"color":"red"}}"#),
            "/root/1/fill/color",
            "#rrggbb",
        ),
    ];
    for (case, text, pointer, fragment) in cases {
        let (found, message) = refused(&text)?;
        assert_eq!(found, pointer, "{case}: {message}");
        assert!(message.contains(fragment), "{case}: {message}");
        assert!(
            !message.contains("step 1"),
            "{case}: the message has no place of the code API: {message}"
        );
    }
    Ok(())
}

#[test]
fn a_stack_size_is_a_literal_and_takes_no_expression() -> Outcome {
    let text = r#"{"version":1,"root":[{"stack":{"items":[],"size":[{"neg":{"param":"x"}},10]}}]}"#;
    let (pointer, message) = refused(text)?;
    assert_eq!(pointer, "/root/0/stack/size/0");
    assert!(message.contains("no expression"), "{message}");
    Ok(())
}

// Expressions -------------------------------------------------------------------------------------------------------

fn with_x(angle: &str) -> String {
    format!(
        r#"{{"version":1,"params":[{{"name":"x","default":0,"min":-10,"max":10,"unit":"","meaning":""}}],"root":[{{"path":"{TRIANGLE}"}},{{"rotate":{{"angle":{angle}}}}}]}}"#
    )
}

#[test]
fn an_expression_is_a_param_or_arithmetic_with_a_param_somewhere_in_it() -> Outcome {
    for angle in [
        r#"{"param":"x"}"#,
        r#"{"neg":{"param":"x"}}"#,
        r#"{"add":[1,{"param":"x"}]}"#,
        r#"{"sub":[{"param":"x"},1]}"#,
        r#"{"mul":[{"param":"x"},{"param":"x"}]}"#,
        r#"{"div":[1,{"param":"x"}]}"#,
        r#"{"add":[{"mul":[2,{"param":"x"}]},{"neg":{"param":"x"}}]}"#,
    ] {
        Drawing::from_json(&with_x(angle), &Inputs::new())?;
    }
    Ok(())
}

#[test]
fn an_expression_that_reads_no_param_is_refused_because_it_is_a_plain_number() -> Outcome {
    let cases = [
        (r#"{"add":[1,2]}"#, "/root/1/rotate/angle"),
        (r#"{"neg":3}"#, "/root/1/rotate/angle"),
        (r#"{"mul":[2,{"neg":3}]}"#, "/root/1/rotate/angle/mul/1"),
    ];
    for (angle, pointer) in cases {
        let text = format!(r#"{{"version":1,"root":[{{"path":"{TRIANGLE}"}},{{"rotate":{{"angle":{angle}}}}}]}}"#);
        let (found, message) = refused(&text)?;
        assert_eq!(found, pointer, "{angle}: {message}");
        assert!(message.contains("plain number"), "{angle}: {message}");
    }
    Ok(())
}

#[test]
fn a_literal_divisor_of_zero_is_refused() -> Outcome {
    let (pointer, message) = refused(&with_x(r#"{"div":[{"param":"x"},0]}"#))?;
    assert_eq!(pointer, "/root/1/rotate/angle/div/1");
    assert!(message.contains("divisor of 0"), "{message}");
    Ok(())
}

#[test]
fn a_malformed_expression_is_refused_where_it_is() -> Outcome {
    let cases = [
        ("two keys", r#"{"param":"x","neg":1}"#, "/root/1/rotate/angle"),
        ("no key", "{}", "/root/1/rotate/angle"),
        (
            "three operands",
            r#"{"add":[1,2,{"param":"x"}]}"#,
            "/root/1/rotate/angle/add",
        ),
        ("one operand", r#"{"mul":[{"param":"x"}]}"#, "/root/1/rotate/angle/mul"),
        (
            "operands that are not an array",
            r#"{"sub":{"param":"x"}}"#,
            "/root/1/rotate/angle/sub",
        ),
        (
            "a param name that is a number",
            r#"{"param":1}"#,
            "/root/1/rotate/angle/param",
        ),
        (
            "a param name with a space",
            r#"{"param":"a b"}"#,
            "/root/1/rotate/angle/param",
        ),
        ("a string", r#""x""#, "/root/1/rotate/angle"),
        ("an array", "[1]", "/root/1/rotate/angle"),
    ];
    for (case, angle, pointer) in cases {
        let (found, message) = refused(&with_x(angle))?;
        assert_eq!(found, pointer, "{case}: {message}");
    }
    Ok(())
}

#[test]
fn an_expression_the_library_does_not_have_is_refused_naming_it_and_the_library_version() -> Outcome {
    let (pointer, message) = refused(&with_x(r#"{"sin":{"first":0,"last":1}}"#))?;
    assert_eq!(pointer, "/root/1/rotate/angle/sin");
    assert!(message.contains("`sin`") && message.contains(LIBRARY), "{message}");
    Ok(())
}

// Shapes ------------------------------------------------------------------------------------------------------------

#[test]
fn a_shape_has_a_path_and_the_attributes_a_generator_writes() -> Outcome {
    let good = format!(
        r#"{{"version":1,"root":[{{"shape":{{"path":"{TRIANGLE}","attributes":{{"glyph":2,"char":"O","word":1,"line":0,"part":3,"copy":4,"level":0.5,"order":1,"origin":[1,2]}}}}}}]}}"#
    );
    let drawing = Drawing::from_json(&good, &Inputs::new())?;
    let shape = drawing.shapes(&bullterrier_vector::Values::new())?.remove(0);
    let attributes = shape.attributes();
    assert_eq!(
        (
            attributes.glyph(),
            attributes.char(),
            attributes.word(),
            attributes.line()
        ),
        (Some(2), Some('O'), Some(1), Some(0))
    );
    assert_eq!(
        (
            attributes.part(),
            attributes.copy(),
            attributes.level(),
            attributes.order()
        ),
        (Some(3), Some(4), Some(0.5), Some(1.0))
    );
    assert_eq!(attributes.origin().map(|point| (point.x, point.y)), Some((1.0, 2.0)));
    for (case, shape, pointer) in [
        ("no path", r#"{"attributes":{}}"#, "/root/0/shape"),
        ("an extra field", r#"{"path":"","style":1}"#, "/root/0/shape/style"),
        (
            "an attribute that is index",
            r#"{"path":"","attributes":{"index":1}}"#,
            "/root/0/shape/attributes/index",
        ),
        (
            "a glyph of 1.5",
            r#"{"path":"","attributes":{"glyph":1.5}}"#,
            "/root/0/shape/attributes/glyph",
        ),
        (
            "a negative word",
            r#"{"path":"","attributes":{"word":-1}}"#,
            "/root/0/shape/attributes/word",
        ),
        (
            "a char of two characters",
            r#"{"path":"","attributes":{"char":"ab"}}"#,
            "/root/0/shape/attributes/char",
        ),
        (
            "an origin of one number",
            r#"{"path":"","attributes":{"origin":[1]}}"#,
            "/root/0/shape/attributes/origin",
        ),
    ] {
        let text = format!(r#"{{"version":1,"root":[{{"shape":{shape}}}]}}"#);
        assert_eq!(refused(&text)?.0, pointer, "{case}");
    }
    Ok(())
}

#[test]
fn a_selection_names_attributes_and_the_values_they_must_have() -> Outcome {
    for ok in [
        r#"{"index":2}"#,
        r#"{"index":[0,2]}"#,
        r#"{"char":"O"}"#,
        r#"{"char":["O","o"]}"#,
        r#"{"origin":[1,2]}"#,
        r#"{"origin":[[1,2],[3,4]]}"#,
        r#"{"order":0.5,"level":[1,2]}"#,
        r#"{"index":[]}"#,
        r#"{"origin":[]}"#,
    ] {
        Drawing::from_json(
            &format!(r#"{{"version":1,"root":[{{"path":"{TRIANGLE}"}},{{"only":{ok}}}]}}"#),
            &Inputs::new(),
        )
        .map_err(|error| format!("{ok}: {error}"))?;
    }
    for (case, select, pointer) in [
        ("an attribute that is not one", r#"{"colour":1}"#, "/root/1/only/colour"),
        ("a char that is a number", r#"{"char":1}"#, "/root/1/only/char"),
        ("an index that is a string", r#"{"index":"1"}"#, "/root/1/only/index"),
        (
            "an index list with a string",
            r#"{"index":[1,"2"]}"#,
            "/root/1/only/index/1",
        ),
        (
            "an origin of three numbers",
            r#"{"origin":[1,2,3]}"#,
            "/root/1/only/origin",
        ),
    ] {
        let text = after_triangle(&format!(r#",{{"only":{select}}}"#));
        assert_eq!(refused(&text)?.0, pointer, "{case}");
    }
    Ok(())
}

#[test]
fn with_values_names_params_and_gives_a_number_or_a_list() -> Outcome {
    let doc = |values: &str| {
        format!(
            r#"{{"version":1,"params":[{{"name":"x","default":0,"min":0,"max":9,"unit":"","meaning":""}}],"root":[{{"path":"{TRIANGLE}"}},{{"translate":{{"dx":{{"param":"x"}},"dy":0}}}},{{"with_values":{values}}}]}}"#
        )
    };
    Drawing::from_json(&doc(r#"{"x":3}"#), &Inputs::new())?;
    Drawing::from_json(&doc(r#"{"x":[3]}"#), &Inputs::new())?;
    assert_eq!(refused(&doc(r#"{"x":"3"}"#))?.0, "/root/2/with_values/x");
    assert_eq!(refused(&doc(r#"{"x":[1,"2"]}"#))?.0, "/root/2/with_values/x/1");
    assert_eq!(refused(&doc("[]"))?.0, "/root/2/with_values");
    Ok(())
}

// Every spelling ----------------------------------------------------------------------------------------------------

#[test]
fn every_spelling_the_format_allows_reads_as_the_same_drawing() -> Outcome {
    let wanted = concat!(
        r##"{"params":[{"default":5,"max":10,"meaning":"m","min":0,"name":"x","unit":"px"}],"##,
        r##""root":[{"path":"M0 0 L10 0 L10 10 Z"},{"fill":{"color":"#ff4f9a"}},"##,
        r##"{"stroke":{"color":"#00112280","width":3}},{"scale":{"by":2}},"##,
        r##"{"translate":{"dx":{"add":[3,{"mul":[14,{"param":"x"}]}]},"dy":0.5}}],"version":1}"##
    );
    let spellings = [
        // whitespace, key order, upper case, an opaque alpha, defaults written out, `3.0`, a pair for a factor
        r##"
        {
          "root": [
            { "path": "M0,0 L10,0 L10,10 Z" },
            { "fill": { "color": "#FF4F9AFF" } },
            { "stroke": { "width": 3.0, "color": "#00112280", "join": "round", "cap": "round", "miter_limit": 4 } },
            { "scale": { "by": [2, 2] } },
            { "translate": { "dy": 5e-1, "dx": { "add": [{ "mul": [{ "param": "x" }, 14] }, 3] } } }
          ],
          "params": [ { "meaning": "m", "unit": "px", "max": 10.0, "min": -0, "default": 5, "name": "x" } ],
          "version": 1
        }"##,
        // relative path commands
        r##"{"version":1,"params":[{"name":"x","default":5,"min":0,"max":10,"unit":"px","meaning":"m"}],
           "root":[{"path":"m0 0 l10 0 v10 z"},{"fill":{"color":"#ff4f9a"}},{"stroke":{"color":"#00112280","width":3}},
           {"scale":{"by":2}},{"translate":{"dx":{"add":[3,{"mul":[14,{"param":"x"}]}]},"dy":0.5}}]}"##,
    ];
    for spelling in spellings {
        assert_eq!(canonical(spelling)?, wanted, "{spelling}");
    }
    Ok(())
}

#[test]
fn a_shared_drawing_used_once_is_written_in_place_and_one_used_twice_is_named_d1() -> Outcome {
    let once = format!(
        r#"{{"version":1,"drawings":{{"mine":[{{"path":"{TRIANGLE}"}}]}},"root":[{{"stack":{{"items":[[{{"drawing":"mine"}},{{"scale":{{"by":2}}}}]]}}}}]}}"#
    );
    assert_eq!(
        canonical(&once)?,
        format!(
            r#"{{"root":[{{"stack":{{"items":[[{{"path":"{TRIANGLE}"}},{{"scale":{{"by":2}}}}]]}}}}],"version":1}}"#
        )
    );
    let twice = format!(
        r#"{{"version":1,"drawings":{{"mine":[{{"path":"{TRIANGLE}"}}]}},"root":[{{"stack":{{"items":[[{{"drawing":"mine"}},{{"scale":{{"by":2}}}}],[{{"drawing":"mine"}},{{"scale":{{"by":3}}}}]]}}}}]}}"#
    );
    let text = canonical(&twice)?;
    assert!(
        text.starts_with(&format!(r#"{{"drawings":{{"d1":[{{"path":"{TRIANGLE}"}}]}}"#)),
        "{text}"
    );
    Ok(())
}

#[test]
fn a_param_default_at_the_end_of_a_turn_is_written_at_its_start() -> Outcome {
    let text = r#"{"version":1,"params":[{"name":"a","default":360,"min":0,"max":360,"unit":"deg","meaning":""}],"root":[{"path":"M0 0"},{"rotate":{"angle":{"param":"a"}}}]}"#;
    assert!(canonical(text)?.contains(r#""default":0"#));
    Ok(())
}
