"""Preset `stipple`: a ball shaded only by dots, dense in its shadow and bare where the light falls."""

import bullterrier_vector as btvec

WIDTH, HEIGHT = 1080, 560
PAPER, INK = "#f3eee4", "#16151a"

spacing = btvec.Param("spacing", default=5, min=3.5, max=14, unit="px", meaning="the distance between the dots")
disc = btvec.Path.circle((WIDTH / 2, HEIGHT / 2), 230)
tone = btvec.Field.ramp(
    (WIDTH / 2 + 170, HEIGHT / 2 + 170), (WIDTH / 2 - 170, HEIGHT / 2 - 170)
)  # dense where it is shaded

drawing = btvec.Drawing(
    btvec.Path.rect(0, 0, WIDTH, HEIGHT).fill(PAPER),
    btvec.Drawing(disc).stipple(spacing, tone=tone, radius=(2.2, 2.2), seed=2).fill(INK),
    size=(WIDTH, HEIGHT),
)
