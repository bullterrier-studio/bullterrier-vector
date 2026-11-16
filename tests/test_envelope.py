"""`envelope`, `arch` and `Path.arc` through the public package. Expected values are by hand: a point `(x, y)` of
the frame goes to `lerp(T(u), B(u), v)`, `u` and `v` its shares of the frame's width and height, `T` and `B` the
points of the curves at that share of their lengths; an arch lifts the middle of the frame's top edge by `bend`
times its width."""

import inspect
import json
import math

import pytest

import bullterrier_vector as btvec

SQUARE = btvec.Path.rect(0, 0, 100, 100)
SLOPE = btvec.Path.line((0, 0), (100, 20))


def vertices(path: btvec.Path) -> list[tuple[float, float]]:
    """The corners of a path made of straight lines."""
    numbers = path.to_svg_path().replace("M", " ").replace("L", " ").replace("Z", " ").split()
    return [(float(x), float(y)) for x, y in zip(numbers[::2], numbers[1::2], strict=True)]


def has(path: btvec.Path, point: tuple[float, float]) -> bool:
    return any(math.dist(vertex, point) < 1e-9 for vertex in vertices(path))


def test_the_corners_of_a_square_bent_under_a_slanted_top_are_by_hand() -> None:
    bent = SQUARE.envelope(SLOPE)  # the frame is its own bounds, the bottom the frame's own
    assert bent.to_svg_path().endswith("Z")
    for corner in [(0, 0), (100, 20), (100, 100), (0, 100)]:
        assert has(bent, corner), corner


def test_the_amount_and_the_bottom_blend_between_the_straight_edge_and_the_curve() -> None:
    column = btvec.Path.line((50, 0), (50, 100))
    bottom = btvec.Path.line((0, 100), (100, 140))
    assert has(column.envelope(SLOPE, bottom, frame=(0, 0, 100, 100)), (50, 10))
    assert has(column.envelope(SLOPE, bottom, frame=btvec.Rect(0, 0, 100, 100)), (50, 120))
    # half the bend: the top at (50, 5), the bottom at (50, 110)
    half = column.envelope(SLOPE, bottom, frame=(0, 0, 100, 100), amount=0.5)
    assert has(half, (50, 5))
    assert has(half, (50, 110))


def test_an_arch_lifts_the_whole_band_by_the_bend_times_the_width() -> None:
    arched = SQUARE.arch(0.2)
    assert has(arched, (50, -20))
    assert has(arched, (50, 80))
    assert has(arched, (25, -15))
    assert has(arched, (25, 85))
    assert has(arched, (100, 100))
    assert has(SQUARE.arch(-0.2), (50, 20))
    assert has(SQUARE.arch(-0.2), (50, 120))


def test_an_arch_keeps_the_height_of_the_frame_and_a_negative_bend_mirrors_a_positive_one() -> None:
    word = btvec.Path.rect(0, 0, 365, 150)  # 0.3 * 365 = 109.5
    assert has(word.arch(0.3), (182.5, -109.5))
    assert has(word.arch(0.3), (182.5, 40.5))
    assert has(word.arch(-0.3), (182.5, 109.5))
    assert has(word.arch(-0.3), (182.5, 259.5))


def test_a_frame_with_no_area_is_refused_naming_it() -> None:
    with pytest.raises(btvec.ParameterError, match="frame"):
        SQUARE.arch(0.2, frame=(0, 0, 0, 10))
    with pytest.raises(btvec.ParameterError, match="frame"):
        btvec.Path.line((0, 0), (10, 0)).arch(0.2)
    with pytest.raises(TypeError):
        SQUARE.arch(0.2, frame=(0, 0, 10))  # ty: ignore[invalid-argument-type]


def test_a_curve_with_no_length_is_a_geometry_error() -> None:
    with pytest.raises(btvec.GeometryError):
        SQUARE.envelope(btvec.Path.from_svg_path("M5 5"))
    with pytest.raises(btvec.GeometryError):
        SQUARE.envelope(btvec.Path())


