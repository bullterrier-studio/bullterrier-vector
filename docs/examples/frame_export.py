"""Frame export: a caller's function of time, looped over frames into a transparent PNG sequence and a clip with alpha.

The drawing knows no time. `at_time` turns seconds into values; `frames` calls it once per frame and writes each frame
exactly as `to_png` would draw it. The background is transparent, ready to lay over footage.
"""

import bullterrier_vector as btvec

spin = btvec.Param("spin", default=0, min=0, max=360, unit="deg", meaning="how far the ring has turned")
pop = btvec.Param("pop", default=1, min=0, max=1.2, unit="", meaning="the badge's scale")

ring = btvec.Path.circle((540, 540), 300).dash([60, 30]).stroke("#ff4f9a", width=24).rotate(spin, about=(540, 540))
badge = btvec.Drawing(ring, size=(1080, 1080)).scale(pop, about=(540, 540))


def at_time(t: float) -> btvec.Drawing:
    """The badge `t` seconds in: it pops in over half a second and turns a quarter turn a second."""
    return badge.with_values(spin=90 * t, pop=btvec.ease.out_back(t / 0.5))


btvec.frames(at_time, width=1080, height=1080, fps=30, seconds=4, out="frames/badge-####.png")
btvec.frames(at_time, width=1080, height=1080, fps=30, seconds=4, out="badge.mov")
