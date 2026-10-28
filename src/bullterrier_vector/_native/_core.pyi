# The stub of the Rust extension `bullterrier_vector._native._core` (crates/python). The functions' documentation is the
# extension's own (`help(_core.polygon_svg)`); stubtest checks this stub against it.

# An int beyond 64 bits raises OverflowError (PyO3's extraction), a float for `sides` TypeError; a value out of range
# raises bullterrier_vector.errors.ParameterError. Stubs carry no docstrings (ruff's PYI021): the details are the
# extension's own.

__all__ = ["polygon_svg"]

def polygon_svg(sides: int, radius: float) -> str: ...
