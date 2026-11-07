"""Cut paper: a headline cut out of a plate, and two circles whose overlap is a lens.

`subtract` cuts the word out of the plate, so the sheet underneath shows through the letters. `resolve` first turns each
glyph's overlapping contours into one outline, so the cut is the letter and not the sum of its pieces. `intersect` keeps
what two circles share, `exclude` what only one covers (both crescents); `progress` slides the circles together, and
each step is computed again for each value. Draw it with `uv run poe look examples/cutout.py progress=0.3` (PNG and SVG
in `dist/look/`).
"""

import bullterrier_vector as btvec

PAPER, SHEET, INK, CORAL, TEAL = "#f4efe6", "#f2b632", "#17161c", "#ee5a3c", "#1f8a8a"
WIDTH, HEIGHT = 1000, 820

progress = btvec.Param(
    "progress", default=0.6, min=0, max=1, unit="fraction", meaning="how far the circles have slid together"
)

font = btvec.Font.bundled("bricolage_grotesque")
headline = btvec.text(
    "CUT", font=font, size=330, axes={"opsz": 96, "wght": 800, "wdth": 100}, anchor=(500, 380), align="center"
).resolve()  # the glyphs' overlapping contours become one outline each

# the sheet shows through the letters cut out of the plate
plate = btvec.Drawing(btvec.Path.rect(70, 70, 860, 400, radius=44)).subtract(headline)

# two discs that slide from 240 apart (progress 0) to 80 apart (progress 1)
reach = 120 - 40 * progress
left = btvec.Drawing(btvec.Path.circle((500, 640), 110)).translate(-reach, 0)
right = btvec.Drawing(btvec.Path.circle((500, 640), 110)).translate(reach, 0)

drawing = btvec.Drawing(
    btvec.Path.rect(0, 0, WIDTH, HEIGHT).fill(PAPER),
    btvec.Path.rect(40, 40, 920, 460, radius=60).fill(SHEET),
    plate.fill(INK),
    left.exclude(right).fill(CORAL),  # what only one disc covers: both crescents, one shape
    left.intersect(right).fill(TEAL),  # the lens, what both cover
    size=(WIDTH, HEIGHT),
)
shapes = drawing.shapes()
svg = drawing.to_svg(WIDTH, HEIGHT)
