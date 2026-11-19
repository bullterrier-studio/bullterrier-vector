//! `Drawing::branch` through the public API: forking threads grown from the outlines of each shape, one shape per
//! root-to-tip path. Oracles: the outward normal of a square's edges by hand, the count of tips worked out as a sum of
//! binomials, lengths and containment measured with `Path::length` and `Path::contains` (which share no code with the
//! step), and points parsed back from the path data.

use crate::common::{Line, Outcome, lines, parameter_refusal};
use bullterrier_vector::{Branch, Deg, Drawing, Error, Inputs, Path, Select, Shape, Values};
use proptest::prelude::*;

/// A 400 by 400 square: 1600 pixels of outline, every edge's outward normal known by hand.
fn square() -> Result<Path, Error> {
    Path::rect(100.0, 100.0, 400.0, 400.0, 0.0)
}

fn drawn(drawing: &Drawing) -> Result<Vec<Shape>, Error> {
    drawing.shapes(&Values::new())
}

/// The points of a shape's one open path.
fn points(shape: &Shape) -> Result<Vec<(f64, f64)>, Box<dyn std::error::Error>> {
    let found: Vec<Line> = lines(shape.path())?;
    match found.as_slice() {
        [(points, false)] => Ok(points.clone()),
        other => Err(format!("not one open line: {other:?}").into()),
    }
}

/// The direction from `a` to `b` in degrees, clockwise on screen.
fn heading(a: (f64, f64), b: (f64, f64)) -> f64 {
    libm::atan2(b.1 - a.1, b.0 - a.0).to_degrees()
}

/// The difference of two directions with sign, in degrees from 0 to 180.
fn apart(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d)
}

fn binomial(n: u64, k: u64) -> u64 {
    (0..k).fold(1, |acc, i| acc * (n - i) / (i + 1))
}

#[test]
fn every_root_grows_one_path_per_set_of_forks_it_takes() -> Outcome {
    // 1600 of outline every 400: 4 roots; 200 long, a fork every 50: 4 stretches; at most 2 forks a path
    let branch = Branch::new(400.0, 200.0).fork_spacing(50.0).forks(2);
    let shapes = drawn(&Drawing::from(square()?).branch(&branch))?;
    let per_root: u64 = (0..=2).map(|k| binomial(4, k)).sum();
    assert_eq!(per_root, 11);
    assert_eq!(shapes.len() as u64, 4 * per_root);
    for root in 0..4 {
        let tree: Vec<&Shape> = shapes.iter().filter(|s| s.attributes().root() == Some(root)).collect();
        assert_eq!(tree.len() as u64, per_root, "root {root}");
        for forks in 0..=2_u32 {
            let taking = tree.iter().filter(|s| s.attributes().forks() == Some(forks)).count() as u64;
            assert_eq!(taking, binomial(4, u64::from(forks)), "root {root}, {forks} forks");
        }
    }
    Ok(())
}

#[test]
fn every_path_starts_on_the_outline_and_leaves_it_outward_along_the_normal() -> Outcome {
    // no wander and no forks: each path is straight along the outward normal of its edge
    let ring = Path::from_svg_path("M100 100 L500 100 L500 500 L100 500 Z M200 200 L200 400 L400 400 L400 200 Z")?;
    let branch = Branch::new(150.0, 60.0).forks(0).wander(Deg(0.0));
    let shapes = drawn(&Drawing::from(ring.clone()).branch(&branch))?;
    assert!(shapes.len() > 8);
    for shape in &shapes {
        let points = points(shape)?;
        let (root, next) = (points[0], points[1]);
        // the outward normal of each edge by hand: out of the square, and into the hole from its edges
        let normal = match root {
            (_, 100.0 | 400.0) => -90.0,
            (_, 500.0 | 200.0) => 90.0,
            (100.0 | 400.0, _) => 180.0,
            (500.0 | 200.0, _) => 0.0,
            _ => return Err(format!("root {root:?} off the outline").into()),
        };
        assert!(apart(heading(root, next), normal) < 1e-9, "{root:?} to {next:?}");
        assert!(!ring.contains(next)?, "{root:?} grows inward to {next:?}");
        for pair in points.windows(2) {
            assert!(apart(heading(pair[0], pair[1]), normal) < 1e-9, "a straight path bends");
        }
        assert_eq!(shape.attributes().origin().map(|p| (p.x, p.y)), Some(root));
    }
    Ok(())
}

