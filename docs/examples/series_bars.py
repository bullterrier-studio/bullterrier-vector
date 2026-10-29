"""A series drawn as bars: a list of numbers turned into Paths with ordinary code, the highest bar in a second ink.

No special node: a series is plain data and a loop. `only` and `without` pick shapes by an attribute; shapes given to
`Drawing` are indexed in order.
"""

from pathlib import Path

import bullterrier_vector as btvec

series = [0.18, 0.35, 0.52, 0.41, 0.77, 0.93, 0.64, 0.49, 0.71, 0.28, 0.15, 0.33]
BASE, TALLEST = 900, 700

bars = btvec.Drawing(
    *[btvec.Path.rect(90 + 76 * i, BASE - TALLEST * v, 56, TALLEST * v, radius=8) for i, v in enumerate(series)],
    size=(1080, 1080),
)
highest = series.index(max(series))
chart = btvec.Drawing(bars.without(index=highest).fill("#15161a"), bars.only(index=highest).fill("#e2412e"))

Path("bars.svg").write_text(chart.to_svg(1080, 1080), encoding="utf-8")
