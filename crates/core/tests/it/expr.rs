//! `Expr` through the public API: arithmetic on Params and numbers builds an expression whose value is worked out when
//! the drawing is drawn. Expected values are written by hand, or computed by a small reference evaluator that does the
//! same arithmetic on plain numbers; the strings an expression prints are written out by hand.

use crate::common::{Outcome, log_uniform, parameter_refusal};
use bullterrier_vector::{Error, Expr, Param, Random, Stagger, Unit, Values, by_index, by_order};
use proptest::prelude::*;

/// A bounded Param from -1000 to 1000 with the given default.
fn param(name: &str, default: f64) -> Result<Param, Error> {
    Param::new(name, default, -1000.0, 1000.0)
}

/// The value of `expr` with the Params at their defaults.
fn value(expr: &Expr) -> Result<f64, Error> {
    expr.value(&Values::new())
}

// Operators ---------------------------------------------------------------------------------------------------------

/// Every way to write `left op right` with a left operand worth 6 and a right operand worth 3: `Expr`, `&Expr`,
/// `Param`, `&Param` and `f64`, on either side (two plain numbers are not an expression).
macro_rules! every_operand_pair {
    ($op:tt, $e6:expr, $p6:expr, $e3:expr, $p3:expr) => {
        [
            $e6.clone() $op $e3.clone(),
            $e6.clone() $op &$e3,
            $e6.clone() $op $p3.clone(),
            $e6.clone() $op &$p3,
            $e6.clone() $op 3.0,
            &$e6 $op $e3.clone(),
            &$e6 $op &$e3,
            &$e6 $op $p3.clone(),
            &$e6 $op &$p3,
            &$e6 $op 3.0,
            $p6.clone() $op $e3.clone(),
            $p6.clone() $op &$e3,
            $p6.clone() $op $p3.clone(),
            $p6.clone() $op &$p3,
            $p6.clone() $op 3.0,
            &$p6 $op $e3.clone(),
            &$p6 $op &$e3,
            &$p6 $op $p3.clone(),
            &$p6 $op &$p3,
            &$p6 $op 3.0,
            6.0 $op $e3.clone(),
            6.0 $op &$e3,
            6.0 $op $p3.clone(),
            6.0 $op &$p3,
        ]
    };
}

#[test]
fn each_operator_works_on_every_pair_of_operand_kinds_in_the_order_written() -> Outcome {
    let (e6, e3) = (Expr::from(6.0), Expr::from(3.0));
    let (p6, p3) = (param("six", 6.0)?, param("three", 3.0)?);
    // 6 and 3 differ, so a swapped operand changes the result of - and /
    let tables = [
        ("+", 9.0, every_operand_pair!(+, e6, p6, e3, p3)),
        ("-", 3.0, every_operand_pair!(-, e6, p6, e3, p3)),
        ("*", 18.0, every_operand_pair!(*, e6, p6, e3, p3)),
        ("/", 2.0, every_operand_pair!(/, e6, p6, e3, p3)),
    ];
    for (operator, expected, expressions) in tables {
        for (position, expression) in expressions.iter().enumerate() {
            assert_eq!(
                value(expression)?,
                expected,
                "operator {operator}, pair {position}: {expression}"
            );
        }
    }
    Ok(())
}

#[test]
fn negation_works_on_every_kind_of_operand() -> Outcome {
    let (e4, p4) = (Expr::from(4.0), param("four", 4.0)?);
    for negated in [-e4.clone(), -&e4, -p4.clone(), -&p4] {
        assert_eq!(value(&negated)?, -4.0, "{negated}");
    }
    Ok(())
}

#[test]
fn a_param_takes_the_value_given_or_else_its_default() -> Outcome {
    let (rise, ink) = (param("rise", 0.5)?, param("ink", 8.0)?);
    // 2 * rise + ink
    let expr = 2.0 * &rise + &ink;
    assert_eq!(value(&expr)?, 9.0);
    assert_eq!(expr.value(&Values::new().set("rise", 0.25))?, 8.5);
    assert_eq!(expr.value(&Values::new().set("rise", 0.25).set("ink", -3.0))?, -2.5);
    // a value for a name the expression does not read changes nothing here
    assert_eq!(expr.value(&Values::new().set("other", 1.0))?, 9.0);
    Ok(())
}