#[test]
fn every_path_of_a_root_is_as_long_as_its_length_and_the_spread_varies_roots_only() -> Outcome {
    for spread in [0.0, 0.6] {
        let branch = Branch::new(200.0, 150.0)
            .fork_spacing(60.0)
            .forks(2)
            .spread(spread)
            .seed(3.0);
        let shapes = drawn(&Drawing::from(square()?).branch(&branch))?;
        let mut lengths = Vec::new();
        for root in 0..8 {
            let tree: Vec<f64> = shapes
                .iter()
                .filter(|s| s.attributes().root() == Some(root))
                .map(|s| s.path().length())
                .collect();
            assert_eq!(tree.len(), 1 + 2 + 1, "2 stretches, 2 forks: {} paths", tree.len());
            for length in &tree {
                assert!(
                    (length - tree[0]).abs() < 1e-6,
                    "spread {spread}: a tree of unequal paths {tree:?}"
                );
                assert!(
                    *length <= 150.0 + 1e-6 && *length >= 150.0 * (1.0 - spread) - 1e-6,
                    "{length}"
                );
            }
            lengths.push(tree[0]);
        }
        let varied = lengths.iter().any(|length| (length - lengths[0]).abs() > 1.0);
        assert_eq!(varied, spread > 0.0, "spread {spread}: {lengths:?}");
    }
    Ok(())
}

#[test]
fn a_fork_starts_on_its_parent_and_leaves_it_at_the_angle() -> Outcome {
    let branch = Branch::new(400.0, 240.0)
        .fork_spacing(60.0)
        .forks(1)
        .angle(Deg(35.0))
        .wander(Deg(25.0))
        .seed(11.0);
    let shapes = drawn(&Drawing::from(square()?).branch(&branch))?;
    let mut checked = 0;
    for root in 0..4 {
        let tree: Vec<&Shape> = shapes.iter().filter(|s| s.attributes().root() == Some(root)).collect();
        let trunk = points(
            tree.iter()
                .find(|s| s.attributes().forks() == Some(0))
                .ok_or("no trunk")?,
        )?;
        for fork in tree.iter().filter(|s| s.attributes().forks() == Some(1)) {
            let fork = points(fork)?;
            assert_eq!(fork[0], trunk[0], "a tree has one root");
            // the shared prefix: equal points up to where the fork leaves
            let shared = fork.iter().zip(&trunk).take_while(|(a, b)| a == b).count();
            assert!(shared >= 1 && shared < fork.len() && shared < trunk.len());
            let at = shared - 1;
            let turn = apart(heading(fork[at], fork[at + 1]), heading(trunk[at], trunk[at + 1]));
            assert!((turn - 35.0).abs() < 1e-6, "the fork turns {turn}");
            checked += 1;
        }
    }
    assert_eq!(checked, 4 * 4);
    Ok(())
}

#[test]
fn the_heading_drifts_at_most_twice_the_wander_from_the_normal() -> Outcome {
    let branch = Branch::new(100.0, 400.0)
        .forks(0)
        .wander(Deg(20.0))
        .wavelength(60.0)
        .seed(5.0);
    let shapes = drawn(&Drawing::from(square()?).branch(&branch))?;
    let mut bent = false;
    for shape in &shapes {
        let points = points(shape)?;
        let start = heading(points[0], points[1]);
        for pair in points.windows(2) {
            let turned = apart(heading(pair[0], pair[1]), start);
            assert!(turned <= 40.0 + 1e-6, "turned {turned}");
            bent |= turned > 5.0;
        }
    }
    assert!(bent, "no path wandered");
    Ok(())
}

