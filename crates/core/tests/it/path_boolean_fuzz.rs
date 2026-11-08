//! The booleans of `Path` under random and degenerate input: edges that touch, run collinear, overlap or nearly
//! coincide, duplicate points, segments a million times shorter than the shape, slivers, self-crossing stars and
//! scribbles, nested rings, curves with loops and cusps, shapes of very different sizes, open sub-paths and many
//! sub-paths. Nothing here uses the library's own region code: the generators, the winding number, the exact area of a
//! set rule (`common::region`) and the search for crossing edges are written in the tests.
//!
//! The slack. A probe point is checked only when it is farther than `slack` from every edge of the input, and a result
//! area may differ from the exact one by `slack` times the perimeter. For straight edges the slack is 1e-13 of the
//! largest coordinate: the engine snaps to a grid of 2^-60 of the half-extent and writes a number, so it is 1000 times
//! what the rounding of a number is. A curve is flattened within the tolerance (0.01) and the oracle samples it at 256
//! points a curve (at most 0.003 off), so a scene with curves adds `CURVE_SLACK`. A scene whose generator puts edges
//! `JITTER` apart (a shape beside a copy nudged by that much) has details of that size, which the engine can lose where
//! they meet a sliver that a number's rounding left of an exact overlap (docs/api.md, "Booleans"): it adds `GRAIN`
//! times the jitter.

use crate::common::region::{canonical_fault, crossing_fault, exact_areas, inside, nearest_edge, sampled};
use crate::common::{Corners, Ring, log_uniform, net_area, polygons_path, rings};
use bullterrier_vector::{Error, Path};
use proptest::prelude::*;
use std::f64::consts::PI;

const TOLERANCE: f64 = 0.01;
const VERBS: [&str; 4] = ["union", "subtract", "intersect", "exclude"];
/// The share of the largest coordinate that a probe must keep from every edge, and a result may stray by.
const SLACK: f64 = 1e-13;
/// What a curve adds: the flattening tolerance and the oracle's own sampling error, rounded up.
const CURVE_SLACK: f64 = 0.02;
const CURVE_STEPS: usize = 256;
/// How far the copy of a shape is nudged in a "near" scene, at most in each direction.
const JITTER: f64 = 1e-7;
/// How many jitters a probe keeps from every edge of a scene with that jitter, and a result may stray by.
// constant: the engine lost a region 1e-8 thick beside an edge 1.8e-9 from the probe (api.md, "Booleans"); a probe
// ten times the largest nudge (1.4e-7) away is clear of every such detail
const GRAIN: f64 = 10.0;

type P = (f64, f64);

// Input as plain data ------------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum Seg {
    Line(P),
    Quad(P, P),
    Cubic(P, P, P),
    /// A cubic that leaves for the point and comes back, its first control the point and its second the segment's
    /// start: a loop, or a cusp when the point is on the way.
    Loop(P),
}

#[derive(Debug, Clone)]
struct Sub {
    start: P,
    segs: Vec<Seg>,
    closed: bool,
}

fn polygon(corners: &[P], closed: bool) -> Sub {
    Sub {
        start: corners[0],
        segs: corners[1..].iter().map(|&p| Seg::Line(p)).collect(),
        closed,
    }
}

fn path_of(subs: &[Sub]) -> Result<Path, Error> {
    let mut path = Path::new();
    for sub in subs {
        path = path.move_to(sub.start)?;
        let mut at = sub.start;
        for seg in &sub.segs {
            path = match *seg {
                Seg::Line(p) => path.line_to(p)?,
                Seg::Quad(c, p) => path.quad_to(c, p)?,
                Seg::Cubic(a, b, p) => path.curve_to(a, b, p)?,
                Seg::Loop(p) => path.curve_to(p, at, p)?,
            };
            at = match *seg {
                Seg::Line(p) | Seg::Quad(_, p) | Seg::Cubic(_, _, p) | Seg::Loop(p) => p,
            };
        }
        if sub.closed {
            path = path.close()?;
        }
    }
    Ok(path)
}

