"""Glyph split and scatter: letters broken into their parts and scattered by seeded random amounts as `scatter` grows.

`split` gives every part of a letter (the dot of an i, the two marks of a colon) its own shape; each keeps its letter's
attributes. Past about 11 degrees of tilt, scattered letters read as a ransom note.
"""

from pathlib import Path

import bullterrier_vector as btvec

font = btvec.Font(Path("fonts/Anybody[wdth,wght].ttf").read_bytes())
scatter = btvec.Param("scatter", default=0, min=0, max=1, unit="fraction", meaning="how far the parts have flown")

parts = (
    btvec.text("MIX:i!", font=font, size=260, axes={"wght": 800}, at=(540, 420), align="center")
    .split()
    .translate(btvec.random(-160, 160, seed=1) * scatter, btvec.random(-120, 120, seed=2) * scatter)
    .rotate(btvec.random(-11, 11, seed=3) * scatter)
    .fill("#15161a")
)
drawing = btvec.Drawing(parts, size=(1080, 640))

for value in (0, 0.5, 1):
    Path(f"scatter-{value}.png").write_bytes(drawing.to_png(1080, 640, scatter=value))
