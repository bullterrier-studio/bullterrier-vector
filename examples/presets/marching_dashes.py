"""Preset `marching_dashes`: rings of dashes that march round, each the other way, on `march`."""

import math

import bullterrier_vector as btvec

WIDTH, HEIGHT = 1080, 560
PAPER, INK, RED = "#f3eee4", "#16151a", "#d4452b"

march = btvec.Param("march", default=0, min=0, max=1, unit="cycle", meaning="how far the dashes have moved")
width = btvec.Param("width", default=12, min=3, max=22, unit="px", meaning="how thick the dashes are")
centre = (WIDTH / 2, HEIGHT / 2)


def ring(radius: float, turn: int, color: str) -> btvec.Drawing:
    """A ring cut into whole dashes (no short one at the seam) that is back where it began at `march` 1."""
    unit = 2 * math.pi * radius / round(2 * math.pi * radius / 48)
    dashed = btvec.Drawing(btvec.Path.circle(centre, radius)).dash([0.62 * unit, 0.38 * unit], phase=march * turn)
    return dashed.stroke(color, width=width)


drawing = btvec.Drawing(
    btvec.Path.rect(0, 0, WIDTH, HEIGHT).fill(PAPER),
    *(ring(r, turn, color) for r, turn, color in ((235, 1, INK), (190, -1, RED), (145, 1, INK), (100, -1, RED))),
    size=(WIDTH, HEIGHT),
)
