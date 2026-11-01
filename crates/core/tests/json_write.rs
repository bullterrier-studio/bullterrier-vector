//! `Drawing::to_json`: the canonical text of formats.md for a drawing built in code. Expected texts are written by hand
//! from the format's rules (sorted keys, no whitespace, shortest numbers, defaults left out, shared drawings named in
//! the order the walk reaches them).

mod common;

use bullterrier_vector::{
    Blend, Cap, Color, Deg, Drawing, Error, Expr, Inputs, Join, Param, Path, Rotate, ScaleBy, Select, Shape, Stroke,
    Unit, Values,
};
use common::{Outcome, parameter_refusal};

const SQUARE: &str = "M0 0 L10 0 L10 10 L0 10 Z";
const PINK: Color = Color::hex(0xff_4f_9a);

fn square() -> Result<Drawing, Error> {
    Ok(Drawing::from(Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)?))
}

/// A square of side 10 whose left edge is at 50: not the same path as `square`.
fn far_square() -> Result<Drawing, Error> {
    Ok(Drawing::from(Path::rect(50.0, 0.0, 10.0, 10.0, 0.0)?))
}

const FAR_SQUARE: &str = "M50 0 L60 0 L60 10 L50 10 Z";

fn slider(name: &str) -> Result<Param, Error> {
    Param::new(name, 0.0, -100.0, 100.0)
}

