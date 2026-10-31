"""`Drawing.to_svg` through the public package: its type, the document it writes (read back with an XML parser), the
sizes it refuses and the Param values it takes by keyword. Expected values are worked out by hand."""

import math
import xml.etree.ElementTree as ET
from typing import Any

import pytest

import bullterrier_vector as btvec

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


@pytest.mark.parametrize(
    ("width", "height", "name", "shown"),
    [
        pytest.param(0, 10, "width", "0", id="zero width"),
        pytest.param(10, 0, "height", "0", id="zero height"),
        pytest.param(0.5, 10, "width", "0.5", id="half a pixel"),
        pytest.param(10, 1080.5, "height", "1080.5", id="a fraction"),
        pytest.param(math.nan, 10, "width", "NaN", id="not a number"),
        pytest.param(10, -4, "height", "-4", id="negative"),
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


@pytest.mark.parametrize("name", ["into", "pen_mm"])
def test_the_names_of_output_options_are_not_params_so_a_value_for_them_raises(name: str):
    with pytest.raises(btvec.ParameterError, match=name):
        btvec.Drawing(LINE).to_svg(10, 10, **{name: 1})
    with pytest.raises(btvec.ParameterError, match=name):
        btvec.Param(name, default=0, min=0, max=1)


def test_a_value_outside_a_params_range_raises():
    shift = btvec.Param("shift", default=3, min=0, max=50, unit="px")
    with pytest.raises(btvec.ParameterError, match="shift"):
        btvec.Drawing(LINE).translate(shift, 0).to_svg(10, 10, shift=51)
