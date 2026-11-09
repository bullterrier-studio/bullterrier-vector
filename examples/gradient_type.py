"""A gradient headline: type filled with a linear gradient, a glow behind it and a ring stroked with a radial one.

A gradient lies in the design frame, not in the shape it paints, so one fade runs across every letter and a letter that
moves shows another part of it; set the paint after the transforms. Its stops carry an alpha, which fades the glow to
nothing. Draw it with `uv run poe look examples/gradient_type.py` (PNG and SVG in `dist/look/`); the mesh raises for a
gradient, as it does for a blurred shadow.
"""

import bullterrier_vector as btvec

PAPER = "#16151a"

fade = btvec.linear_gradient([(0, "#ff0066"), (0.55, "#ffcc00"), (1, "#33d6ff")], start=(60, 0), end=(540, 0))
glow = btvec.radial_gradient([(0, "#ff006699"), (1, "#ff006600")], center=(450, 210), radius=230)
ring = btvec.radial_gradient([(0, "#ffffff00"), (1, "#ffffffe0")], center=(450, 210), radius=190)

paper = btvec.Path.rect(0, 0, 900, 420).fill(PAPER)
halo = btvec.Path.circle((450, 210), 230).fill(glow)
edge = btvec.Path.circle((450, 210), 185).fill("none").stroke(ring, width=6)
headline = btvec.text(
    "Gradient",
    font=btvec.Font.bundled("bricolage_grotesque"),
    size=170,
    axes={"opsz": 96, "wdth": 75, "wght": 800},
    anchor=(60, 290),
).fill(fade)

drawing = btvec.Drawing(paper, halo, edge, headline, size=(900, 420))
svg = drawing.to_svg(900, 420)
