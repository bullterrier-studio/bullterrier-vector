"""Hatched poster, the third reference piece: flow-field hatching inside giant letters, for print and for a pen plotter.

Two words in a heavy, condensed variable font are filled with evenly spaced streamlines that follow a noise flow and
grow denser towards the bottom until they close into solid ink. The O is drawn instead as rings in a second ink, each
following both its outer edge and its counter. Written as a PNG, as an SVG for print, and as a plotter file with one
layer per ink.
"""

from pathlib import Path

import bullterrier_vector as btvec

WIDTH, HEIGHT = 1080, 1350
PAPER, INK, ACCENT = "#f1ece1", "#15161a", "#e2412e"

font = btvec.Font(Path("fonts/Anybody[wdth,wght].ttf").read_bytes())

letters = btvec.text(
    "LOW\nEND",
    font=font,
    size=600,
    axes={"wght": 900},
    leading=560,
    anchor=(WIDTH / 2, 620),
    align="center",
    width=1000,
    fit="wdth",
)
accent = letters.only(char="O")
rest = letters.without(char="O")

# A tone of 0 is black. Black at the foot, white near the top: spacing runs from 2.4 px (lines merge into solid ink at a
# 2.4 px stroke) to 14 px.
tone = btvec.Field.ramp((0, 1300), (0, 150))
hatching = rest.hatch(flow=btvec.Flow.noise(wavelength=520, seed=11), spacing=(2.4, 14), tone=tone)

# The O's normalised distance is 0 on its outer edge and 1 on its counter, so every ring follows both.
rings = btvec.trace(btvec.Field.distance(accent, normalized=True), levels=[i / 12 for i in range(1, 12)])

ink = btvec.Drawing(hatching.stroke(INK, width=2.4), rings.stroke(ACCENT, width=2.4), size=(WIDTH, HEIGHT))
poster = btvec.Drawing(btvec.Path.rect(0, 0, WIDTH, HEIGHT).fill(PAPER), ink)

Path("low-end.png").write_bytes(poster.to_png(2 * WIDTH, 2 * HEIGHT))
Path("low-end.svg").write_text(poster.to_svg(WIDTH, HEIGHT), encoding="utf-8")
# Paper is not drawn by a pen: the plotter file is the ink alone, 300 x 375 mm, one layer per stroke colour.
Path("low-end-plotter.svg").write_text(ink.to_plotter_svg(300, 375, pen_mm=0.5), encoding="utf-8")
