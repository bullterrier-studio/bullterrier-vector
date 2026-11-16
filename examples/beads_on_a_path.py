"""Beads on a path: a petal stamped twelve times round a ring, each turned to the way the ring runs there, `march`
moving them round.

`place_along` copies a drawing to evenly spaced stops along a path and turns each copy to the path's direction there. It
does not bend (`along` does): the petal keeps its shape. `count` stops are evenly round a closed path (an open path has
a stop on both ends); `spacing=40` would put one every 40 pixels instead. `start` is a fraction of the path's length and
wraps on a closed one, so a Param moves them round. A drawing of one shape also gets the share 0..1 of its copy as
`order`, so `by_order` sizes the petals round the ring. Draw it with `uv run poe look examples/beads_on_a_path.py
march=0.25` (PNG and SVG in `dist/look/`).
"""

import bullterrier_vector as btvec

PAPER, INK, GUIDE = "#f3eee4", "#15161a", "#d8d2c4"

march = btvec.Param("march", default=0, min=0, max=1, unit="cycle", meaning="how far the petals have gone round")
ring = btvec.Path.circle((540, 540), 360)
petal = btvec.Path.from_svg_path(
    "M0 -60 C 40 -24 40 24 0 60 C -40 24 -40 -24 0 -60 Z"
)  # long axis on y: radial on the ring

drawing = btvec.Drawing(
    btvec.Path.rect(0, 0, 1080, 1080).fill(PAPER),
    ring.fill("none").stroke(GUIDE, width=2),
    btvec.Drawing(petal).place_along(ring, count=12, start=march).scale(btvec.by_order(0.5, 1)).fill(INK),
    size=(1080, 1080),
)
