//! `Drawing::to_plot_svg`: strokes only, one layer per pen, ordered to cut pen-up travel, simplified within a tolerance,
//! solids as pen-width hatching. Oracles: the SVG is parsed back (`roxmltree`); the travel of the order is measured
//! here, compared with the given order and with a brute-force search over every order and direction of a few strokes;
//! simplification is checked against the original points by their distance to the result; hatching by the length
//! it must have (area over pen width).

use crate::common::{Outcome, parameter_refusal, to_segment};
use bullterrier_vector::{Color, Drawing, Error, Path, Plot, Shadow, Size, Stroke, Values};

type Line = Vec<(f64, f64)>;

/// A pen: its stroke colour, whether it is a layer, and its lines in order.
struct Pen {
    colour: String,
    layer: bool,
    lines: Vec<Line>,
}

fn plot(drawing: &Drawing, side: f64, options: &Plot) -> Result<String, Error> {
    drawing.to_plot_svg(Size::px(side, side), &Values::new(), options)
}

fn pens(svg: &str) -> Result<Vec<Pen>, Box<dyn std::error::Error>> {
    let document = roxmltree::Document::parse(svg)?;
    let mut found = Vec::new();
    for group in document.root_element().children().filter(roxmltree::Node::is_element) {
        assert_eq!(group.tag_name().name(), "g", "only layers under the root");
        assert_eq!(group.attribute("fill"), Some("none"), "a plot has no fill");
        let layer = svg.contains("inkscape:groupmode=\"layer\"");
        let mut lines = Vec::new();
        for path in group.children().filter(roxmltree::Node::is_element) {
            assert_eq!(path.tag_name().name(), "path");
            let numbers: Vec<f64> = path
                .attribute("d")
                .ok_or("a path without data")?
                .split(|c: char| c.is_ascii_alphabetic() || c == ' ')
                .filter_map(|piece| piece.parse().ok())
                .collect();
            lines.push(numbers.chunks(2).map(|pair| (pair[0], pair[1])).collect());
        }
        found.push(Pen {
            colour: group
                .attribute("stroke")
                .ok_or("a layer without a pen colour")?
                .to_owned(),
            layer,
            lines,
        });
    }
    Ok(found)
}

fn red() -> Result<Color, Error> {
    Color::parse("#ff0000")
}

fn blue() -> Result<Color, Error> {
    Color::parse("#0000ff")
}

fn dist(a: (f64, f64), b: (f64, f64)) -> f64 {
    libm::hypot(a.0 - b.0, a.1 - b.1)
}

/// The pen-up travel of `lines` drawn in this order from the origin.
fn travel(lines: &[Line]) -> f64 {
    let mut at = (0.0, 0.0);
    let mut total = 0.0;
    for line in lines {
        total += dist(at, line[0]);
        at = line[line.len() - 1];
    }
    total
}

/// A short list of deterministic pseudo-random numbers in 0..100.
fn scatter(count: usize, seed: u64) -> Vec<f64> {
    let mut state = seed;
    (0..count)
        .map(|_| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            f64::from(u32::try_from(state >> 40).unwrap_or(0) % 1000) / 10.0
        })
        .collect()
}

/// `count` separate short strokes, as a drawing in the order they are made.
fn strokes(count: usize, seed: u64) -> Result<(Drawing, Vec<Line>), Error> {
    let n = scatter(count * 4, seed);
    let mut lines = Vec::new();
    let mut shapes = Vec::new();
    for k in 0..count {
        let line = vec![(n[4 * k], n[4 * k + 1]), (n[4 * k + 2], n[4 * k + 3])];
        shapes.push(Path::polyline(line.clone(), false)?.stroke(Stroke::new(red()?).width(1.0)));
        lines.push(line);
    }
    Ok((Drawing::new(shapes), lines))
}

