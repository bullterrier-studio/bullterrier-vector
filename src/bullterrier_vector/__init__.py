"""Typography-first generative vector graphics in code: SVG or pixels, the same from Rust, Python and the browser.

For now the package holds only the walking skeleton: `polygon_svg` draws a regular polygon as SVG.
"""

from bullterrier_vector._native._core import polygon_svg
from bullterrier_vector.errors import BullterrierVectorError, ParameterError

__all__ = ["BullterrierVectorError", "ParameterError", "polygon_svg"]
