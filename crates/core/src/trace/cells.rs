//! `trace_cells`: a map of square cells of one known size, each cell one colour, traced exactly. Cells of one colour that
//! touch along a side are one region; its outline is the cracks between its cells and the others, followed from corner
//! to corner and cut down to the corners where it turns. Nothing is fitted or smoothed: every edge lies on a cell side,
//! so the area of a colour's shape is exactly its cells' count times the cell's area, and the colours' shapes neither
//! overlap nor leave a gap inside the map.

#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "the lattice has at most MAX_CELLS cells and corner numbers fit u32 for it, checked before any is made"
)]

use crate::Error;
use crate::field::MapData;
use crate::geometry::Point;
use crate::path::Path;

/// The most cells one call traces.
// constant: 2^24 is the most pixels a map holds (a cell is at least one pixel)
const MAX_CELLS: usize = 1 << 24;

/// A crack between two cells, walked with its colour on the right (clockwise on the screen, y down).
#[derive(Clone, Copy)]
struct Edge {
    colour: u8,
    /// 0 right, 1 down, 2 left, 3 up.
    direction: u8,
    /// The corner it starts at, numbered by row of corners.
    from: u32,
}

/// The colours of the cells of `map` read cell by cell: one byte each, `columns` by `rows`. Every pixel of a cell must be
/// the cell's colour.
fn lattice(map: &MapData, cell: usize) -> Result<Vec<u8>, Error> {
    let (columns, rows) = (map.width / cell, map.height / cell);
    let mut colours = Vec::with_capacity(columns * rows);
    for row in 0..rows {
        for column in 0..columns {
            let colour = map.bytes[row * cell * map.width + column * cell];
            for y in row * cell..(row + 1) * cell {
                let start = y * map.width + column * cell;
                if map.bytes[start..start + cell].iter().any(|&byte| byte != colour) {
                    return Err(Error::parameter(
                        "cell",
                        format!(
                            "the cell at column {column}, row {row} is not one colour: the map is not cells of {cell} pixels"
                        ),
                    ));
                }
            }
            colours.push(colour);
        }
    }
    Ok(colours)
}

/// One shape's outline for each colour of `map`, in order of the colour byte, cut into cells of `cell` map pixels (the
/// colour byte is the palette index). `charge` is called with the number of cracks before they are made.
pub(crate) fn trace(
    map: &MapData,
    cell: usize,
    max_colors: usize,
    charge: &mut dyn FnMut(usize) -> Result<(), Error>,
) -> Result<Vec<(u8, Path)>, Error> {
    if !map.width.is_multiple_of(cell) || !map.height.is_multiple_of(cell) {
        return Err(Error::parameter(
            "cell",
            format!(
                "must divide the map's width and height, got {cell} for {} by {}",
                map.width, map.height
            ),
        ));
    }
    let (columns, rows) = (map.width / cell, map.height / cell);
    if columns * rows > MAX_CELLS {
        return Err(Error::parameter("cell", "is too small: more than 2^24 cells"));
    }
    let colours = lattice(map, cell)?;
    let mut present = [false; 256];
    for &colour in &colours {
        present[usize::from(colour)] = true;
    }
    let count = present.iter().filter(|&&p| p).count();
    if count > max_colors {
        return Err(Error::parameter(
            "max_colors",
            format!("the map has {count} colours, more than {max_colors}"),
        ));
    }
    let at = |column: usize, row: usize| colours[row * columns + column];
    // the cracks, counted first so that the work is charged before the memory is
    let differs = |column: usize, row: usize, other: Option<(usize, usize)>| {
        other.is_none_or(|(c, r)| at(c, r) != at(column, row))
    };
    let sides = |column: usize, row: usize| {
        [
            differs(column, row, row.checked_sub(1).map(|r| (column, r))),
            differs(column, row, (column + 1 < columns).then_some((column + 1, row))),
            differs(column, row, (row + 1 < rows).then_some((column, row + 1))),
            differs(column, row, column.checked_sub(1).map(|c| (c, row))),
        ]
    };
    let mut total = 0_usize;
    for row in 0..rows {
        for column in 0..columns {
            total += sides(column, row).iter().filter(|&&d| d).count();
        }
    }
    charge(total)?;
    let across = (columns + 1) as u32;
    let corner = |column: usize, row: usize| row as u32 * across + column as u32;
    let mut edges = Vec::with_capacity(total);
    for row in 0..rows {
        for column in 0..columns {
            let colour = at(column, row);
            let [top, right, bottom, left] = sides(column, row);
            let mut add = |wanted: bool, direction: u8, c: usize, r: usize| {
                if wanted {
                    edges.push(Edge {
                        colour,
                        direction,
                        from: corner(c, r),
                    });
                }
            };
            add(top, 0, column, row);
            add(right, 1, column + 1, row);
            add(bottom, 2, column + 1, row + 1);
            add(left, 3, column, row + 1);
        }
    }
    loops(map, edges, columns, rows)
}

/// The outlines of the colours from their cracks: each crack is followed corner to corner back to where it began.
fn loops(map: &MapData, mut edges: Vec<Edge>, columns: usize, rows: usize) -> Result<Vec<(u8, Path)>, Error> {
    let across = (columns + 1) as u32;
    edges.sort_unstable_by_key(|e| (e.colour, e.from, e.direction));
    let step = |edge: &Edge| -> u32 {
        match edge.direction {
            0 => edge.from + 1,
            1 => edge.from + across,
            2 => edge.from - 1,
            _ => edge.from - across,
        }
    };
    let (sx, sy) = (map.frame.width / columns as f64, map.frame.height / rows as f64);
    let point = |corner: u32| {
        Point::new(
            map.frame.x + f64::from(corner % across) * sx,
            map.frame.y + f64::from(corner / across) * sy,
        )
    };
    let mut used = vec![false; edges.len()];
    let mut shapes: Vec<(u8, Vec<Path>)> = Vec::new();
    for first in 0..edges.len() {
        if used[first] {
            continue;
        }
        let colour = edges[first].colour;
        let mut corners: Vec<Point> = Vec::new();
        let mut current = first;
        let mut loop_edges: Vec<usize> = Vec::new();
        loop {
            used[current] = true;
            loop_edges.push(current);
            let edge = edges[current];
            let to = step(&edge);
            let begin = edges.partition_point(|e| (e.colour, e.from) < (colour, to));
            // turn towards the colour first, so that two cells touching at a corner are two regions
            let next = [1_u8, 0, 3]
                .iter()
                .map(|turn| (edge.direction + turn) % 4)
                .find_map(|direction| {
                    (begin..edges.len())
                        .take_while(|&i| edges[i].colour == colour && edges[i].from == to)
                        .find(|&i| edges[i].direction == direction && !used[i])
                });
            match next {
                Some(next) => current = next,
                None => break,
            }
        }
        // the edge before the first is the loop's last
        let mut previous = edges[*loop_edges.last().unwrap_or(&first)].direction;
        for &index in &loop_edges {
            let edge = edges[index];
            if edge.direction != previous {
                corners.push(point(edge.from));
            }
            previous = edge.direction;
        }
        let path = Path::polyline(corners, true)?;
        match shapes.last_mut() {
            Some((last, paths)) if *last == colour => paths.push(path),
            _ => shapes.push((colour, vec![path])),
        }
    }
    Ok(shapes
        .into_iter()
        .map(|(colour, paths)| (colour, Path::joined(paths.iter())))
        .collect())
}