#[test]
fn a_cyclic_param_wraps_the_value_it_is_given() -> Outcome {
    let angle = Param::new("angle", 0.0, 0.0, 360.0)?.unit(Unit::Deg)?;
    let expr = &angle + 1.0;
    assert_eq!(expr.value(&Values::new().set("angle", 370.0))?, 11.0);
    assert_eq!(expr.value(&Values::new().set("angle", -10.0))?, 351.0);
    Ok(())
}

#[test]
fn a_value_outside_the_range_of_a_bounded_param_is_refused_naming_the_param() -> Outcome {
    let rise = Param::new("rise", 0.5, 0.0, 1.0)?;
    let (name, message) = parameter_refusal((2.0 * &rise).value(&Values::new().set("rise", 1.5)))?;
    assert!(name == "rise" && message.contains("1.5"), "{name}: {message}");
    Ok(())
}

#[test]
fn a_param_given_a_list_cannot_be_read_without_shapes() -> Outcome {
    let pulse = param("pulse", 0.0)?;
    let (name, message) = parameter_refusal((&pulse + 1.0).value(&Values::new().set("pulse", vec![1.0, 2.0])))?;
    assert!(name == "pulse" && message.contains("list"), "{name}: {message}");
    Ok(())
}

// Printing ----------------------------------------------------------------------------------------------------------

#[test]
fn an_expression_prints_infix_with_the_fewest_parentheses_that_keep_its_shape() -> Outcome {
    let (a, b, c) = (param("a", 1.0)?, param("b", 2.0)?, param("c", 3.0)?);
    let pulse = param("pulse", 0.0)?;
    // (expression, what it prints)
    let rows = [
        (1.0 + 0.08 * &pulse, "1 + 0.08 * pulse"),
        ((&a + &b) * &c, "(a + b) * c"),
        (-&a, "-(a)"),
        (-(&a + &b), "-(a + b)"),
        (&a + &b * &c, "a + b * c"),
        (&a * &b + &c, "a * b + c"),
        (&a * (&b + &c), "a * (b + c)"),
        // the left operand of an operator of the same rank needs no parentheses, the right one does
        ((&a - &b) - &c, "a - b - c"),
        (&a - (&b - &c), "a - (b - c)"),
        (&a - (&b + &c), "a - (b + c)"),
        ((&a / &b) * &c, "a / b * c"),
        (&a / (&b * &c), "a / (b * c)"),
        (&a + (&b + &c), "a + (b + c)"),
        (&a * (&b * &c), "a * (b * c)"),
        (&a / &b / &c, "a / b / c"),
        (&a / (&b / &c), "a / (b / c)"),
        // a negation closes itself, so it needs no parentheses as an operand
        (-&a * &b, "-(a) * b"),
        (&a * -&b, "a * -(b)"),
        (&a - -&b, "a - -(b)"),
        (-(-&a), "-(-(a))"),
        // numbers print as the shortest text that reads back as the same number
        (Expr::from(3.0) * &a, "3 * a"),
        (Expr::from(-1.5) * &a, "-1.5 * a"),
        (0.1 + &a, "0.1 + a"),
        (&a / 1e-7, "a / 0.0000001"),
    ];
    for (expression, text) in rows {
        assert_eq!(expression.to_string(), text);
    }
    Ok(())
}

#[test]
fn a_negative_zero_is_stored_and_printed_as_zero() -> Outcome {
    let minus_zero = Expr::from(-0.0);
    assert_eq!(minus_zero.to_string(), "0");
    assert!(value(&minus_zero)?.is_sign_positive());
    assert_eq!(minus_zero, Expr::from(0.0));
    let a = param("a", 1.0)?;
    assert_eq!((&a + -0.0).to_string(), "a + 0");
    Ok(())
}

