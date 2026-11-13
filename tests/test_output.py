"""`Drawing.to_svg` through the public package: its type, the document it writes (read back with an XML parser), the
sizes it refuses and the Param values it takes by keyword. Expected values are worked out by hand."""

import math
import struct
import xml.etree.ElementTree as ET
import zlib
from typing import Any

import pytest

import bullterrier_vector as btvec

PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"  # the eight bytes every PNG file opens with (PNG specification, section 5.2)

SVG = "{http://www.w3.org/2000/svg}"
LINE = btvec.Path.polyline([(0, 0), (10, 20)])
# the same class, untyped: the wrong types below are what is tested
LOOSE: Any = btvec.Drawing(LINE)


def parse(text: str) -> ET.Element:
    """The root of the SVG document `text`."""
    return ET.fromstring(text)  # noqa: S314  our own output


def path_data(drawing: btvec.Drawing, width: int, height: int, **values: float) -> list[str]:
    """The path data of every element of the drawing's SVG."""
    root = parse(drawing.to_svg(width, height, **values))
    return [element.attrib["d"] for element in root.findall(f"{SVG}path")]


def test_to_svg_returns_the_document_as_text():
    svg = btvec.Drawing(LINE, size=(100, 50)).to_svg(200, 200)
    assert isinstance(svg, str)
    root = parse(svg)
    assert root.tag == f"{SVG}svg"
    assert (root.attrib["width"], root.attrib["height"], root.attrib["viewBox"]) == ("200", "200", "0 0 200 200")
    # the frame of 100 by 50 is doubled and centred down by 50
    assert path_data(btvec.Drawing(LINE, size=(100, 50)), 200, 200) == ["M0 50 L20 90"]


def test_an_empty_drawing_is_a_document_with_no_elements():
    root = parse(btvec.Drawing().to_svg(30, 20))
    assert (root.attrib["width"], root.attrib["height"], len(root)) == ("30", "20", 0)


def test_a_whole_size_given_as_a_float_is_accepted():
    assert parse(btvec.Drawing().to_svg(30.0, 20.0)).attrib["width"] == "30"


# the sizes every output refuses, whatever its largest side: (width, height, the side named, the value shown)
SIZE_REFUSALS = [
    pytest.param(0, 10, "width", "0", id="zero width"),
    pytest.param(10, 0, "height", "0", id="zero height"),
    pytest.param(0.5, 10, "width", "0.5", id="half a pixel"),
    pytest.param(10, 1080.5, "height", "1080.5", id="a fraction"),
    pytest.param(math.nan, 10, "width", "NaN", id="not a number"),
    pytest.param(10, -4, "height", "-4", id="negative"),
]


@pytest.mark.parametrize(
    ("width", "height", "name", "shown"),
    [
        *SIZE_REFUSALS,
        pytest.param(100_001, 10, "width", "100001", id="one past the limit"),
        pytest.param(10, 10**9, "height", "1000000000", id="a billion"),
    ],
)
def test_a_size_that_is_not_a_whole_number_of_pixels_from_1_to_100000_raises_naming_the_side(
    width: float, height: float, name: str, shown: str
):
    with pytest.raises(btvec.ParameterError, match=name) as raised:
        btvec.Drawing(LINE).to_svg(width, height)
    assert shown in str(raised.value)


@pytest.mark.parametrize(
    "call",
    [
        pytest.param(lambda: LOOSE.to_svg("10", 10), id="a text width"),
        pytest.param(lambda: LOOSE.to_svg(10), id="one side"),
        pytest.param(lambda: LOOSE.to_svg(width=10, height=10), id="sides by keyword"),
    ],
)
def test_the_sides_are_two_numbers_given_by_position(call):
    with pytest.raises(TypeError):
        call()


