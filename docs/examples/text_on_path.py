"""Text on a path: a line of type laid out straight, then bent around a circle; `turn` moves it around.

`along` maps each letter's place on its baseline to a distance along the path, so it works for any shapes, not only
text. `start` is a fraction of the path's length and wraps on a closed path, so a `cycle` Param turns the text round.
"""

from pathlib import Path

import bullterrier_vector as btvec

font = btvec.Font(Path("fonts/Anybody[wdth,wght].ttf").read_bytes())
orbit = btvec.Path.circle((540, 540), 360)

turn = btvec.Param("turn", default=0, min=0, max=1, unit="cycle", meaning="how far the text has travelled around")
line = btvec.text("ROUND AND ROUND AND ROUND AND ", font=font, size=72, axes={"wght": 700}, tracking=60)

drawing = btvec.Drawing(
    orbit.stroke("#d8d2c4", width=2),
    line.along(orbit, start=turn).fill("#15161a"),
    size=(1080, 1080),
)
Path("orbit.svg").write_text(drawing.to_svg(1080, 1080, turn=0.125), encoding="utf-8")