#[test]
fn expressions_are_equal_when_they_are_built_the_same_way() -> Outcome {
    let (a, b) = (param("a", 1.0)?, param("b", 2.0)?);
    assert_eq!(&a + &b, &a + &b);
    assert_eq!(&a + &b, a.clone() + b.clone());
    assert_eq!(Expr::from(&a), Expr::from(a.clone()));
    // the order of the operands is part of the expression: a + b is not b + a
    assert_ne!(&a + &b, &b + &a);
    assert_ne!(&a + &b, &a - &b);
    assert_ne!(-&a, Expr::from(&a));
    assert_ne!(Expr::from(1.0), Expr::from(2.0));
    // a Param of the same name that differs in a field is another Param
    assert_ne!(Expr::from(&a), Expr::from(param("a", 1.5)?));
    Ok(())
}

// What it is worth --------------------------------------------------------------------------------------------------

#[test]
fn a_division_by_zero_is_refused_naming_the_expression() -> Outcome {
    let (x, y) = (param("x", 5.0)?, param("y", 2.0)?);
    // (the division, the expression the message names)
    let rows = [
        (Expr::from(1.0) / 0.0, "(1 / 0)"),
        (&x / 0.0, "(x / 0)"),
        (&x / (&y - 2.0), "(x / (y - 2))"),
        (3.0 + &x / (&y - &y), "(x / (y - y))"),
        (&x / -0.0, "(x / 0)"),
    ];
    for (expression, named) in rows {
        let (_, message) = parameter_refusal(value(&expression))?;
        assert!(
            message.contains("division by zero") && message.contains(named),
            "{message}"
        );
    }
    Ok(())
}

#[test]
fn a_value_that_overflows_or_is_not_a_number_is_refused() -> Outcome {
    let x = param("x", 1000.0)?;
    let huge = 1e300;
    // (the expression, a text the message holds)
    let rows = [
        (&x * huge * huge, "inf"),
        (&x * huge * -huge, "-inf"),
        (&x + f64::INFINITY, "inf"),
        (Expr::from(f64::NAN), "NaN"),
        (-&x + f64::NAN, "NaN"),
        (Expr::from(1.0) / 1e-300 / 1e-300, "inf"),
    ];
    for (expression, text) in rows {
        let (_, message) = parameter_refusal(value(&expression))?;
        assert!(
            message.contains("finite") && message.contains(text),
            "{expression}: {message}"
        );
    }
    Ok(())
}

#[test]
fn an_expression_with_no_param_is_a_constant_and_one_with_a_param_is_not() -> Outcome {
    let x = param("x", 1.0)?;
    assert_eq!((Expr::from(2.0) * 3.0 + 1.0).constant(), Some(7.0));
    assert_eq!(Expr::from(-4.5).constant(), Some(-4.5));
    assert_eq!(Expr::from(-0.0).constant().map(f64::is_sign_positive), Some(true));
    assert_eq!((-Expr::from(2.0)).constant(), Some(-2.0));
    assert_eq!(Expr::from(&x).constant(), None);
    assert_eq!((2.0 * &x).constant(), None);
    assert_eq!((&x * 2.0).constant(), None);
    assert_eq!((-&x).constant(), None);
    // a Param that cancels out is still a Param
    assert_eq!((&x - &x).constant(), None);
    // a constant with no finite value has no number to give; value() says why
    assert_eq!((Expr::from(1.0) / 0.0).constant(), None);
    assert_eq!(Expr::from(f64::NAN).constant(), None);
    Ok(())
}

#[test]
fn an_expression_deeper_than_128_levels_is_refused() -> Outcome {
    // 1 + 1 + ... : each addition is one level deeper than the one before
    let deep = |levels: usize| {
        let mut expression = Expr::from(1.0);
        for _ in 1..levels {
            expression = expression + 1.0;
        }
        expression
    };
    assert_eq!(value(&deep(1))?, 1.0);
    assert_eq!(value(&deep(128))?, 128.0);
    let (_, message) = parameter_refusal(value(&deep(129)))?;
    assert!(message.contains("128") && message.contains("129"), "{message}");
    parameter_refusal(value(&deep(200)))?;
    // the same through negation and through the right operand
    let mut negated = Expr::from(1.0);
    for _ in 1..129 {
        negated = -negated;
    }
    parameter_refusal(value(&negated))?;
    let mut right = Expr::from(1.0);
    for _ in 1..129 {
        right = 1.0 + right;
    }
    parameter_refusal(value(&right))?;
    // 128 levels through the right operand is accepted; 1 - (1 - (... 1)) with 127 subtractions is 0
    let mut ok = Expr::from(1.0);
    for _ in 1..128 {
        ok = 1.0 - ok;
    }
    assert_eq!(value(&ok)?, 0.0);
    Ok(())
}

