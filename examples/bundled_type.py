"""Type in the two fonts that come with the library: no font file to find or ship.

Fraunces, soft and wonky, over Bricolage Grotesque, narrow and heavy. `Font.bundled_names()` lists the fonts; the axes
of each are `Font.axes`. A saved drawing names a bundled font by its digest, so its JSON loads with no bytes supplied.
Draw it with `uv run poe look examples/bundled_type.py` (PNG and SVG in `dist/look/`).
"""

import bullterrier_vector as btvec

PAPER, INK, PINK = "#f3eee4", "#16151a", "#ff4f9a"

serif = btvec.Font.bundled("fraunces")
sans = btvec.Font.bundled("bricolage_grotesque")

# Fraunces opens at its black 9 point cut: the axes pick the soft, wonky display weight
headline = btvec.text(
    "Soft & wonky",
    font=serif,
    size=135,
    axes={"opsz": 144, "wght": 600, "SOFT": 100, "WONK": 1},
    anchor=(50, 190),
).fill(INK)

# Bricolage Grotesque: its narrowest width and heaviest weight
subline = btvec.text(
    "Bricolage Grotesque",
    font=sans,
    size=110,
    axes={"opsz": 96, "wdth": 75, "wght": 800},
    anchor=(50, 330),
).fill(PINK)

drawing = btvec.Drawing(btvec.Path.rect(0, 0, 900, 400).fill(PAPER), headline, subline, size=(900, 400))
