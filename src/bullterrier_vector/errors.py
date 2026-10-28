"""The exceptions bullterrier-vector raises: catch `BullterrierVectorError` for every failure the library reports on
purpose.

Each subclass also inherits the builtin exception it replaces, so code that already catches `ValueError` keeps working.
The Rust extension raises `ParameterError` itself, importing it from here.

Dependency direction: the bottom layer: it imports nothing from bullterrier_vector.
"""

__all__ = ["BullterrierVectorError", "ParameterError"]


class BullterrierVectorError(Exception):
    """The base of every exception bullterrier-vector raises on purpose."""


class ParameterError(BullterrierVectorError, ValueError):
    """A value is outside what the library accepts; the message names the parameter, the valid range and the value."""
