"""Text on a path: a line of type laid out straight, then bent round a circle; `cycle` turns it round.

`along` carries any shapes along a guide path: a point `(x, y)` goes `start` lengths of the path plus `x` along it, and
`y` to the right of the way the path runs there (so letters stand outside a clockwise circle). Here the shapes are the
letters of a text, so the baseline becomes the circle. `start` is a fraction of the path's length and wraps on a closed
path, so a `cycle` Param turns the text round. The font is a subset of Anybody (SIL OFL 1.1, examples/fonts/OFL.txt).
Draw it with `uv run poe look examples/text_on_path.py cycle=0.125` (PNG and SVG in `dist/look/`).
"""

from pathlib import Path

import bullterrier_vector as btvec

PAPER, INK, GUIDE = "#f3eee4", "#15161a", "#d8d2c4"

font = btvec.Font((Path(__file__).parent / "fonts" / "anybody-subset.ttf").read_bytes(), name="anybody")
orbit = btvec.Path.circle((540, 540), 360)

cycle = btvec.Param("cycle", default=0, min=0, max=1, unit="cycle", meaning="how far the text has travelled round")
line = btvec.text("ROUND AND ROUND AND ROUND AND ", font=font, size=108, axes={"wght": 700}, tracking=60)

drawing = btvec.Drawing(
    btvec.Path.rect(0, 0, 1080, 1080).fill(PAPER),
    orbit.fill("none").stroke(GUIDE, width=2),
    line.along(orbit, start=cycle).fill(INK),
    size=(1080, 1080),
)
