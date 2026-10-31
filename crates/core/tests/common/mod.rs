//! What the integration tests share: the outcome type, readers of the errors the library returns, and the helpers
//! that spread proptest sizes over orders of magnitude. Each test file uses some of them.
#![allow(dead_code, reason = "each test file uses a different part")]

pub(crate) mod ops;

use bullterrier_vector::{Error, Path};
use proptest::prelude::*;
use std::f64::consts::PI;

pub(crate) type Outcome = Result<(), Box<dyn std::error::Error>>;

/// The parameter and message of the `Error::Parameter` an operation returned; an error if it succeeded or failed
/// otherwise.
pub(crate) fn parameter_refusal<T>(outcome: Result<T, Error>) -> Result<(String, String), Box<dyn std::error::Error>> {
    match outcome {
        Err(Error::Parameter { parameter, message }) => Ok((parameter, message)),
        Err(other) => Err(format!("expected a parameter error, got {other}").into()),
        Ok(_) => Err("expected a parameter error, but it succeeded".into()),
    }
}

/// The verb and message of the `Error::Geometry` an operation returned; an error if it succeeded or failed otherwise.
pub(crate) fn geometry_refusal<T>(outcome: Result<T, Error>) -> Result<(String, String), Box<dyn std::error::Error>> {
    match outcome {
        Err(Error::Geometry { verb, message }) => Ok((verb, message)),
        Err(other) => Err(format!("expected a geometry error, got {other}").into()),
        Ok(_) => Err("expected a geometry error, but it succeeded".into()),
    }
}

pub(crate) fn distance(from: (f64, f64), to: (f64, f64)) -> f64 {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    (dx * dx + dy * dy).sqrt()
}

/// Numbers spread evenly over the orders of magnitude from `10^low` to `10^high`: sizes are log-uniform.
pub(crate) fn log_uniform(low: f64, high: f64) -> impl Strategy<Value = f64> {
    (low..high).prop_map(|exponent| libm::pow(10.0, exponent))
}

/// A path command: its letter and the numbers after it.
pub(crate) type Command = (char, Vec<f64>);

/// The commands of canonical path data: each letter with the numbers after it.
pub(crate) fn commands(data: &str) -> Result<Vec<Command>, Box<dyn std::error::Error>> {
    let mut found: Vec<Command> = Vec::new();
    for token in data.split(' ').filter(|token| !token.is_empty()) {
        let (letter, number) = match token.chars().next() {
            Some(first @ ('M' | 'L' | 'Q' | 'C' | 'Z')) => (Some(first), &token[1..]),
            _ => (None, token),
        };
        if let Some(letter) = letter {
            found.push((letter, Vec::new()));
        }
        if !number.is_empty() {
            found
                .last_mut()
                .ok_or("a number before any command")?
                .1
                .push(number.parse()?);
        }
    }
    Ok(found)
}

/// One builder step: a verb with the points it takes.
#[derive(Debug, Clone)]
pub(crate) enum Step {
    Move((f64, f64)),
    Line((f64, f64)),
    Quad((f64, f64), (f64, f64)),
    Curve((f64, f64), (f64, f64), (f64, f64)),
    Close,
}

/// A step whose points come from `point`.
pub(crate) fn step(point: impl Strategy<Value = (f64, f64)> + Clone) -> impl Strategy<Value = Step> {
    prop_oneof![
        point.clone().prop_map(Step::Move),
        point.clone().prop_map(Step::Line),
        (point.clone(), point.clone()).prop_map(|(c, p)| Step::Quad(c, p)),
        (point.clone(), point.clone(), point).prop_map(|(a, b, p)| Step::Curve(a, b, p)),
        Just(Step::Close),
    ]
}

/// The path that starts at `start` and then takes `steps` in order.
pub(crate) fn build(start: (f64, f64), steps: &[Step]) -> Result<Path, Error> {
    let mut path = Path::start(start)?;
    for step in steps {
        path = match *step {
            Step::Move(p) => path.move_to(p),
            Step::Line(p) => path.line_to(p),
            Step::Quad(c, p) => path.quad_to(c, p),
            Step::Curve(a, b, p) => path.curve_to(a, b, p),
            Step::Close => path.close(),
        }?;
    }
    Ok(path)
}

/// The corners of a polyline.
pub(crate) type Corners = Vec<(f64, f64)>;

/// How many times the closed polyline through `corners` winds round `point`: the angles the corners subtend at the
/// point, added with their signs, make 2 pi for each turn. Only meaningful off the polyline.
pub(crate) fn winding(corners: &[(f64, f64)], point: (f64, f64)) -> i32 {
    let mut turned = 0.0;
    for (index, from) in corners.iter().enumerate() {
        let to = corners[(index + 1) % corners.len()];
        let (ax, ay) = (from.0 - point.0, from.1 - point.1);
        let (bx, by) = (to.0 - point.0, to.1 - point.1);
        turned += libm::atan2(ax * by - ay * bx, ax * bx + ay * by);
    }
    // the cast cannot overflow: the turns are a few at most
    #[expect(clippy::cast_possible_truncation, reason = "a few turns at most")]
    let turns = libm::round(turned / (2.0 * PI)) as i32;
    turns
}

/// The distance from `point` to the nearest edge of the closed polyline through `corners`.
pub(crate) fn clearance(corners: &[(f64, f64)], point: (f64, f64)) -> f64 {
    let mut nearest = f64::INFINITY;
    for (index, from) in corners.iter().enumerate() {
        let to = corners[(index + 1) % corners.len()];
        let (along_x, along_y) = (to.0 - from.0, to.1 - from.1);
        let share = (((point.0 - from.0) * along_x + (point.1 - from.1) * along_y)
            / (along_x * along_x + along_y * along_y))
            .clamp(0.0, 1.0);
        let closest = (from.0 + along_x * share, from.1 + along_y * share);
        nearest = nearest.min(distance(point, closest));
    }
    nearest
}

/// The area the closed polygon through `corners` encloses, positive when it runs clockwise on screen (`y` down): the
/// shoelace formula.
pub(crate) fn shoelace_area(corners: &[(f64, f64)]) -> f64 {
    let mut doubled = 0.0;
    for (index, (x, y)) in corners.iter().enumerate() {
        let (next_x, next_y) = corners[(index + 1) % corners.len()];
        doubled += x * next_y - next_x * y;
    }
    doubled / 2.0
}
