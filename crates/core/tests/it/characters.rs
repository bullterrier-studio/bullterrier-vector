//! `characters` through the public API. Oracles: cell arithmetic by hand, and the numbers of the bundled font read with
//! fontTools (`JetBrains Mono [wght]` at its default: 1000 units to the em, every advance 600, capital height 730,
//! `H` inked from x 93 to 507 and y 0 to 730, `.` from y -10 to 156 on the same baseline, `hhea` ascender 1020 and
//! descender -300).

#![expect(clippy::cast_possible_truncation, reason = "row and column numbers are tiny")]

use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Characters, Color, Drawing, Font, Inputs, Shape, Values, characters};

const CELL: (f64, f64) = (6.0, 12.0);
/// The size at which the line box of the font (1.32 em, ascender and descender about the middle of a capital) fits a
/// cell 12 high; 600 units of advance would allow 10 across.
const FIT_SIZE_6X12: f64 = 12.0 / 1.33;

fn mono() -> Result<Font, bullterrier_vector::Error> {
    Font::bundled("jetbrains_mono")
}

fn grid(rows: &str) -> Result<Vec<Shape>, Box<dyn std::error::Error>> {
    Ok(characters(rows, Characters::new(CELL)).shapes(&Values::new())?)
}

fn near(left: f64, right: f64) -> bool {
    (left - right).abs() < 1e-6
}

#[test]
fn one_shape_per_inked_character_with_its_char_row_and_column() -> Outcome {
    let rows = "#@.\n .#\n\nab";
    let shapes = grid(rows)?;
    let mut wanted = Vec::new();
    for (row, line) in rows.split('\n').enumerate() {
        for (column, character) in line.chars().enumerate() {
            if character != ' ' {
                wanted.push((character, row as u32, column as u32));
            }
        }
    }
    let found: Vec<_> = shapes
        .iter()
        .map(|shape| {
            let attributes = shape.attributes();
            (
                attributes.char().unwrap_or('?'),
                attributes.row().unwrap_or(99),
                attributes.column().unwrap_or(99),
            )
        })
        .collect();
    assert_eq!(found, wanted);
    Ok(())
}

#[test]
fn a_ligature_of_the_font_does_not_merge_cells() -> Outcome {
    let shapes = grid("->==!=<=")?;
    let columns: Vec<_> = shapes.iter().filter_map(|shape| shape.attributes().column()).collect();
    assert_eq!(columns, (0..8).collect::<Vec<_>>());
    let chars: String = shapes.iter().filter_map(|shape| shape.attributes().char()).collect();
    assert_eq!(chars, "->==!=<=");
    Ok(())
}

#[test]
fn a_capital_is_centred_in_its_cell_and_a_full_stop_stands_on_its_baseline() -> Outcome {
    let size = 10.0;
    let font = mono()?;
    // two rows of two cells: H, . / ., H
    let drawing = characters("H.\n.H", Characters::new(CELL).font(&font).size(size));
    let shapes = drawing.shapes(&Values::new())?;
    let units = size / 1000.0;
    for shape in &shapes {
        let attributes = shape.attributes();
        let (row, column) = (
            f64::from(attributes.row().ok_or("row")?),
            f64::from(attributes.column().ok_or("column")?),
        );
        let (cx, cy) = (CELL.0 * (column + 0.5), CELL.1 * (row + 0.5));
        let bounds = shape.path().bounds()?;
        // the advance box (600 units) is centred on the cell, the middle of a capital (365 units) is on its middle
        let baseline = cy + 365.0 * units;
        match attributes.char() {
            Some('H') => {
                assert!(near(bounds.x, cx + (93.0 - 300.0) * units), "{bounds:?}");
                assert!(
                    near(bounds.x + bounds.width, cx + (507.0 - 300.0) * units),
                    "{bounds:?}"
                );
                assert!(near(bounds.y, cy - 365.0 * units), "{bounds:?}");
                assert!(near(bounds.y + bounds.height, baseline), "{bounds:?}");
            }
            Some('.') => {
                assert!(near(bounds.x, cx + (217.0 - 300.0) * units), "{bounds:?}");
                assert!(near(bounds.y + bounds.height, baseline + 10.0 * units), "{bounds:?}");
                assert!(near(bounds.y, baseline - 156.0 * units), "{bounds:?}");
            }
            other => return Err(format!("unexpected {other:?}").into()),
        }
        let origin = attributes.origin().ok_or("origin")?;
        assert!(
            near(origin.x, cx) && near(origin.y, cy),
            "origin {origin:?} for cell ({cx}, {cy})"
        );
    }
    Ok(())
}

