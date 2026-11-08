"""A halftone: discs filled with a screen of dots, one tone per disc, over paper.

The dots sit on a square lattice anchored to the origin of the design frame, turned 45 degrees, so they keep their place
when a shape moves; `tone` runs from 0 (solid) to 1 (no dots) and may differ per shape. Run it with
`uv run poe look examples/halftone.py`.
"""

import bullterrier_vector as btvec

COUNT = 7

paper = btvec.Drawing(btvec.Path.rect(0, 0, 640, 360)).fill("#f4ecd8")

# a row of discs whose tone steps from nearly solid to nearly bare
row = btvec.Drawing(*(btvec.Path.circle((60 + 90 * i, 90), 38) for i in range(COUNT)))
ramp = row.halftone("#1a1a1a", tone=btvec.by_index(0.1, 0.9), pitch=6)

# one big disc with a coarser screen turned another way, and a stroke over its dots
big = btvec.Path.circle((320, 250), 90).halftone("#c4372b", tone=0.45, pitch=9, angle=15).stroke("#c4372b", width=3)

drawing = btvec.Drawing(paper, ramp, big, size=(640, 360))
svg = drawing.to_svg(640, 360)
tones = [halftone.tone for shape in ramp.shapes() if (halftone := shape.style.halftone)]  # 0.1 up to 0.9 in equal steps
