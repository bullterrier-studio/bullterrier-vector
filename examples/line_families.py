"""Line families: three hills drawn only from lines whose look follows a tone.

The tone is the normalised distance to each hill's own edge, 0 on the edge and 1 in the middle. The first hill is
ridgelines lifted by it, with the lines behind a nearer one hidden; the second is rings whose width falls from 5 pixels
at the edge to 0.6 in the middle; the third is stippled dots, dense where the tone is 0. Nothing is drawn but lines and
dots, so the piece prints at any size and the same drawing writes a plotter's SVG (one layer per pen).
Transparent, for any paper. Draw it with `uv run poe look examples/line_families.py` (PNG and SVG in `dist/look/`).
The same program in Rust and JavaScript: `crates/core/examples/line_families.rs`,
`crates/wasm/examples/line_families.mjs`.
"""

import bullterrier_vector as btvec

WIDTH, HEIGHT = 1080, 420
INK, RED = "#16151a", "#c8341c"

hills = [btvec.Path.circle((180 + 360 * i, 215), 160) for i in range(3)]
tones = [btvec.Field.distance(hill, normalized=True) for hill in hills]

drawing = btvec.Drawing(
    btvec.Drawing(hills[0]).rows(8, tone=tones[0], amplitude=70).stroke(INK, width=1.2),
    btvec.Drawing(hills[1]).bands(8, tone=tones[1], width=(5, 0.6), family="rings").fill(INK),
    btvec.Drawing(hills[2]).stipple(6, tone=tones[2], radius=(1.6, 1.6), seed=2).fill(RED),
    size=(WIDTH, HEIGHT),
)

# The frames of the piece: the SVG, and the SVG a pen plotter reads (a pen 0.8 pixels wide: the rings and the dots
# become hatching of that width).
frames = {
    "still": drawing.to_svg(WIDTH, HEIGHT),
    "plot": drawing.to_plot_svg(WIDTH, HEIGHT, pen_width=0.8),
}
