"""Cut paper: a headline on stacked layers of paper, each one `step` wider than the one above it.

`offset` grows the headline's outline, so every layer is the lettering, bigger, in a new colour and nesting inside the
next one down. `merge` and `resolve` first make the letters one shape, so the layers join where the letters come close
and the cut line, an `outline` ring around the outermost layer, runs round the whole word once. `step` is a Param: at 0
the layers are the lettering alone. Draw it with `uv run poe look examples/cut_paper_layers.py step=12` (PNG and SVG in
`dist/look/`).
"""

import bullterrier_vector as btvec

PAPER, INK = "#f4efe6", "#17161c"
LAYERS = ["#1f8a8a", "#f2b632", "#ee5a3c", "#f4efe6"]  # the paper from the lettering outwards
WIDTH, HEIGHT = 1000, 420

step = btvec.Param("step", default=14, min=0, max=30, unit="px", meaning="how much wider each layer is than the last")

font = btvec.Font.bundled("bricolage_grotesque")
lettering = (
    btvec.text(
        "LAYERS", font=font, size=210, axes={"opsz": 96, "wght": 800, "wdth": 100}, anchor=(500, 280), align="center"
    )
    .merge()
    .resolve()  # the letters' overlapping contours become one outline
)

# the outermost layer first, so each nearer one is drawn over it
layers = [lettering.offset(step * (n + 1)).fill(color) for n, color in reversed(list(enumerate(LAYERS)))]
cut_line = lettering.offset(step * (len(LAYERS) + 1)).outline(3).fill(INK)  # a ring around the outermost layer

drawing = btvec.Drawing(
    btvec.Path.rect(0, 0, WIDTH, HEIGHT).fill(PAPER),
    *layers,
    lettering.fill(INK),
    cut_line,
    size=(WIDTH, HEIGHT),
)
shapes = drawing.shapes()
svg = drawing.to_svg(WIDTH, HEIGHT)