#[test]
fn the_threads_are_a_function_of_the_seed_and_of_their_own_shape() -> Outcome {
    let svg = |drawing: &Drawing| -> Result<Vec<String>, Error> {
        Ok(drawn(drawing)?.iter().map(|s| s.path().to_svg_path()).collect())
    };
    let branch = |seed: f64| Branch::new(150.0, 120.0).fork_spacing(40.0).seed(seed);
    let alone = svg(&Drawing::from(square()?).branch(&branch(4.0)))?;
    assert_eq!(alone, svg(&Drawing::from(square()?).branch(&branch(4.0)))?);
    assert_ne!(alone, svg(&Drawing::from(square()?).branch(&branch(5.0)))?);
    // the seed places the roots and the forks, not only the drift
    let still = |seed: f64| -> Result<Vec<String>, Error> {
        svg(&Drawing::from(square()?).branch(&branch(seed).wander(Deg(0.0))))
    };
    assert_ne!(still(4.0)?, still(5.0)?);
    // a later shape does not move the first one's threads
    let two = Drawing::new([
        Drawing::from(square()?),
        Drawing::from(Path::circle((900.0, 300.0), 80.0)?),
    ]);
    let both = svg(&two.branch(&branch(4.0)))?;
    assert_eq!(both[..alone.len()], alone[..]);
    Ok(())
}

#[test]
fn a_request_past_the_limits_is_refused_naming_the_argument_before_any_thread_is_made() -> Outcome {
    let square = Drawing::from(square()?);
    let named = |branch: Branch| parameter_refusal(drawn(&square.branch(&branch))).map(|(name, _)| name);
    assert!(named(Branch::new(1e-6, 100.0))?.ends_with("spacing"));
    assert!(named(Branch::new(400.0, 1e6).fork_spacing(1.0).forks(1_000_000))?.ends_with("fork_spacing"));
    assert!(named(Branch::new(400.0, 1e9).wavelength(1e-3))?.ends_with("wavelength"));
    for (branch, argument) in [
        (Branch::new(0.0, 100.0), "spacing"),
        (Branch::new(100.0, -1.0), "length"),
        (Branch::new(100.0, 100.0).fork_spacing(0.0), "fork_spacing"),
        (Branch::new(100.0, 100.0).spread(1.5), "spread"),
        (Branch::new(100.0, 100.0).seed(0.5), "seed"),
        (Branch::new(100.0, 100.0).wander(Deg(f64::NAN)), "wander"),
    ] {
        let name = named(branch)?;
        assert!(name.ends_with(argument), "{argument}: named {name}");
    }
    Ok(())
}

#[test]
fn a_document_reads_back_to_the_same_threads_and_selects_the_trunks() -> Outcome {
    let branch = Branch::new(300.0, 160.0).fork_spacing(50.0).spread(0.3).seed(2.0);
    let drawing = Drawing::from(square()?).branch(&branch);
    let text = drawing.to_json()?;
    assert!(text.contains(r#""branch":{"#), "{text}");
    let back = Drawing::from_json(&text, &Inputs::new())?;
    assert_eq!(back.to_json()?, text);
    let paths =
        |d: &Drawing| -> Result<Vec<String>, Error> { Ok(drawn(d)?.iter().map(|s| s.path().to_svg_path()).collect()) };
    assert_eq!(paths(&back)?, paths(&drawing)?);
    let trunks = drawn(&drawing.only(Select::new().attribute("forks", 0)))?;
    let roots = drawn(&drawing)?
        .iter()
        .filter(|s| s.attributes().forks() == Some(0))
        .count();
    assert_eq!(trunks.len(), roots);
    assert_eq!(roots, 1600 / 300);
    Ok(())
}

/// The points every path of a request is charged: a point per step from the root to the tip, and the root. The step is a
/// 24th of the wavelength (api.md, `branch`).
fn charged_points(length: f64, wavelength: f64) -> usize {
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "a small count")]
    let steps = (length / (wavelength / 24.0)).ceil() as usize;
    steps + 1
}

/// A path has the points it was charged, in steps of one size that end exactly at its length.
fn assert_charged(shape: &Shape, length: f64, wavelength: f64) -> Outcome {
    let points = points(shape)?;
    let charged = charged_points(length, wavelength);
    assert_eq!(points.len(), charged, "length {length}, wavelength {wavelength}");
    let steps: Vec<f64> = points
        .windows(2)
        .map(|pair| crate::common::distance(pair[0], pair[1]))
        .collect();
    let each = length / (charged - 1) as f64;
    for step in &steps {
        assert!((step - each).abs() <= 1e-9 * each.max(1.0), "step {step}, not {each}");
    }
    assert!(
        (shape.path().length() - length).abs() <= 1e-9 * length,
        "ends short of its length"
    );
    Ok(())
}

