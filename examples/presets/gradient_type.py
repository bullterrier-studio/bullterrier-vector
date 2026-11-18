"""Preset `gradient_type`: a headline filled with one gradient, a glow and a ring."""

import bullterrier_vector as btvec

PAPER = "#16151a"

weight = btvec.Param("weight", default=800, min=300, max=800, meaning="how heavy the headline's letters are")

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
    axes={"opsz": 96, "wdth": 75, "wght": weight},
    anchor=(60, 290),
).fill(fade)

drawing = btvec.Drawing(paper, halo, edge, headline, size=(900, 420))
