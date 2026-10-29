"""Radial repeater: twelve copies of a petal, each turned 30 degrees further about the centre, in multiplied ink.

Every copy carries its index, so a per-item number (`by_index`) can vary it: here each copy is a little more opaque
than the last. `spin` is an angle, so any value wraps.
"""

from pathlib import Path

import bullterrier_vector as btvec

CENTRE = (540, 540)
petal = (
    btvec.Path()
    .move_to(CENTRE)
    .curve_to((470, 400), (500, 220), (540, 160))
    .curve_to((580, 220), (610, 400), CENTRE)
    .close()
)
spin = btvec.Param("spin", default=0, min=0, max=360, unit="deg", meaning="how far the flower has turned")

flower = (
    petal.fill("#ff4f9a")
    .blend("multiply")
    .repeat(12, rotate=30, about=CENTRE)
    .opacity(btvec.by_index(0.35, 1))
    .rotate(spin, about=CENTRE)
)
drawing = btvec.Drawing(btvec.Path.rect(0, 0, 1080, 1080).fill("#f3eee4"), flower)

Path("flower.svg").write_text(drawing.to_svg(1080, 1080, spin=15), encoding="utf-8")