fn moved(subs: &[Sub], mut by: impl FnMut(P) -> P) -> Vec<Sub> {
    let mut out = Vec::new();
    for sub in subs {
        let mut segs = Vec::new();
        for seg in &sub.segs {
            segs.push(match *seg {
                Seg::Line(p) => Seg::Line(by(p)),
                Seg::Quad(c, p) => Seg::Quad(by(c), by(p)),
                Seg::Cubic(a, b, p) => Seg::Cubic(by(a), by(b), by(p)),
                Seg::Loop(p) => Seg::Loop(by(p)),
            });
        }
        out.push(Sub {
            start: by(sub.start),
            segs,
            closed: sub.closed,
        });
    }
    out
}

fn has_curves(subs: &[Sub]) -> bool {
    subs.iter()
        .any(|sub| sub.segs.iter().any(|seg| !matches!(seg, Seg::Line(_))))
}

// Generators ---------------------------------------------------------------------------------------------------------

type Subs = Vec<Sub>;

/// Corners on a 9 by 9 lattice, so edges coincide, touch at points, run collinear and cross at corners; some corners
/// repeat and some sub-paths are open.
fn lattice() -> BoxedStrategy<Subs> {
    let unit = prop_oneof![Just(1.0), Just(125.0), Just(0.001)];
    let corner = (0..9_i32, 0..9_i32, 1..3_usize);
    let sub = (prop::collection::vec(corner, 3..8), 0..8_u8);
    (unit, prop::collection::vec(sub, 1..4))
        .prop_map(|(unit, subs)| {
            subs.iter()
                .map(|(corners, open)| {
                    let corners: Vec<P> = corners
                        .iter()
                        .flat_map(|&(x, y, repeat)| vec![(f64::from(x) * unit, f64::from(y) * unit); repeat])
                        .collect();
                    polygon(&corners, *open != 0)
                })
                .collect()
        })
        .boxed()
}

/// One or two scribbles: random points in a 1000-unit square joined in the order they come.
fn blob() -> BoxedStrategy<Subs> {
    let corner = (0.0..1000.0_f64, 0.0..1000.0_f64);
    prop::collection::vec(prop::collection::vec(corner, 3..12), 1..3)
        .prop_map(|subs| subs.iter().map(|corners| polygon(corners, true)).collect())
        .boxed()
}

/// A star polygon {n/k}: every k-th point of n on a circle, so it winds `k` times and crosses itself.
fn star() -> BoxedStrategy<Subs> {
    (
        prop::sample::select(vec![5_usize, 7, 9, 11, 13]),
        2..6_usize,
        200.0..800.0_f64,
        200.0..800.0_f64,
        100.0..500.0_f64,
        any::<bool>(),
    )
        .prop_map(|(n, k, cx, cy, radius, backwards)| {
            let k = 1 + k % ((n - 1) / 2);
            let corners: Vec<P> = (0..n)
                .map(|i| {
                    #[expect(clippy::cast_precision_loss, reason = "a dozen points")]
                    let angle = 2.0 * PI * ((i * k) % n) as f64 / n as f64 * if backwards { -1.0 } else { 1.0 };
                    (cx + radius * libm::cos(angle), cy + radius * libm::sin(angle))
                })
                .collect();
            vec![polygon(&corners, true)]
        })
        .boxed()
}

/// Squares inside squares, each running the other way from the one outside it (rings), or all the same way (winding
/// number 2, 3, ... which the nonzero rule fills).
fn nested() -> BoxedStrategy<Subs> {
    (300.0..700.0_f64, 2..6_usize, 20.0..60.0_f64, any::<bool>())
        .prop_map(|(centre, levels, step, alternate)| {
            (0..levels)
                .map(|level| {
                    #[expect(clippy::cast_precision_loss, reason = "a few levels")]
                    let half = 300.0 - step * level as f64;
                    let mut corners = vec![
                        (centre - half, centre - half),
                        (centre + half, centre - half),
                        (centre + half, centre + half),
                        (centre - half, centre + half),
                    ];
                    if alternate && level % 2 == 1 {
                        corners.reverse();
                    }
                    polygon(&corners, true)
                })
                .collect()
        })
        .boxed()
}

