"""Trace a sequence of maps steadily: a disc drifts across five frames of a video, each frame a map with blocks of
dropout, and tracing each frame alone makes the dropouts pop in and out as holes. `Field.smooth_maps` averages each map
with its neighbours, which mostly averages the dropouts away (measured: 3 to 7 outlines a frame alone, 1 to 3
steadied). The library keeps no sequence: the caller holds the list of maps and passes it in. Whole numbers only, so
the Rust program (`crates/core/examples/trace_sequence.rs`) draws the same frames. Draw it with
`uv run poe look examples/trace_sequence.py` (the last frame, in PNG and SVG in `dist/look/`).
"""

import bullterrier_vector as btvec

SIDE, FRAMES = 64, 5
LEVELS = [0.5]


def frame_bytes(i: int) -> bytes:
    """A disc of tone 200 on 50, moving 2 pixels a frame; a 3 by 3 block in about one in twelve drops out to 50."""

    def tone(x: int, y: int) -> int:
        inside = (x - 24 - 2 * i) ** 2 + (y - 32) ** 2 < 18**2
        dropout = ((x // 3) * 73856093 ^ (y // 3) * 19349663 ^ (i + 1) * 83492791) % 12 == 0
        return 200 if inside and not dropout else 50

    return bytes(tone(x, y) for y in range(SIDE) for x in range(SIDE))


box = (0, 0, 640, 640)
maps = [btvec.Field.map(SIDE, SIDE, frame_bytes(i), box=box, name="frame") for i in range(FRAMES)]
steady = btvec.Field.smooth_maps(maps, sigma=1.5)


def trace(field: btvec.Field) -> btvec.Drawing:
    """The shape of the disc, one paper."""
    return btvec.Drawing(
        btvec.Drawing.trace_levels(field, LEVELS, smooth=1.0, tolerance=0.4, close=1.0).fill("#f2b632"), size=(640, 640)
    )


alone = [trace(m) for m in maps]  # each frame on its own: the dropouts are holes
drawing = [trace(m) for m in steady]  # steadied: the disc alone
frames = {"alone": alone[-1].to_svg(640, 640), "steady": drawing[-1].to_svg(640, 640)}
