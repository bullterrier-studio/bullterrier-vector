"""Halftone dots from a tone map: a grayscale map (width, height, bytes) drawn as a dot screen inside a shape.

The dots are vector: they print sharp at any size and appear in the SVG as they do in pixels.
"""

from pathlib import Path

import bullterrier_vector as btvec

photo = btvec.Field.map(540, 675, Path("portrait.gray").read_bytes(), box=(0, 0, 1080, 1350))
disc = btvec.Path.circle((540, 675), 480)

drawing = btvec.Drawing(
    btvec.Path.rect(0, 0, 1080, 1350).fill("#f3eee4"),
    disc.halftone("#15161a", tone=photo, pitch=12, angle=45),
)
Path("halftone.svg").write_text(drawing.to_svg(1080, 1350), encoding="utf-8")