/// A triangle or a rectangle 100 to 1000 long and `10^-9` to `10^-3` thick, slanted.
fn sliver() -> BoxedStrategy<Subs> {
    (
        log_uniform(-9.0, -3.0),
        100.0..1000.0_f64,
        0.0..500.0_f64,
        0.0..500.0_f64,
        -0.2..0.2_f64,
        any::<bool>(),
    )
        .prop_map(|(thick, length, x, y, slant, triangle)| {
            let far = (x + length, y + slant * length);
            let corners = if triangle {
                vec![(x, y), far, (x + length / 2.0, y + thick)]
            } else {
                vec![(x, y), far, (far.0, far.1 + thick), (x, y + thick)]
            };
            vec![polygon(&corners, true)]
        })
        .boxed()
}

/// A square of 800 whose every corner is followed by a segment of `10^-9` to `10^-3`.
fn tiny_steps() -> BoxedStrategy<Subs> {
    prop::collection::vec((log_uniform(-9.0, -3.0), 0.0..2.0 * PI), 4)
        .prop_map(|steps| {
            let base = [(100.0, 100.0), (900.0, 100.0), (900.0, 900.0), (100.0, 900.0)];
            let corners: Vec<P> = base
                .iter()
                .zip(&steps)
                .flat_map(|(&(x, y), &(length, angle))| {
                    [(x, y), (x + length * libm::cos(angle), y + length * libm::sin(angle))]
                })
                .collect();
            vec![polygon(&corners, true)]
        })
        .boxed()
}

/// Closed outlines of lines, quadratics, cubics and loops through random points of a 100-unit square.
fn curvy() -> BoxedStrategy<Subs> {
    let point = || (0.0..100.0_f64, 0.0..100.0_f64);
    let seg = prop_oneof![
        point().prop_map(Seg::Line),
        (point(), point()).prop_map(|(c, p)| Seg::Quad(c, p)),
        (point(), point(), point()).prop_map(|(a, b, p)| Seg::Cubic(a, b, p)),
        point().prop_map(Seg::Loop),
    ];
    let sub = (point(), prop::collection::vec(seg, 1..5)).prop_map(|(start, segs)| Sub {
        start,
        segs,
        closed: true,
    });
    prop::collection::vec(sub, 1..3).boxed()
}

/// Ten to thirty small triangles and squares in a 100-unit square, some open.
fn many() -> BoxedStrategy<Subs> {
    let one = (0.0..100.0_f64, 0.0..100.0_f64, 1.0..20.0_f64, any::<bool>(), 0..5_u8);
    prop::collection::vec(one, 10..30)
        .prop_map(|items| {
            items
                .iter()
                .map(|&(x, y, size, square, open)| {
                    let mut corners = vec![(x, y), (x + size, y), (x + size, y + size)];
                    if square {
                        corners.push((x, y + size));
                    }
                    polygon(&corners, open != 0)
                })
                .collect()
        })
        .boxed()
}

fn kind() -> BoxedStrategy<Subs> {
    prop_oneof![
        3 => lattice(),
        2 => blob(),
        2 => star(),
        2 => nested(),
        2 => sliver(),
        1 => tiny_steps(),
        3 => curvy(),
        1 => many(),
    ]
    .boxed()
}

/// A scene: the two operands and the size of the finest detail the generator itself makes (0 when it makes none).
/// Independent shapes, a shape and the same shape with every point moved by up to `JITTER` (edges nearly coincident), a
/// huge shape with a tiny one inside, or both at some other size.
fn scenes() -> BoxedStrategy<(Subs, Subs, f64)> {
    let near = (kind(), prop::collection::vec((-1.0..1.0_f64, -1.0..1.0_f64), 1..9)).prop_map(|(a, jitter)| {
        let mut next = 0;
        let b = moved(&a, |(x, y)| {
            let (jx, jy) = jitter[next % jitter.len()];
            next += 1;
            (x + JITTER * jx, y + JITTER * jy)
        });
        (a, b, JITTER)
    });
    let ratio = (blob(), blob(), 0.0..6.0_f64, -6.0..-3.0_f64).prop_map(|(big, small, up, down)| {
        let (up, down) = (libm::pow(10.0, up), libm::pow(10.0, down));
        (
            moved(&big, |(x, y)| (x * up, y * up)),
            moved(&small, |(x, y)| (500.0 * up + x * down, 500.0 * up + y * down)),
            0.0,
        )
    });
    // two straight-edged shapes, both 1e-100 to 1e60 times their size (curves would need a tolerance to match)
    let straight = || prop_oneof![lattice(), blob(), star(), nested(), sliver(), many()];
    let scaled = (straight(), straight(), -100.0..60.0_f64).prop_map(|(a, b, power)| {
        let by = libm::pow(10.0, power);
        (
            moved(&a, |(x, y)| (x * by, y * by)),
            moved(&b, |(x, y)| (x * by, y * by)),
            0.0,
        )
    });
    prop_oneof![5 => (kind(), kind(), Just(0.0)), 3 => near, 2 => ratio, 2 => scaled].boxed()
}

