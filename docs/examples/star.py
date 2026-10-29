"""A star: build a Path, measure it, and draw it.

A Path is a plain value: every verb on it computes at once. A style verb (`fill`, `stroke`) turns it into a Drawing.
"""

from pathlib import Path

import bullterrier_vector as btvec

star = btvec.Path.star((200, 200), outer=150, inner=60, points=5)

tip = star.point_at(0)  # a closed outline is measured from its topmost point: the top tip
halfway = star.point_at(0.5)  # half its length further on, clockwise
direction = star.tangent_at(0.5)  # a unit vector along the outline there; Points add and scale

drawing = btvec.Drawing(
    star.fill("#ffcc33").stroke("#1a1a1a", width=4),
    btvec.Path.circle(tip, 8).fill("#e2412e"),
    btvec.Path.line(halfway, halfway + 40 * direction).stroke("#e2412e"),
    size=(400, 400),
)
Path("star.svg").write_text(drawing.to_svg(400, 400), encoding="utf-8")
