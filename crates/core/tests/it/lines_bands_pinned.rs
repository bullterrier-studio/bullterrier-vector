//! `bands` pinned: the SVG of every family at a spread of settings, as a length and an FNV-1a hash of the text, so that
//! moving the maths of the carriers (one module shared with the halftone screens) cannot change a single byte.

use crate::common::Outcome;
use bullterrier_vector::{Bands, Drawing, Field, Path, Size, Values};

/// What the code gave before the carriers moved into the shared layout module.
const PINNED: [&str; 13] = [
    "lines: path 41653 ca555aff58f4b3bd, svg 41802 04f8294cd494d4ef",
    "lines angle: path 91611 379e882397c5efb1, svg 91760 4b5d0e4512d1d63f",
    "lines disc: path 81710 22a7c3b2868a5da4, svg 81859 030e3aee4c4b3e90",
    "waves: path 63586 2659d9c1fa8b053d, svg 63735 07041b4f3c7ec10d",
    "waves angle: path 47265 82548f250ced2a10, svg 47414 ba2a17b689f61a46",
    "rings: path 65080 3c819fabe9a7e036, svg 65229 2426f826acf88546",
    "rings center: path 44433 3042456a25303c1e, svg 44582 69b068e285b22cec",
    "rings disc: path 115368 062acf11265ba25d, svg 115517 1f315828396f2d55",
    "spiral: path 63372 e85e258d584679b4, svg 63521 7fb8d33687e216e0",
    "spiral turned: path 58305 6513425b678e4a51, svg 58454 d30860295fdee5ef",
    "rays: path 169841 1202bc55e99195e9, svg 169990 49e2380520eed25f",
    "rays center: path 76012 e71630842a246268, svg 76161 cbc72441b4034828",
    "rays far center: path 38494 236f2aaad104a895, svg 38643 aab5d755caeb88b1",
];

fn fnv(text: &str) -> (usize, u64) {
    let hash = text.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    });
    (text.len(), hash)
}

fn tone() -> Result<Field, bullterrier_vector::Error> {
    Field::ramp((10.0, 10.0), (190.0, 120.0))
}

/// Every family, over a rectangle off the origin and over a disc, at several angles, centers and sizes.
fn cases() -> Result<Vec<(&'static str, Path, Bands)>, bullterrier_vector::Error> {
    let rect = Path::rect(13.0, 21.0, 170.0, 97.0, 0.0)?;
    let disc = Path::circle((100.0, 70.0), 55.0)?;
    let t = tone;
    Ok(vec![
        ("lines", rect.clone(), Bands::lines(9.0, t()?, (0.5, 4.0))?),
        (
            "lines angle",
            rect.clone(),
            Bands::lines(7.5, t()?, (0.5, 4.0))?.angle(23.0)?,
        ),
        (
            "lines disc",
            disc.clone(),
            Bands::lines(6.0, t()?, (1.0, 3.0))?.angle(-70.0)?,
        ),
        ("waves", rect.clone(), Bands::waves(9.0, t()?, (0.5, 4.0), 30.0, 3.0)?),
        (
            "waves angle",
            disc.clone(),
            Bands::waves(8.0, t()?, (0.5, 4.0), 22.0, 4.0)?.angle(40.0)?,
        ),
        ("rings", rect.clone(), Bands::rings(9.0, t()?, (0.5, 4.0))?),
        (
            "rings center",
            rect.clone(),
            Bands::rings(11.0, t()?, (0.5, 4.0))?.center((30.0, 20.0))?,
        ),
        ("rings disc", disc.clone(), Bands::rings(5.0, t()?, (0.5, 2.0))?),
        ("spiral", rect.clone(), Bands::spiral(9.0, t()?, (0.5, 4.0))?),
        (
            "spiral turned",
            disc.clone(),
            Bands::spiral(7.0, t()?, (0.5, 3.0))?
                .angle(35.0)?
                .center((90.0, 60.0))?,
        ),
        ("rays", rect.clone(), Bands::rays(9.0, t()?, (0.5, 4.0))?.angle(10.0)?),
        (
            "rays center",
            disc,
            Bands::rays(12.0, t()?, (0.5, 4.0))?.center((140.0, 40.0))?,
        ),
        (
            "rays far center",
            rect,
            Bands::rays(15.0, t()?, (0.5, 4.0))?.center((260.0, 150.0))?,
        ),
    ])
}

#[test]
fn the_path_and_the_svg_of_every_family_are_byte_for_byte_what_they_were() -> Outcome {
    let mut found = Vec::new();
    for (name, region, bands) in cases()? {
        let (length, hash) = fnv(&region.bands(&bands)?.to_svg_path());
        let drawn = Drawing::from(region)
            .bands(&bands)
            .to_svg(Size::px(200.0, 150.0), &Values::new())?;
        let (svg_length, svg_hash) = fnv(&drawn);
        found.push(format!(
            "{name}: path {length} {hash:016x}, svg {svg_length} {svg_hash:016x}"
        ));
    }
    assert_eq!(
        found, PINNED,
        "the bands changed: the first difference is a family, not a number"
    );
    Ok(())
}
