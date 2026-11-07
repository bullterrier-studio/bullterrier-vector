"""Draw-on, letter by letter: one `progress` staggered across the letters, each outline drawing itself on.

`stagger` turns the one `progress` into a number for each letter: 0 to 1, each a little after the last. `slice(0, ..)`
keeps that fraction of every outline, from its topmost point, so the start does not jump while the letters change. A
letter with a counter draws its outer edge first, then the counter. Nothing here knows about text. The font is a subset
of Anybody (SIL OFL 1.1, examples/fonts/OFL.txt). Draw it with `uv run poe look examples/draw_on.py progress=0.5`
(PNG and SVG in `dist/look/`).
"""

from pathlib import Path

import bullterrier_vector as btvec

font = btvec.Font((Path(__file__).parent / "fonts" / "anybody-subset.ttf").read_bytes(), name="anybody")
progress = btvec.Param("progress", default=1, min=0, max=1, unit="fraction", meaning="how much of the word is drawn")

word = btvec.text("WRITE", font=font, size=240, axes={"wght": 300}, anchor=(540, 380), align="center")
each = btvec.stagger(progress, spread=0.6)  # per letter: 0 to 1, each a little after the last
drawing = btvec.Drawing(word.slice(0, each).stroke("#15161a", width=6), size=(1080, 600))