def test_a_param_takes_its_value_by_keyword_and_its_default_otherwise():
    shift = btvec.Param("shift", default=3, min=0, max=50, unit="px")
    drawing = btvec.Drawing(LINE).translate(shift, 0)
    assert path_data(drawing, 100, 100) == ["M3 0 L13 20"]
    assert path_data(drawing, 100, 100, shift=7) == ["M7 0 L17 20"]


def test_a_value_for_no_param_raises_a_parameter_error_naming_it():
    with pytest.raises(btvec.ParameterError, match="shift"):
        btvec.Drawing(LINE).to_svg(10, 10, shift=1)


@pytest.mark.parametrize("name", ["into", "pen_mm", "window"])
def test_the_names_of_output_options_are_not_params_so_a_value_for_them_raises(name: str):
    with pytest.raises(btvec.ParameterError, match=name):
        btvec.Drawing(LINE).to_svg(10, 10, **{name: 1})
    with pytest.raises(btvec.ParameterError, match=name):
        btvec.Param(name, default=0, min=0, max=1)


def test_a_value_outside_a_params_range_raises():
    shift = btvec.Param("shift", default=3, min=0, max=50, unit="px")
    with pytest.raises(btvec.ParameterError, match="shift"):
        btvec.Drawing(LINE).translate(shift, 0).to_svg(10, 10, shift=51)


def pixel(image: bytes, width: int, x: int, y: int) -> tuple[int, ...]:
    """The (red, green, blue, alpha) of the pixel in column `x` and row `y` of a straight-alpha RGBA image."""
    start = (y * width + x) * 4
    return tuple(image[start : start + 4])


def test_to_rgba_returns_bytes_of_four_per_pixel_transparent_where_nothing_is_drawn():
    image = btvec.Drawing().to_rgba(30, 20)
    assert isinstance(image, bytes)
    assert image == bytes(30 * 20 * 4)


def test_to_rgba_rows_run_from_the_top_and_the_frame_is_fitted_like_svg():
    # a 100 by 50 frame in 200 by 200 is doubled and centred down by 50: its top left quarter (a 50 by 25 rectangle at
    # the origin) fills columns 0..100 and rows 50..100
    quarter = btvec.Drawing(btvec.Path.rect(0, 0, 50, 25).fill("#ff0000"), size=(100, 50))
    image = quarter.to_rgba(200, 200)
    assert len(image) == 200 * 200 * 4
    assert pixel(image, 200, 10, 60) == (255, 0, 0, 255)
    assert pixel(image, 200, 10, 49) == (0, 0, 0, 0)
    assert pixel(image, 200, 110, 60) == (0, 0, 0, 0)
    assert pixel(image, 200, 10, 110) == (0, 0, 0, 0)


def test_to_rgba_is_straight_alpha_not_premultiplied():
    half = btvec.Path.rect(-5, -5, 30, 30).fill("#c8643280")
    red, green, blue, alpha = pixel(btvec.Drawing(half).to_rgba(4, 4), 4, 2, 1)
    assert alpha == 128
    # a premultiplied colour would be about half as large
    assert (abs(red - 0xC8), abs(green - 0x64), abs(blue - 0x32)) <= (1, 1, 1)


def test_to_rgba_opacity_applies_to_the_fill_and_stroke_together():
    square = btvec.Path.rect(10, 10, 20, 20).fill("#ff0000").stroke("#0000ff", width=10, join="miter")
    image = square.opacity(0.5).to_rgba(40, 40)
    # (12, 20) holds fill and stroke: the stroke shows, and the pair is as faint as the fill alone at (20, 20)
    red, green, blue, alpha = pixel(image, 40, 12, 20)
    assert (red, green) == (0, 0)
    assert abs(alpha - 128) <= 1
    assert abs(blue - 255) <= 1
    fill_red, _, _, fill_alpha = pixel(image, 40, 20, 20)
    assert abs(fill_red - 255) <= 1
    assert abs(fill_alpha - 128) <= 1