fn probe_units() -> impl Strategy<Value = Vec<[f64; 4]>> {
    prop::collection::vec([0.0..1.0_f64, 0.0..1.0, 0.0..1.0, 0.0..1.0], 60)
}

// A scene with its oracle --------------------------------------------------------------------------------------------

struct Scene {
    path_a: Path,
    path_b: Path,
    polys_a: Vec<Corners>,
    polys_b: Vec<Corners>,
    /// The largest coordinate, or the width of the scene if that is more.
    scale: f64,
    slack: f64,
}

fn fail(error: &impl ToString) -> TestCaseError {
    TestCaseError::fail(error.to_string())
}

fn region(path: &Path) -> Result<Vec<Ring>, TestCaseError> {
    rings(path).map_err(|e| fail(&e))
}

impl Scene {
    fn new(a: &[Sub], b: &[Sub], grain: f64) -> Result<Self, TestCaseError> {
        let (path_a, path_b) = (path_of(a)?, path_of(b)?);
        let polys_a = sampled(&path_a, CURVE_STEPS).map_err(|e| fail(&e))?;
        let polys_b = sampled(&path_b, CURVE_STEPS).map_err(|e| fail(&e))?;
        let scale = polys_a
            .iter()
            .chain(&polys_b)
            .flatten()
            .fold(0.0_f64, |most, &(x, y)| most.max(x.abs()).max(y.abs()));
        let curves = has_curves(a) || has_curves(b);
        Ok(Self {
            path_a,
            path_b,
            polys_a,
            polys_b,
            scale,
            slack: SLACK * scale + GRAIN * grain + if curves { CURVE_SLACK } else { 0.0 },
        })
    }

    fn polys(&self) -> Vec<Corners> {
        self.polys_a.iter().chain(&self.polys_b).cloned().collect()
    }

    /// Four results (union, subtract, intersect, exclude), then `a` and `b` resolved.
    fn results(&self) -> Result<Vec<Path>, TestCaseError> {
        let (a, b) = (&self.path_a, &self.path_b);
        Ok(vec![
            a.union(b, TOLERANCE)?,
            a.subtract(b, TOLERANCE)?,
            a.intersect(b, TOLERANCE)?,
            a.exclude(b, TOLERANCE)?,
            a.resolve(TOLERANCE)?,
            b.resolve(TOLERANCE)?,
        ])
    }