def test_the_drawing_shares_one_frame_between_its_shapes() -> None:
    bars = btvec.Drawing(btvec.Path.rect(0, 0, 100, 10), btvec.Path.rect(0, 50, 100, 10))
    first, second = (shape.path for shape in bars.arch(0.2).shapes())
    assert has(first, (50, -20))  # the frame is (0, 0, 100, 60): the first bar's top edge is the frame's
    assert has(second, (50, 30))  # lifted by the same 20 at every height
    (alone,) = (shape.path for shape in bars.only(index=1).arch(0.2).shapes())
    assert has(alone, (50, 30))  # a frame of its own, its top edge at 50: lifted 20, as the frame argument gives
    _, framed = (shape.path for shape in bars.arch(0.2, frame=(0, 50, 100, 10)).shapes())
    assert has(framed, (50, 30))


def test_amount_and_bend_are_per_shape_numbers_and_params() -> None:
    both = btvec.Drawing(SQUARE, SQUARE)
    flat, bent = (s.path for s in both.envelope(SLOPE, amount=btvec.by_index(0, 1)).shapes())
    assert has(flat, (100, 0))
    assert has(bent, (100, 20))
    amount = btvec.Param("amount", default=0, min=0, max=1)
    drawing = btvec.Drawing(SQUARE).envelope(SLOPE, amount=amount)
    assert has(drawing.shapes()[0].path, (100, 0))
    assert has(drawing.shapes(amount=0.5)[0].path, (100, 10))
    bend = btvec.Param("bend", default=0, min=-1, max=1)
    assert has(btvec.Drawing(SQUARE).arch(bend).shapes(bend=0.2)[0].path, (50, -20))


def test_the_defaults_of_the_step_are_the_documented_ones() -> None:
    envelope = inspect.signature(btvec.Drawing.envelope).parameters
    assert envelope["amount"].default == 1
    assert envelope["bottom"].default is None
    assert envelope["frame"].default is None
    assert inspect.signature(btvec.Drawing.arch).parameters["frame"].default is None


def test_the_steps_are_calls_in_the_document_and_read_back() -> None:
    drawing = btvec.Drawing(SQUARE).envelope(SLOPE, frame=(0, 0, 100, 100), amount=0.5).arch(0.1)
    steps = json.loads(drawing.to_json())["root"]
    assert steps[-2] == {"envelope": {"top": "M0 0 L100 20", "frame": [0, 0, 100, 100], "amount": 0.5}}
    assert steps[-1] == {"arch": {"bend": 0.1}}
    again = btvec.Drawing.from_json(drawing.to_json())
    assert again.to_json() == drawing.to_json()
    assert again.shapes()[0].path.to_svg_path() == drawing.shapes()[0].path.to_svg_path()


def test_an_arc_runs_clockwise_on_the_screen_from_plus_x() -> None:
    quarter = btvec.Path.arc((10, 20), 5, 0, 90)
    assert quarter.to_svg_path().startswith("M15 20 C")
    assert quarter.to_svg_path().endswith(" 10 25")
    back = btvec.Path.arc((10, 20), 5, 0, -90)
    assert back.to_svg_path().endswith(" 10 15")
    assert quarter.length == pytest.approx(5 * math.pi / 2, rel=1e-3)
    full = btvec.Path.arc((0, 0), 10, 0, 360)
    assert not full.to_svg_path().endswith("Z")
    assert full.length == pytest.approx(20 * math.pi, rel=1e-3)


@pytest.mark.parametrize(
    ("radius", "start", "end", "name"),
    [(0, 0, 90, "radius"), (1, 0, 400, "end"), (1, 5, 5, "end"), (1, float("nan"), 5, "start")],
)
def test_a_bad_arc_is_refused_naming_the_argument(radius: float, start: float, end: float, name: str) -> None:
    with pytest.raises(btvec.ParameterError, match=name):
        btvec.Path.arc((0, 0), radius, start, end)


@pytest.mark.parametrize("step", ["envelope", "arch"])
def test_a_document_frame_without_four_numbers_is_a_format_error_naming_frame(step: str) -> None:
    arguments = {"top": "M0 0 L10 2"} if step == "envelope" else {"bend": 0.2}
    text = json.dumps({"version": 1, "root": [{"path": "M0 0 L9 9"}, {step: {**arguments, "frame": [0, 0, 1]}}]})
    with pytest.raises(btvec.FormatError, match=r"frame: must be four numbers"):
        btvec.Drawing.from_json(text)
