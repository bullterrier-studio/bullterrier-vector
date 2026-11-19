//! Every verb that reads a field charges the reads against one budget before the work runs away: a tone that is slow to
//! read (a deep sum of noise) is refused, naming the argument that sets how often it is read, while the same call over
//! a ramp draws. The charge of a read follows the documented rule: a unit for a ramp or an operation, a unit for each
//! octave of noise.

use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{
    Bands, Color, Drawing, Error, Field, Flow, Halftone, Hatch, Path, Rect, Rows, Size, Stipple, Trace, Values,
};

/// A balanced sum of `leaves` copies of `leaf`: `2 * leaves - 1` nodes, each copy read at every read of the sum.
fn balanced(leaf: &Field, leaves: usize) -> Result<Field, Error> {
    if leaves == 1 {
        return Ok(leaf.clone());
    }
    balanced(leaf, leaves / 2)?.add(&balanced(leaf, leaves - leaves / 2)?)
}

/// A balanced mix of `leaves` copies of `leaf`, half and half at each level.
fn balanced_flow(leaf: &Flow, leaves: usize) -> Result<Flow, Error> {
    if leaves == 1 {
        return Ok(leaf.clone());
    }
    balanced_flow(leaf, leaves / 2)?.mix(&balanced_flow(leaf, leaves - leaves / 2)?, 0.5)
}

/// The deepest sum of eight-octave noise arithmetic allows: 4096 leaves, 8191 nodes, about 37,000 units a read.
fn deep() -> Result<Field, Error> {
    balanced(&Field::noise(40.0, 0.0, 1.0, 8)?, 4096)
}

fn rect(width: f64, height: f64) -> Result<Path, Error> {
    Path::rect(0.0, 0.0, width, height, 0.0)
}

/// A verb that reads a field: its name, the argument its refusal names, and a call of it over a tone.
type Verb = (&'static str, &'static str, fn(&Field) -> Result<(), Error>);

/// The verbs that read a field, each over about 40,000 samples.
fn verbs() -> Vec<Verb> {
    vec![
        ("hatch", "spacing", |tone| {
            rect(400.0, 200.0)?.hatch(&Hatch::between(2.0, 8.0, tone.clone()), None)?;
            Ok(())
        }),
        ("rows", "spacing", |tone| {
            rect(400.0, 400.0)?.rows(&Rows::new(2.0, tone.clone(), 1.0)?)?;
            Ok(())
        }),
        ("bands", "spacing", |tone| {
            rect(400.0, 400.0)?.bands(&Bands::lines(2.0, tone.clone(), (0.2, 1.0))?)?;
            Ok(())
        }),
        ("stipple", "spacing", |tone| {
            rect(400.0, 400.0)?.stipple(&Stipple::new(2.0, tone.clone(), 0.5)?)?;
            Ok(())
        }),
        ("halftone", "pitch", |tone| {
            let drawing =
                Drawing::from(rect(400.0, 400.0)?).halftone(Halftone::new(Color::hex(0), tone.clone()).pitch(2.0));
            drawing.to_svg(Size::px(400.0, 400.0), &Values::new())?;
            Ok(())
        }),
        ("trace_levels", "cell", |tone| {
            let trace = Trace::new()
                .frame(Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 200.0,
                    height: 200.0,
                })
                .cell(1.0);
            Drawing::trace_levels(tone, [0.5], &trace).shapes(&Values::new())?;
            Ok(())
        }),
        ("contours", "cell", |tone| {
            tone.contours(
                &[0.5],
                Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 200.0,
                    height: 200.0,
                },
                1.0,
            )?;
            Ok(())
        }),
    ]
}

#[test]
fn every_verb_that_reads_a_field_refuses_a_tone_too_slow_for_its_samples() -> Outcome {
    let (deep, ramp) = (deep()?, Field::ramp((0.0, 0.0), (400.0, 400.0))?);
    for (verb, parameter, draw) in verbs() {
        // the control: the same call over a cheap tone draws, so the refusal below is the cost of the tone
        draw(&ramp).map_err(|error| format!("{verb} over a ramp: {error}"))?;
        let (named, message) = parameter_refusal(draw(&deep)).map_err(|error| format!("{verb}: {error}"))?;
        assert!(named.ends_with(parameter), "{verb} names {named}, not {parameter}");
        assert!(message.contains("units of work"), "{verb}: {message}");
    }
    Ok(())
}

#[test]
fn hatch_charges_the_reads_of_its_lines_as_it_draws_them() -> Outcome {
    // a sum of 4096 ramps costs 8191 units a read: the 2,500 starts of the lattice are 20 million units, well inside the
    // budget, so only the reads of the lines it draws (thousands of steps, several reads each) can pass it
    let tone = balanced(&Field::ramp((0.0, 0.0), (100.0, 100.0))?, 4096)?;
    let (named, message) = parameter_refusal(rect(100.0, 100.0)?.hatch(&Hatch::between(2.0, 2.0, tone), None))?;
    assert_eq!(named, "spacing");
    assert!(message.contains("units of work"), "{message}");
    // no tone, and a mix of 64 angles: 127 units a read, a few nanoseconds in fact, so a million steps of three reads each
    // pass the budget well before they are drawn
    let flow = balanced_flow(&Flow::angle(30.0)?, 64)?;
    let refused = rect(1000.0, 1000.0)?.hatch(&Hatch::new(2.0).flow(flow), None);
    assert_eq!(parameter_refusal(refused)?.0, "spacing");
    Ok(())
}

#[test]
fn hatch_charges_the_starts_of_its_lattice_before_it_draws_a_line() -> Outcome {
    // no line starts below -5000, as a sum of 4096 noises is never under -4096: only the lattice reads the tone, once at each
    // start and once more for `below`
    let hatch = Hatch::between(2.0, 8.0, deep()?).below(-5000.0);
    let (named, message) = parameter_refusal(rect(400.0, 200.0)?.hatch(&hatch, None))?;
    assert_eq!(named, "spacing");
    assert!(message.contains("units of work"), "{message}");
    Ok(())
}

#[test]
fn a_normal_hatch_over_a_noise_tone_draws() -> Outcome {
    let tone = Field::noise(60.0, 0.0, 2.0, 3)?
        .mul(&Field::constant(0.5)?)?
        .add(&Field::constant(0.5)?)?;
    let lines = rect(400.0, 200.0)?.hatch(
        &Hatch::between(3.0, 9.0, tone).flow(Flow::noise(200.0, 0.0, 1.0)?),
        None,
    )?;
    assert_ne!(lines.to_svg_path(), "");
    Ok(())
}

#[test]
fn a_read_of_noise_costs_a_unit_an_octave_and_an_operation_a_unit() -> Outcome {
    // the documented rule, worked out here: 64 leaves of 8 octaves and 63 additions
    let tone = balanced(&Field::noise(40.0, 0.0, 1.0, 8)?, 64)?;
    let each = 64 * 8 + 63;
    let message = parameter_refusal(tone.contours(
        &[0.5],
        Rect {
            x: 0.0,
            y: 0.0,
            width: 700.0,
            height: 700.0,
        },
        1.0,
    ))?
    .1;
    assert!(message.contains(&format!("at a cost of {each} each")), "{message}");
    Ok(())
}