#[test]
fn only_strokes_are_written_one_layer_per_pen_in_the_order_the_pens_first_appear() -> Outcome {
    let line = |y: f64| Path::polyline([(0.0, y), (50.0, y)], false);
    let drawing = Drawing::new(vec![
        line(10.0)?.stroke(Stroke::new(blue()?).width(3.0)),
        line(20.0)?.stroke(Stroke::new(red()?).width(1.0)),
        line(30.0)?.stroke(Stroke::new(blue()?).width(2.0)),
        Path::circle((50.0, 50.0), 5.0)?.stroke(Stroke::new(red()?).width(1.0)),
    ]);
    let found = pens(&plot(&drawing, 100.0, &Plot::new(0.5)?)?)?;
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].colour, "#0000ff");
    assert_eq!(found[1].colour, "#ff0000");
    assert_eq!((found[0].lines.len(), found[1].lines.len()), (2, 2));
    assert!(found.iter().all(|pen| pen.layer));
    // a plot has no fill and the stroke widths of the shapes are the pen's
    let svg = plot(&drawing, 100.0, &Plot::new(0.5)?)?;
    assert!(svg.contains("stroke-width=\"0.5\"") && !svg.contains("stroke-width=\"3\""));
    Ok(())
}

#[test]
fn a_pen_is_one_layer_whether_it_strokes_or_fills() -> Outcome {
    let drawing = Drawing::new(vec![
        Path::rect(0.0, 0.0, 20.0, 20.0, 0.0)?.fill(red()?),
        Path::polyline([(30.0, 30.0), (60.0, 60.0)], false)?.stroke(Stroke::new(red()?).width(1.0)),
    ]);
    let found = pens(&plot(&drawing, 100.0, &Plot::new(1.0)?)?)?;
    assert_eq!(found.len(), 1);
    Ok(())
}

#[test]
fn the_numbers_are_in_output_pixels() -> Outcome {
    let drawing = Path::polyline([(10.0, 10.0), (20.0, 30.0)], false)
        .map(|path| path.stroke(Stroke::new(red().unwrap_or(Color::NONE)).width(1.0)))?
        .with_size(100.0, 100.0);
    let found = pens(&plot(&drawing, 300.0, &Plot::new(1.0)?.tolerance(0.0))?)?;
    assert_eq!(found[0].lines[0], vec![(30.0, 30.0), (60.0, 90.0)]);
    assert!(plot(&drawing, 300.0, &Plot::new(1.0)?)?.contains("stroke-width=\"3\""));
    Ok(())
}

#[test]
fn the_order_has_less_pen_up_travel_than_the_given_order_and_is_the_nearest_neighbour_one() -> Outcome {
    for (count, seed) in [(40, 7), (700, 11)] {
        nearest_neighbour_case(count, seed)?;
    }
    Ok(())
}

fn nearest_neighbour_case(count: usize, seed: u64) -> Outcome {
    let (drawing, given) = strokes(count, seed)?;
    let found = pens(&plot(&drawing, 100.0, &Plot::new(1.0)?.tolerance(0.0))?)?;
    let ordered = &found[0].lines;
    assert_eq!(ordered.len(), count);
    assert!(
        travel(ordered) < 0.5 * travel(&given),
        "{} against {}",
        travel(ordered),
        travel(&given)
    );
    // brute force: from the pen, the nearest end of any stroke left (a stroke can be drawn either way), tie to the first
    let mut left = given.clone();
    let mut at = (0.0, 0.0);
    for line in ordered {
        let mut best: Option<(f64, usize, bool)> = None;
        for (k, other) in left.iter().enumerate() {
            for flip in [false, true] {
                let start = if flip { other[other.len() - 1] } else { other[0] };
                let d = dist(at, start);
                if best.is_none_or(|b| d < b.0 - 1e-12) {
                    best = Some((d, k, flip));
                }
            }
        }
        let (_, k, flip) = best.ok_or("no stroke left")?;
        let mut want = left.remove(k);
        if flip {
            want.reverse();
        }
        assert_eq!(*line, want);
        at = line[line.len() - 1];
    }
    Ok(())
}

