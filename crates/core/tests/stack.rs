//! Own binary: it runs on the smallest thread stack the library supports and is built unoptimised (`profile.stack` in
//! Cargo.toml, run by `poe rust-test`): an unoptimised frame is the largest one, so the bound holds for every build.
//!
//! The deepest structures a caller can build or load, used on a 256 KB stack (docs/design-decisions.md, "Bounded
//! depth"). An overflow aborts the process, so a test here that fails is a crash. The Python counterpart on the
//! shipped release build is `tests/test_stack_depth.py`.

#[path = "it/common/mod.rs"]
mod common;

use bullterrier_vector::{
    Branch, Color, Drawing, Error, Expr, Field, Flow, Hatch, Inputs, Mesh, Param, Path, Random, Size, Stagger, Text,
    Values, by_index, by_order, text,
};
use common::fonts::synthetic;
use common::{Outcome, parameter_refusal};

/// The smallest thread stack the library supports, in bytes.
const SMALLEST_STACK: usize = 256 * 1024;

/// The value of `expr` with the Params at their defaults.
fn value(expr: &Expr) -> Result<f64, Error> {
    expr.value(&Values::new())
}

#[test]
fn a_million_level_expression_is_printed_compared_and_dropped_on_the_smallest_stack() -> Outcome {
    // a loop builds it, since operators never fail; nothing a caller can do to it may overflow a small stack
    let handle = std::thread::Builder::new().stack_size(SMALLEST_STACK).spawn(|| {
        let run = || -> Outcome {
            let x = Param::new("x", 1.0, -1000.0, 1000.0)?;
            let (mut sum, mut same, mut negated) = (Expr::from(&x), Expr::from(&x), Expr::from(1.0));
            for _ in 0..1_000_000 {
                sum = sum + &x;
                same = same + &x;
                negated = -negated;
            }
            let clone = sum.clone();
            assert_eq!(sum, clone);
            assert_eq!(sum, same, "built the same way");
            assert_ne!(sum, negated);
            // the Param is 1 level and each of the million additions one more
            assert_eq!(sum.to_string(), "(an expression 1000001 levels deep)");
            assert!(format!("{sum:?}").len() < 1000);
            parameter_refusal(value(&sum))?;
            drop(sum);
            drop(same);
            drop(negated);
            parameter_refusal(value(&clone))?;
            Ok(())
        };
        run().map_err(|error| error.to_string())
    })?;
    handle.join().map_err(|_| "the thread panicked")??;
    Ok(())
}

#[test]
fn a_million_level_per_item_expression_is_refused_printed_and_dropped_on_the_smallest_stack() -> Outcome {
    // each per-item number nests the one before as an operand, so the deepest structure with the new node is built
    // by a loop that never fails; nothing a caller can do to it may overflow a small stack
    let handle = std::thread::Builder::new().stack_size(SMALLEST_STACK).spawn(|| {
        let run = || -> Outcome {
            let x = Param::new("x", 1.0, -1000.0, 1000.0)?;
            let mut nested: [Expr; 4] = [Expr::from(&x), Expr::from(&x), Expr::from(&x), Expr::from(&x)];
            for _ in 0..250_000 {
                let [index, order, random, stagger] = nested;
                nested = [
                    by_index(index, 1.0),
                    by_order(0.0, order),
                    Random::new(0.0, random).seed(3.0).into(),
                    Stagger::new(stagger).spread(0.5).into(),
                ];
            }
            let clones = nested.clone();
            for deep in &nested {
                assert!(deep.to_string().starts_with("(an expression "), "{deep}");
                assert!(format!("{deep:?}").len() < 1000);
                assert_eq!(deep.constant(), None);
                parameter_refusal(value(deep))?;
            }
            drop(nested);
            for deep in &clones {
                parameter_refusal(value(deep))?;
            }
            Ok(())
        };
        run().map_err(|error| error.to_string())
    })?;
    handle.join().map_err(|_| "the thread panicked")??;
    Ok(())
}

#[test]
fn a_glyph_of_the_deepest_composites_the_reader_draws_is_shaped_and_drawn_on_the_smallest_stack() -> Outcome {
    let handle = std::thread::Builder::new().stack_size(SMALLEST_STACK).spawn(|| {
        let run = || -> Outcome {
            let drawing = text("AAA", Text::new(&synthetic()?).size(100.0)).fill(Color::hex(0x00_00_00));
            let shapes = drawing.shapes(&Values::new())?;
            assert_eq!(shapes.len(), 3);
            assert!(
                drawing
                    .to_svg(Size::px(400.0, 200.0), &Values::new())?
                    .contains("<path")
            );
            drop(drawing);
            Ok(())
        };
        run().map_err(|error| error.to_string())
    })?;
    handle.join().map_err(|_| "the thread panicked")??;
    Ok(())
}

