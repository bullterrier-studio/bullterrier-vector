"""A halftone: discs and panels filled with a screen of dots, over paper.

The dots sit on a layout anchored to the origin of the design frame (a grid, turned 45 degrees by default), so they keep
their place when a shape moves. `tone` runs from 0 (solid) to 1 (no dots): a number, which may differ per shape (the
row of discs), or a `Field` read at the center of each dot (the panels, whose tone is a ramp). Each panel is a dot shape
on a screen: ellipses on lines, euclidean dots on rings, diamonds on waves, a triangle of the caller's on rays. Every
dot covers `1 - tone` of its own cell, so a ramp from solid to bare fades evenly on every screen.
Run it with `uv run poe look examples/halftone.py`.
The same program in Rust and JavaScript: `crates/core/examples/halftone.rs`, `crates/wasm/examples/halftone.mjs`.
"""

from typing import Any

import bullterrier_vector as btvec

WIDTH, HEIGHT = 640, 520
INK = "#1a1a1a"
COUNT = 7

paper = btvec.Drawing(btvec.Path.rect(0, 0, WIDTH, HEIGHT)).fill("#f4ecd8")

# a row of discs whose tone steps from nearly solid to nearly bare
row = btvec.Drawing(*(btvec.Path.circle((60 + 90 * i, 60), 38) for i in range(COUNT)))
ramp = row.halftone(INK, tone=btvec.by_index(0.1, 0.9), spacing=6)

# four panels, each a shape of dot on a screen, the tone a ramp from solid to bare
TRIANGLE = btvec.Path.from_svg_path("M0 -1 L1 1 L-1 1 Z")
PANELS: list[tuple[str | btvec.Path, dict[str, Any], tuple[tuple[int, int], tuple[int, int]]]] = [
    ("ellipse", {"aspect": 2, "angle": 20, "spacing": 7}, ((0, 0), (1, 0))),
    ("euclidean", {"family": "rings", "spacing": 6}, ((0, 0), (0, 1))),
    ("diamond", {"family": "waves", "amplitude": 1, "wavelength": 6, "angle": 0, "spacing": 7}, ((0, 0), (1, 1))),
    (TRIANGLE, {"family": "rays", "spacing": 8}, ((0, 1), (1, 0))),
]
panels = []
for i, (shape, options, (start, end)) in enumerate(PANELS):
    left, top = 30 + 150 * i, 130
    tone = btvec.Field.ramp((left + 120 * start[0], top + 120 * start[1]), (left + 120 * end[0], top + 120 * end[1]))
    panel = btvec.Drawing(btvec.Path.rect(left, top, 120, 120)).halftone(INK, tone=tone, shape=shape, **options)
    panels.append(panel)

# one big disc with a coarser screen turned another way, and a stroke over its dots
big = btvec.Path.circle((320, 390), 100).halftone("#c4372b", tone=0.45, spacing=9, angle=15).stroke("#c4372b", width=3)

drawing = btvec.Drawing(paper, ramp, *panels, big, size=(WIDTH, HEIGHT))
svg = drawing.to_svg(WIDTH, HEIGHT)
frames = {"still": svg}
tones = [halftone.tone for shape in ramp.shapes() if (halftone := shape.style.halftone)]  # 0.1 up to 0.9 in equal steps
