"""Marching dashes: a dash pattern whose `phase` is a state; going from 0 to 1 moves the dashes one pattern length on.

Dashing is geometry (the dashes are pieces of the path), so it reaches SVG, pixels and a plotter alike.
"""

from pathlib import Path

import bullterrier_vector as btvec

march = btvec.Param("march", default=0, min=0, max=1, unit="cycle", meaning="how far the dashes have moved")
loop = btvec.Path.rect(140, 140, 800, 520, radius=60)
drawing = btvec.Drawing(loop, size=(1080, 800)).dash([28, 16], phase=march).stroke("#15161a", width=8)

for value in (0, 0.25, 0.5):
    Path(f"dashes-{value}.svg").write_text(drawing.to_svg(1080, 800, march=value), encoding="utf-8")
