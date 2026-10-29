"""Frame export: a caller's function of time, looped over frames into a transparent PNG sequence and a clip with alpha.

The drawing knows no time. `at_time` turns seconds into values; `frames` calls it once per frame and writes each frame
exactly as `to_png` would draw it. The background is transparent, ready to lay over footage, and the word is knocked
out of the badge, so the footage shows through the letters.
"""

from pathlib import Path

import bullterrier_vector as btvec

CENTRE = (540, 540)
spin = btvec.Param("spin", default=0, min=0, max=360, unit="deg", meaning="how far the ring has turned")
pop = btvec.Param("pop", default=1, min=0.2, max=1.2, unit="", meaning="the badge's scale")

font = btvec.Font(Path("fonts/Anybody[wdth,wght].ttf").read_bytes())
ring = btvec.Path.circle(CENTRE, 300).dash([60, 30]).stroke("#ff4f9a", width=24).rotate(spin, about=CENTRE)
disc = btvec.Path.circle(CENTRE, 240).fill("#ff4f9a")
word = btvec.text("LIVE", font=font, size=170, axes={"wght": 900}, anchor=(540, 600), align="center").knockout()
badge = btvec.Drawing(ring, disc, word, size=(1080, 1080)).scale(pop, about=CENTRE)


def at_time(t: float) -> btvec.Drawing:
    """The badge `t` seconds in: it pops in over half a second and turns a quarter turn a second."""
    return badge.with_values(spin=90 * t, pop=0.2 + 0.8 * btvec.ease.out_back(t / 0.5))


btvec.frames(at_time, width=1080, height=1080, fps=30, seconds=4, out="frames/badge-####.png")
btvec.frames(at_time, width=1080, height=1080, fps=30, seconds=4, out="badge.mov")
