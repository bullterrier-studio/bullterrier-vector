"""A ring from offsets and a boolean: computed at once on a Path, then the same verbs as steps on a Drawing.

`offset` and `subtract` mean the same on a Path (computed now) and on a Drawing (steps computed at draw time, so their
numbers may be Params).
"""

from pathlib import Path

import bullterrier_vector as btvec

circle = btvec.Path.circle((200, 200), 120)

ring = circle.offset(14).subtract(circle.offset(-14))  # a Path, now: an outer circle and a hole
on_band = ring.contains((200, 80))  # True; the centre, (200, 200), is in the hole: False

half_width = btvec.Param("half_width", default=14, min=1, max=60, unit="px", meaning="half the ring's width")
disc = btvec.Drawing(circle, size=(400, 400))
live = disc.offset(half_width).subtract(disc.offset(-half_width)).fill("#0d0d0d")

for value in (4, 14, 40):
    Path(f"ring-{value}.svg").write_text(live.to_svg(400, 400, half_width=value), encoding="utf-8")
