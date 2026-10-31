"""What tests in every folder share. Helpers used by one folder live in that folder's conftest.py."""

import os
import struct
import zlib

import pytest


@pytest.fixture(autouse=True)
def no_enclosing_git(monkeypatch: pytest.MonkeyPatch) -> None:
    """Run every test as if no git were running it. A git hook (the gate that runs before a push), a linked worktree,
    `rebase -x` or `bisect` hands the repository it serves to its children in `GIT_*` variables, `GIT_DIR` among them,
    which would send the git of a test, in this process or in a subprocess, to that repository instead of the
    throwaway one: its `git init`, `git config` and `git commit` would land in the real repository."""
    for name in [name for name in os.environ if name.startswith("GIT_")]:
        monkeypatch.delenv(name)


@pytest.fixture(autouse=True)
def no_enclosing_hook(monkeypatch: pytest.MonkeyPatch) -> None:
    """Run every test as if no pre-commit hook were running it. The pre-push hook runs the gate, and so pytest, with
    `PRE_COMMIT_REMOTE_NAME`, `PRE_COMMIT_REMOTE_BRANCH`, `PRE_COMMIT_FROM_REF` and the like in its environment, which
    `tools/pre_push.py` reads: a test that does not set its own values would get the hook's, and pass or fail by what
    is being pushed. A test that needs one sets it."""
    for name in [name for name in os.environ if name.startswith("PRE_COMMIT_")]:
        monkeypatch.delenv(name)


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
    Raises `ValueError` for anything it does not read exactly: a bad signature, a chunk cut short or with a wrong CRC, a
    header of another kind of image, a stream that is not the rows the header promises, or data after the last chunk."""
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
        if name == b"IHDR":
            header = body
        elif name == b"IDAT":
            compressed += body
        elif name == b"IEND":
            ended = True
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
