"""Bent type, the deformer half of the kinetic type piece: one line that arches, ripples, twists and wobbles.

`progress` (0 to 1) bends the line: it arches up, a sine runs along it, its middle turns and the outlines shake a
little. `phase` (in cycles) moves the sine and the shaking, so a loop of `phase` 0 to 1 repeats exactly. Every
deformer cuts the outlines into short straight pieces, so the bend is smooth at any size. Each is anchored in the
design frame, so one wave runs across all the letters; the twist names its center (`about`) to turn the whole line
about its middle instead of each letter about itself. The font comes with the library (SIL OFL 1.1).
Draw it with `uv run poe look examples/bent_type.py progress=1 phase=0.25` (PNG and SVG in `dist/look/`).
The same program in Rust and JavaScript: `crates/core/examples/bent_type.rs`, `crates/wasm/examples/bent_type.mjs`.
"""

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

# Three frames of the piece: calm, half bent, bent and the ripple a quarter of a cycle on.
FRAMES = {"calm": (0, 0), "half": (0.5, 0.125), "bent": (1, 0.25)}
frames = {name: drawing.to_svg(WIDTH, HEIGHT, progress=p, phase=ph) for name, (p, ph) in FRAMES.items()}