#[test]
fn a_path_has_exactly_the_points_it_is_charged_when_its_length_is_a_whole_number_of_steps() -> Outcome {
    // 30 long at a step of 80 / 24 is 9 steps, so 10 points: the growth must not add a tail step of a rounding error
    for (length, wavelength) in [(30.0, 80.0), (100.0, 2.4)] {
        let branch = Branch::new(1600.0, length)
            .forks(0)
            .wander(Deg(0.0))
            .wavelength(wavelength);
        let shapes = drawn(&Drawing::from(square()?).branch(&branch))?;
        assert_eq!(shapes.len(), 1);
        assert_charged(&shapes[0], length, wavelength)?;
    }
    Ok(())
}

proptest! {
    /// Every path of every tree, forks included, has the points its length was charged, whatever the length and step.
    #[test]
    fn every_path_has_the_points_it_was_charged_and_equal_steps(
        length in 1.0_f64..400.0,
        wavelength in 2.0_f64..120.0,
        forks in 0_usize..3,
        shorter in prop_oneof![Just(0.0), 0.0_f64..1.0],
    ) {
        let branch = Branch::new(800.0, length)
            .fork_spacing(length / 3.0)
            .forks(forks)
            .spread(shorter)
            .wavelength(wavelength);
        let shapes = drawn(&Drawing::from(square().map_err(|e| TestCaseError::fail(e.to_string()))?).branch(&branch))
            .map_err(|e| TestCaseError::fail(e.to_string()))?;
        for shape in &shapes {
            let points = points(shape).map_err(|e| TestCaseError::fail(e.to_string()))?;
            // a root's paths share their steps; the tree is as long as its charged bound or shorter
            prop_assert!(points.len() <= charged_points(length, wavelength));
            let steps: Vec<f64> = points.windows(2).map(|p| crate::common::distance(p[0], p[1])).collect();
            let first = steps[0];
            for step in &steps {
                prop_assert!((step - first).abs() <= 1e-9 * first.max(1.0), "steps {} and {}", first, step);
            }
            prop_assert!(first <= wavelength / 24.0 * (1.0 + 1e-12), "a step of {} past the 24th", first);
            if shorter == 0.0 {
                prop_assert_eq!(points.len(), charged_points(length, wavelength));
                prop_assert!((shape.path().length() - length).abs() <= 1e-9 * length);
            }
        }
    }
}

#[test]
fn a_path_of_one_point_past_the_limit_is_refused_by_the_bound_not_grown() -> Outcome {
    // a step of 1 (wavelength 24) over 10,000,000: 10,000,000 steps and so 10,000,001 points, one past MAX_SEGMENTS
    let branch = Branch::new(1600.0, 10_000_000.0).forks(0).wavelength(24.0);
    let (name, message) = parameter_refusal(drawn(&Drawing::from(square()?).branch(&branch)))?;
    assert!(name.ends_with("wavelength"), "{name}");
    assert!(message.contains("10000001 points"), "{message}");
    Ok(())
}

#[test]
fn a_document_with_forks_that_are_not_a_whole_number_from_0_is_refused_naming_forks() -> Outcome {
    // `forks` is a `usize` in Rust, so only a document can say a fraction or a negative number
    let written = Drawing::from(square()?)
        .branch(&Branch::new(300.0, 100.0).forks(7))
        .to_json()?;
    assert!(written.contains(r#""forks":7"#), "{written}");
    let document = |forks: &str| written.replace(r#""forks":7"#, &format!(r#""forks":{forks}"#));
    for forks in ["0.5", "-1"] {
        let refusal = Drawing::from_json(&document(forks), &Inputs::new())
            .err()
            .ok_or("read")?
            .to_string();
        assert!(
            refusal.contains("/branch/forks: must be a whole number from 0"),
            "forks {forks}: {refusal}"
        );
    }
    assert!(
        Drawing::from_json(&document("2"), &Inputs::new())
            .and_then(|d| drawn(&d))
            .is_ok()
    );
    Ok(())
}
