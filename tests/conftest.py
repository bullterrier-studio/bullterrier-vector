"""What the tests share: the project's root, a scratch copy of it for the tooling tests, and the environment `uv run`
gives a command. A helper one file uses stays in that file."""

import os
import shutil
import struct
import sys
import zlib
from pathlib import Path

import pytest
from hypothesis import settings

ROOT = Path(__file__).resolve().parents[1]


@pytest.fixture(autouse=True, scope="session")
def no_enclosing_git_or_push():
    """Hide what a git hook hands the process: its repository (GIT_DIR, GIT_INDEX_FILE, GIT_WORK_TREE; also set by a
    linked worktree, `rebase -x` and `bisect`) and the push the pre-push gate is running for (PRE_COMMIT_*). A test's
    git would act on that repository, and the push would decide what tools/pre_push.py does in the tests. Session-wide,
    so the module fixtures that copy the environment never see them either."""
    hidden = {name: os.environ.pop(name) for name in list(os.environ) if name.startswith(("GIT_", "PRE_COMMIT_"))}
    yield
    os.environ.update(hidden)


@pytest.fixture
def project_copy(tmp_path):
    """`project_copy(*names)` copies the project's src/ (without bytecode caches) and the files `names` at its root into
    tmp_path, and returns tmp_path: a scratch project to write one fault into, the real one untouched."""

    def copy(*names: str) -> Path:
        shutil.copytree(ROOT / "src", tmp_path / "src", ignore=shutil.ignore_patterns("__pycache__", "*.pyc"))
        for name in names:
            shutil.copy(ROOT / name, tmp_path)
        return tmp_path

    return copy


@pytest.fixture
def uv_run_env() -> dict[str, str]:
    """The environment a command gets under `uv run`: this one, with its executables first on PATH."""
    return {**os.environ, "PATH": os.pathsep.join([str(Path(sys.executable).parent), os.environ.get("PATH", "")])}


# property tests are deterministic (the same cases on every run, so a failure reproduces) and never time out
settings.register_profile("project", derandomize=True, deadline=None)
settings.load_profile("project")


PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"


def _unfilter(kind: int, row: bytearray, previous: bytes, bytes_per_pixel: int) -> bytearray:
    """The row `row` with PNG filter `kind` undone, given the row above it (PNG specification, section 9)."""
    for i in range(len(row)):
        left = row[i - bytes_per_pixel] if i >= bytes_per_pixel else 0
        up = previous[i]
        up_left = previous[i - bytes_per_pixel] if i >= bytes_per_pixel else 0
        if kind == 0:
            predicted = 0
        elif kind == 1:
            predicted = left
        elif kind == 2:
            predicted = up
        elif kind == 3:
            predicted = (left + up) // 2
        elif kind == 4:
            estimate = left + up - up_left
            distances = (abs(estimate - left), abs(estimate - up), abs(estimate - up_left))
            predicted = (left, up, up_left)[distances.index(min(distances))]
        else:
            raise ValueError(f"unknown row filter {kind}")
        row[i] = (row[i] + predicted) & 0xFF
    return row


def decode_8_bit_rgba_png(data: bytes) -> tuple[int, int, bytes]:
    """`(width, height, pixels)` of a non-interlaced 8-bit RGBA PNG: the pixels as `Drawing.to_rgba` returns them.
    Written from the PNG specification with the standard library only, so it is an oracle independent of the encoder.
    Raises `ValueError` for anything it does not read exactly: a bad signature, a chunk cut short or with a wrong CRC,
    a chunk that is not IHDR (first), IDAT or IEND (last), a header of another kind of image, a stream that is not the
    rows the header promises, or data after the last chunk."""
    if not data.startswith(PNG_SIGNATURE):
        raise ValueError("not a PNG signature")
    at = len(PNG_SIGNATURE)
    header: bytes | None = None
    compressed = b""
    ended = False
    while at < len(data):
        if ended:
            raise ValueError("data after IEND")
        if at + 8 > len(data):
            raise ValueError("chunk cut short")
        length, name = struct.unpack(">I4s", data[at : at + 8])
        end = at + 12 + length
        if end > len(data):
            raise ValueError(f"chunk {name!r} cut short")
        body = data[at + 8 : at + 8 + length]
        (crc,) = struct.unpack(">I", data[at + 8 + length : end])
        if crc != zlib.crc32(name + body):
            raise ValueError(f"chunk {name!r} has a wrong CRC")
        if header is None and name != b"IHDR":
            raise ValueError(f"the first chunk is {name!r}, not IHDR")
        if name == b"IHDR":
            if header is not None:
                raise ValueError("a second IHDR")
            header = body
        elif name == b"IDAT":
            compressed += body
        elif name == b"IEND":
            ended = True
        else:
            raise ValueError(f"chunk {name!r} is none of IHDR, IDAT and IEND")
        at = end
    if header is None or not ended:
        raise ValueError("no IHDR or no IEND")
    width, height, depth, colour, method, filtering, interlace = struct.unpack(">IIBBBBB", header)
    if (depth, colour, method, filtering, interlace) != (8, 6, 0, 0, 0):
        raise ValueError("not an 8-bit RGBA PNG without interlacing")
    try:
        stream = zlib.decompress(compressed)
    except zlib.error as error:
        raise ValueError(f"bad zlib stream: {error}") from error
    stride = width * 4
    if len(stream) != height * (1 + stride):
        raise ValueError("the stream is not one filter byte and one row per row")
    rows: list[bytes] = []
    previous = bytes(stride)
    for y in range(height):
        start = y * (1 + stride)
        row = _unfilter(stream[start], bytearray(stream[start + 1 : start + 1 + stride]), previous, 4)
        rows.append(bytes(row))
        previous = rows[-1]
    return width, height, b"".join(rows)


@pytest.fixture(scope="session")
def decode_png():
    """`decode_png(data)`: `(width, height, pixels)` of a PNG written by `Drawing.to_png`, read by a decoder of ours
    that checks every CRC; `ValueError` if the file is not exactly an 8-bit RGBA PNG."""
    return decode_8_bit_rgba_png