#[test]
fn an_svg_of_the_deepest_nesting_and_one_nested_a_hundred_thousand_levels_are_read_on_the_smallest_stack() -> Outcome {
    let handle = std::thread::Builder::new().stack_size(SMALLEST_STACK).spawn(|| {
        let run = || -> Outcome {
            let nested = |groups: usize| {
                format!(
                    r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 9 9">{}<rect width="1" height="1"/>{}</svg>"#,
                    "<g>".repeat(groups),
                    "</g>".repeat(groups)
                )
            };
            // the root and the rect are two of the 128 levels
            let drawing = Drawing::from_svg(&nested(126))?;
            assert_eq!(drawing.shapes(&Values::new())?.len(), 1);
            parameter_refusal(Drawing::from_svg(&nested(127)))?;
            // far past anything a reader accepts: refused, not an overflow
            parameter_refusal(Drawing::from_svg(&nested(100_000)))?;
            drop(drawing);
            Ok(())
        };
        run().map_err(|error| error.to_string())
    })?;
    handle.join().map_err(|_| "the thread panicked")??;
    Ok(())
}

/// `MAX_DEPTH`: the most drawings nested, a lone drawing counting as 1; also the deepest expression.
const DEEPEST: usize = 16;

/// The leaf of every nesting: a circle moved by `x`.
fn leaf(x: &Expr) -> Result<Drawing, Error> {
    Ok(Path::circle((5.0, 5.0), 2.0)?
        .fill(Color::hex(0x11_22_33))
        .translate(x, 0.0))
}

fn ink() -> Result<Drawing, Error> {
    Ok(Path::circle((5.0, 5.0), 4.0)?.fill(Color::hex(0x44_55_66)))
}

/// `levels` drawings on top of the leaf, each made by `step` from `ink` and the drawing below it.
fn nested(
    x: &Expr,
    levels: usize,
    step: impl FnMut(usize, &Drawing, Drawing) -> Result<Drawing, Error>,
) -> Result<Drawing, Error> {
    nested_over(leaf(x)?, levels, step)
}

/// Like [`nested`], over `bottom`.
fn nested_over(
    bottom: Drawing,
    levels: usize,
    mut step: impl FnMut(usize, &Drawing, Drawing) -> Result<Drawing, Error>,
) -> Result<Drawing, Error> {
    let mut drawing = bottom;
    for level in 0..levels {
        drawing = step(level, &ink()?, drawing)?;
    }
    Ok(drawing)
}

/// Everything a caller does with the drawing `build` makes, on the smallest stack: draw it (as it is, with a value and as
/// a mesh unless `mesh_refused`), print it, list its Params, save it (a nesting past what the format holds is refused,
/// which must not overflow either), load what was saved and draw that, and drop it.
fn deepest_drawing_on_the_smallest_stack(
    mesh_refused: bool,
    build: impl FnOnce(&Expr) -> Result<Drawing, Error> + Send + 'static,
) -> Outcome {
    let handle = std::thread::Builder::new().stack_size(SMALLEST_STACK).spawn(move || {
        let run = || -> Result<(), Error> {
            let x = Expr::from(&Param::new("x", 1.0, 0.0, 10.0)?);
            let drawing = build(&x)?;
            let (size, values) = (Size::px(10.0, 10.0), Values::new());
            drawing.shapes(&values)?;
            drawing.to_svg(size, &values)?;
            drawing.to_rgba(size, &values)?;
            drawing.to_rgba(size, &Values::new().set("x", 3.0))?;
            drawing
                .with_values(&Values::new().set("x", 2.0))
                .to_rgba(size, &values)?;
            let mesh = drawing.to_mesh(size, &values, &mut Mesh::new());
            assert_eq!(mesh_refused, matches!(mesh, Err(Error::Parameter { .. })), "{mesh:?}");
            drawing.parameters()?;
            assert_ne!(format!("{drawing:?}"), "");
            if let Ok(text) = drawing.to_json() {
                Drawing::from_json(&text, &Inputs::new())?.to_rgba(size, &values)?;
            }
            drop(drawing);
            Ok(())
        };
        run().map_err(|error| error.to_string())
    })?;
    handle.join().map_err(|_| "the thread panicked")??;
    Ok(())
}

#[test]
fn drawings_nested_to_the_limit_in_a_stack_are_drawn_and_dropped_on_the_smallest_stack() -> Outcome {
    deepest_drawing_on_the_smallest_stack(false, |x| {
        nested(x, DEEPEST - 1, |_, ink, drawing| {
            Ok(Drawing::new([drawing, ink.clone()]))
        })
    })
}

