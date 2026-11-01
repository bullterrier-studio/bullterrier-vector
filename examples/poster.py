"""A small poster made only of paths: what the output calls can already draw.

A fan of rotated stars with fading opacity, two multiply inks that overlap into a third colour, a thick round-joined
zigzag and a ruling of hairlines. Draw it with `uv run poe look examples/poster.py` (PNG and SVG in `dist/look/`).
"""

import bullterrier_vector as btvec

PAPER, PINK, BLUE, INK = "#f6f1e4", "#ff4f9a", "#2e64e2", "#15161a"

layers = [btvec.Path.rect(0, 0, 800, 600).fill(PAPER)]

# twelve copies of a star, each turned 6 degrees further and a little more transparent
star = btvec.Path.star((400, 300), outer=230, inner=95, points=5)
layers.extend(btvec.Drawing(star).rotate(i * 6).stroke(INK, width=1.5).opacity(0.9 - i * 0.05) for i in range(12))

# two inks that multiply where they overlap, as printed inks do
layers.append(btvec.Path.circle((310, 300), 150).fill(PINK).blend("multiply"))
layers.append(btvec.Path.circle((490, 300), 150).fill(BLUE).blend("multiply"))

# a thick zigzag with round joins and ends
zigzag = btvec.Path.polyline(
    [(80, 520), (200, 440), (260, 540), (400, 430), (520, 540), (600, 440), (720, 520)], closed=False
)
layers.append(zigzag.stroke(INK, width=14, join="round", cap="round"))

# hairlines from half a pixel wide up
layers.extend(btvec.Path.line((60, 70 + 6 * i), (740, 82 + 6 * i)).stroke(INK, width=0.5 + 0.25 * i) for i in range(8))

drawing = btvec.Drawing(*layers, size=(800, 600))
svg = drawing.to_svg(800, 600)
rgba = drawing.to_rgba(800, 600)
shapes = drawing.shapes()
