//! The picture the SVG makes of `multiply`, against the pixels `to_rgba` draws: inks over paper, inks on nothing, a
//! group at an opacity, drawn by a second renderer (`resvg`). The markup is in `svg_shapes`; this is what it shows.

use crate::common::Outcome;
use crate::common::svg_oracle::svg_agrees;
use bullterrier_vector::{Blend, Cap, Color, Drawing, Error, Path, Stroke};

fn color(text: &str) -> Result<Color, Error> {
    Color::parse(text)
}

fn disc(x: f64, y: f64, radius: f64, fill: &str) -> Result<Drawing, Error> {
    Ok(Drawing::from(Path::circle((x, y), radius)?).fill(color(fill)?))
}

fn paper() -> Result<Drawing, Error> {
    Ok(Drawing::from(Path::rect(0.0, 0.0, 120.0, 120.0, 0.0)?).fill(color("#e8dcc0")?))
}

/// Three inks that overlap, each multiplied.
fn inks() -> Result<Drawing, Error> {
    Ok(Drawing::new([
        disc(45.0, 50.0, 28.0, "#00b4e6")?.blend(Blend::Multiply),
        disc(75.0, 50.0, 28.0, "#ec008c")?.blend(Blend::Multiply),
        disc(60.0, 78.0, 28.0, "#fff200")?.blend(Blend::Multiply),
    ]))
}

#[test]
fn the_svg_multiplies_as_the_pixels_do() -> Outcome {
    let rows = [
        ("inks over paper", Drawing::new([paper()?, inks()?])),
        ("inks on nothing", inks()?),
        (
            "an ink over a paper that is a translucent rectangle",
            Drawing::new([
                Drawing::from(Path::rect(10.0, 10.0, 100.0, 100.0, 0.0)?).fill(color("#e8dcc080")?),
                inks()?,
            ]),
        ),
        (
            "a group at an opacity, fill and stroke together",
            Drawing::new([
                paper()?,
                Drawing::from(Path::circle((50.0, 60.0), 30.0)?)
                    .fill(color("#ec008c")?)
                    .stroke(Stroke::new(color("#00b4e6")?).width(14.0))
                    .opacity(0.6)
                    .blend(Blend::Multiply),
            ]),
        ),
        (
            "a translucent ink",
            Drawing::new([
                paper()?,
                disc(50.0, 60.0, 30.0, "#00b4e6")?,
                disc(70.0, 60.0, 30.0, "#ec008c90")?.blend(Blend::Multiply),
            ]),
        ),
        (
            "a stroke only",
            Drawing::new([
                paper()?,
                disc(50.0, 60.0, 30.0, "#fff200")?,
                Drawing::from(Path::circle((70.0, 60.0), 30.0)?)
                    .fill(Color::NONE)
                    .stroke(Stroke::new(color("#00b4e6")?).width(12.0).cap(Cap::Butt))
                    .blend(Blend::Multiply),
            ]),
        ),
        (
            "an ink multiplied with another that is at an opacity",
            Drawing::new([
                paper()?,
                disc(50.0, 60.0, 30.0, "#00b4e6")?.opacity(0.5).blend(Blend::Multiply),
                disc(70.0, 60.0, 30.0, "#ec008c")?.opacity(0.5).blend(Blend::Multiply),
            ]),
        ),
        (
            "an ink over the hole of a knockout",
            Drawing::new([
                paper()?,
                disc(60.0, 60.0, 25.0, "#000000")?.knockout(),
                disc(75.0, 60.0, 30.0, "#ec008c")?.blend(Blend::Multiply),
            ]),
        ),
    ];
    for (case, drawing) in rows {
        let agreement = svg_agrees(&drawing, 120, 120)?;
        assert!(
            agreement.interior <= 4 && agreement.edge_over_40 * 100 <= agreement.edge_pixels,
            "{case}: {agreement:?}"
        );
    }
    Ok(())
}
