"""Preset `text_on_path`: a word riding round a ring of guide line, moved along it by `progress`."""

import bullterrier_vector as btvec

WIDTH, HEIGHT = 1080, 560
PAPER, INK, GUIDE = "#f3eee4", "#16151a", "#d4452b"

progress = btvec.Param(
    "progress", default=0, min=0, max=1, unit="fraction", meaning="how far the text has travelled, up to half way round"
)
ring = btvec.Path.circle((WIDTH / 2, HEIGHT / 2), 165)
line = btvec.text(
    "ROUND AND ROUND AND ",
    font=btvec.Font.bundled("bricolage_grotesque"),
    size=118,
    axes={"opsz": 96, "wdth": 75, "wght": 800},
    tracking=14,
)

drawing = btvec.Drawing(
    btvec.Path.rect(0, 0, WIDTH, HEIGHT).fill(PAPER),
    ring.fill("none").stroke(GUIDE, width=2),
    line.along(ring, start=0.5 * progress).fill(INK),
    size=(WIDTH, HEIGHT),
)
