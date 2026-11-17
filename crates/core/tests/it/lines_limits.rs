//! The limits and degenerate regions of `rows`, `bands` and `stipple`: every allocation is refused before it is made
//! naming the argument that causes it, and a region that fills nothing gives an empty path in all three.

use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Bands, Field, Path, Point, Rows, Stipple};

fn flat() -> Result<Field, bullterrier_vector::Error> {
    Field::constant(0.5)
}

fn bands_of(family: &str) -> Result<Bands, bullterrier_vector::Error> {
    let (spacing, width) = (10.0, (1.0, 3.0));
    match family {
        "rings" => Bands::rings(spacing, flat()?, width),
        "spiral" => Bands::spiral(spacing, flat()?, width),
        _ => Bands::rays(spacing, flat()?, width),
    }
}

#[test]
fn a_wide_thin_region_is_refused_before_its_lattice_is_made() -> Outcome {
    // 0.4 high holds no row at spacing 1, but the lattice of columns along 3e7 would still be made
    let region = Path::rect(0.0, 0.0, 3e7, 0.4, 0.0)?;
    let (name, _) = parameter_refusal(region.rows(&Rows::new(1.0, flat()?, 1.0)?))?;
    assert_eq!(name, "spacing");
    Ok(())
}

#[test]
fn a_centre_too_far_to_draw_round_is_refused_naming_the_centre() -> Outcome {
    let region = Path::rect(0.0, 0.0, 100.0, 100.0, 0.0)?;
    for family in ["rings", "spiral", "rays"] {
        for centre in [(1e20, 1e20), (1e8, 0.0)] {
            let bands = bands_of(family)?.centre(Point::new(centre.0, centre.1))?;
            let (name, _) = parameter_refusal(region.bands(&bands))?;
            assert_eq!(name, "centre", "{family} {centre:?}");
        }
    }
    Ok(())
}

#[test]
fn a_region_that_fills_nothing_gives_an_empty_path_in_every_verb() -> Outcome {
    let point = Path::start((10.0, 10.0))?;
    let line = Path::polyline([(0.0, 0.0), (100.0, 50.0)], false)?;
    let thin = Path::polyline([(0.0, 5.0), (100.0, 5.0), (50.0, 5.0)], true)?;
    for (what, region) in [("a point", &point), ("a line", &line), ("a flat triangle", &thin)] {
        let rows = region.rows(&Rows::new(5.0, flat()?, 1.0)?)?;
        assert_eq!(rows.to_svg_path(), "", "rows of {what}");
        let stipple = region.stipple(&Stipple::new(5.0, flat()?, 1.0)?)?;
        assert_eq!(stipple.to_svg_path(), "", "stipple of {what}");
        let lines = region.bands(&Bands::lines(5.0, flat()?, (1.0, 3.0))?)?;
        assert_eq!(lines.to_svg_path(), "", "lines of {what}");
        for family in ["rings", "spiral", "rays"] {
            let drawn = region.bands(&bands_of(family)?)?;
            assert_eq!(drawn.to_svg_path(), "", "{family} of {what}");
        }
    }
    Ok(())
}
