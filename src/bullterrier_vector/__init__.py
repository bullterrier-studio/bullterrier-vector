"""A typography-first vector graphics scripting library: pictures written as code, drawn as SVG or pixels.

For now the package holds the geometry value `Path` (with `Point` and `Rect`), drawings (`Drawing`, `Shape`, `Style`,
`Param`, `Expr`), their JSON (`Drawing.to_json`, `Drawing.from_json`, `drawing_schema`) and the exceptions.
"""

from bullterrier_vector._native._core import Drawing, Expr, Param, Path, Point, Rect, Shape, Style, drawing_schema
from bullterrier_vector.errors import BullterrierVectorError, FormatError, GeometryError, ParameterError

__all__ = [
    "BullterrierVectorError",
    "Drawing",
    "Expr",
    "FormatError",
    "GeometryError",
    "Param",
    "ParameterError",
    "Path",
    "Point",
    "Rect",
    "Shape",
    "Style",
    "drawing_schema",
]