#[test]
fn drawings_nested_through_the_other_of_a_boolean_step_are_drawn_and_dropped_on_the_smallest_stack() -> Outcome {
    deepest_drawing_on_the_smallest_stack(false, |x| {
        nested(x, DEEPEST - 1, |level, ink, drawing| {
            Ok(match level % 4 {
                0 => ink.union(drawing, 0.01),
                1 => ink.subtract(drawing, 0.01),
                2 => ink.intersect(drawing, 0.01),
                _ => ink.exclude(drawing, 0.01),
            })
        })
    })
}

#[test]
fn drawings_nested_through_the_field_and_flow_of_a_hatch_are_drawn_and_dropped_on_the_smallest_stack() -> Outcome {
    // each level hatches by the distance field and along the contour of the level below: drawings inside values
    deepest_drawing_on_the_smallest_stack(false, |x| {
        nested(x, DEEPEST - 1, |_, ink, drawing| {
            let hatch =
                Hatch::between(4.0, 5.0, Field::distance(drawing.clone(), false)?).flow(Flow::contour(drawing)?);
            Ok(ink.hatch(&hatch, None))
        })
    })
}

#[test]
fn drawings_nested_through_the_target_of_a_morph_step_are_drawn_and_dropped_on_the_smallest_stack() -> Outcome {
    deepest_drawing_on_the_smallest_stack(false, |x| {
        nested(x, DEEPEST - 1, |_, ink, drawing| Ok(ink.morph(drawing, 0.5)))
    })
}

#[test]
fn clips_nested_through_the_region_are_drawn_and_dropped_on_the_smallest_stack() -> Outcome {
    deepest_drawing_on_the_smallest_stack(true, |x| {
        nested(x, DEEPEST - 1, |_, ink, drawing| Ok(ink.clip(drawing)))
    })
}

#[test]
fn masks_nested_through_the_source_are_drawn_and_dropped_on_the_smallest_stack() -> Outcome {
    // a mask's source may be clipped but not masked, so each level is a mask and a clip: two drawings
    deepest_drawing_on_the_smallest_stack(true, |x| {
        nested(x, (DEEPEST - 1) / 2, |_, ink, drawing| {
            let source = Path::circle((5.0, 5.0), 3.0)?
                .fill(Color::hex(0xff_ff_ff))
                .clip(drawing);
            Ok(ink.mask(source))
        })
    })
}

#[test]
fn drawings_nested_to_the_limit_over_the_deepest_expression_are_drawn_and_dropped_on_the_smallest_stack() -> Outcome {
    deepest_drawing_on_the_smallest_stack(false, |x| {
        let mut deepest = x.clone();
        for _ in 0..DEEPEST - 1 {
            deepest = deepest + 1.0;
        }
        nested(&deepest, DEEPEST - 1, |_, ink, drawing| {
            Ok(Drawing::new([drawing, ink.clone()]))
        })
    })
}

#[test]
fn drawings_nested_to_the_limit_over_the_deepest_glyph_are_drawn_and_dropped_on_the_smallest_stack() -> Outcome {
    deepest_drawing_on_the_smallest_stack(false, |x| {
        let glyphs = text("AAA", Text::new(&synthetic()?).size(100.0))
            .fill(Color::hex(0x00_00_00))
            .translate(x, 0.0);
        nested_over(glyphs, DEEPEST - 1, |_, ink, drawing| {
            Ok(Drawing::new([drawing, ink.clone()]))
        })
    })
}

/// The deepest document of drawings nested over a leaf that reads `x`, loaded, drawn and dropped on the smallest stack.
fn deepest_document_on_the_smallest_stack(build_x: impl FnOnce(Expr) -> Expr + Send + 'static) -> Outcome {
    let handle = std::thread::Builder::new().stack_size(SMALLEST_STACK).spawn(move || {
        let run = || -> Result<(), Error> {
            let x = build_x(Expr::from(&Param::new("x", 1.0, 0.0, 10.0)?));
            // the deepest nesting `to_json` accepts: a stack takes about four levels of the document
            let text = (1..DEEPEST)
                .rev()
                .find_map(|levels| {
                    nested(&x, levels, |_, ink, drawing| Ok(Drawing::new([drawing, ink.clone()])))
                        .ok()?
                        .to_json()
                        .ok()
                })
                .ok_or_else(|| Error::Parameter {
                    parameter: "to_json".into(),
                    message: "no nesting is saved".into(),
                })?;
            let drawing = Drawing::from_json(&text, &Inputs::new())?;
            let (size, values) = (Size::px(10.0, 10.0), Values::new());
            drawing.to_svg(size, &values)?;
            drawing.to_rgba(size, &Values::new().set("x", 3.0))?;
            drop(drawing);
            Ok(())
        };
        run().map_err(|error| error.to_string())
    })?;
    handle.join().map_err(|_| "the thread panicked")??;
    Ok(())
}

#[test]
fn the_deepest_document_is_loaded_drawn_and_dropped_on_the_smallest_stack() -> Outcome {
    deepest_document_on_the_smallest_stack(|x| x)
}

