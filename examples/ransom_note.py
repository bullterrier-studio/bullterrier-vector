"""A ransom note: one headline whose letters come from three typefaces at different sizes and weights.

Each letter is a `run` with a font, size and axes of its own; `text` shapes them apart and sets them on shared lines, so
the headline wraps to a measure and each line is stretched to exactly that width (`fit="size"` scales every run by one
factor, keeping their proportions). The same text then goes round an arc with `along`. The fonts are the two bundled
ones (SIL OFL 1.1) and a subset of Anybody (examples/fonts/OFL.txt). Draw it with
`uv run poe look examples/ransom_note.py`.
"""

from pathlib import Path

import bullterrier_vector as btvec

PAPER, INK, PINK = "#f3eee4", "#16151a", "#ff4f9a"
MEASURE = 860

serif = btvec.Font.bundled("fraunces")
sans = btvec.Font.bundled("bricolage_grotesque")
mono = btvec.Font((Path(__file__).parent / "fonts" / "anybody-subset.ttf").read_bytes(), name="anybody")

# a letter's look: (font, size, axes), taken in turn
LOOKS = [
    (serif, 150, {"opsz": 144, "wght": 900, "WONK": 1}),
    (sans, 110, {"opsz": 96, "wdth": 75, "wght": 800}),
    (mono, 130, {"wdth": 150, "wght": 900}),
    (serif, 95, {"opsz": 9, "wght": 300}),
    (sans, 170, {"opsz": 12, "wdth": 100, "wght": 400}),
]
HEADLINE = "SEE THE CAT NO MORE"

runs = []
for at, letter in enumerate(HEADLINE):
    font, size, axes = LOOKS[at % len(LOOKS)]
    runs.append(btvec.run(letter, font=font, size=size, axes=axes))

# wrapped to the measure at whole words, then every line stretched to it: the letters keep their different sizes
note = btvec.text(runs, width=MEASURE, fit="size", align="center", anchor=(540, 240))

# the same runs along an arc, its first letters bent over the top of a circle
arc = btvec.Path.circle((540, 1090), 200)
bent = btvec.text(runs[:9], size=80, anchor=(0, 0), tracking=80).along(arc, start=0.74)

drawing = btvec.Drawing(
    btvec.Path.rect(0, 0, 1080, 1350).fill(PAPER),
    note.fill(INK),
    bent.fill(PINK),
    size=(1080, 1350),
)