def test_to_rgba_takes_a_param_by_keyword():
    shift = btvec.Param("shift", default=0, min=0, max=50, unit="px")
    drawing = btvec.Drawing(btvec.Path.rect(0, 0, 5, 5).fill("#00ff00")).translate(shift, 0)
    assert pixel(drawing.to_rgba(30, 10), 30, 2, 2) == (0, 255, 0, 255)
    assert pixel(drawing.to_rgba(30, 10, shift=20), 30, 2, 2) == (0, 0, 0, 0)
    assert pixel(drawing.to_rgba(30, 10, shift=20), 30, 22, 2) == (0, 255, 0, 255)
    with pytest.raises(btvec.ParameterError, match="elsewhere"):
        drawing.to_rgba(30, 10, elsewhere=1)


# the sizes the pixel outputs refuse: those every output refuses, then the pixel limits
PIXEL_SIZE_REFUSALS = [
    *SIZE_REFUSALS,
    pytest.param(65_001, 10, "width", "65001", id="one past the limit wide"),
    pytest.param(10, 65_001, "height", "65001", id="one past the limit high"),
    pytest.param(100_000, 1, "width", "100000", id="the svg limit"),
    pytest.param(16_385, 8192, "size", "16385", id="more than 2^27 pixels"),
    pytest.param(65_000, 65_000, "size", "65000", id="the largest square"),
]


@pytest.mark.parametrize("output", ["to_rgba", "to_png"])
@pytest.mark.parametrize(("width", "height", "name", "shown"), PIXEL_SIZE_REFUSALS)
def test_a_pixel_output_refuses_a_size_outside_the_pixel_limits_naming_the_side(
    output: str, width: float, height: float, name: str, shown: str
):
    with pytest.raises(btvec.ParameterError, match=name) as raised:
        getattr(btvec.Drawing(LINE), output)(width, height)
    assert shown in str(raised.value)


@pytest.mark.parametrize("output", ["to_rgba", "to_png"])
def test_a_pixel_output_refuses_a_shape_past_ten_million_device_pixels_naming_it(output: str):
    far = btvec.Path.rect(0, 0, 2e7, 4).fill("#000000")
    with pytest.raises(btvec.ParameterError, match="shape 0"):
        getattr(btvec.Drawing(far), output)(10, 10)


@pytest.mark.parametrize("output", ["to_rgba", "to_png"])
@pytest.mark.parametrize(
    "call",
    [
        pytest.param(lambda output: output("10", 10), id="a text width"),
        pytest.param(lambda output: output(10), id="one side"),
        pytest.param(lambda output: output(width=10, height=10), id="sides by keyword"),
    ],
)
def test_the_sides_of_a_pixel_output_are_two_numbers_given_by_position(output: str, call):
    with pytest.raises(TypeError):
        call(getattr(LOOSE, output))


STAR = btvec.Drawing(
    btvec.Path.star((200, 200), outer=150, inner=60, points=5).fill("#ffcc33").stroke("#1a1a1a", width=4),
    size=(400, 400),
)
# overlapping translucent shapes: alpha between 0 and 255 and mixed colours
OVERLAP = btvec.Drawing(
    btvec.Path.circle((30, 30), 20).fill("#ff000080"),
    btvec.Path.circle((45, 30), 20).fill("#00ff0080").opacity(0.5),
    btvec.Path.circle((38, 45), 20).fill("#0000ffc0").stroke("#000000", width=3),
)
EVERYTHING = btvec.Drawing(btvec.Path.rect(-10, -10, 70_000, 70_000).fill("#336699"))


@pytest.mark.parametrize(
    ("drawing", "width", "height"),
    [
        pytest.param(btvec.Drawing(), 30, 20, id="nothing drawn"),
        pytest.param(STAR, 400, 300, id="a star with a stroke"),
        pytest.param(OVERLAP, 90, 80, id="translucent shapes overlapping"),
        pytest.param(EVERYTHING, 1, 1, id="one pixel"),
        pytest.param(EVERYTHING, 65_000, 1, id="the widest row"),
        pytest.param(EVERYTHING, 1, 65_000, id="the tallest column"),
    ],
)
def test_to_png_decodes_to_exactly_the_bytes_to_rgba_returns(
    drawing: btvec.Drawing, width: int, height: int, decode_png
):
    png = drawing.to_png(width, height)
    assert isinstance(png, bytes)
    assert decode_png(png) == (width, height, drawing.to_rgba(width, height))


