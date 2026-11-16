"""Wobble outline: letter outlines displaced by seeded noise that moves with `phase`.

Keep it small: a large wobble melts the letters and spikes at sharp corners. The weight is the font's own axis (a
subset of Anybody, SIL OFL 1.1, examples/fonts/OFL.txt), described by the font (its range and default), so a tool can
build a slider for it.
"""

from pathlib import Path

import bullterrier_vector as btvec

font = btvec.Font((Path(__file__).parent / "fonts" / "anybody-subset.ttf").read_bytes(), name="anybody")
weight = font.axes["wght"]  # a Param named "wght", with the font's range and default
phase = btvec.Param("phase", default=0, min=0, max=1, unit="cycle", meaning="where the noise has moved to")

word = btvec.text("SHAKY", font=font, size=320, axes={"wght": weight}, anchor=(540, 470), align="center")
drawing = btvec.Drawing(
    word.wobble(amplitude=3, wavelength=50, phase=phase, seed=4).stroke("#15161a", width=3), size=(1080, 640)
)

# Three frames, weight and noise moving together. Draw one: `uv run poe look examples/wobble_outline.py phase=0.3`.
frames = [drawing.to_svg(1080, 640, wght=w, phase=ph) for w, ph in ((200, 0), (350, 0.3), (500, 0.6))]