#[test]
fn the_default_size_fits_the_line_box_of_the_font_in_the_cell() -> Outcome {
    // 6 by 12: across, 6 / 0.6 = 10; down the line box (ascender and descender about the middle of a capital) allows 12 / 1.33
    let shapes = grid("H")?;
    let bounds = shapes[0].path().bounds()?;
    assert!(near(bounds.height, 0.730 * FIT_SIZE_6X12), "{}", bounds.height);
    // 12 by 6: across 20, down 6 / 1.33: the height decides again
    let wide = characters("H", Characters::new((12.0, 6.0))).shapes(&Values::new())?;
    assert!(near(wide[0].path().bounds()?.height, 0.730 * 6.0 / 1.33));
    // 3 by 30: across decides, 3 / 0.6 = 5
    let narrow = characters("H", Characters::new((3.0, 30.0))).shapes(&Values::new())?;
    assert!(near(narrow[0].path().bounds()?.height, 0.730 * 5.0));
    Ok(())
}

#[test]
fn colours_land_on_the_cell_they_are_given_for() -> Outcome {
    let colors = vec![
        vec![
            Color::hex(0x00ff_0000),
            Color::hex(0x0000_ff00),
            Color::hex(0x0000_00ff),
        ],
        vec![
            Color::hex(0x0011_1111),
            Color::hex(0x0022_2222),
            Color::hex(0x0033_3333),
        ],
    ];
    let shapes = characters("a b\ncde", Characters::new(CELL).colors(colors.clone())).shapes(&Values::new())?;
    assert_eq!(shapes.len(), 5, "the space makes no shape but keeps its colour");
    for shape in &shapes {
        let attributes = shape.attributes();
        let (row, column) = (
            attributes.row().ok_or("row")? as usize,
            attributes.column().ok_or("column")? as usize,
        );
        assert_eq!(shape.style().fill(), &colors[row][column], "row {row} column {column}");
    }
    Ok(())
}

#[test]
fn colours_that_do_not_match_the_rows_are_refused_naming_the_row() -> Outcome {
    let one = vec![Color::hex(0)];
    for (rows, colors, row) in [
        ("ab\ncd", vec![vec![Color::hex(0); 2]], "1 rows of colours"),
        ("ab\ncd", vec![vec![Color::hex(0); 2], one.clone()], "row 1"),
        ("ab", vec![vec![Color::hex(0); 3]], "row 0"),
    ] {
        let drawing = characters(rows, Characters::new(CELL).colors(colors));
        let (parameter, message) = parameter_refusal(drawing.shapes(&Values::new()))?;
        assert_eq!(parameter, "source (characters).colors");
        assert!(message.contains(row), "{message}");
    }
    Ok(())
}

#[test]
fn control_characters_are_refused_with_their_place() -> Outcome {
    for (rows, place) in [
        ("ab\ncd\te", "row 1, column 2"),
        ("\u{7}", "row 0, column 0"),
        ("a\r\nb", "row 0, column 1"),
    ] {
        let (parameter, message) = parameter_refusal(characters(rows, Characters::new(CELL)).shapes(&Values::new()))?;
        assert_eq!(parameter, "source (characters).content");
        assert!(message.contains(place), "{message}");
    }
    Ok(())
}

#[test]
fn a_character_the_font_lacks_is_the_notdef_glyph_as_text_draws_it() -> Outcome {
    let font = mono()?;
    let lacking = characters("\u{2603}", Characters::new(CELL).font(&font).size(10.0)).shapes(&Values::new())?;
    assert_eq!(lacking.len(), 1);
    assert_eq!(lacking[0].attributes().char(), Some('\u{2603}'));
    let as_text =
        bullterrier_vector::text("\u{2603}", bullterrier_vector::Text::new(&font).size(10.0)).shapes(&Values::new())?;
    assert_eq!(as_text.len(), 1);
    let (cell, text) = (lacking[0].path().bounds()?, as_text[0].path().bounds()?);
    assert!(near(cell.width, text.width) && near(cell.height, text.height));
    Ok(())
}

#[test]
fn empty_rows_and_all_space_rows_draw_nothing() -> Outcome {
    for rows in ["", "   ", "\n\n", "  \n  "] {
        assert!(grid(rows)?.is_empty(), "{rows:?}");
    }
    Ok(())
}

#[test]
fn a_ragged_grid_places_each_row_from_the_left() -> Outcome {
    let shapes = grid("HHH\nH")?;
    let columns: Vec<_> = shapes
        .iter()
        .map(|shape| (shape.attributes().row(), shape.attributes().column()))
        .collect();
    assert_eq!(
        columns,
        [
            (Some(0), Some(0)),
            (Some(0), Some(1)),
            (Some(0), Some(2)),
            (Some(1), Some(0))
        ]
    );
    Ok(())
}

#[test]
fn a_cell_that_is_not_a_positive_finite_pair_is_refused_naming_cell() -> Outcome {
    for cell in [(0.0, 12.0), (6.0, -1.0), (f64::NAN, 12.0), (6.0, f64::INFINITY)] {
        let (parameter, _) = parameter_refusal(characters("H", Characters::new(cell)).shapes(&Values::new()))?;
        assert_eq!(parameter, "source (characters).cell");
    }
    Ok(())
}