def test_to_png_takes_a_param_by_keyword():
    shift = btvec.Param("shift", default=0, min=0, max=50, unit="px")
    drawing = btvec.Drawing(btvec.Path.rect(0, 0, 5, 5).fill("#00ff00")).translate(shift, 0)
    assert drawing.to_png(30, 10) != drawing.to_png(30, 10, shift=20)
    with pytest.raises(btvec.ParameterError, match="elsewhere"):
        drawing.to_png(30, 10, elsewhere=1)


def test_to_png_gives_the_same_bytes_every_time():
    assert STAR.to_png(100, 100) == STAR.to_png(100, 100)


def chunk(name: bytes, body: bytes) -> bytes:
    """A PNG chunk: length, name, body and the CRC-32 of name and body."""
    return struct.pack(">I", len(body)) + name + body + struct.pack(">I", zlib.crc32(name + body))


def png_file(width: int, height: int, stream: bytes) -> bytes:
    """A PNG of 8-bit RGBA whose zlib stream holds `stream`: a filter byte, then the bytes of the row, per row."""
    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    return PNG_SIGNATURE + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(stream)) + chunk(b"IEND", b"")


# Two rows of two pixels, and the second row as each of the five filters writes it, worked out by hand from the
# specification (byte n of a pixel looks at byte n of the pixel to its left, above it and above-left, 0 outside):
# Sub subtracts the left, Up the one above, Average the floor of their mean, Paeth the one of left, above and above-left
# nearest to left + above - above-left (a tie goes left, then above); every difference is taken modulo 256.
ABOVE = [50, 10, 150, 101, 50, 50, 200, 100]
BELOW = [10, 10, 100, 100, 200, 77, 9, 3]
FILTERED_BELOW = [
    pytest.param(0, BELOW, id="none"),
    pytest.param(1, [10, 10, 100, 100, 190, 67, 165, 159], id="sub"),
    pytest.param(2, [216, 0, 206, 255, 150, 27, 65, 159], id="up"),
    pytest.param(3, [241, 5, 25, 50, 170, 47, 115, 159], id="average"),
    # byte 4 takes the left (the above-left equals the above), 5 the above, 6 the above-left, 7 the left on a tie
    pytest.param(4, [216, 0, 206, 255, 190, 27, 115, 159], id="paeth"),
]


@pytest.mark.parametrize(("kind", "filtered"), FILTERED_BELOW)
def test_the_decoder_undoes_each_of_the_five_row_filters(kind: int, filtered: list[int], decode_png):
    stream = bytes([0, *ABOVE, kind, *filtered])
    assert decode_png(png_file(2, 2, stream)) == (2, 2, bytes(ABOVE + BELOW))


def test_the_decoder_refuses_a_filter_that_is_none_of_the_five(decode_png):
    with pytest.raises(ValueError, match="filter"):
        decode_png(png_file(1, 1, bytes([5, 0, 0, 0, 0])))


def flipped(data: bytes, at: int) -> bytes:
    """`data` with the lowest bit of the byte at `at` flipped."""
    return data[:at] + bytes([data[at] ^ 1]) + data[at + 1 :]


