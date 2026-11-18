//! `characters` and `marks`: a grid of glyphs and the overlay marks of a heads-up display.

use crate::drawing::{PyDrawing, checked};
use crate::expr::Num;
use crate::raised;
use crate::text::{PyFont, utf8};
use bullterrier_vector as vector;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::PyString;

/// The font a grid or a label is set in unless the caller gives one.
const MONOSPACED: &str = "jetbrains_mono";

/// `font`, or the bundled monospaced one.
fn font_of(font: Option<&Bound<'_, PyFont>>) -> PyResult<vector::Font> {
    match font {
        Some(font) => Ok(font.get().0.clone()),
        None => raised(vector::Font::bundled(MONOSPACED)),
    }
}

/// The text of a grid: a `str` with `\n` between its rows, or a sequence of `str`, one for each row.
fn rows_of(rows: &Bound<'_, PyAny>) -> PyResult<String> {
    if let Ok(text) = rows.cast::<PyString>() {
        return Ok(utf8("rows", text)?.to_owned());
    }
    let wrong = || {
        PyTypeError::new_err(format!(
            "rows: expected a str or a list of str, got {}",
            rows.repr().map_or_else(|_| String::new(), |text| text.to_string())
        ))
    };
    let mut lines = Vec::new();
    for line in rows.try_iter().map_err(|_| wrong())? {
        let line = line?;
        let text = line.cast::<PyString>().map_err(|_| wrong())?;
        lines.push(utf8("rows", text)?.to_owned());
    }
    Ok(lines.join("\n"))
}

/// The cell: one number for a square one, or `(width, height)`.
fn cell_of(cell: &Bound<'_, PyAny>) -> PyResult<(f64, f64)> {
    if let Ok(side) = cell.extract::<f64>() {
        return Ok((side, side));
    }
    cell.extract::<(f64, f64)>().map_err(|_| {
        PyTypeError::new_err(format!(
            "cell: expected a number or a (width, height) tuple of numbers, got {}",
            cell.repr().map_or_else(|_| String::new(), |text| text.to_string())
        ))
    })
}

/// The colours of a grid: a list of rows, each a list of colour texts, as bullterrier-image's `characters` returns.
fn colors_of(colors: &Bound<'_, PyAny>) -> PyResult<Vec<Vec<vector::Color>>> {
    let rows: Vec<Vec<String>> = colors.extract().map_err(|_| {
        PyTypeError::new_err("colors: expected a list of rows, each a list of colour texts such as '#rrggbb'")
    })?;
    rows.iter()
        .map(|row| row.iter().map(|text| raised(vector::Color::parse(text))).collect())
        .collect()
}

/// A drawing of a grid of characters, one outlined glyph in each cell. `rows` is the text (a `str` with `\n` between
/// its rows, or a list of `str`), `cell` the width and height of a cell (one number for a square one). A character
/// that has an outline becomes one shape standing in its cell, with `char`, `row` and `column` attributes; a space
/// makes none. `size` is pixels to the em, by default the size that fits a glyph to its cell. `colors` is a list of
/// rows of colour texts, one for each character (what bullterrier-image's `characters` returns as `colors`), and
/// fills each shape; without it the shapes have no paint. `font` defaults to the bundled `jetbrains_mono`.
#[pyfunction]
#[pyo3(signature = (rows, *, cell, font=None, size=None, colors=None))]
pub(crate) fn characters(
    rows: &Bound<'_, PyAny>,
    cell: &Bound<'_, PyAny>,
    font: Option<&Bound<'_, PyFont>>,
    size: Option<Num>,
    colors: Option<&Bound<'_, PyAny>>,
) -> PyResult<PyDrawing> {
    let mut options = vector::Characters::new(&font_of(font)?, cell_of(cell)?);
    if let Some(size) = size {
        options = options.size(size.0);
    }
    if let Some(colors) = colors {
        options = options.colors(colors_of(colors)?);
    }
    checked(vector::characters(rows_of(rows)?, options))
}

/// The items of a mark given as a tuple: `N` numbers, then optionally a label.
fn parts<'py, const N: usize>(item: &Bound<'py, PyAny>, what: &str) -> PyResult<([vector::Expr; N], Option<String>)> {
    let wrong = || {
        PyTypeError::new_err(format!(
            "{what}: expected {N} numbers, then optionally a label, got {}",
            item.repr().map_or_else(|_| String::new(), |text| text.to_string())
        ))
    };
    let items: Vec<Bound<'py, PyAny>> = item.extract().map_err(|_| wrong())?;
    if items.len() != N && items.len() != N + 1 {
        return Err(wrong());
    }
    let label = match items.get(N) {
        Some(label) => Some(utf8("label", label.cast::<PyString>().map_err(|_| wrong())?)?.to_owned()),
        None => None,
    };
    let numbers = items
        .iter()
        .take(N)
        .map(|number| number.extract::<Num>().map(|Num(expr)| expr))
        .collect::<PyResult<Vec<_>>>()?;
    let numbers = <[vector::Expr; N]>::try_from(numbers).map_err(|_| wrong())?;
    Ok((numbers, label))
}

/// Drawings of overlay marks in the look of a heads-up display, in design pixels. `boxes` are `(x, y, width, height)`
/// or `(x, y, width, height, label)` tuples, each bracketed in its four corners, an arm `bracket` times the smaller
/// side long (up to 0.5, default 0.25); `points` are `(x, y)` or `(x, y, label)` tuples, each a crosshair whose
/// lines reach `cross` pixels either way (default 12). A label stands on a leader line from the box's top right corner
/// or from the point, up and to the right by `cross` plus `label_size` each way (default 12), set in `font` (default
/// the bundled `jetbrains_mono`). Every shape has `role` (`bracket`, `cross`, `leader` or `label`) and `mark` (the
/// boxes in order, then the points); none has a paint.
#[pyfunction]
#[pyo3(signature = (boxes=None, points=None, *, bracket=None, cross=None, label_size=None, font=None))]
pub(crate) fn marks(
    boxes: Option<&Bound<'_, PyAny>>,
    points: Option<&Bound<'_, PyAny>>,
    bracket: Option<Num>,
    cross: Option<Num>,
    label_size: Option<Num>,
    font: Option<&Bound<'_, PyFont>>,
) -> PyResult<PyDrawing> {
    let mut options = vector::Marks::new(&font_of(font)?);
    if let Some(boxes) = boxes {
        let items: Vec<Bound<'_, PyAny>> = boxes.extract()?;
        let mut found = Vec::new();
        for item in &items {
            let ([x, y, width, height], label) = parts(item, "boxes")?;
            let mut mark = vector::BoxMark::new(x, y, width, height);
            if let Some(label) = label {
                mark = mark.label(label);
            }
            found.push(mark);
        }
        options = options.boxes(found);
    }
    if let Some(points) = points {
        let items: Vec<Bound<'_, PyAny>> = points.extract()?;
        let mut found = Vec::new();
        for item in &items {
            let ([x, y], label) = parts(item, "points")?;
            let mut mark = vector::PointMark::new(x, y);
            if let Some(label) = label {
                mark = mark.label(label);
            }
            found.push(mark);
        }
        options = options.points(found);
    }
    for (number, set) in [
        (
            bracket,
            vector::Marks::bracket as fn(vector::Marks, vector::Expr) -> vector::Marks,
        ),
        (cross, vector::Marks::cross),
        (label_size, vector::Marks::label_size),
    ] {
        if let Some(number) = number {
            options = set(options, number.0);
        }
    }
    checked(vector::marks(options))
}
