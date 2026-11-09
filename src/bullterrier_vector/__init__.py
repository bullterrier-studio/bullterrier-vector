"""A typography-first vector graphics scripting library: pictures written as code, drawn as SVG or pixels.

For now the package holds the geometry value `Path` (with `Point` and `Rect`), type (`Font`, `text`), drawings
(`Drawing`, `Shape`, `Style`, `Shadow`, `Halftone`, `Gradient`, `Param`, `Expr`, the per-item numbers `by_index`,
`by_order`, `random` and `stagger`), their JSON (`Drawing.to_json`, `Drawing.from_json`, `drawing_schema`),
`frames` (a function of time to a PNG sequence or a clip) and the exceptions.
"""

from bullterrier_vector._native._core import (
    Drawing,
    Expr,
    Font,
    Gradient,
    Halftone,
    Mesh,
    Param,
    Path,
    Point,
    Rect,
    Shadow,
    Shape,
    Style,
    by_index,
    by_order,
    drawing_schema,
    linear_gradient,
    radial_gradient,
    random,
    stagger,
    text,
)
from bullterrier_vector.errors import BullterrierVectorError, FormatError, GeometryError, ParameterError
from bullterrier_vector.frame_export import frames

__all__ = [
    "BullterrierVectorError",
    "Drawing",
    "Expr",
    "Font",
    "FormatError",
    "GeometryError",
    "Gradient",
    "Halftone",
    "Mesh",
    "Param",
    "ParameterError",
    "Path",
    "Point",
    "Rect",
    "Shadow",
    "Shape",
    "Style",
    "by_index",
    "by_order",
    "drawing_schema",
    "frames",
    "linear_gradient",
    "radial_gradient",
    "random",
    "stagger",
    "text",
]
