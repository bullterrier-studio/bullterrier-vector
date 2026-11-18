//! Trace a sequence of maps steadily: the program of `examples/trace_sequence.py` in Rust. A disc drifts across five
//! frames, each a map with blocks of dropout; each frame traced alone shows the dropouts as holes that pop in and out, and
//! `Field::smooth_maps` then `Field::persist_holes` leave the disc alone.
//!
//! `cargo run -p bullterrier-vector --example trace_sequence` prints the number of outlines in each frame traced alone
//! and steadied; `... --example trace_sequence -- still` writes the SVG of the last steadied frame to standard output.

use bullterrier_vector::{Color, Drawing, Error, Field, Rect, Size, Trace, Values};
use std::io::Write;

const SIDE: usize = 64;
const FRAMES: usize = 5;
const EXTENT: f64 = 640.0;

/// A disc of tone 200 on 50, moving 2 pixels a frame; a 3 by 3 block in about one in twelve drops out to 50.
fn frame_bytes(i: usize) -> Vec<u8> {
    (0..SIDE * SIDE)
        .map(|p| {
            let (x, y) = (p % SIDE, p / SIDE);
            let (dx, dy) = (x.abs_diff(24 + 2 * i), y.abs_diff(32));
            let inside = dx * dx + dy * dy < 18 * 18;
            let hash = ((x / 3) * 73_856_093) ^ ((y / 3) * 19_349_663) ^ ((i + 1) * 83_492_791);
            let dropout = hash.is_multiple_of(12);
            if inside && !dropout { 200 } else { 50 }
        })
        .collect()
}

fn trace(map: &Field) -> Drawing {
    let tidy = Trace::new().smooth(1.0).tolerance(0.4).close(1.0);
    Drawing::new([Drawing::trace_levels(map, [0.5], &tidy).fill(Color::hex(0xf2_b632))]).with_size(EXTENT, EXTENT)
}

fn outlines(drawing: &Drawing) -> Result<usize, Error> {
    Ok(drawing
        .to_svg(Size::px(EXTENT, EXTENT), &Values::new())?
        .matches('M')
        .count())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let frame = Rect {
        x: 0.0,
        y: 0.0,
        width: EXTENT,
        height: EXTENT,
    };
    let maps = (0..FRAMES)
        .map(|i| Field::map(SIDE, SIDE, &frame_bytes(i), Some(frame), Some("frame")))
        .collect::<Result<Vec<_>, _>>()?;
    let steady = Field::persist_holes(&Field::smooth_maps(&maps, 1.5)?, &[0.5], 1, 2)?;
    let mut out = std::io::stdout();
    match std::env::args().nth(1).as_deref() {
        Some("still") => {
            let svg = trace(&steady[FRAMES - 1]).to_svg(Size::px(EXTENT, EXTENT), &Values::new())?;
            out.write_all(svg.as_bytes())?;
        }
        Some(name) => return Err(format!("expected still, got {name}").into()),
        None => {
            for (i, (alone, steady)) in maps.iter().zip(&steady).enumerate() {
                writeln!(
                    out,
                    "frame {i}: {} outlines alone, {} steadied",
                    outlines(&trace(alone))?,
                    outlines(&trace(steady))?
                )?;
            }
        }
    }
    Ok(())
}