/// The canonical text of a document whose root is the square followed by `steps` (JSON text of each call).
fn root_of(steps: &str) -> String {
    format!(r#"{{"root":[{{"path":"{SQUARE}"}}{steps}],"version":1}}"#)
}

/// `root_of`, for a document that declares the Param `x` (from -100 to 100, defaulting to 0, plain).
fn root_of_x(steps: &str) -> String {
    let param = r#"{"default":0,"max":100,"meaning":"","min":-100,"name":"x","unit":""}"#;
    format!(r#"{{"params":[{param}],"root":[{{"path":"{SQUARE}"}}{steps}],"version":1}}"#)
}

// The first look ----------------------------------------------------------------------------------------------------

#[test]
fn the_first_look_of_formats_md_is_written_as_the_document_shows() -> Outcome {
    let spin = Param::new("spin", 0.0, 0.0, 360.0)?
        .unit(Unit::Deg)?
        .meaning("how far the square has turned");
    let square = Drawing::from(Path::rect(150.0, 150.0, 100.0, 100.0, 0.0)?)
        .fill(PINK)
        .rotate(Rotate::new(Deg(&spin)).about((200.0, 200.0)));
    let text = square.with_size(400.0, 400.0).to_json()?;
    assert_eq!(
        text,
        concat!(
            r#"{"params":[{"default":0,"max":360,"meaning":"how far the square has turned","min":0,"name":"spin","unit":"deg"}],"#,
            r##""root":[{"stack":{"items":[[{"path":"M150 150 L250 150 L250 250 L150 250 Z"},{"fill":{"color":"#ff4f9a"}},"##,
            r#"{"rotate":{"about":[200,200],"angle":{"param":"spin"}}}]],"size":[400,400]}}],"version":1}"#
        )
    );
    Ok(())
}

#[test]
fn a_path_alone_is_the_whole_root() -> Outcome {
    assert_eq!(square()?.to_json()?, root_of(""));
    Ok(())
}

// Defaults, colours, words ------------------------------------------------------------------------------------------

#[test]
fn an_argument_equal_to_its_default_is_left_out() -> Outcome {
    let plain = square()?.stroke(PINK).to_json()?;
    assert_eq!(plain, root_of(r##",{"stroke":{"color":"#ff4f9a"}}"##));
    let same = square()?
        .stroke(
            Stroke::new(PINK)
                .width(2.0)
                .join(Join::Round)
                .cap(Cap::Round)
                .miter_limit(4.0),
        )
        .to_json()?;
    assert_eq!(same, plain);
    let different = square()?
        .stroke(
            Stroke::new(PINK)
                .width(3.5)
                .join(Join::Miter)
                .cap(Cap::Square)
                .miter_limit(2.0),
        )
        .to_json()?;
    assert_eq!(
        different,
        root_of(r##",{"stroke":{"cap":"square","color":"#ff4f9a","join":"miter","miter_limit":2,"width":3.5}}"##)
    );
    Ok(())
}

#[test]
fn a_colour_is_lower_case_and_an_opaque_alpha_is_left_out() -> Outcome {
    for (given, written) in [
        ("#FF4F9A", "#ff4f9a"),
        ("#FF4F9AFF", "#ff4f9a"),
        ("#FF4F9A80", "#ff4f9a80"),
        ("#ff4f9a00", "#ff4f9a00"),
        ("none", "none"),
    ] {
        let text = square()?.fill(Color::parse(given)?).to_json()?;
        assert_eq!(
            text,
            root_of(&format!(r#",{{"fill":{{"color":"{written}"}}}}"#)),
            "{given}"
        );
    }
    Ok(())
}

#[test]
fn a_word_argument_is_written_as_its_word() -> Outcome {
    let text = square()?.blend(Blend::Multiply).to_json()?;
    assert_eq!(text, root_of(r#",{"blend":{"mode":"multiply"}}"#));
    Ok(())
}

// Numbers -----------------------------------------------------------------------------------------------------------

#[test]
fn numbers_are_written_in_their_shortest_round_trip_form() -> Outcome {
    for (number, written) in [
        (1080.0, "1080"),
        (0.6, "0.6"),
        (-0.0, "0"),
        (0.1 + 0.2, "0.30000000000000004"),
        (1e-6, "0.000001"),
        (1e-7, "1e-7"),
        (1.5e-9, "1.5e-9"),
        (1e20, "100000000000000000000"),
        (1e21, "1e+21"),
        (-2.5e22, "-2.5e+22"),
        (123_456.789, "123456.789"),
        (f64::MIN_POSITIVE, "2.2250738585072014e-308"),
    ] {
        let text = square()?.translate(number, 0.0).to_json()?;
        assert_eq!(
            text,
            root_of(&format!(r#",{{"translate":{{"dx":{written},"dy":0}}}}"#)),
            "{number}"
        );
    }
    Ok(())
}

#[test]
fn a_whole_number_valued_float_is_written_without_a_fraction() -> Outcome {
    let text = square()?.translate(3.0, -4.0).to_json()?;
    assert_eq!(text, root_of(r#",{"translate":{"dx":3,"dy":-4}}"#));
    Ok(())
}

// Expressions -------------------------------------------------------------------------------------------------------

#[test]
fn a_literal_operand_of_add_and_mul_is_written_first() -> Outcome {
    let burst = slider("burst")?;
    let param = r#"{"param":"burst"}"#;
    let wanted = format!(r#"{{"add":[3,{{"mul":[14,{param}]}}]}}"#);
    let described = r#"{"default":0,"max":100,"meaning":"","min":-100,"name":"burst","unit":""}"#;
    for angle in [
        3.0 + 14.0 * &burst,
        &burst * 14.0 + 3.0,
        14.0 * &burst + 3.0,
        3.0 + &burst * 14.0,
    ] {
        let text = square()?.rotate(Deg(angle)).to_json()?;
        let call = format!(r#",{{"rotate":{{"angle":{wanted}}}}}"#);
        assert_eq!(
            text,
            root_of_x(&call).replace(
                r#"{"default":0,"max":100,"meaning":"","min":-100,"name":"x","unit":""}"#,
                described
            )
        );
    }
    Ok(())
}

#[test]
fn sub_and_div_keep_the_order_the_code_gives() -> Outcome {
    let x = slider("x")?;
    let param = r#"{"param":"x"}"#;
    for (angle, wanted) in [
        (10.0 - &x, format!(r#"{{"sub":[10,{param}]}}"#)),
        (&x - 10.0, format!(r#"{{"sub":[{param},10]}}"#)),
        (&x / 2.0, format!(r#"{{"div":[{param},2]}}"#)),
        (2.0 / &x, format!(r#"{{"div":[2,{param}]}}"#)),
    ] {
        let text = square()?.rotate(Deg(angle)).to_json()?;
        assert_eq!(text, root_of_x(&format!(r#",{{"rotate":{{"angle":{wanted}}}}}"#)));
    }
    Ok(())
}

#[test]
fn operands_that_are_not_literals_keep_their_order() -> Outcome {
    let (a, b) = (slider("a")?, slider("b")?);
    let text = square()?.rotate(Deg(&b + &a)).to_json()?;
    let described =
        |name: &str| format!(r#"{{"default":0,"max":100,"meaning":"","min":-100,"name":"{name}","unit":""}}"#);
    assert_eq!(
        text,
        format!(
            r#"{{"params":[{},{}],"root":[{{"path":"{SQUARE}"}},{{"rotate":{{"angle":{{"add":[{{"param":"b"}},{{"param":"a"}}]}}}}}}],"version":1}}"#,
            described("b"),
            described("a")
        )
    );
    Ok(())
}

#[test]
fn a_part_with_no_param_is_folded_to_its_value() -> Outcome {
    let x = slider("x")?;
    // (2 * 3) + x
    let folded = (Expr::from(2.0) * 3.0) + &x;
    let text = square()?.rotate(Deg(folded)).to_json()?;
    assert_eq!(text, root_of_x(r#",{"rotate":{"angle":{"add":[6,{"param":"x"}]}}}"#));
    // -(2 + 1) * x
    let negated = -(Expr::from(2.0) + 1.0) * &x;
    let text = square()?.rotate(Deg(negated)).to_json()?;
    assert_eq!(text, root_of_x(r#",{"rotate":{"angle":{"mul":[-3,{"param":"x"}]}}}"#));
    Ok(())
}

#[test]
fn drawings_whose_expressions_are_written_alike_are_one_shared_drawing() -> Outcome {
    // two steps that canonical form writes as one text are one call, so the square they build on is shared by the
    // merged step and the text reads back to itself
    let x = slider("x")?;
    let described = r#"{"default":0,"max":100,"meaning":"","min":-100,"name":"x","unit":""}"#;
    let param = r#"{"param":"x"}"#;
    let pairs = [
        (
            "operands of mul in either order",
            14.0 * &x,
            &x * 14.0,
            format!(r#"{{"mul":[14,{param}]}}"#),
        ),
        (
            "operands of add in either order",
            3.0 + &x,
            &x + 3.0,
            format!(r#"{{"add":[3,{param}]}}"#),
        ),
        (
            "a constant part and its value",
            &x * (Expr::from(2.0) + 3.0),
            &x * Expr::from(5.0),
            format!(r#"{{"mul":[5,{param}]}}"#),
        ),
        (
            "both at once, nested",
            3.0 + 14.0 * &x,
            &x * 14.0 + 3.0,
            format!(r#"{{"add":[3,{{"mul":[14,{param}]}}]}}"#),
        ),
    ];
    for (case, first, second, wanted) in pairs {
        let piece = Drawing::new([square()?.translate(first, 0.0), square()?.translate(second, 0.0)]);
        let text = piece.to_json()?;
        assert_eq!(
            text,
            format!(
                concat!(
                    r#"{{"drawings":{{"d1":[{{"path":"{p}"}},{{"translate":{{"dx":{w},"dy":0}}}}]}},"params":[{d}],"#,
                    r#""root":[{{"stack":{{"items":[[{{"drawing":"d1"}}],[{{"drawing":"d1"}}]]}}}}],"version":1}}"#
                ),
                p = SQUARE,
                w = wanted,
                d = described
            ),
            "{case}"
        );
        assert_eq!(Drawing::from_json(&text, &Inputs::new())?.to_json()?, text, "{case}");
    }
    Ok(())
}

#[test]
fn expressions_written_differently_stay_two_calls() -> Outcome {
    // subtraction and division do not commute: `3 - x` and `x - 3` are two steps on one shared square
    let x = slider("x")?;
    let piece = Drawing::new([square()?.translate(3.0 - &x, 0.0), square()?.translate(&x - 3.0, 0.0)]);
    let text = piece.to_json()?;
    assert!(
        text.contains(r#"{"sub":[3,{"param":"x"}]}"#) && text.contains(r#"{"sub":[{"param":"x"},3]}"#),
        "{text}"
    );
    assert_eq!(text.matches(r#""drawing":"d1""#).count(), 2, "{text}");
    assert!(!text.contains("d2"), "{text}");
    assert_eq!(Drawing::from_json(&text, &Inputs::new())?.to_json()?, text);
    Ok(())
}

#[test]
fn a_negation_of_a_param_is_a_neg() -> Outcome {
    let x = slider("x")?;
    let text = square()?.rotate(Deg(-&x)).to_json()?;
    assert_eq!(text, root_of_x(r#",{"rotate":{"angle":{"neg":{"param":"x"}}}}"#));
    Ok(())
}

#[test]
fn a_constant_part_that_has_no_value_is_refused() -> Outcome {
    let x = slider("x")?;
    let never = &x * (Expr::from(1.0) / 0.0);
    let (parameter, message) = parameter_refusal(square()?.rotate(Deg(never)).to_json())?;
    assert_eq!(parameter, "expression");
    assert!(message.contains("division by zero"), "{message}");
    Ok(())
}

#[test]
fn a_literal_divisor_of_zero_cannot_be_written() -> Outcome {
    let x = slider("x")?;
    // `x / 0` always fails to draw, and the format has no spelling for it (a literal zero divisor fails the schema)
    for divisor in [Expr::from(0.0), Expr::from(2.0) - 2.0] {
        let (parameter, message) = parameter_refusal(square()?.rotate(Deg(&x / divisor)).to_json())?;
        assert_eq!(parameter, "expression");
        assert!(message.contains("divides by a literal 0"), "{message}");
    }
    Ok(())
}

// Params ------------------------------------------------------------------------------------------------------------

#[test]
fn params_are_listed_once_in_the_order_the_walk_first_reads_them() -> Outcome {
    let (b, a) = (slider("b")?, slider("a")?);
    let drawing = square()?.translate(&b, &a).translate(&a, &b);
    let text = drawing.to_json()?;
    let listed = |name: &str| format!(r#"{{"default":0,"max":100,"meaning":"","min":-100,"name":"{name}","unit":""}}"#);
    assert!(
        text.starts_with(&format!(r#"{{"params":[{},{}],"root""#, listed("b"), listed("a"))),
        "{text}"
    );
    Ok(())
}

#[test]
fn a_param_fixed_by_with_values_is_still_listed() -> Outcome {
    let x = slider("x")?;
    let drawing = square()?.translate(&x, 0.0).with_values(&Values::new().set("x", 5.0));
    let text = drawing.to_json()?;
    assert!(
        text.contains(r#""params":[{"default":0,"max":100,"meaning":"","min":-100,"name":"x","unit":""}]"#),
        "{text}"
    );
    assert!(text.contains(r#"{"with_values":{"x":5}}"#), "{text}");
    Ok(())
}

#[test]
fn the_meaning_of_a_param_is_escaped_as_json_requires() -> Outcome {
    let x = slider("x")?.meaning("a \"quote\", a \\, a\ttab and \u{1} é, \u{7f} \u{2028} \u{1f600}, \u{8}\u{c}\r\n");
    let text = square()?.translate(&x, 0.0).to_json()?;
    // only the quote, the backslash and the controls below a space are escaped; the rest is written as it is
    let wanted =
        "\"meaning\":\"a \\\"quote\\\", a \\\\, a\\ttab and \\u0001 é, \u{7f} \u{2028} \u{1f600}, \\b\\f\\r\\n\"";
    assert!(text.contains(wanted), "{text}");
    Ok(())
}

// Calls with a shape of their own -----------------------------------------------------------------------------------

#[test]
fn scale_writes_one_number_when_both_directions_are_equal() -> Outcome {
    for (by, wanted) in [
        (ScaleBy::new(2.0), r#"{"by":2}"#),
        (ScaleBy::new((2.0, 2.0)), r#"{"by":2}"#),
        (ScaleBy::new((2.0, 3.0)), r#"{"by":[2,3]}"#),
        (ScaleBy::new(-1.0).about((5.0, 6.0)), r#"{"about":[5,6],"by":-1}"#),
    ] {
        let text = square()?.scale(by).to_json()?;
        assert_eq!(text, root_of(&format!(r#",{{"scale":{wanted}}}"#)));
    }
    Ok(())
}

#[test]
fn scale_writes_one_number_when_both_directions_are_written_alike() -> Outcome {
    // 1e-7 / 1e-7 and 2 - 1 are not the same expression, but both are the number 1; 3 + x and x + 3 are one expression
    let x = slider("x")?;
    let param = r#"{"param":"x"}"#;
    for (case, by, wanted) in [
        (
            "two constants with one value",
            ScaleBy::new((Expr::from(1e-7) / 1e-7, Expr::from(2.0) - 1.0)),
            r#"{"by":1}"#.to_owned(),
        ),
        (
            "two spellings of one expression",
            ScaleBy::new((3.0 + &x, &x + 3.0)),
            format!(r#"{{"by":{{"add":[3,{param}]}}}}"#),
        ),
        (
            "two expressions that differ",
            ScaleBy::new((3.0 + &x, &x + 4.0)),
            format!(r#"{{"by":[{{"add":[3,{param}]}},{{"add":[4,{param}]}}]}}"#),
        ),
    ] {
        let text = square()?.scale(by).to_json()?;
        assert!(text.contains(&format!(r#"{{"scale":{wanted}}}"#)), "{case}: {text}");
        assert_eq!(Drawing::from_json(&text, &Inputs::new())?.to_json()?, text, "{case}");
    }
    Ok(())
}

#[test]
fn only_and_without_write_their_attributes() -> Outcome {
    let four = Drawing::new([square()?, square()?, square()?, square()?]);
    let text = four
        .only(Select::new().attribute("index", [0_u32, 2]).attribute("order", 0.5))
        .to_json()?;
    assert!(text.contains(r#"{"only":{"index":[0,2],"order":0.5}}"#), "{text}");
    let text = four.without(Select::new().attribute("index", 3_u32)).to_json()?;
    assert!(text.contains(r#"{"without":{"index":3}}"#), "{text}");
    let text = four.without(Select::new().attribute("char", 'O')).to_json()?;
    assert!(text.contains(r#"{"without":{"char":"O"}}"#), "{text}");
    let text = four
        .only(Select::new().attribute("origin", (1.0, 2.0).into_point()))
        .to_json()?;
    assert!(text.contains(r#"{"only":{"origin":[1,2]}}"#), "{text}");
    Ok(())
}

trait IntoPoint {
    fn into_point(self) -> bullterrier_vector::Point;
}

impl IntoPoint for (f64, f64) {
    fn into_point(self) -> bullterrier_vector::Point {
        bullterrier_vector::Point::new(self.0, self.1)
    }
}

// Stacks ------------------------------------------------------------------------------------------------------------

#[test]
fn a_stack_writes_its_items_bottom_first_and_its_size() -> Outcome {
    let stack = Drawing::new([square()?, far_square()?.translate(5.0, 0.0)]).with_size(100.0, 50.0);
    let text = stack.to_json()?;
    assert_eq!(
        text,
        format!(
            r#"{{"root":[{{"stack":{{"items":[[{{"path":"{SQUARE}"}}],[{{"path":"{FAR_SQUARE}"}},{{"translate":{{"dx":5,"dy":0}}}}]],"size":[100,50]}}}}],"version":1}}"#
        )
    );
    Ok(())
}

#[test]
fn a_drawing_of_one_item_and_no_size_is_the_item_itself() -> Outcome {
    assert_eq!(Drawing::new([square()?]).to_json()?, square()?.to_json()?);
    Ok(())
}

#[test]
fn a_drawing_of_one_item_with_a_size_is_a_stack_of_it() -> Outcome {
    let text = square()?.with_size(20.0, 20.0).to_json()?;
    assert_eq!(
        text,
        format!(r#"{{"root":[{{"stack":{{"items":[[{{"path":"{SQUARE}"}}]],"size":[20,20]}}}}],"version":1}}"#)
    );
    Ok(())
}

// Shared drawings ---------------------------------------------------------------------------------------------------

#[test]
fn a_prefix_two_calls_build_on_is_written_once_and_named_in_walk_order() -> Outcome {
    // formats.md "Sharing a prefix of a chain", with a path where the text was
    let a = square()?.scale(2.0);
    let b = a.rotate(Deg(10.0));
    let red = Color::hex(0xe2_41_2e);
    let blue = Color::hex(0x2a_5b_ff);
    let ink = Color::hex(0x15_16_1a);
    let piece = Drawing::new([a.fill(red), b.fill(blue), b.stroke(ink)]);
    assert_eq!(
        piece.to_json()?,
        format!(
            concat!(
                r#"{{"drawings":{{"d1":[{{"path":"{p}"}},{{"scale":{{"by":2}}}}],"#,
                r#""d2":[{{"drawing":"d1"}},{{"rotate":{{"angle":10}}}}]}},"#,
                r##""root":[{{"stack":{{"items":[[{{"drawing":"d1"}},{{"fill":{{"color":"#e2412e"}}}}],"##,
                r##"[{{"drawing":"d2"}},{{"fill":{{"color":"#2a5bff"}}}}],"##,
                r##"[{{"drawing":"d2"}},{{"stroke":{{"color":"#15161a"}}}}]]}}}}],"version":1}}"##
            ),
            p = SQUARE
        )
    );
    Ok(())
}

#[test]
fn a_drawing_used_once_is_written_in_place_however_often_it_is_named_in_code() -> Outcome {
    let once = square()?.scale(2.0);
    let piece = Drawing::new([once.fill(PINK), far_square()?.translate(1.0, 1.0)]);
    let text = piece.to_json()?;
    assert!(!text.contains("drawings"), "{text}");
    Ok(())
}

#[test]
fn equal_drawings_built_apart_are_one_shared_drawing() -> Outcome {
    let apart = |_: usize| -> Result<Drawing, Error> { Ok(square()?.scale(2.0).fill(PINK)) };
    let piece = Drawing::new([apart(0)?, apart(1)?]);
    assert_eq!(
        piece.to_json()?,
        format!(
            concat!(
                r##"{{"drawings":{{"d1":[{{"path":"{p}"}},{{"scale":{{"by":2}}}},{{"fill":{{"color":"#ff4f9a"}}}}]}},"##,
                r#""root":[{{"stack":{{"items":[[{{"drawing":"d1"}}],[{{"drawing":"d1"}}]]}}}}],"version":1}}"#
            ),
            p = SQUARE
        )
    );
    Ok(())
}

#[test]
fn a_shared_drawing_that_takes_another_names_the_inner_one_first() -> Outcome {
    // `inner` is taken by `outer`'s stack; `outer` is used twice, `inner` once
    let inner = square()?.scale(3.0);
    let outer = Drawing::new([inner.clone(), far_square()?]).with_size(10.0, 10.0);
    let piece = Drawing::new([outer.fill(PINK), outer.rotate(Deg(5.0))]);
    let text = piece.to_json()?;
    assert!(
        text.starts_with(
            r#"{"drawings":{"d1":[{"stack":{"items":[[{"path":"M0 0 L10 0 L10 10 L0 10 Z"},{"scale":{"by":3}}]"#
        ),
        "{text}"
    );
    assert!(!text.contains("d2"), "{text}");
    Ok(())
}

// A shape put back --------------------------------------------------------------------------------------------------

fn only_shape(drawing: &Drawing) -> Result<Shape, Box<dyn std::error::Error>> {
    drawing.shapes(&Values::new())?.pop().ok_or_else(|| "no shape".into())
}

#[test]
fn a_shape_is_written_with_its_style_as_the_steps_that_set_it() -> Outcome {
    let painted = only_shape(
        &square()?
            .fill(PINK)
            .stroke(Stroke::new(Color::hex(0)).width(4.0))
            .opacity(0.5)
            .blend(Blend::Multiply),
    )?;
    let text = Drawing::from(painted).to_json()?;
    assert_eq!(
        text,
        format!(
            concat!(
                r##"{{"root":[{{"shape":{{"path":"{p}"}}}},{{"fill":{{"color":"#ff4f9a"}}}},"##,
                r##"{{"stroke":{{"color":"#000000","width":4}}}},{{"opacity":{{"alpha":0.5}}}},"##,
                r#"{{"blend":{{"mode":"multiply"}}}}],"version":1}}"#
            ),
            p = SQUARE
        )
    );
    Ok(())
}

#[test]
fn shapes_put_back_that_differ_only_in_style_share_the_shape_call() -> Outcome {
    // the `shape` call holds the path and attributes; the style is the steps after it, so the black shape and the
    // shape with no fill build on one shared call, and the black one, used twice, is shared too
    let black = Drawing::from(only_shape(&square()?)?);
    let unfilled = Drawing::from(only_shape(&square()?.fill(Color::NONE))?);
    let piece = Drawing::new([black.clone(), unfilled, black]);
    let text = piece.to_json()?;
    assert_eq!(
        text,
        format!(
            concat!(
                r##"{{"drawings":{{"d1":[{{"shape":{{"path":"{p}"}}}}],"##,
                r##""d2":[{{"drawing":"d1"}},{{"fill":{{"color":"#000000"}}}}]}},"##,
                r#""root":[{{"stack":{{"items":[[{{"drawing":"d2"}}],"#,
                r#"[{{"drawing":"d1"}},{{"fill":{{"color":"none"}}}}],[{{"drawing":"d2"}}]]}}}}],"version":1}}"#
            ),
            p = SQUARE
        )
    );
    assert_eq!(Drawing::from_json(&text, &Inputs::new())?.to_json()?, text);
    Ok(())
}

#[test]
fn shapes_put_back_from_different_places_in_a_list_are_one_call() -> Outcome {
    // the shape at index 1 of a stack and the same shape alone at index 0 write one text, so they are one call
    let stack = Drawing::new([square()?, far_square()?]);
    let second = only_shape(&stack.only(Select::new().attribute("index", 1_u32)))?;
    let from_stack = Drawing::from(stack.shapes(&Values::new())?.remove(1));
    let alone = Drawing::from(second);
    let piece = Drawing::new([from_stack.rotate(Deg(5.0)), alone.rotate(Deg(5.0))]);
    let text = piece.to_json()?;
    assert_eq!(text.matches(r#""shape":"#).count(), 1, "{text}");
    assert_eq!(Drawing::from_json(&text, &Inputs::new())?.to_json()?, text);
    Ok(())
}

#[test]
fn a_shape_with_no_fill_writes_a_fill_of_none() -> Outcome {
    let line = Path::line((0.0, 0.0), (10.0, 0.0))?;
    let shape = only_shape(&Drawing::from(line).stroke(PINK))?;
    let text = Drawing::from(shape).to_json()?;
    assert!(
        text.contains(r##"{"fill":{"color":"none"}},{"stroke":{"color":"#ff4f9a"}}"##),
        "{text}"
    );
    Ok(())
}

#[test]
fn a_shape_writes_the_attributes_its_generators_wrote() -> Outcome {
    let two = Drawing::new([square()?, square()?]);
    let shape = two.shapes(&Values::new())?.pop().ok_or("no shape")?;
    let text = Drawing::from(shape).to_json()?;
    assert!(text.contains(r#"{"shape":{"attributes":{"order":1},"path":"#), "{text}");
    Ok(())
}

// Errors and limits -------------------------------------------------------------------------------------------------

#[test]
fn a_drawing_that_holds_an_error_is_refused_with_it() -> Outcome {
    let broken = square()?.scale(0.0);
    let held = broken.error().cloned().ok_or("no error")?;
    assert_eq!(broken.to_json(), Err(held));
    Ok(())
}

#[test]
fn a_drawing_past_max_calls_is_refused() -> Outcome {
    let mut drawing = square()?;
    for _ in 0..100_001 {
        drawing = drawing.translate(1.0, 1.0);
    }
    let (parameter, message) = parameter_refusal(drawing.to_json())?;
    assert_eq!(parameter, "to_json");
    assert!(message.contains("MAX_CALLS 100000"), "{message}");
    Ok(())
}

#[test]
fn a_drawing_nested_past_max_depth_is_refused() -> Outcome {
    let nested = |levels: usize| -> Result<Drawing, Error> {
        let mut drawing = square()?;
        for _ in 0..levels {
            drawing = Drawing::new([drawing, square()?]);
        }
        Ok(drawing)
    };
    assert!(nested(20)?.to_json().is_ok());
    let (parameter, message) = parameter_refusal(nested(40)?.to_json())?;
    assert_eq!(parameter, "to_json");
    assert!(message.contains("MAX_DEPTH 128"), "{message}");
    Ok(())
}