/// An expression of exactly `nodes` nodes (a number, a Param or an operator is one) that is shallow: an odd count is
/// an operator over two halves, an even one a negation, the only operator with one operand.
fn with_nodes(nodes: usize) -> Expr {
    if nodes <= 1 {
        Expr::from(1.0)
    } else if nodes.is_multiple_of(2) {
        -with_nodes(nodes - 1)
    } else {
        let left = (nodes - 1) / 2;
        with_nodes(left) + with_nodes(nodes - 1 - left)
    }
}

#[test]
fn an_expression_of_more_than_10000_nodes_is_refused_counting_shared_nodes_each_time() -> Outcome {
    value(&with_nodes(10_000))?;
    let (_, message) = parameter_refusal(value(&with_nodes(10_001)))?;
    assert!(
        message.contains("MAX_EXPR_NODES") && message.contains("10000"),
        "{message}"
    );
    // `e + e` shares its operand but is worked out twice: 28 doublings are 2^29 - 1 nodes and 19 s of work
    let x = param("x", 1.0)?;
    let mut shared = Expr::from(&x);
    for _ in 0..28 {
        shared = &shared + &shared;
    }
    let (_, message) = parameter_refusal(value(&shared))?;
    assert!(message.contains("MAX_EXPR_NODES"), "{message}");
    Ok(())
}

#[test]
fn an_expression_a_million_levels_deep_is_refused_and_dropped_without_a_crash() -> Outcome {
    // operators never fail, so a loop can build one; only a call refuses it
    let x = param("x", 1.0)?;
    let mut sum = Expr::from(&x);
    let mut negated = Expr::from(1.0);
    for _ in 0..1_000_000 {
        sum = sum + &x;
        negated = -negated;
    }
    let shared = sum.clone();
    parameter_refusal(value(&sum))?;
    parameter_refusal(value(&negated))?;
    drop(sum);
    drop(negated);
    parameter_refusal(value(&shared))?;
    Ok(())
}

/// The smallest thread stack the library supports (docs/design-decisions.md, "Bounded depth"), in bytes.
const SMALLEST_STACK: usize = 256 * 1024;

