"""A brush stroke that draws itself on and thins to a point where it ends.

`stroke(.., profile=)` makes the width follow the profile along the path: 0 at the start, full from 15 % to 85 %, 0 at
the end. The stroke is built from the path as it is when drawn, so `slice(0, progress)` thins the tip where the stroke
now ends. Draw it with `uv run poe look examples/brush_taper.py progress=0.7` (PNG and SVG in `dist/look/`).
"""

import bullterrier_vector as btvec

progress = btvec.Param("progress", default=1, min=0, max=1, unit="fraction", meaning="how much of the stroke is drawn")

swash = btvec.Path.from_svg_path("M120 380 C 260 80 420 520 560 300 S 820 120 960 240")
taper = [(0, 0), (0.15, 1), (0.85, 1), (1, 0)]
drawing = btvec.Drawing(
    swash.stroke("#15161a", width=36, profile=taper).slice(0, progress).shadow(dx=6, dy=8, blur=12),
    size=(1080, 600),
)
