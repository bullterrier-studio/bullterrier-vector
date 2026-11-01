"""`Font` and the container of the file: a WOFF2 file that unpacks past `MAX_FONT_BYTES` (a decompression bomb) is
refused before it is read as a font, and one that unpacks to exactly that many bytes is not refused for its size.

The WOFF2 files are written here, byte by byte, from the specification (a header, a table directory of one table and a
Brotli stream of that table), so that the stream can be made quickly and large: fontTools compresses at the highest
quality, which takes a minute for 32 MiB. The table holds sparse noise, which Brotli packs at about 30 to 1; an all-zero
table would pack at over 100 to 1, which the decoder refuses as an implausible claim before the limit is reached.
"""

import random
import struct

import brotli
import pytest

import bullterrier_vector as btvec

MAX_FONT_BYTES = 32 * 1024 * 1024  # the documented limit: a font of 32 MiB, packed or not
SFNT_HEADER = 12 + 16  # the offset table and the one table record of the font the file unpacks to


def noise(length: int) -> bytes:
    """`length` bytes of which one in 64 is not zero, the same every run."""
    rng = random.Random(7)  # noqa: S311  test data, not a secret
    data = bytearray(length)
    for at in rng.sample(range(length), length // 64):
        data[at] = rng.randrange(1, 256)
    return bytes(data)


def base128(value: int) -> bytes:
    """`value` as the UIntBase128 of the WOFF2 specification: seven bits to the byte, high bit on all but the last."""
    digits = [value & 0x7F]
    value >>= 7
    while value:
        digits.append(0x80 | (value & 0x7F))
        value >>= 7
    return bytes(reversed(digits))


def woff2(table: bytes) -> bytes:
    """A WOFF2 file whose only table, `ZZZZ`, holds `table`; the font it unpacks to is `SFNT_HEADER` bytes and the table
    padded to four."""
    directory = bytes([0x3F]) + b"ZZZZ" + base128(len(table))
    stream = brotli.compress(table, quality=5, mode=brotli.MODE_FONT)
    unpacked = SFNT_HEADER + len(table) + (-len(table) % 4)
    length = 48 + len(directory) + len(stream)
    length += -length % 4  # the file ends on a multiple of four
    header = struct.pack(
        ">4s4sIHHIIHHIIIII", b"wOF2", b"\0\1\0\0", length, 1, 0, unpacked, len(stream), 1, 0, 0, 0, 0, 0, 0
    )
    return (header + directory + stream).ljust(length, b"\0")


def test_a_woff2_that_unpacks_to_more_than_the_limit_is_refused_naming_the_limit() -> None:
    bomb = woff2(noise(MAX_FONT_BYTES - SFNT_HEADER + 4))
    assert len(bomb) < MAX_FONT_BYTES // 8  # it is the unpacking that is large, not the file
    with pytest.raises(btvec.ParameterError, match=r"data.*unpacks to 33554436 bytes.*MAX_FONT_BYTES"):
        btvec.Font(bomb)


def test_a_woff2_that_unpacks_to_the_limit_is_not_refused_for_its_size() -> None:
    packed = woff2(noise(MAX_FONT_BYTES - SFNT_HEADER))
    with pytest.raises(btvec.ParameterError, match=r"data") as refusal:
        btvec.Font(packed)  # it is no font, but it is not too large
    assert "MAX_FONT_BYTES" not in str(refusal.value)
