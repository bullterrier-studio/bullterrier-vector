"""A typography-first vector graphics scripting library: pictures written as code, drawn as SVG or pixels.

For now the package holds only its exceptions; the geometry (`Path`) arrives with its bindings.
"""

from bullterrier_vector._native import _core as _core  # loads the extension, so a broken build fails the import
from bullterrier_vector.errors import BullterrierVectorError, GeometryError, ParameterError

__all__ = ["BullterrierVectorError", "GeometryError", "ParameterError"]
