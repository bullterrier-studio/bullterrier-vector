"""A typography-first vector graphics scripting library: pictures written as code, drawn as SVG or pixels.

For now the package holds only the walking skeleton: `polygon_svg` draws a regular polygon as SVG.
"""

from bullterrier_vector._native._core import polygon_svg
from bullterrier_vector.errors import BullterrierVectorError, ParameterError

__all__ = ["BullterrierVectorError", "ParameterError", "polygon_svg"]
