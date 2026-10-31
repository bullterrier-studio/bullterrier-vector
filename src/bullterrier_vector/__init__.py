"""A typography-first vector graphics scripting library: pictures written as code, drawn as SVG or pixels.

For now the package holds the geometry value `Path` (with `Point` and `Rect`) and its exceptions.
"""

from bullterrier_vector._native._core import Path, Point, Rect
from bullterrier_vector.errors import BullterrierVectorError, GeometryError, ParameterError

__all__ = ["BullterrierVectorError", "GeometryError", "ParameterError", "Path", "Point", "Rect"]
