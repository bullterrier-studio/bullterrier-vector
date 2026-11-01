//! A star: the drawing of `examples/star.py` in Rust. Build a `Path`, measure it, draw it.
//!
//! `cargo run -p bullterrier-vector --example star` prints the size of the SVG and a checksum of the pixels;
//! `... --example star -- svg` writes the SVG document, `-- png` the PNG file and `-- rgba` the raw 400 by 400
//! straight-alpha pixels to standard output, to redirect to a file. The core does no IO of its own; this example does.

use bullterrier_vector::{Color, Drawing, Error, Path, Point, Size, Stroke, Values};
use std::io::Write;

/// The side of the picture, in pixels.
const SIDE: f64 = 400.0;

/// The star: a filled and stroked outline, a dot on its tip, a line along its tangent halfway round.
fn star() -> Result<Drawing, Error> {
    let outline = Path::star((200.0, 200.0), 150.0, 60.0, 5)?;
    let tip = outline.point_at(0.0, None)?; // a closed outline is measured from its topmost point: the top tip
    let halfway = outline.point_at(0.5, None)?; // half its length further on, clockwise
    let direction = outline.tangent_at(0.5, None)?; // a unit vector along the outline there
    let end = Point::new(halfway.x + 40.0 * direction.x, halfway.y + 40.0 * direction.y);
    Ok(Drawing::new([
        outline
            .fill(Color::hex(0xff_cc_33))
            .stroke(Stroke::new(Color::hex(0x1a_1a_1a)).width(4.0)),
        Path::circle(tip, 8.0)?.fill(Color::hex(0xe2_41_2e)),
        Path::line(halfway, end)?.stroke(Color::hex(0xe2_41_2e)),
    ])
    .with_size(SIDE, SIDE))
}

/// The 32-bit FNV-1a hash of `bytes`: a short number to compare two pictures' pixels by eye.
fn checksum(bytes: &[u8]) -> u32 {
    // constant: the FNV-1a offset basis and prime, the published parameters of the 32-bit hash
    bytes.iter().fold(0x811c_9dc5_u32, |hash, byte| {
        (hash ^ u32::from(*byte)).wrapping_mul(0x0100_0193)
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let drawing = star()?;
    let size = Size::px(SIDE, SIDE);
    let values = Values::new();
    let mut out = std::io::stdout();
    match std::env::args().nth(1).as_deref() {
        Some("svg") => out.write_all(drawing.to_svg(size, &values)?.as_bytes())?,
        Some("png") => out.write_all(&drawing.to_png(size, &values)?)?,
        Some("rgba") => out.write_all(&drawing.to_rgba(size, &values)?)?,
        Some(other) => return Err(format!("expected svg, png or rgba, got {other}").into()),
        None => {
            let svg = drawing.to_svg(size, &values)?;
            let rgba = drawing.to_rgba(size, &values)?;
            writeln!(
                out,
                "svg: {} bytes, rgba: {} bytes, checksum {:08x}",
                svg.len(),
                rgba.len(),
                checksum(&rgba)
            )?;
        }
    }
    Ok(())
}
