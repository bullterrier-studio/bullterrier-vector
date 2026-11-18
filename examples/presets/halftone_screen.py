"""Preset `halftone_screen`: a headline as a dot screen, solid at its foot and bare at its top."""

import bullterrier_vector as btvec

WIDTH, HEIGHT = 1080, 560
PAPER, RED = "#f3eee4", "#d4452b"

pitch = btvec.Param("pitch", default=9, min=5, max=18, unit="px", meaning="the distance between the dots")
angle = btvec.Param("angle", default=45, min=0, max=45, unit="", meaning="the angle the rows of dots are turned by")
word = btvec.text(
    "TONE",
    font=btvec.Font.bundled("bricolage_grotesque"),
    size=420,
    axes={"opsz": 96, "wdth": 75, "wght": 800},
    anchor=(WIDTH / 2, 430),
    align="center",
).resolve()
fade = btvec.Field.ramp((0, 430), (0, 150))  # 0 (solid) at the baseline, 1 (bare) at the cap height

drawing = btvec.Drawing(
    btvec.Path.rect(0, 0, WIDTH, HEIGHT).fill(PAPER),
    word.halftone(RED, tone=fade, pitch=pitch, angle=angle),
    size=(WIDTH, HEIGHT),
)
