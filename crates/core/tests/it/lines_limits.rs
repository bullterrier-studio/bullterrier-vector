//! The limits and degenerate regions of `rows`, `bands` and `stipple`: every allocation is refused before it is made
//! naming the argument that causes it, and a region that fills nothing gives an empty path in all three.

use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Bands, Family, Field, Path, Point, Rows, Stipple};

fn flat() -> Result<Field, bullterrier_vector::Error> {
    Field::constant(0.5)
}

fn bands_of(family: &str) -> Result<Bands, bullterrier_vector::Error> {
    let (spacing, width) = (10.0, (1.0, 3.0));
    match family {
        "rings" => Ok(Bands::new(spacing, flat()?, width)?.family(Family::Rings)),
        "spiral" => Ok(Bands::new(spacing, flat()?, width)?.family(Family::Spiral)),
        _ => Ok(Bands::new(spacing, flat()?, width)?.family(Family::Rays)),
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
fn a_tall_thin_region_between_two_lattice_columns_is_refused_by_its_row_count_first() -> Outcome {
    // 0.3 wide at x 0.1 holds no column at spacing 1, but 2e7 tall holds 2e7 rows of two points each: refused by the
    // count of points the rows would make, before any row is built
    let region = Path::rect(0.1, 0.0, 0.3, 2e7, 0.0)?;
    let (name, message) = parameter_refusal(region.rows(&Rows::new(1.0, flat()?, 1.0)?))?;
    assert_eq!(name, "spacing");
    assert!(message.contains("a region this large"), "{message}");
    Ok(())
}

#[test]
fn a_center_too_far_to_draw_round_is_refused_naming_the_center() -> Outcome {
    let region = Path::rect(0.0, 0.0, 100.0, 100.0, 0.0)?;
    for family in ["rings", "spiral", "rays"] {
        for center in [(1e20, 1e20), (1e8, 0.0)] {
            let bands = bands_of(family)?.center(Point::new(center.0, center.1));
            let (name, _) = parameter_refusal(region.bands(&bands))?;
            assert_eq!(name, "center", "{family} {center:?}");
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
        let stipple = region.stipple(&Stipple::new(5.0, flat()?, (1.0, 1.0))?)?;
        assert_eq!(stipple.to_svg_path(), "", "stipple of {what}");
        let lines = region.bands(&Bands::new(5.0, flat()?, (1.0, 3.0))?)?;
        assert_eq!(lines.to_svg_path(), "", "lines of {what}");
        for family in ["rings", "spiral", "rays"] {
            let drawn = region.bands(&bands_of(family)?)?;
            assert_eq!(drawn.to_svg_path(), "", "{family} of {what}");
        }
    }
    Ok(())
}

#[test]
fn a_small_shape_far_from_the_origin_lays_only_the_rows_near_it() -> Outcome {
    // the lattice runs through the origin, but only its rows inside the box are made: 10 at spacing 1, not a million
    let far = Path::rect(1e6, 1e6, 10.0, 10.0, 0.0)?;
    assert_eq!(
        far.bands(&Bands::new(1.0, flat()?, (0.2, 0.2))?)?
            .to_svg_path()
            .matches('M')
            .count(),
        10
    );
    assert_eq!(
        far.rows(&Rows::new(1.0, flat()?, 0.0)?)?
            .to_svg_path()
            .matches('M')
            .count(),
        10
    );
    assert_eq!(
        far.stipple(&Stipple::new(1.0, Field::constant(0.0)?, (0.1, 0.1))?)?
            .to_svg_path()
            .matches('M')
            .count(),
        100
    );
    Ok(())
}