#[test]
fn a_million_level_expression_is_printed_compared_and_dropped_on_the_smallest_stack() -> Outcome {
    // a loop builds it, since operators never fail; nothing a caller can do to it may overflow a small stack
    let handle = std::thread::Builder::new().stack_size(SMALLEST_STACK).spawn(|| {
        let run = || -> Outcome {
            let x = param("x", 1.0)?;
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
            let x = param("x", 1.0)?;
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
fn a_per_item_expression_is_deep_enough_at_128_levels_and_too_deep_at_129() -> Outcome {
    // a Param is 1 level and each per-item number over it one more
    let x = param("x", 1.0)?;
    let nest = |levels: usize| {
        let mut deepest = Expr::from(&x);
        for _ in 1..levels {
            deepest = by_order(deepest, 0.0);
        }
        deepest
    };
    assert!(!nest(128).to_string().starts_with("(an expression"));
    assert!(nest(129).to_string().starts_with("(an expression 129"));
    Ok(())
}

// Against plain arithmetic ------------------------------------------------------------------------------------------

/// An expression tree, written as data so the same tree can be built as an `Expr` and worked out on plain numbers.
#[derive(Debug, Clone)]
enum Tree {
    Number(f64),
    Variable(usize),
    Negate(Box<Tree>),
    Add(Box<Tree>, Box<Tree>),
    Subtract(Box<Tree>, Box<Tree>),
    Multiply(Box<Tree>, Box<Tree>),
    Divide(Box<Tree>, Box<Tree>),
}

/// The tree as an expression over `params`, using the operators.
fn build(tree: &Tree, params: &[Param]) -> Expr {
    match tree {
        Tree::Number(number) => Expr::from(*number),
        Tree::Variable(index) => Expr::from(&params[*index]),
        Tree::Negate(inner) => -build(inner, params),
        Tree::Add(left, right) => build(left, params) + build(right, params),
        Tree::Subtract(left, right) => build(left, params) - build(right, params),
        Tree::Multiply(left, right) => build(left, params) * build(right, params),
        Tree::Divide(left, right) => build(left, params) / build(right, params),
    }
}

/// The value of the tree worked out on plain numbers with the variables at `given`, in the order the tree says; `None`
/// where it divides by zero or any step is not a finite number.
fn reference(tree: &Tree, given: &[f64]) -> Option<f64> {
    let number = match tree {
        // a negative zero enters as zero
        Tree::Number(number) => *number + 0.0,
        Tree::Variable(index) => given[*index],
        Tree::Negate(inner) => -reference(inner, given)?,
        Tree::Add(left, right) => reference(left, given)? + reference(right, given)?,
        Tree::Subtract(left, right) => reference(left, given)? - reference(right, given)?,
        Tree::Multiply(left, right) => reference(left, given)? * reference(right, given)?,
        Tree::Divide(left, right) => {
            let (dividend, divisor) = (reference(left, given)?, reference(right, given)?);
            if divisor == 0.0 {
                return None;
            }
            dividend / divisor
        }
    };
    number.is_finite().then_some(number)
}

fn number() -> impl Strategy<Value = f64> {
    prop_oneof![
        Just(0.0),
        Just(-0.0),
        (log_uniform(-3.0, 6.0), any::<bool>()).prop_map(|(size, negative)| if negative { -size } else { size }),
        (log_uniform(150.0, 154.0), any::<bool>()).prop_map(|(size, negative)| if negative { -size } else { size }),
    ]
}

fn tree() -> impl Strategy<Value = Tree> {
    let leaf = prop_oneof![number().prop_map(Tree::Number), (0..3usize).prop_map(Tree::Variable)];
    leaf.prop_recursive(5, 40, 2, |inner| {
        prop_oneof![
            inner.clone().prop_map(|tree| Tree::Negate(Box::new(tree))),
            (inner.clone(), inner.clone()).prop_map(|(l, r)| Tree::Add(Box::new(l), Box::new(r))),
            (inner.clone(), inner.clone()).prop_map(|(l, r)| Tree::Subtract(Box::new(l), Box::new(r))),
            (inner.clone(), inner.clone()).prop_map(|(l, r)| Tree::Multiply(Box::new(l), Box::new(r))),
            (inner.clone(), inner).prop_map(|(l, r)| Tree::Divide(Box::new(l), Box::new(r))),
        ]
    })
}

proptest! {
    // The value is the plain arithmetic of the tree, bit for bit but for the sign of a zero, or an error exactly where plain arithmetic divides
    // by zero or leaves the finite numbers.
    #[test]
    fn the_value_is_the_arithmetic_done_on_plain_numbers(
        tree in tree(),
        given in prop::array::uniform3(-1000.0f64..1000.0),
    ) {
        let params = [param("p", 0.0), param("q", 0.0), param("r", 0.0)]
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        let values = Values::new().set("p", given[0]).set("q", given[1]).set("r", given[2]);
        let outcome = build(&tree, &params).value(&values);
        match reference(&tree, &given) {
            Some(expected) => {
                let found = outcome.map_err(|error| TestCaseError::fail(error.to_string()))?;
                // a zero is never negative: the sign of a zero is the one thing the library normalises
                prop_assert_eq!(found.to_bits(), (expected + 0.0).to_bits());
            }
            None => prop_assert!(matches!(outcome, Err(Error::Parameter { .. })), "{outcome:?}"),
        }
    }
}