GOOD = png_file(1, 1, bytes([0, 1, 2, 3, 4]))
IDAT_AT = len(PNG_SIGNATURE) + 12 + 13  # after the signature and the IHDR chunk (12 bytes around its 13)
BAD_FILES = [
    pytest.param(GOOD[:8], "no IHDR", id="signature only"),
    pytest.param(b"\x89PNG\r\n\x1a\x0b" + GOOD[8:], "signature", id="bad signature"),
    pytest.param(flipped(GOOD, IDAT_AT + 8), "CRC", id="a bit flipped in the compressed data"),
    pytest.param(flipped(GOOD, len(PNG_SIGNATURE) + 8 + 2), "CRC", id="a bit flipped in the header"),
    pytest.param(flipped(GOOD, len(GOOD) - 1), "CRC", id="a bit flipped in a CRC"),
    pytest.param(GOOD[: IDAT_AT + 14], "cut short", id="the IDAT chunk truncated"),
    pytest.param(GOOD[:-12], "IEND", id="no IEND chunk"),
    pytest.param(GOOD + b"\x00", "after IEND", id="data after IEND"),
    pytest.param(GOOD[:33] + chunk(b"gAMA", bytes(4)) + GOOD[33:], "gAMA", id="a chunk besides IHDR, IDAT and IEND"),
    pytest.param(GOOD[:8] + GOOD[33:] + GOOD[8:33], "IHDR", id="IHDR not the first chunk"),
    pytest.param(png_file(1, 2, bytes([0, 1, 2, 3, 4])), "stream", id="fewer rows than the header says"),
    pytest.param(
        PNG_SIGNATURE + chunk(b"IHDR", struct.pack(">IIBBBBB", 1, 1, 8, 2, 0, 0, 0)) + chunk(b"IEND", b""),
        "8-bit RGBA",
        id="another colour type",
    ),
]


@pytest.mark.parametrize(("data", "reason"), BAD_FILES)
def test_the_decoder_refuses_a_file_that_is_not_exactly_an_8_bit_rgba_png(data: bytes, reason: str, decode_png):
    with pytest.raises(ValueError, match=reason):
        decode_png(data)


def test_the_decoder_reads_a_good_file(decode_png):
    assert decode_png(GOOD) == (1, 1, bytes([1, 2, 3, 4]))


# --- shadows ----------------------------------------------------------------------------------------------------------

SHADOWED = btvec.Path.rect(20, 20, 20, 20).fill("#ff0000")


def test_a_shadow_without_a_blur_is_the_shape_moved_behind_it():
    image = SHADOWED.shadow(dx=10, dy=0, blur=0, color="#000000").to_rgba(60, 60)
    assert pixel(image, 60, 25, 30) == (255, 0, 0, 255)  # the shape, over its shadow
    assert pixel(image, 60, 45, 30) == (0, 0, 0, 255)  # the shadow alone, 10 to the right
    assert pixel(image, 60, 15, 30) == (0, 0, 0, 0)  # nothing to the left


def test_a_blurred_shadow_follows_the_gaussian():
    # a blur of 8 is a deviation of 4; the shadow of the square 20 to 40 across, in the middle row, left of the shape
    image = SHADOWED.shadow(dx=0, dy=0, blur=8, color="#000000").to_rgba(60, 60)

    def share(x: float, low: float, high: float) -> float:
        return (math.erf((high - x) / (4 * math.sqrt(2))) - math.erf((low - x) / (4 * math.sqrt(2)))) / 2

    for x in range(5, 19):
        wanted = 255 * share(x + 0.5, 20, 40) * share(30.5, 20, 40)
        assert abs(pixel(image, 60, x, 30)[3] - wanted) <= 2, x


def test_a_blurred_shadow_is_a_filter_on_a_group_in_the_svg_and_a_hard_one_a_copy():
    blurred = parse(SHADOWED.shadow(dx=4, dy=6, blur=10).to_svg(100, 100))
    (filtered,) = blurred.findall(f"{SVG}filter")
    (blur,) = filtered.findall(f"{SVG}feGaussianBlur")
    assert blur.attrib["stdDeviation"] == "5"
    (group,) = blurred.findall(f"{SVG}g")
    assert group.attrib["filter"] == f"url(#{filtered.attrib['id']})"
    hard = parse(SHADOWED.shadow(dx=4, dy=6, blur=0).to_svg(100, 100))
    assert hard.findall(f"{SVG}filter") == []
    assert [element.attrib["d"] for element in hard.findall(f"{SVG}path")] == [
        "M24 26 L44 26 L44 46 L24 46 Z",
        "M20 20 L40 20 L40 40 L20 40 Z",
    ]


