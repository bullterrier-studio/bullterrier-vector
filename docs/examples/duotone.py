"""Duotone colours on a drawing: every fill and stroke colour mapped by its lightness onto two inks.

A duotone is a gradient map with two colours. `posterize` reduces colours to a few levels instead. Both change the
colours of shapes, not pixels, so the result stays vector.
"""

from pathlib import Path

import bullterrier_vector as btvec

palette = ["#2b2d42", "#8d99ae", "#edf2f4", "#ef233c", "#d90429", "#ffb703"]
tiles = btvec.Drawing(
    *[btvec.Path.rect(60 + 160 * i, 60, 140, 480, radius=12).fill(color) for i, color in enumerate(palette)],
    size=(1080, 600),
)

duotone = tiles.gradient_map(["#1b1446", "#ffd166"])
posterized = tiles.posterize(3)

Path("tiles-duotone.svg").write_text(duotone.to_svg(1080, 600), encoding="utf-8")
Path("tiles-posterized.svg").write_text(posterized.to_svg(1080, 600), encoding="utf-8")
