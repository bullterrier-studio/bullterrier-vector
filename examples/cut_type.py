"""A headline faded out by a mask, over stripes cut to a circular window.

`clip(region)` keeps what lies inside a region and `mask(source)` fades by the luminance of other shapes (white keeps,
black hides); each shape is cut on its own, so a headline of letters is one mask in the document, used by every letter.
Both lie in the design frame, like a gradient. Draw it with `uv run poe look examples/cut_type.py` (PNG and SVG in
`dist/look/`); the mesh raises for a clip and a mask, as it does for a gradient.
"""

import bullterrier_vector as btvec

paper = btvec.Path.rect(0, 0, 900, 420).fill("#16151a")

window = btvec.Path.circle((450, 210), 190)
stripes = btvec.Drawing(*[btvec.Path.rect(x, 0, 24, 420).fill("#33d6ff") for x in range(0, 900, 48)])
cut_stripes = stripes.clip(window)

fade = btvec.linear_gradient([(0, "#ffffff"), (1, "#000000")], start=(60, 0), end=(840, 0))
headline = btvec.text(
    "Fade out",
    font=btvec.Font.bundled("bricolage_grotesque"),
    size=170,
    axes={"opsz": 96, "wdth": 75, "wght": 800},
    anchor=(60, 290),
).fill("#ffcc00")
faded = headline.mask(btvec.Path.rect(0, 0, 900, 420).fill(fade))

drawing = btvec.Drawing(paper, cut_stripes, faded, size=(900, 420))
svg = drawing.to_svg(900, 420)
