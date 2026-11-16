"""Marching dashes: a dash pattern whose `phase` is a state; going from 0 to 1 moves the dashes one pattern length on.

Dashing is geometry (the dashes are pieces of the path), so it reaches SVG, pixels and a plotter alike. `phase` is in
cycles of the whole pattern (28 + 16 = 44 pixels here), so frame 0 and frame 1 are the same picture and a loop is
seamless. Draw it with `uv run poe look examples/marching_dashes.py march=0.25` (PNG and SVG in `dist/look/`).
"""

import bullterrier_vector as btvec

march = btvec.Param("march", default=0, min=0, max=1, unit="cycle", meaning="how far the dashes have moved")
loop = btvec.Path.line((140, 400), (940, 400))
drawing = btvec.Drawing(loop, size=(1080, 800)).dash([28, 16], phase=march).stroke("#15161a", width=8)
