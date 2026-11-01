"""Frame export: a caller's function of time looped over frames into a transparent PNG sequence or a clip.

This is the one part of the library that knows time; drawings and parameters stay timeless, and `scene` is whatever the
caller makes of `t`. The helper writes files and runs ffmpeg, so it lives outside the Rust core.
"""

import math
import os
import re
import shutil
import subprocess
import tempfile
from collections.abc import Callable, Iterator
from itertools import chain
from pathlib import Path
from typing import IO, cast

from bullterrier_vector._native._core import Drawing
from bullterrier_vector.errors import ParameterError

__all__ = ["frames"]

# constant: the codec choices are the promise of the `.mov` and `.webm` kinds. ProRes 4444 keeps a 16-bit alpha;
# VP9 carries its alpha as a second stream, which needs `-auto-alt-ref 0`.
_PRORES = ["-c:v", "prores_ks", "-profile:v", "4444", "-pix_fmt", "yuva444p10le", "-f", "mov"]
# constant: libvpx-vp9 constant-quality 15 with no bitrate cap is its "visually lossless" end of the scale (0 to 63),
# for a clip meant to be laid over footage and edited, not streamed.
_VP9_CRF = "15"
_VP9 = ["-c:v", "libvpx-vp9", "-pix_fmt", "yuva420p", "-auto-alt-ref", "0", "-b:v", "0", "-crf", _VP9_CRF, "-f", "webm"]
_CLIPS = {".mov": _PRORES, ".webm": _VP9}

_NUMBERED_NAME = re.compile(r"([^#]*)(#+)([^#]*)")


def frames(
    scene: Callable[[float], Drawing],
    *,
    width: int,
    height: int,
    fps: float = 30,
    seconds: float,
    out: str | os.PathLike[str],
) -> None:
    """Draw `scene(t)` at `width` by `height` pixels for every frame of a `seconds`-long clip at `fps` frames a second,
    and write the frames to `out`.

    `scene` takes a time in seconds and returns the `Drawing` to show then, usually `drawing.with_values(...)`. There
    are `round(seconds * fps)` frames (Python's `round`: a half goes to the even number), frame `i` is drawn at
    `t = i / fps` and the first at `t = 0`; `t = seconds` is never drawn, so a loop of `seconds` repeats without a
    doubled frame. Each frame is exactly the pixels `Drawing.to_png` (and `to_rgba`) give for `scene(t)` at that
    size, transparent where nothing is drawn.

    `out` is a `str` or path whose suffix, in any case, says what is written:

    - `.png` with one run of `#` in the file name (`"frames/badge-####.png"`): a numbered PNG sequence. The run is the
      minimum number of digits of the zero-based index, padded with zeros; an index that needs more digits widens it
      (`##` gives `00` to `99`, then `100`). A `#` in a folder name is literal.
    - `.mov`: one ProRes 4444 clip with alpha. `.webm`: one VP9 clip with alpha. Both need ffmpeg (with `prores_ks` or
      `libvpx-vp9`) on the `PATH`; the frames are piped to it as raw RGBA. Clips are lossy: they decode to pixels close
      to the PNG ones, not equal.

    The folder is created, with its parents, once the first frame is drawn. Existing files of the same names are
    overwritten; files of a longer earlier run stay. If `scene` or a write fails, the frames already written stay; a
    clip is written under a hidden name next to `out` and moved into place only when ffmpeg is done, so a failure
    leaves no half-written clip and an existing `out` untouched.

    Raises:
        ParameterError: `fps` or `seconds` not a finite number above 0, `seconds` shorter than half a frame, `out` of
            no known kind or a `.png` without exactly one run of `#`, or a size the outputs refuse (raised by the
            first draw). For the arguments that are not sizes, nothing is drawn or written.
        OSError: a file or folder cannot be written, or ffmpeg is missing or fails (its message is in the error).

    Whatever `scene` raises propagates.
    """
    for name, value in (("fps", fps), ("seconds", seconds)):
        if not (math.isfinite(value) and value > 0):
            raise ParameterError(f"{name} must be a finite number above 0, got {value!r}")
    count = round(seconds * fps)
    if count < 1:
        raise ParameterError(
            f"seconds must cover at least half a frame: {seconds!r} seconds at {fps!r} fps is no frame"
        )

    target = Path(out)
    kind = target.suffix.lower()
    if kind == ".png":
        numbered = _NUMBERED_NAME.fullmatch(target.name)
        if numbered is None:
            raise ParameterError(f"out must have exactly one run of '#' in the file name of a .png, got {str(out)!r}")
        prefix, run, suffix = numbered.groups()
        for i in range(count):
            png = scene(i / fps).to_png(width, height)
            if i == 0:  # after the first draw, so a size the outputs refuse leaves no folder behind
                target.parent.mkdir(parents=True, exist_ok=True)
            (target.parent / f"{prefix}{i:0{len(run)}d}{suffix}").write_bytes(png)
    elif kind in _CLIPS:
        _write_clip(
            (scene(i / fps).to_rgba(width, height) for i in range(count)),
            width=int(width),
            height=int(height),
            fps=fps,
            target=target,
            codec=_CLIPS[kind],
        )
    else:
        raise ParameterError(f"out must end in .png, .mov or .webm, got {str(out)!r}")


def _write_clip(
    pixels: Iterator[bytes], *, width: int, height: int, fps: float, target: Path, codec: list[str]
) -> None:
    """Pipe the RGBA frames `pixels` (at least one) to ffmpeg and move its clip to `target` when it is done."""
    ffmpeg = shutil.which("ffmpeg")
    if ffmpeg is None:
        raise OSError("ffmpeg was not found on the PATH; writing a clip needs it")
    first = next(pixels)  # drawn before ffmpeg starts and before the folder exists, so a refused size leaves nothing
    target.parent.mkdir(parents=True, exist_ok=True)
    partial = (target.parent / f".{target.name}.partial").absolute()
    argv = [
        ffmpeg, "-hide_banner", "-loglevel", "error", "-y",
        "-f", "rawvideo", "-pixel_format", "rgba", "-video_size", f"{width}x{height}", "-framerate", str(fps),
        "-i", "pipe:0", *codec, str(partial),
    ]  # fmt: skip
    try:
        with tempfile.TemporaryFile() as log:
            # argv is a list run without a shell: ffmpeg found on the PATH, fixed flags, numbers and our own path
            with subprocess.Popen(argv, stdin=subprocess.PIPE, stdout=subprocess.DEVNULL, stderr=log) as process:  # noqa: S603  see above
                stdin = cast(IO[bytes], process.stdin)  # PIPE was asked for
                try:
                    for rgba in chain([first], pixels):
                        stdin.write(rgba)
                    stdin.close()
                except BrokenPipeError:
                    pass  # ffmpeg stopped reading; its exit status and message, next, say why
                status = process.wait()
            if status != 0:
                log.seek(0)
                raise OSError(f"ffmpeg failed (exit status {status}): {log.read().decode(errors='replace').strip()}")
        partial.replace(target)
    finally:
        partial.unlink(missing_ok=True)