/// The least pen-up travel over every order and direction of `lines`.
fn optimal(lines: &[Line]) -> f64 {
    fn go(left: &mut Vec<Line>, at: (f64, f64), so_far: f64, best: &mut f64) {
        if so_far >= *best {
            return;
        }
        if left.is_empty() {
            *best = so_far;
            return;
        }
        for k in 0..left.len() {
            let line = left.remove(k);
            for flip in [false, true] {
                let (start, end) = if flip { (line[1], line[0]) } else { (line[0], line[1]) };
                go(left, end, so_far + dist(at, start), best);
            }
            left.insert(k, line);
        }
    }
    let mut best = f64::MAX;
    go(&mut lines.to_vec(), (0.0, 0.0), 0.0, &mut best);
    best
}

#[test]
fn on_small_cases_the_travel_is_within_a_measured_bound_of_the_best_order() -> Outcome {
    for seed in 1..=12 {
        let (drawing, given) = strokes(7, seed)?;
        let found = pens(&plot(&drawing, 100.0, &Plot::new(1.0)?.tolerance(0.0))?)?;
        let (got, best) = (travel(&found[0].lines), optimal(&given));
        assert!(got >= best - 1e-9, "seed {seed}: {got} under the optimum {best}");
        assert!(
            got <= 1.6 * best + 1e-9,
            "seed {seed}: {got} against the optimum {best}"
        );
        assert!(
            got <= travel(&given) + 1e-9,
            "seed {seed}: no worse than the given order"
        );
    }
    Ok(())
}

#[test]
fn a_closed_loop_is_entered_at_its_nearest_point() -> Outcome {
    // a pen at the origin and a square whose nearest corner is not its start
    let square = Path::polyline([(50.0, 50.0), (60.0, 50.0), (60.0, 60.0), (50.0, 60.0)], true)?;
    let drawing = Drawing::new(vec![
        Path::polyline([(1.0, 0.0), (2.0, 0.0)], false)?.stroke(Stroke::new(red()?).width(1.0)),
        square.stroke(Stroke::new(red()?).width(1.0)),
        Path::polyline([(55.0, 52.0), (95.0, 52.0)], false)?.stroke(Stroke::new(red()?).width(1.0)),
    ]);
    let found = pens(&plot(&drawing, 100.0, &Plot::new(1.0)?.tolerance(0.0))?)?;
    let lines = &found[0].lines;
    // after the first stroke ends at (2, 0) the loop is entered at its corner nearest, (50, 50), and is closed back there
    assert_eq!(lines[1][0], (50.0, 50.0));
    assert_eq!(lines[1][lines[1].len() - 1], (50.0, 50.0));
    assert_eq!(lines[1].len(), 5);
    Ok(())
}

#[test]
fn simplifying_keeps_every_original_point_within_the_tolerance_of_the_result() -> Outcome {
    let noise = scatter(600, 3);
    let points: Line = (0..600)
        .map(|i| {
            let x = f64::from(u16::try_from(i).unwrap_or(0)) * 0.5;
            (x, 40.0 + 20.0 * libm::sin(x / 15.0) + noise[i] / 1000.0)
        })
        .collect();
    let drawing = Path::polyline(points.clone(), false)?.stroke(Stroke::new(red()?).width(1.0));
    let tolerance = 0.4;
    let found = pens(&plot(&drawing, 300.0, &Plot::new(1.0)?.tolerance(tolerance))?)?;
    let simple = &found[0].lines[0];
    assert!(simple.len() < points.len() / 4, "{} of {}", simple.len(), points.len());
    assert_eq!((simple[0], simple[simple.len() - 1]), (points[0], points[599]));
    for p in &points {
        let nearest = simple
            .windows(2)
            .map(|w| to_segment(*p, w[0], w[1]))
            .fold(f64::MAX, f64::min);
        assert!(nearest <= tolerance + 1e-9, "{p:?} is {nearest} from the result");
    }
    // no tolerance keeps every point
    let exact = pens(&plot(&drawing, 300.0, &Plot::new(1.0)?.tolerance(0.0))?)?;
    assert_eq!(exact[0].lines[0].len(), 600);
    Ok(())
}

