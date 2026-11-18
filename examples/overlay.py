"""Overlay: a character grid and the marks of a heads-up display on it.

A shaded disc is written as text, one character for each cell (a ramp from space to `@`, picked by whole numbers only,
so the Python, Rust and JavaScript programs write the same rows), and `characters` stands one outlined glyph in each
cell. Over it `marks` draws a bracket round a box and a crosshair on a point, each with a label on a leader line. Every
glyph and mark is a shape with attributes (`char`, `row`, `column`; `role`, `mark`), so the grid is inked and the
marks are styled by selecting: the labels are filled, the brackets, crosshairs and leaders are strokes.
Draw it with `uv run poe look examples/overlay.py` (PNG and SVG in `dist/look/`).
The same program in Rust and JavaScript: `crates/core/examples/overlay.rs`, `crates/wasm/examples/overlay.mjs`.
"""

import bullterrier_vector as btvec

COLUMNS, ROWS = 36, 18
CELL = (12, 20)
WIDTH, HEIGHT = COLUMNS * CELL[0], ROWS * CELL[1]
RAMP = " .:-=+*#%@"
INK, GREEN = "#1f2a24", "#1c9b5a"
RADIUS_SQUARED = 170 * 170  # a disc 170 pixels across from its middle


def tone(column: int, row: int) -> str:
    """The character of a cell: the shade falls with the squared distance from the middle, in whole pixels."""
    across = 6 * (2 * column + 1 - COLUMNS)
    down = 10 * (2 * row + 1 - ROWS)
    return RAMP[max(0, 9 - (across * across + down * down) * 9 // RADIUS_SQUARED)]


rows = "\n".join("".join(tone(column, row) for column in range(COLUMNS)) for row in range(ROWS))
grid = btvec.characters(rows, cell=CELL).fill(INK)
hud = btvec.marks(
    [(96, 60, 240, 240, "DISC")],
    [(216, 180, "CENTRE")],
    bracket=0.2,
    cross=10,
    text_size=14,
)
drawing = btvec.Drawing(
    grid,
    hud.only(role="text").fill(GREEN),
    hud.without(role="text").stroke(GREEN, width=1.5),
    size=(WIDTH, HEIGHT),
)

frames = {"still": drawing.to_svg(WIDTH, HEIGHT)}