def test_a_mesh_has_the_shadow_of_a_blur_of_zero_and_refuses_a_blur():
    mesh = SHADOWED.shadow(blur=0).to_mesh(100, 100)
    assert len(mesh.indices) == 12  # two rectangles of two triangles
    with pytest.raises(btvec.ParameterError, match="shadow"):
        SHADOWED.shadow(blur=16).to_mesh(100, 100)


def test_a_blur_past_a_thousand_output_pixels_of_deviation_raises_naming_the_blur():
    far = btvec.Path.rect(-90_000, 0, 10, 10).fill("#ff0000")
    far.shadow(blur=2000).to_rgba(100, 100)
    with pytest.raises(btvec.ParameterError, match="blur"):
        far.shadow(blur=2001).to_rgba(100, 100)


# --- halftones --------------------------------------------------------------------------------------------------------

DOTS = btvec.Path.rect(5, 5, 90, 90).halftone("#000000", tone=0.5, pitch=10, angle=0)


def test_a_halftone_covers_the_share_of_its_shape_the_tone_leaves_over():
    image = DOTS.to_rgba(100, 100)
    # 81 dots of half a cell of 100 each; a dot is a polygon a hair under the circle
    covered = sum(image[at + 3] for at in range(0, len(image), 4)) / 255
    assert covered == pytest.approx(81 * 50, rel=0.015)
    assert pixel(image, 100, 20, 20) == (0, 0, 0, 255)  # the middle of a dot
    assert pixel(image, 100, 25, 25) == (0, 0, 0, 0)  # between four dots


def test_a_halftone_is_a_clipped_group_of_one_path_of_dots_in_the_svg():
    root = parse(DOTS.to_svg(100, 100))
    (clip,) = root.findall(f"{SVG}clipPath")
    (group,) = root.findall(f"{SVG}g")
    assert group.attrib["clip-path"] == f"url(#{clip.attrib['id']})"
    (outline,) = clip.findall(f"{SVG}path")
    assert outline.attrib["d"] == "M5 5 L95 5 L95 95 L5 95 Z"
    (dots,) = group.findall(f"{SVG}path")
    assert dots.attrib["fill"] == "#000000"
    assert dots.attrib["d"].count("M") == 81


def test_a_mesh_has_the_triangles_of_the_dots():
    mesh = DOTS.to_mesh(100, 100)
    assert len(mesh.indices) > 81 * 3 * 8
    assert all(mesh.vertices[at + 5] == 1.0 for at in range(0, len(mesh.vertices), 8))


def test_a_pitch_that_asks_for_millions_of_dots_is_refused_naming_the_pitch():
    big = btvec.Path.circle((500, 500), 500).halftone("#000000", tone=0.5, pitch=0.01)
    for draw in (big.to_rgba, big.to_svg, big.to_mesh):
        with pytest.raises(btvec.ParameterError, match="pitch"):
            draw(100, 100)


# --- knockouts --------------------------------------------------------------------------------------------------------

RED_SQUARE = btvec.Path.rect(0, 0, 60, 60).fill("#ff0000")
HOLE = btvec.Path.rect(20, 20, 20, 20).fill("#0000ff").knockout()
KNOCKED = btvec.Drawing(RED_SQUARE, HOLE)


def test_a_knockout_cuts_a_hole_in_the_pixels_and_paints_nothing_itself():
    image = KNOCKED.to_rgba(60, 60)
    assert pixel(image, 60, 30, 30) == (0, 0, 0, 0)
    assert pixel(image, 60, 10, 10) == (255, 0, 0, 255)
    assert pixel(image, 60, 50, 30) == (255, 0, 0, 255)


