"""Preset `bent_type`: one line that arches, ripples, twists and wobbles on `progress` and `phase`."""

import bullterrier_vector as btvec

WIDTH, HEIGHT = 1080, 560
PAPER, INK = "#f3eee4", "#16151a"

font = btvec.Font.bundled("bricolage_grotesque")
progress = btvec.Param("progress", default=0, min=0, max=1, unit="fraction", meaning="how far the line is bent")
phase = btvec.Param("phase", default=0, min=0, max=1, unit="cycle", meaning="where the ripple and the shaking are")

line = btvec.text(
    "NO SLEEP", font=font, size=210, axes={"opsz": 96, "wdth": 75, "wght": 800}, anchor=(WIDTH / 2, 380), align="center"
)
bent = (
    line.arch(0.25 * progress)
    .wave(amplitude=10 * progress, wavelength=240, phase=phase)
    .twist(20 * progress, about=(WIDTH / 2, 300), radius=520)
    .wobble(amplitude=2 * progress, wavelength=40, phase=phase, seed=7)
)
drawing = btvec.Drawing(btvec.Path.rect(0, 0, WIDTH, HEIGHT).fill(PAPER), bent.fill(INK), size=(WIDTH, HEIGHT))
