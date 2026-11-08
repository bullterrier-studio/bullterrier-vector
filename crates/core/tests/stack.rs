//! Own binary: it runs on the smallest thread stack the library supports and is built unoptimised (`profile.stack` in
//! Cargo.toml, run by `poe rust-test`): an unoptimised frame is the largest one, so the bound holds for every build.
//!
//! The deepest structures a caller can build or load, used on a 256 KB stack (docs/design-decisions.md, "Bounded
//! depth"). An overflow aborts the process, so a test here that fails is a crash. The Python counterpart on the
//! shipped release build is `tests/test_stack_depth.py`.

#[path = "it/common/mod.rs"]
mod common;

use bullterrier_vector::{Color, Error, Expr, Param, Random, Size, Stagger, Text, Values, by_index, by_order, text};
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
