"""Trace: a relief traced into stacked cut-paper layers, and a dithered strip traced exactly.

A map of tone is plain numbers: here two hills made from whole numbers only, so the Python, Rust and JavaScript programs
build the same bytes. `trace_levels` turns the map into one filled shape for each tone level (the region at or above
it), stacked from the lowest and painted lighter to darker, with the edges smoothed a little, curves fitted through
them, and specks and thin gaps below a size removed. Under it a strip of ordered dither (cells of a pixel and a half,
two colours) is traced exactly: every cell of a colour that touches another is merged into one straight-edged region,
nothing is rounded, and each colour is one shape. Nothing is drawn but filled shapes, so the piece cuts with a knife.
Transparent, for any paper. Draw it with `uv run poe look examples/trace.py` (PNG and SVG in `dist/look/`).
The same program in Rust and JavaScript: `crates/core/examples/trace.rs`, `crates/wasm/examples/trace.mjs`.
"""

import bullterrier_vector as btvec

WIDTH, HEIGHT = 960, 1180
PAPERS = ["#e9dcc3", "#c9a678", "#8a5a3b"]  # for the levels, lowest first
INK, RED = "#16151a", "#c8341c"

# the relief: 96 by 96 pixels over the top 960 by 960 of the picture, tone 0 (dark) to 255
SIDE = 96
relief_bytes = bytes(
    max(0, 255 - ((x - 34) ** 2 + (y - 40) ** 2) // 6, 200 - ((x - 68) ** 2 + (y - 56) ** 2) // 5)
    for y in range(SIDE)
    for x in range(SIDE)
)
relief = btvec.Field.map(SIDE, SIDE, relief_bytes, box=(0, 0, 960, 960), name="relief")

# the strip: 80 by 12 cells of 2 pixels, an ordered dither (a 4 by 4 threshold) of a ramp across, as a 160 by 24 map
COLUMNS, ROWS = 80, 12
BAYER = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]]
dither_bytes = bytes(
    1 if BAYER[(y // 2) % 4][(x // 2) % 4] * 16 + 8 < (x // 2) * 255 // (COLUMNS - 1) else 0
    for y in range(ROWS * 2)
    for x in range(COLUMNS * 2)
)
strip = btvec.Field.map(COLUMNS * 2, ROWS * 2, dither_bytes, box=(60, 1000, 840, 126), name="dither")

layers = [
    btvec.Drawing.trace_levels(relief, level, smooth=12, tolerance=3, close=8, min_area=600).fill(paper)
    for level, paper in zip([0.2, 0.45, 0.7], PAPERS, strict=True)
]
dither = btvec.Drawing.trace_cells(strip, 2, max_colors=2)
drawing = btvec.Drawing(
    *layers,
    dither.only(label=1).fill(RED),
    dither.only(label=0).fill(INK),
    size=(WIDTH, HEIGHT),
)

frames = {"still": drawing.to_svg(WIDTH, HEIGHT)}
