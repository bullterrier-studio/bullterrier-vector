"""Draw-on, letter by letter: one `progress` staggered across the letters, each outline drawing itself on.

Every closed outline is measured from its topmost point, so the start does not jump while the letters change. A letter
with a counter draws its outer edge first, then the counter.
"""

from pathlib import Path

import bullterrier_vector as btvec

font = btvec.Font(Path("fonts/Anybody[wdth,wght].ttf").read_bytes())
progress = btvec.Param("progress", default=1, min=0, max=1, unit="fraction", meaning="how much of the word is drawn")

word = btvec.text("WRITE", font=font, size=300, axes={"wght": 300}, at=(540, 420), align="center")
each = btvec.ease.out_cubic(btvec.stagger(progress, spread=0.6))  # per letter: 0 to 1, each a little after the last
drawing = btvec.Drawing(word.slice(0, each).stroke("#15161a", width=6), size=(1080, 600))

for value in (0.25, 0.5, 0.75, 1):
    Path(f"write-{value}.png").write_bytes(drawing.to_png(1080, 600, progress=value))
