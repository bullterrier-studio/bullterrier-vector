"""Preset `brush_taper`: a brush stroke that swells from nothing and thins to a point, over a word, on `width`."""

import bullterrier_vector as btvec

WIDTH, HEIGHT = 1080, 560
PAPER, INK, RED = "#f3eee4", "#16151a", "#d4452b"

width = btvec.Param("width", default=56, min=8, max=90, unit="px", meaning="how thick the brush is at its fullest")
word = btvec.text(
    "Brush",
    font=btvec.Font.bundled("fraunces"),
    size=330,
    axes={"opsz": 144, "wght": 700, "SOFT": 100, "WONK": 1},
    anchor=(WIDTH / 2, 350),
    align="center",
).fill(INK)
swash = btvec.Path.from_svg_path("M110 460 C 300 410 520 505 720 460 S 930 410 980 395")
stroke = swash.stroke(
    RED, width=width, profile=[(0, 0), (0.1, 0.55), (0.25, 0.9), (0.5, 1), (0.75, 0.9), (0.9, 0.55), (1, 0)]
)

drawing = btvec.Drawing(btvec.Path.rect(0, 0, WIDTH, HEIGHT).fill(PAPER), word, stroke, size=(WIDTH, HEIGHT))