#[test]
fn a_grid_of_more_characters_than_a_text_may_hold_is_refused_before_anything_is_drawn() -> Outcome {
    let rows = "@".repeat(100_001);
    let (parameter, message) = parameter_refusal(characters(rows, Characters::new(CELL)).shapes(&Values::new()))?;
    assert_eq!(parameter, "source (characters).content");
    assert!(message.contains("MAX_GLYPHS"), "{message}");
    Ok(())
}

#[test]
fn a_grid_saves_to_json_and_loads_the_same_drawing() -> Outcome {
    let colors = vec![vec![Color::hex(0x00aa_bbcc), Color::hex(0x0001_0203)]];
    let drawing = characters("a\u{e9}", Characters::new((7.0, 9.0)).size(8.0).colors(colors));
    let json = drawing.to_json()?;
    let loaded = Drawing::from_json(&json, &Inputs::new())?;
    assert_eq!(loaded.to_json()?, json);
    let (left, right) = (drawing.shapes(&Values::new())?, loaded.shapes(&Values::new())?);
    assert_eq!(left.len(), right.len());
    assert!(left.iter().zip(&right).all(|(left, right)| left == right));
    Ok(())
}

#[test]
fn a_selection_by_char_finds_the_glyphs() -> Outcome {
    let drawing = characters("#.#\n.#.", Characters::new(CELL));
    let hashes = drawing.only(bullterrier_vector::Select::new().attribute("char", '#'));
    let shapes = hashes.shapes(&Values::new())?;
    assert_eq!(shapes.len(), 3);
    assert!(shapes.iter().all(|shape| shape.attributes().char() == Some('#')));
    Ok(())
}

#[test]
fn a_document_reads_the_text_of_the_grid_as_content() -> Outcome {
    let json = r#"{"version":1,"root":[{"characters":{"content":"a\nb","cell":[7,9]}}]}"#;
    let drawing = Drawing::from_json(json, &Inputs::new())?;
    assert_eq!(drawing.shapes(&Values::new())?.len(), 2);
    assert!(drawing.to_json()?.contains(r#""content":"a\nb""#));
    Ok(())
}

#[test]
fn a_font_left_out_is_the_bundled_monospaced_one() -> Outcome {
    let left_out = characters("#.\nab", Characters::new(CELL));
    assert_eq!(
        left_out.shapes(&Values::new())?,
        characters("#.\nab", Characters::new(CELL).font(&mono()?)).shapes(&Values::new())?
    );
    assert!(!left_out.to_json()?.contains("\"font\""), "a default is not written");
    Ok(())
}

#[test]
fn a_refusal_names_the_argument_the_caller_gave_once_whatever_the_cause() -> Outcome {
    let font = mono()?;
    for (case, drawing, named) in [
        (
            "a grid whose cells run past the coordinates",
            characters("A".repeat(1000), Characters::new(1e98)),
            "source (characters).cell",
        ),
        (
            "a size that takes a glyph out of the coordinates",
            characters("A", Characters::new(8.0).size(1e200)),
            "source (characters).size",
        ),
        (
            "a size that, with the cells, takes a glyph out of the coordinates",
            characters("AA", Characters::new(5e99).size(1e99)),
            "source (characters).size",
        ),
        (
            "a size that no glyph of the font could be set at",
            characters("A", Characters::new(8.0).font(&font).size(1e300)),
            "source (characters).size",
        ),
    ] {
        let (parameter, message) = parameter_refusal(drawing.shapes(&Values::new()))?;
        assert_eq!(parameter, named, "{case}: {message}");
    }
    Ok(())
}

#[test]
fn a_grid_of_dense_glyphs_is_refused_for_its_outline_segments_before_any_cell_is_made() -> Outcome {
    // one medium shade outlines hundreds of segments (a stipple of its own): 100,000 of them pass 10,000,000
    let one = grid("\u{2592}")?;
    let weight = one
        .iter()
        .map(|shape| shape.path().to_svg_path().matches(['L', 'C']).count())
        .sum::<usize>();
    assert!(weight > 100, "the glyph is dense: {weight} segments");
    let (parameter, message) =
        parameter_refusal(characters("\u{2592}".repeat(100_000), Characters::new(CELL)).shapes(&Values::new()))?;
    assert_eq!(parameter, "source (characters)");
    assert!(message.contains("MAX_OUTLINE_SEGMENTS"), "{message}");
    Ok(())
}

#[test]
fn order_runs_from_0_to_1_over_the_shapes_and_a_lone_shape_is_0() -> Outcome {
    // the spaces make no shape and take no place in the order
    let orders = |rows: &str| -> Result<Vec<f64>, Box<dyn std::error::Error>> {
        Ok(grid(rows)?
            .iter()
            .map(|shape| shape.attributes().order().unwrap_or(f64::NAN))
            .collect())
    };
    let four = orders("A B\nCD")?;
    for (found, wanted) in four.iter().zip([0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0]) {
        assert!(near(*found, wanted), "{four:?}");
    }
    assert_eq!(orders(" A ")?, [0.0]);
    Ok(())
}
