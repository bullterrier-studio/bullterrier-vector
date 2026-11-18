"""Preset `draw_on`: a word whose outlines draw themselves on, letter by letter, as `progress` grows."""

import bullterrier_vector as btvec

WIDTH, HEIGHT = 1080, 600
PAPER, INK, PINK = "#f3eee4", "#16151a", "#ff4f9a"

progress = btvec.Param("progress", default=1, min=0, max=1, unit="fraction", meaning="how much of the word is drawn")

word = btvec.text(
    "WRITE",
    font=btvec.Font.bundled("bricolage_grotesque"),
    size=300,
    axes={"opsz": 96, "wdth": 100, "wght": 800},
    anchor=(WIDTH / 2, 410),
    align="center",
).resolve()  # one outline per letter, so a T is not two bars
drawn = word.slice(0, btvec.stagger(progress, spread=0.6))
drawing = btvec.Drawing(
    btvec.Path.rect(0, 0, WIDTH, HEIGHT).fill(PAPER),
    drawn.translate(8, 8).stroke(PINK, width=5),
    drawn.stroke(INK, width=5),
    size=(WIDTH, HEIGHT),
)
