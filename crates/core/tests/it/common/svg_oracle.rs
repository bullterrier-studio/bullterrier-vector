//! A second renderer for the SVG a drawing writes: `resvg` draws it, and the picture is compared with `to_rgba`. It
//! says whether the SVG shows what the pixels show, which the XML alone cannot (a blur is a filter, not a number).
//! Anti-aliased edges are not compared closely: the two renderers cover a curved edge differently by up to a quarter
//! of the range (measured), where inside a smooth area they agree to a few levels.

use bullterrier_vector::{Drawing, Size, Values};
use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg;

/// How far the SVG of a drawing is from its pixels.
#[derive(Debug)]
pub(crate) struct Agreement {
    /// The largest difference of a channel (alpha, or a colour times its alpha) in a pixel that is not on an edge.
    pub(crate) interior: u8,
    /// How many pixels on an edge differ by more than 40 levels in some channel.
    pub(crate) edge_over_40: usize,
    /// How many pixels are on an edge.
    pub(crate) edge_pixels: usize,
}

/// The channels of a pixel as `[alpha, red * alpha, green * alpha, blue * alpha]`, each 0 to 255.
fn premultiplied([red, green, blue, alpha]: [u8; 4]) -> [i32; 4] {
    let times = |channel: u8| (i32::from(channel) * i32::from(alpha) + 127) / 255;
    [i32::from(alpha), times(red), times(green), times(blue)]
}

/// The pixels of `drawing` as `to_rgba` draws them and as `resvg` draws its SVG, compared.
pub(crate) fn svg_agrees(
    drawing: &Drawing,
    width: usize,
    height: usize,
) -> Result<Agreement, Box<dyn std::error::Error>> {
    #[expect(clippy::cast_precision_loss, reason = "test sizes are small")]
    let size = Size::px(width as f64, height as f64);
    let ours = drawing.to_rgba(size, &Values::new())?;
    let svg = drawing.to_svg(size, &Values::new())?;
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default())?;
    let mut pixmap = Pixmap::new(u32::try_from(width)?, u32::try_from(height)?).ok_or("no pixmap")?;
    resvg::render(&tree, Transform::identity(), &mut pixmap.as_mut());
    // the pixmap is premultiplied, `to_rgba` straight
    let theirs: Vec<[i32; 4]> = pixmap
        .data()
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&[red, green, blue, alpha]| [i32::from(alpha), i32::from(red), i32::from(green), i32::from(blue)])
        .collect();
    let ours: Vec<[i32; 4]> = ours
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&pixel| premultiplied(pixel))
        .collect();
    let (mut interior, mut edge_over_40, mut edge_pixels) = (0, 0, 0);
    for y in 0..height {
        for x in 0..width {
            let differs = (0..4)
                .map(|channel| (ours[y * width + x][channel] - theirs[y * width + x][channel]).abs())
                .max()
                .unwrap_or(0);
            // an edge: the picture changes by 24 levels or more somewhere in the 3 by 3 pixels around it
            let nearby = |channel: usize| {
                let values = (y.saturating_sub(1)..=(y + 1).min(height - 1))
                    .flat_map(|row| (x.saturating_sub(1)..=(x + 1).min(width - 1)).map(move |column| (row, column)))
                    .map(|(row, column)| ours[row * width + column][channel]);
                (values.clone().max().unwrap_or(0), values.min().unwrap_or(0))
            };
            let on_edge = (0..4).any(|channel| {
                let (most, least) = nearby(channel);
                most - least >= 24
            });
            if on_edge {
                edge_pixels += 1;
                edge_over_40 += usize::from(differs > 40);
            } else {
                interior = interior.max(differs);
            }
        }
    }
    Ok(Agreement {
        interior: u8::try_from(interior)?,
        edge_over_40,
        edge_pixels,
    })
}
