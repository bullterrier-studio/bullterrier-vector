"""Preset `glyph_scatter`: a word broken into its parts and flung about by seeded random amounts."""

import bullterrier_vector as btvec

WIDTH, HEIGHT = 1080, 600
PAPER, INK, RED = "#f3eee4", "#16151a", "#d4452b"

scatter = btvec.Param("scatter", default=0.6, min=0, max=1, unit="fraction", meaning="how far the parts have flown")

parts = (
    btvec.text(
        "SCATTER",
        font=btvec.Font.bundled("bricolage_grotesque"),
        size=280,
        axes={"opsz": 96, "wdth": 75, "wght": 800},
        anchor=(WIDTH / 2, 390),
        align="center",
    )
    .resolve()
    .split()
    .translate(0, btvec.random(-110, 110, seed=2) * scatter)
    .rotate(btvec.random(-15, 15, seed=3) * scatter)
)
drawing = btvec.Drawing(btvec.Path.rect(0, 0, WIDTH, HEIGHT).fill(PAPER), parts.fill(INK), size=(WIDTH, HEIGHT))