def test_a_knockout_is_a_mask_over_what_is_below_it_in_the_svg():
    root = parse(KNOCKED.to_svg(60, 60))
    (mask,) = root.findall(f"{SVG}mask")
    (group,) = root.findall(f"{SVG}g")
    assert group.attrib["mask"] == f"url(#{mask.attrib['id']})"
    assert [element.attrib["d"] for element in group.findall(f"{SVG}path")] == ["M0 0 L60 0 L60 60 L0 60 Z"]
    assert [element.attrib["fill"] for element in mask.findall(f"{SVG}path")] == ["#000000"]
    assert mask.find(f"{SVG}rect") is not None


def test_a_mesh_has_the_triangles_of_what_is_left_of_the_shape_below():
    mesh = KNOCKED.to_mesh(60, 60)
    # a square with a square hole in it, and the hole is not blue: no vertex of the mesh has a blue channel
    assert len(mesh.indices) > 12
    assert all(mesh.vertices[at + 4] == 0.0 for at in range(0, len(mesh.vertices), 8))


# A window ------------------------------------------------------------------------------------------------------------

WINDOWED = btvec.Drawing(
    btvec.Path.rect(20, 14, 50, 30).fill("#3070c0").shadow(dx=6, dy=9, blur=20, color="#000000b0"),
    size=(120, 90),
)


def rows_of(image: bytes, width: int, x: int, y: int, w: int, h: int) -> bytes:
    """The rows `y..y + h` and columns `x..x + w` of the straight-alpha RGBA image `image` that is `width` across."""
    return b"".join(image[(row * width + x) * 4 : (row * width + x + w) * 4] for row in range(y, y + h))


@pytest.mark.parametrize("window", [(7, 5, 60, 40), (0, 0, 1, 1), (119, 89, 1, 1), (33, 0, 41, 90), (0, 0, 120, 90)])
def test_a_window_is_the_rows_and_columns_of_the_whole_render(window: tuple[int, int, int, int]):
    whole = WINDOWED.to_rgba(120, 90)
    assert WINDOWED.to_rgba(120, 90, window=window) == rows_of(whole, 120, *window)


def test_a_png_window_decodes_to_the_pixels_of_the_rgba_window(decode_png):
    window = (7, 5, 60, 40)
    png = WINDOWED.to_png(120, 90, window=window)
    assert decode_png(png) == (60, 40, WINDOWED.to_rgba(120, 90, window=window))


def test_a_window_may_lie_in_a_canvas_past_the_limits_of_a_whole_render():
    drawing = btvec.Drawing(btvec.Path.rect(99_990, 99_990, 8, 8).fill("#ff0000"), size=(100_000, 100_000))
    image = drawing.to_rgba(100_000, 100_000, window=(99_988, 99_988, 12, 12))
    assert pixel(image, 12, 2, 2) == (255, 0, 0, 255)
    assert pixel(image, 12, 1, 1) == (0, 0, 0, 0)
    with pytest.raises(btvec.ParameterError, match="width"):
        drawing.to_rgba(100_000, 100_000)


@pytest.mark.parametrize("output", ["to_rgba", "to_png"])
@pytest.mark.parametrize(
    "window",
    [
        pytest.param((100, 0, 30, 10), id="past the right edge"),
        pytest.param((-1, 0, 5, 5), id="negative"),
        pytest.param((0, 0, 0, 5), id="no width"),
        pytest.param((0.5, 0, 5, 5), id="half a pixel"),
        pytest.param((0, 0, 65_001, 1), id="past the side limit"),
    ],
)
def test_a_window_outside_the_output_or_not_whole_raises_naming_window(output: str, window: tuple[float, ...]):
    with pytest.raises(btvec.ParameterError, match="window"):
        getattr(btvec.Drawing(LINE), output)(120, 90, window=window)