    /// Points to ask about: random ones, ones just off a corner and ones just off the middle of an edge, each farther
    /// than the slack from every edge.
    fn probes(&self, units: &[[f64; 4]]) -> Vec<P> {
        let polys = self.polys();
        let corners: Vec<P> = polys.iter().flatten().copied().collect();
        let edges: Vec<(P, P)> = polys
            .iter()
            .flat_map(|ring| ring.iter().enumerate().map(|(i, &p)| (p, ring[(i + 1) % ring.len()])))
            .collect();
        if corners.is_empty() {
            return Vec::new();
        }
        let (mut low, mut high) = ((f64::INFINITY, f64::INFINITY), (f64::NEG_INFINITY, f64::NEG_INFINITY));
        for &(x, y) in &corners {
            low = (low.0.min(x), low.1.min(y));
            high = (high.0.max(x), high.1.max(y));
        }
        let pick = |share: f64, count: usize| {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss,
                reason = "a share of a count"
            )]
            let index = (share * count as f64) as usize;
            index.min(count - 1)
        };
        let offsets = [3.0, 30.0, 1000.0, 1e5];
        units
            .iter()
            .enumerate()
            .map(|(number, u)| {
                let off = self.slack * offsets[pick(u[1], 4)];
                match number % 3 {
                    0 => (
                        low.0 + (u[0] * 1.2 - 0.1) * (high.0 - low.0),
                        low.1 + (u[1] * 1.2 - 0.1) * (high.1 - low.1),
                    ),
                    1 => {
                        let (x, y) = corners[pick(u[0], corners.len())];
                        (
                            x + off * libm::cos(2.0 * PI * u[2]),
                            y + off * libm::sin(2.0 * PI * u[2]),
                        )
                    }
                    _ => {
                        let (from, to) = edges[pick(u[0], edges.len())];
                        let length = libm::hypot(to.0 - from.0, to.1 - from.1);
                        let side = if u[2] < 0.5 { off } else { -off };
                        (
                            from.0 + u[3] * (to.0 - from.0) - side * (to.1 - from.1) / length,
                            from.1 + u[3] * (to.1 - from.1) + side * (to.0 - from.0) / length,
                        )
                    }
                }
            })
            .filter(|&point| point.0.is_finite() && point.1.is_finite() && nearest_edge(&polys, point) > self.slack)
            .collect()
    }

    /// The most a result's area may differ from the exact one: every edge moved by the slack, and the rounding of the
    /// area itself.
    fn area_bound(&self) -> f64 {
        let perimeter: f64 = self
            .polys()
            .iter()
            .map(|ring| {
                ring.iter()
                    .enumerate()
                    .map(|(i, &p)| crate::common::distance(p, ring[(i + 1) % ring.len()]))
                    .sum::<f64>()
            })
            .sum();
        self.slack * perimeter + 1e-12 * self.scale * self.scale
    }
}

// Properties ---------------------------------------------------------------------------------------------------------

/// Every result region against the set rule applied to the winding of the inputs, at the probes; and its area
/// against the exact area.
/// A verb's name, its set rule, and the polygons the rule is asked of.
type Row<'a> = (&'a str, fn(bool, bool) -> bool, &'a [Corners], &'a [Corners]);

fn check_against_the_rule(scene: &Scene, units: &[[f64; 4]]) -> Result<(), TestCaseError> {
    let results = scene.results()?;
    let bound = scene.area_bound();
    let none: &[Corners] = &[];
    let (a, b) = (scene.polys_a.as_slice(), scene.polys_b.as_slice());
    // (name, the set rule, the polygons it is asked of)
    let rows: [Row; 6] = [
        ("union", |x, y| x || y, a, b),
        ("subtract", |x, y| x && !y, a, b),
        ("intersect", |x, y| x && y, a, b),
        ("exclude", |x, y| x != y, a, b),
        ("resolve a", |x, _| x, a, none),
        ("resolve b", |x, _| x, b, none),
    ];
    let exact = exact_areas(a, b, &[rows[0].1, rows[1].1, rows[2].1, rows[3].1]);
    let exact_a = exact_areas(a, none, &[rows[4].1]);
    let exact_b = exact_areas(b, none, &[rows[5].1]);
    let exact = [exact[0], exact[1], exact[2], exact[3], exact_a[0], exact_b[0]];
    for ((path, (name, rule, polys_a, polys_b)), exact) in results.iter().zip(rows).zip(exact) {
        let found = region(path)?;
        let got = net_area(&found);
        prop_assert!(
            (got - exact).abs() <= bound,
            "{name}: area {got}, exact {exact}, bound {bound}"
        );
        for point in scene.probes(units) {
            let want = rule(inside(polys_a, point), inside(polys_b, point));
            prop_assert_eq!(inside(&found, point), want, "{} at {:?}", name, point);
        }
    }
    Ok(())
}

