"""Preset `waves`: a headline made only of wavy lines, fat at the edge of each letter and thin at its heart."""

import bullterrier_vector as btvec

WIDTH, HEIGHT = 1080, 560
PAPER, INK = "#f3eee4", "#16151a"

spacing = btvec.Param("spacing", default=10, min=6, max=20, unit="px", meaning="the distance between the wave lines")
word = btvec.text(
    "WAVES",
    font=btvec.Font.bundled("bricolage_grotesque"),
    size=330,
    axes={"opsz": 96, "wdth": 75, "wght": 800},
    anchor=(WIDTH / 2, 400),
    align="center",
).resolve()
tone = btvec.Field.distance(word, normalized=True)

drawing = btvec.Drawing(
    btvec.Path.rect(0, 0, WIDTH, HEIGHT).fill(PAPER),
    word.bands(
        spacing, tone=tone, width=(7, 0.8), family="waves", angle=0, wavelength=4 * spacing, amplitude=0.4 * spacing
    ).fill(INK),
    size=(WIDTH, HEIGHT),
)
