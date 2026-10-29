"""Wobble outline: letter outlines displaced by seeded noise that moves with `phase`.

Keep it small: a large wobble melts the letters and spikes at sharp corners. The weight is the font's own axis,
described by the font (its range and default), so a tool can build a slider for it.
"""

from pathlib import Path

import bullterrier_vector as btvec

font = btvec.Font(Path("fonts/Anybody[wdth,wght].ttf").read_bytes())
weight = font.axes["wght"]  # a Param named "wght", with the font's range and default
phase = btvec.Param("phase", default=0, min=0, max=1, unit="cycle", meaning="where the noise has moved to")

word = btvec.text("SHAKY", font=font, size=320, axes={"wght": weight}, anchor=(540, 470), align="center")
drawing = btvec.Drawing(
    word.wobble(amplitude=3, wavelength=50, phase=phase, seed=4).stroke("#15161a", width=3), size=(1080, 640)
)

for weight_value, phase_value in ((300, 0), (600, 0.3), (900, 0.6)):
    svg = drawing.to_svg(1080, 640, wght=weight_value, phase=phase_value)
    Path(f"shaky-{weight_value}.svg").write_text(svg, encoding="utf-8")
