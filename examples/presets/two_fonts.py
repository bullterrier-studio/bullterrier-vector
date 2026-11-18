"""Preset `two_fonts`: a serif over a heavy sans, both bundled; `weight` lightens the serif."""

import bullterrier_vector as btvec

WIDTH, HEIGHT = 1080, 600
PAPER, INK, PINK = "#f3eee4", "#16151a", "#ff4f9a"

weight = btvec.Param("weight", default=600, min=100, max=900, meaning="how heavy the serif line's letters are")

serif = btvec.text(
    "Soft & wonky",
    font=btvec.Font.bundled("fraunces"),
    size=170,
    axes={"opsz": 144, "wght": weight, "SOFT": 100, "WONK": 1},
    anchor=(WIDTH / 2, 290),
    align="center",
).fill(INK)
sans = btvec.text(
    "BRICOLAGE GROTESQUE",
    font=btvec.Font.bundled("bricolage_grotesque"),
    size=84,
    axes={"opsz": 96, "wdth": 75, "wght": 800},
    anchor=(WIDTH / 2, 440),
    align="center",
).fill(PINK)

drawing = btvec.Drawing(btvec.Path.rect(0, 0, WIDTH, HEIGHT).fill(PAPER), serif, sans, size=(WIDTH, HEIGHT))