#[test]
fn the_deepest_document_over_the_deepest_expression_is_loaded_drawn_and_dropped_on_the_smallest_stack() -> Outcome {
    deepest_document_on_the_smallest_stack(|x| (1..DEEPEST).fold(x, |deepest, _| deepest + 1.0))
}

#[test]
fn arithmetic_on_fields_to_the_deepest_nesting_is_read_written_and_hatched_on_the_smallest_stack() -> Outcome {
    let handle = std::thread::Builder::new().stack_size(SMALLEST_STACK).spawn(|| {
        let run = || -> Outcome {
            // 16 operations deep over the distance to a circle, the most the library builds, noise of 8 octaves beside
            let mut tone = Field::distance(Path::circle((20.0, 20.0), 10.0)?, false)?;
            for level in 0..16 {
                let noise = Field::noise(40.0, 0.0, f64::from(level), 8)?;
                tone = match level % 3 {
                    0 => tone.max(&noise)?,
                    1 => tone.add(&noise)?,
                    _ => tone.abs()?,
                };
            }
            parameter_refusal(tone.abs())?;
            let square = Path::rect(0.0, 0.0, 40.0, 40.0, 0.0)?;
            let drawing = Drawing::from(square).hatch(&Hatch::between(4.0, 8.0, tone.clone()), None);
            let document = drawing.to_json()?;
            let back = Drawing::from_json(&document, &Inputs::new())?;
            assert_eq!(back.to_json()?, document);
            assert_ne!(back.shapes(&Values::new())?.len(), 0);
            assert!(tone.at((10.0, 10.0)).is_finite());
            drop(tone);
            Ok(())
        };
        run().map_err(|error| error.to_string())
    })?;
    handle.join().map_err(|_| "the thread panicked")??;
    Ok(())
}

#[test]
fn a_flow_mixed_and_turned_to_the_deepest_nesting_is_read_written_and_hatched_on_the_smallest_stack() -> Outcome {
    let handle = std::thread::Builder::new().stack_size(SMALLEST_STACK).spawn(|| {
        let run = || -> Outcome {
            // 16 mixes and turns deep, a mix and a turn by turns, one leaf added at each mix: the most the library builds
            let mut flow = Flow::angle(0.0)?;
            for level in 1..=16 {
                flow = if level % 2 == 0 {
                    flow.rotate(f64::from(level) * 3.0)?
                } else {
                    flow.mix(&Flow::angle(f64::from(level) * 7.0)?, 0.5)?
                };
            }
            parameter_refusal(flow.mix(&Flow::angle(1.0)?, 0.5))?;
            parameter_refusal(flow.rotate(1.0))?;
            let square = Path::rect(0.0, 0.0, 40.0, 40.0, 0.0)?;
            let drawing = Drawing::from(square).hatch(&Hatch::new(8.0).flow(flow.clone()), None);
            let document = drawing.to_json()?;
            let back = Drawing::from_json(&document, &Inputs::new())?;
            assert_eq!(back.to_json()?, document);
            parameter_refusal(back.shapes(&Values::new())).map_or_else(|_| (), |_| ());
            let lines = back.shapes(&Values::new())?;
            assert!(lines.iter().any(|shape| shape.path().to_svg_path().len() > 1));
            assert!(flow.at((10.0, 10.0)) < 180.0);
            drop(flow);
            Ok(())
        };
        run().map_err(|error| error.to_string())
    })?;
    handle.join().map_err(|_| "the thread panicked")??;
    Ok(())
}

#[test]
fn a_branch_with_the_most_forks_its_bound_allows_is_grown_written_and_read_on_the_smallest_stack() -> Outcome {
    let handle = std::thread::Builder::new().stack_size(SMALLEST_STACK).spawn(|| {
        let run = || -> Outcome {
            // one root, a fork in each of 40 stretches: 2^40 tips is refused by the bound before anything grows; 12 forks
            // in 12 stretches grow 2^12 tips, each a chain of 12 forks walked back to its root without recursion
            let square = Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)?;
            parameter_refusal(
                Drawing::from(square.clone())
                    .branch(&Branch::new(1000.0, 40.0).fork_spacing(1.0).forks(40.0).wavelength(48.0))
                    .shapes(&Values::new()),
            )?;
            let drawing =
                Drawing::from(square).branch(&Branch::new(1000.0, 12.0).fork_spacing(1.0).forks(12.0).wavelength(48.0));
            let document = drawing.to_json()?;
            let back = Drawing::from_json(&document, &Inputs::new())?;
            assert_eq!(back.to_json()?, document);
            assert_eq!(back.shapes(&Values::new())?.len(), 1 << 12);
            Ok(())
        };
        run().map_err(|error| error.to_string())
    })?;
    handle.join().map_err(|_| "the thread panicked")??;
    Ok(())
}