#[test]
fn a_solid_is_hatched_at_the_pen_width_so_the_lines_have_the_length_of_area_over_width() -> Outcome {
    let drawing = Path::rect(10.0, 10.0, 40.0, 20.0, 0.0)?.fill(red()?);
    let found = pens(&plot(&drawing, 100.0, &Plot::new(2.0)?)?)?;
    let length: f64 = found[0]
        .lines
        .iter()
        .map(|line| line.windows(2).map(|w| dist(w[0], w[1])).sum::<f64>())
        .sum();
    assert!((length - 800.0 / 2.0).abs() < 0.1 * 400.0, "{length}");
    for line in &found[0].lines {
        for p in line {
            assert!(
                (10.0 - 1e-6..=50.0 + 1e-6).contains(&p.0) && (10.0 - 1e-6..=30.0 + 1e-6).contains(&p.1),
                "{p:?}"
            );
        }
    }
    // a solid smaller than the pen is its outline
    let dot = Path::circle((50.0, 50.0), 0.4)?.fill(red()?);
    assert_ne!(pens(&plot(&dot, 100.0, &Plot::new(2.0)?)?)?[0].lines.len(), 0);
    Ok(())
}

#[test]
fn what_a_pen_cannot_draw_is_refused_naming_the_shape_or_the_argument() -> Outcome {
    let line = || Path::polyline([(0.0, 0.0), (10.0, 10.0)], false);
    let stroked = line()?.stroke(Stroke::new(red()?).width(1.0));
    let drawing = Drawing::new(vec![stroked.clone()]);
    for width in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let refusal = parameter_refusal(Plot::new(width).and_then(|plot_| plot(&drawing, 100.0, &plot_)))?;
        assert_eq!(refusal.0, "pen_width", "{width}");
    }
    for tolerance in [-1.0, f64::NAN] {
        let refusal = parameter_refusal(Plot::new(1.0).and_then(|p| plot(&drawing, 100.0, &p.tolerance(tolerance))))?;
        assert_eq!(refusal.0, "tolerance");
    }
    let gradient = Path::rect(0.0, 0.0, 9.0, 9.0, 0.0)?.fill(bullterrier_vector::Gradient::linear(
        (0.0, 0.0),
        (9.0, 0.0),
        [(0.0, red()?), (1.0, blue()?)],
    )?);
    let shadowed = line()?.stroke(Stroke::new(red()?).width(1.0)).shadow(Shadow::new());
    for (drawing, what) in [(gradient, "gradient"), (shadowed, "shadow")] {
        let refusal = parameter_refusal(plot(&drawing, 100.0, &Plot::new(1.0)?))?;
        assert_eq!(refusal.0, "shape 0", "{what}");
    }
    let refusal = parameter_refusal(drawing.to_plot_svg(Size::px(0.0, 10.0), &Values::new(), &Plot::new(1.0)?))?;
    assert_eq!(refusal.0, "width");
    // an empty drawing is a document with no layer
    assert!(pens(&plot(&Drawing::new(Vec::<Drawing>::new()), 10.0, &Plot::new(1.0)?)?)?.is_empty());
    Ok(())
}

#[test]
fn the_same_drawing_gives_the_same_plot() -> Outcome {
    let (drawing, _) = strokes(30, 5)?;
    let options = Plot::new(1.0)?;
    assert_eq!(plot(&drawing, 100.0, &options)?, plot(&drawing, 100.0, &options)?);
    Ok(())
}
