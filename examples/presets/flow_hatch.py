"""Preset `flow_hatch`: a headline filled with lines that follow its letters, crowding at the edges."""

import bullterrier_vector as btvec

WIDTH, HEIGHT = 1080, 560
PAPER, INK = "#f3eee4", "#16151a"

weight = btvec.Param("weight", default=800, min=300, max=800, meaning="the weight of the letters")
word = btvec.text(
    "FLOW",
    font=btvec.Font.bundled("bricolage_grotesque"),
    size=470,
    axes={"opsz": 96, "wdth": 75, "wght": weight},
    anchor=(WIDTH / 2, 435),
    align="center",
).resolve()
flow = btvec.Flow.contour(word).mix(btvec.Flow.noise(wavelength=420, seed=9), 0.3)
tone = btvec.Field.distance(word, normalized=True)

drawing = btvec.Drawing(
    btvec.Path.rect(0, 0, WIDTH, HEIGHT).fill(PAPER),
    word.hatch((3, 9), tone=tone, flow=flow).stroke(INK, width=1.4),
    size=(WIDTH, HEIGHT),
)
