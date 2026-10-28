"""The Rust core's home in the package: `_core` here is the extension built from crates/python (`_core.pyi` is its
stub, checked by stubtest).

Internal, and a leaf: it imports nothing from the rest of the package (the extension itself imports
`bullterrier_vector.errors` when it raises).
"""
