"""Eaten letters: a word eaten at the rim and grown over by two halos, rust and pale, like mould on a wet page.

The signed distance to the letters (negative inside) is bent by seeded noise that grows with the distance from the
edge, up to 40 pixels at 60 pixels away; `trace_levels` cuts the result at three distances into stacked fills: the pale
halo out to 40 pixels, the rust one to 18, and the letter itself at 0. The noise's `wavelength` and the `core` level
are Params: the frame "fine" draws a finer noise with the core eaten 6 pixels in. Only the library: no pixels are
read back and no map is written.
Draw it with `uv run poe look examples/eaten_letters.py` (PNG and SVG in `dist/look/`).
The same program in Rust and JavaScript: `crates/core/examples/eaten_letters.rs`,
`crates/wasm/examples/eaten_letters.mjs`.
"""

import bullterrier_vector as btvec

WIDTH, HEIGHT = 1200, 600
PAPER, PALE, RUST, INK = "#efe8d6", "#dcd2b6", "#c8421e", "#16211c"

font = btvec.Font.bundled("fraunces")
word = btvec.text(
    "SPORE", font=font, size=300, axes={"wght": 900, "opsz": 144, "WONK": 0}, anchor=(600, 395), align="center"
).resolve()
wavelength = btvec.Param("wavelength", default=45, min=10, max=120, unit="px", meaning="the size of a bite")
core = btvec.Param("core", default=0, min=-20, max=20, unit="px", meaning="how far the letter is eaten in")

edge = btvec.Field.distance(word)
bite = btvec.Field.noise(wavelength, seed=3, octaves=3)
# the noise grows from nothing on the edge to its full 40 pixels at 60 pixels from it
grow = abs(edge).min(60) * (1 / 60)
eaten = edge + bite * 40 * grow

# trace_levels keeps where a field is at or above a level, so the letters are where minus the eaten distance is; the
# three levels are read in one pass, and each shape, the region of one level in order, is painted by its index
traced = btvec.Drawing.trace_levels(
    -eaten, [-40, -18, -core], box=(0, 0, WIDTH, HEIGHT), cell=2, smooth=1.5, tolerance=0.4, min_area=20
)
layers = [traced.only(index=index).fill(color) for index, color in enumerate((PALE, RUST, INK))]
drawing = btvec.Drawing(btvec.Path.rect(0, 0, WIDTH, HEIGHT).fill(PAPER), *layers, size=(WIDTH, HEIGHT))

# The frames of the piece, as SVG text.
frames = {"still": drawing.to_svg(WIDTH, HEIGHT), "fine": drawing.to_svg(WIDTH, HEIGHT, wavelength=30, core=-6)}
