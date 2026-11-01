"""The example programs in examples/ run as far as they can draw and draw what their comments say. Each runs with
`runpy` in an empty folder, which it must leave empty: the picture stays in a variable. Expected values are worked out
by hand: the star's from the geometry of a five-pointed star, the bars' from the series printed in the program."""

import runpy
import xml.etree.ElementTree as ET
from pathlib import Path

import pytest

import bullterrier_vector as btvec

EXAMPLES = Path(__file__).resolve().parents[1] / "examples"


@pytest.fixture
def run(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    """Run an example in an empty folder; gives the variables it left."""
    monkeypatch.chdir(tmp_path)

    def run_example(name: str) -> dict[str, object]:
        found = runpy.run_path(str(EXAMPLES / f"{name}.py"), run_name="__main__")
        assert list(tmp_path.iterdir()) == []
        return found

    return run_example


def test_star_draws_the_star_a_dot_on_its_top_tip_and_a_line_from_its_far_side(run):
    star, dot, line = run("star")["shapes"]
    assert [shape.attributes["index"] for shape in (star, dot, line)] == [0, 1, 2]
    assert [shape.attributes["order"] for shape in (star, dot, line)] == [0.0, 0.5, 1.0]

    # the star is painted: filled yellow, stroked 4 pixels dark
    assert (star.style.fill, star.style.stroke, star.style.stroke_width) == ("#ffcc33ff", "#1a1a1aff", 4.0)
    # five tips on a circle of 150 about (200, 200): the top tip is at (200, 50)
    assert star.path.bounds.y == pytest.approx(50)
    assert star.path.contains((200, 200))

    # the dot: a circle of radius 8 on the top tip, filled red only
    assert (dot.style.fill, dot.style.stroke) == ("#e2412eff", None)
    assert dot.path.bounds == btvec.Rect(192, 42, 16, 16)

    # the line starts half the outline further on, which is the inner corner at the bottom, (200, 260), and is 40 long
    assert (line.style.fill, line.style.stroke) == (None, "#e2412eff")
    assert line.path.length == pytest.approx(40)
    assert line.path.point_at(0).x == pytest.approx(200)
    assert line.path.point_at(0).y == pytest.approx(260)


def test_star_writes_its_svg_at_the_size_of_its_frame_one_path_a_shape(run):
    found = run("star")
    star, dot, line = found["shapes"]
    svg = found["svg"]
    assert isinstance(svg, str)
    root = ET.fromstring(svg)  # noqa: S314  our own output
    assert (root.attrib["width"], root.attrib["height"], root.attrib["viewBox"]) == ("400", "400", "0 0 400 400")
    elements = root.findall("{http://www.w3.org/2000/svg}path")
    assert len(elements) == 3

    # the frame is the output, so the data is the shapes' own, and the paint as the comments of the example say
    assert [element.attrib["d"] for element in elements] == [shape.path.to_svg_path() for shape in (star, dot, line)]
    star_element, dot_element, line_element = (element.attrib for element in elements)
    assert (star_element["fill"], star_element["stroke"], star_element["stroke-width"]) == ("#ffcc33", "#1a1a1a", "4")
    assert (dot_element["fill"], "stroke" in dot_element) == ("#e2412e", False)
    assert (line_element["fill"], line_element["stroke"], line_element["stroke-width"]) == ("none", "#e2412e", "2")


SERIES = [0.18, 0.35, 0.52, 0.41, 0.77, 0.93, 0.64, 0.49, 0.71, 0.28, 0.15, 0.33]


def test_series_bars_draws_twelve_bars_with_the_tallest_in_the_second_ink_on_top(run):
    bars = run("series_bars")["shapes"]
    assert len(bars) == 12
    assert [bar.attributes["index"] for bar in bars] == list(range(12))

    # the tallest (index 5 of the series) is last in paint order and the only one in #e2412e
    assert [bar.style.fill for bar in bars] == ["#15161aff"] * 11 + ["#e2412eff"]
    assert all(bar.style.stroke is None for bar in bars)

    # each bar stands on the line y = 900, 56 wide, 76 apart from x = 90, 700 high at a value of 1; the order each
    # bar got in the first stack survives `only` and `without`
    painted_in_series_order = [*range(5), *range(6, 12), 5]
    for bar, i in zip(bars, painted_in_series_order, strict=True):
        bounds = bar.path.bounds
        assert (bounds.x, bounds.width) == (90 + 76 * i, 56)
        assert bounds.height == pytest.approx(700 * SERIES[i])
        assert bounds.y + bounds.height == pytest.approx(900)
        assert bar.attributes["order"] == pytest.approx(i / 11)


def test_badge_frames_writes_one_second_at_24_frames_a_second_numbered_from_zero(run):
    written = run("badge_frames")["written"]
    assert written == [f"badge-{i:04d}.png" for i in range(24)]


def test_poster_overlaps_two_multiply_inks_on_paper_and_keeps_its_stack_of_shapes(run):
    found = run("poster")
    shapes, rgba = found["shapes"], found["rgba"]
    # paper, twelve stars, two inks, the zigzag, eight hairlines
    assert len(shapes) == 1 + 12 + 2 + 1 + 8
    assert [shape.style.blend for shape in shapes[13:15]] == ["multiply", "multiply"]
    assert len(rgba) == 800 * 600 * 4

    def pixel(x: int, y: int) -> tuple[int, ...]:
        start = (y * 800 + x) * 4
        return tuple(rgba[start : start + 4])

    # the corner is bare paper, opaque
    assert pixel(2, 2) == (246, 241, 228, 255)
    # where the inks overlap, each channel is the product of both inks and the paper under them, over 255 twice
    pink, blue, paper = (255, 79, 154), (46, 100, 226), (246, 241, 228)
    expected = tuple(a * b * c / 255**2 for a, b, c in zip(pink, blue, paper, strict=True))
    assert pixel(400, 300) == pytest.approx((*expected, 255), abs=1.5)
    # and the pink alone is only its own product with the paper
    assert pixel(200, 300)[:3] == pytest.approx(tuple(a * c / 255 for a, c in zip(pink, paper, strict=True)), abs=1.5)
    # the file the picture would be is well formed XML with one path per shape that has geometry
    root = ET.fromstring(found["svg"])  # noqa: S314  our own output
    assert len(root.findall("{http://www.w3.org/2000/svg}path")) == len(shapes)


def test_kinetic_type_draws_a_shape_a_letter_inside_the_frame_and_a_kick_thickens_only_the_letter_it_hits(run):
    found = run("kinetic_type")
    letters = found["letters"]
    calm = letters.shapes()
    assert [shape.attributes["char"] for shape in calm] == list("NOSLEEP")
    assert [shape.attributes["line"] for shape in calm] == [0, 0, 1, 1, 1, 1, 1]
    for shape in calm:
        bounds = shape.path.bounds
        assert bounds.x >= 0
        assert bounds.x + bounds.width <= 1080
        assert bounds.y >= 0
        assert bounds.y + bounds.height <= 1350

    # `pulse` has an entry per character of "NO\nSLEEP": 8, the line break included. A kick on the N (entry 0) makes
    # the N wider, and the letters after it follow its advance; the second line is untouched.
    kicked = letters.shapes(pulse=[1, 0, 0, 0, 0, 0, 0, 0])
    assert kicked[0].path.bounds.width > calm[0].path.bounds.width
    assert kicked[1].path.bounds.width == pytest.approx(calm[1].path.bounds.width)
    assert [shape.path for shape in kicked[2:]] == [shape.path for shape in calm[2:]]


def test_kinetic_type_at_a_time_rises_the_plane_and_swaps_the_hairlines_for_the_print(run):
    at_time = run("kinetic_type")["at_time"]
    before, after = at_time(2.0).shapes(), at_time(4.0).shapes()
    paper, plane, *rest = before
    assert len(before) == len(after) == 1 + 1 + 7 + 7  # paper, plane, the letters printed, the letters as hairlines
    assert paper.path.bounds == btvec.Rect(0, 0, 1080, 1350)
    # half risen at 2 s (1 s in a rise of 2 s), all the way at 4 s
    assert plane.path.bounds.y == 1350 / 2
    assert after[1].path.bounds.y == 0
    assert [shape.style.opacity for shape in rest] == [0.0] * 7 + [1.0] * 7
    assert [shape.style.opacity for shape in after[2:]] == [1.0] * 7 + [0.0] * 7
    # the kick starts on the beat at 4 s with the first letter, the next ones a little behind it
    assert after[2].path.bounds.width > rest[0].path.bounds.width