proptest! {
    #[test]
    fn a_point_clear_of_every_edge_is_in_the_result_when_the_set_rule_says_so_and_the_area_is_exact(
        (a, b, grain) in scenes(),
        units in probe_units(),
    ) {
        check_against_the_rule(&Scene::new(&a, &b, grain)?, &units)?;
    }

    #[test]
    fn moving_both_shapes_by_any_vector_does_not_change_the_answer(
        (a, b, grain) in scenes(),
        units in probe_units(),
        by in (-1.0e4..1.0e4_f64, -1.0e4..1.0e4_f64),
    ) {
        let (a, b) = (moved(&a, |(x, y)| (x + by.0, y + by.1)), moved(&b, |(x, y)| (x + by.0, y + by.1)));
        check_against_the_rule(&Scene::new(&a, &b, grain)?, &units)?;
    }

    #[test]
    fn every_result_is_canonical_and_does_not_cross_itself(
        (a, b, grain) in scenes(),
    ) {
        let scene = Scene::new(&a, &b, grain)?;
        for (number, path) in scene.results()?.iter().enumerate() {
            let found = region(path)?;
            prop_assert_eq!(canonical_fault(&found), None, "result {}", number);
            prop_assert_eq!(crossing_fault(&found, scene.slack), None, "result {}", number);
        }
    }

    #[test]
    fn resolving_a_result_again_keeps_its_region(
        (a, b, grain) in scenes(),
        units in probe_units(),
    ) {
        let scene = Scene::new(&a, &b, grain)?;
        let bound = scene.area_bound();
        for (number, path) in scene.results()?.iter().enumerate() {
            let again = path.resolve(TOLERANCE)?;
            let (before, after) = (region(path)?, region(&again)?);
            prop_assert_eq!(canonical_fault(&after), None);
            prop_assert!(
                (net_area(&before) - net_area(&after)).abs() <= bound,
                "result {}: {} then {}", number, net_area(&before), net_area(&after)
            );
            // the second run may move an edge by the slack, so probes keep that far from the first result's edges
            for point in scene.probes(&units) {
                if nearest_edge(&before, point) > 2.0 * scene.slack {
                    prop_assert_eq!(inside(&before, point), inside(&after, point), "result {} at {:?}", number, point);
                }
            }
        }
    }

    #[test]
    fn the_four_results_relate_as_sets_do(
        (a, b, grain) in scenes(),
        units in probe_units(),
    ) {
        let scene = Scene::new(&a, &b, grain)?;
        let bound = scene.area_bound();
        let found: Vec<Vec<Ring>> = scene.results()?.iter().map(region).collect::<Result<_, _>>()?;
        let area: Vec<f64> = found.iter().map(|rings| net_area(rings)).collect();
        // |A u B| + |A n B| = |A| + |B|; |A \ B| + |A n B| = |A|; |A ^ B| = |A u B| - |A n B|
        prop_assert!((area[0] + area[2] - area[4] - area[5]).abs() <= 4.0 * bound, "union {:?}", area);
        prop_assert!((area[1] + area[2] - area[4]).abs() <= 3.0 * bound, "subtract {:?}", area);
        prop_assert!((area[3] - area[0] + area[2]).abs() <= 3.0 * bound, "exclude {:?}", area);
        // the verbs the other way round
        let (a, b) = (&scene.path_a, &scene.path_b);
        let swapped = [b.union(a, TOLERANCE)?, b.intersect(a, TOLERANCE)?, b.exclude(a, TOLERANCE)?];
        for (number, other) in [(0, 0), (2, 1), (3, 2)].iter().map(|&(x, y)| (x, &swapped[y])) {
            let other = region(other)?;
            prop_assert!((net_area(&found[number]) - net_area(&other)).abs() <= 2.0 * bound, "{} swapped", VERBS[number]);
            for point in scene.probes(&units) {
                let near = nearest_edge(&found[number], point).min(nearest_edge(&other, point));
                if near > 2.0 * scene.slack {
                    prop_assert_eq!(inside(&found[number], point), inside(&other, point), "{} swapped at {:?}", VERBS[number], point);
                }
            }
        }
        // the intersection lies in both, the difference outside the second, and they make the first
        for point in scene.probes(&units) {
            let clear = found.iter().all(|rings| nearest_edge(rings, point) > 2.0 * scene.slack);
            if clear {
                let (in_intersect, in_subtract) = (inside(&found[2], point), inside(&found[1], point));
                prop_assert!(!in_intersect || (inside(&found[4], point) && inside(&found[5], point)), "intersect at {:?}", point);
                prop_assert!(!in_subtract || !inside(&found[5], point), "subtract at {:?}", point);
                prop_assert_eq!(in_intersect || in_subtract, inside(&found[4], point), "split of a at {:?}", point);
            }
        }
    }

    #[test]
    fn a_path_made_of_nothing_but_lone_points_and_lines_has_no_region(
        points in prop::collection::vec((-1.0e3..1.0e3_f64, -1.0e3..1.0e3_f64), 1..9),
    ) {
        let mut path = Path::new();
        for &point in &points {
            path = path.move_to(point)?;
        }
        path = path.move_to(points[0])?.line_to(points[points.len() - 1])?;
        prop_assert_eq!(path.resolve(TOLERANCE)?.to_svg_path(), "");
        let square = polygons_path(&[vec![(0.0, 0.0), (9.0, 0.0), (9.0, 9.0), (0.0, 9.0)]])?;
        // lone points and lines enclose nothing and do not take part: not even the grid the square is rounded to
        prop_assert_eq!(
            square.union(&path, TOLERANCE)?.to_svg_path(),
            square.resolve(TOLERANCE)?.to_svg_path()
        );
    }
}

