"""A badge that spins and grows over one second, written as a transparent PNG sequence.

Drawings have no time: `scene` is the caller's function from seconds to a drawing, and `btvec.frames` calls it once a
frame and writes each frame as `to_png` would. The frames go to a temporary folder here; give `out` a real path to keep
them, or end it in `.mov` (ProRes 4444) or `.webm` (VP9) for a clip with alpha, which needs ffmpeg on the PATH.
"""

import tempfile
from pathlib import Path

import bullterrier_vector as btvec

turn = btvec.Param("turn", default=0, min=0, max=1, unit="cycle")
badge = btvec.Drawing(
    btvec.Path.star((100, 100), outer=80, inner=32, points=5)
    .fill("#ffcc33")
    .stroke("#1a1a1a", width=4)
    .scale(0.2 + 0.8 * turn, about=(100, 100))
    .rotate(360 * turn, about=(100, 100)),
    size=(200, 200),
)


def scene(t: float) -> btvec.Drawing:
    """The badge `t` seconds in: one turn in the first second."""
    return badge.with_values(turn=t)


drawing = scene(0.75)  # one frame, for `uv run poe look examples/badge_frames.py`

with tempfile.TemporaryDirectory() as folder:
    btvec.frames(scene, width=200, height=200, fps=24, seconds=1, out=Path(folder) / "badge-####.png")
    written = sorted(path.name for path in Path(folder).iterdir())
