"""Glyph split and scatter: letters broken into their parts and scattered by seeded random amounts as `scatter` grows.

`split` gives every part of a letter (the dot and the stem of an exclamation mark) its own shape; each keeps its
letter's attributes and turns about its own centre. Every `random` has its own `seed`; at `scatter` 0 the word stands as
set. Past about 11 degrees of tilt, scattered letters read as a ransom note. The font is a subset of Anybody (SIL OFL
1.1, examples/fonts/OFL.txt). Draw it with `uv run poe look examples/glyph_scatter.py scatter=0.5` (PNG and SVG in
`dist/look/`).
"""

from pathlib import Path

import bullterrier_vector as btvec

font = btvec.Font((Path(__file__).parent / "fonts" / "anybody-subset.ttf").read_bytes(), name="anybody")
scatter = btvec.Param("scatter", default=0, min=0, max=1, unit="fraction", meaning="how far the parts have flown")

parts = (
    btvec.text("MIX!", font=font, size=260, axes={"wght": 800}, anchor=(540, 420), align="center")
    .split()
    .translate(btvec.random(-160, 160, seed=1) * scatter, btvec.random(-120, 120, seed=2) * scatter)
    .rotate(btvec.random(-11, 11, seed=3) * scatter)
    .fill("#15161a")
)
drawing = btvec.Drawing(parts, size=(1080, 640))