// Shapes beside far larger ones -----------------------------------------------------------------------------------------

/// The engine rounds to a frame as wide as all of its input, so a result that is only a small piece of it is rounded
/// as coarsely as the input is large (a corner at 0.006 moves by 1e-13 beside a shape at 700). Found by the fuzz: the
/// hair a subtract left (a hole of area 3e-18 outside every outer ring) was kept, as it was judged by the result's
/// own size.
#[test]
fn a_hair_left_beside_a_far_larger_shape_is_not_a_hole() -> Result<(), TestCaseError> {
    let open = |start: P, rest: &[P]| polygon(&[&[start], rest].concat(), false);
    let a = vec![
        open((0.003, 0.0), &[(0.004, 0.0), (0.008, 0.002), (0.006, 0.001)]),
        open((0.0, 0.005), &[(0.008, 0.0), (0.0, 0.006)]),
    ];
    let square = |near: f64, far: f64| polygon(&[(near, near), (far, near), (far, far), (near, far)], true);
    let b = vec![
        square(113.63698046060347, 713.6369804606035),
        square(133.63698046060347, 693.6369804606035),
    ];
    let scene = Scene::new(&a, &b, 0.0)?;
    for (number, path) in scene.results()?.iter().enumerate() {
        let found = region(path)?;
        prop_assert_eq!(canonical_fault(&found), None, "result {}", number);
        prop_assert_eq!(crossing_fault(&found, scene.slack), None, "result {}", number);
    }
    Ok(())
}

/// A shape beside a copy nudged by 1e-7, moved by 7759.6: the move rounds the exact overlap of two edges of the shape
/// (a corner on another edge, doubled back) into a sliver 1e-12 thick, and the engine then loses a quadrilateral of area
/// 8e-16 at the corner, 1e-8 thick and 7e-8 long, which the oracle says is inside both shapes (api.md, "Booleans").
/// Found by the fuzz; the scene claims what is above its jitter: the area is exact and every probe 1.4e-6 from an edge
/// is right.
#[test]
fn a_detail_the_size_of_the_jitter_may_vanish_but_the_area_and_everything_clear_of_it_hold() -> Result<(), TestCaseError>
{
    let a = vec![polygon(
        &[
            (250.0, 375.0),
            (500.0, 125.0),
            (250.0, 500.0),
            (250.0, 500.0),
            (375.0, 250.0),
            (375.0, 250.0),
        ],
        false,
    )];
    let b = vec![polygon(
        &[
            (250.000_000_065_906_6, 375.000_000_018_706_1),
            (500.0, 124.999_999_958_428_67),
            (250.000_000_065_906_6, 500.000_000_018_706_1),
            (250.0, 499.999_999_958_428_7),
            (375.000_000_065_906_6, 250.000_000_018_706_1),
            (375.0, 249.999_999_958_428_67),
        ],
        false,
    )];
    let by = 7759.609_348_622_621;
    let (a, b) = (moved(&a, |(x, y)| (x + by, y)), moved(&b, |(x, y)| (x + by, y)));
    // probes on the first corner of every edge's neighbourhood, then each corner in turn, just off it
    let units: Vec<[f64; 4]> = (0..60).map(|k| [f64::from(k) / 60.0, 0.0, 0.0, 0.0]).collect();
    check_against_the_rule(&Scene::new(&a, &b, JITTER)?, &units)
}
