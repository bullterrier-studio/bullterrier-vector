"""Flow hatch: a headline filled with evenly spaced lines that follow its letters.

Each letter is hatched along the direction of its own outline, bent a little by noise, and the lines crowd together
near the edge of a letter (3 pixels apart) and open out towards its middle (9 apart): the spacing is read from the
normalised distance field of the letters, 0 on an edge and 1 on the middle of a stroke. The field and the flow take
the headline itself, a drawing, so they follow its weight: a `Param` of the text, drawn at 800 and at 300 (the frame
"light"). Nothing is drawn but lines, so the picture prints at any size and plots with a pen.
Draw it with `uv run poe look examples/flow_hatch.py` (PNG and SVG in `dist/look/`).
The same program in Rust and JavaScript: `crates/core/examples/flow_hatch.rs`, `crates/wasm/examples/flow_hatch.mjs`.
"""

import bullterrier_vector as btvec

WIDTH, HEIGHT = 1080, 560
PAPER, INK = "#f3eee4", "#16151a"

font = btvec.Font.bundled("bricolage_grotesque")
weight = btvec.Param("weight", default=800, min=300, max=800, meaning="the weight of the letters")
word = btvec.text(
    "FLOW", font=font, size=330, axes={"opsz": 96, "wdth": 75, "wght": weight}, anchor=(540, 420), align="center"
).resolve()

# the field and the flow read the headline as a drawing: at the weight a draw gives it
flow = btvec.Flow.contour(word).mix(btvec.Flow.noise(wavelength=420, seed=9), 0.3)
tone = btvec.Field.distance(word, normalized=True)

drawing = btvec.Drawing(
    btvec.Path.rect(0, 0, WIDTH, HEIGHT).fill(PAPER),
    word.hatch((3, 9), tone=tone, flow=flow).stroke(INK, width=1.4),
    size=(WIDTH, HEIGHT),
)

# The frames of the piece, as SVG text.
frames = {"still": drawing.to_svg(WIDTH, HEIGHT), "light": drawing.to_svg(WIDTH, HEIGHT, weight=300)}
