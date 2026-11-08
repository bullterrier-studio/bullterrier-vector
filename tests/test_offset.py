"""`offset` and `outline` through the public package, on `Path` and as steps of a `Drawing`: call shapes of docs/api.md,
the geometry by bounds and areas worked out by hand on a 10 by 10 square, per-shape numbers, refusals and JSON."""

import json
import math
from typing import Any

import pytest

import bullterrier_vector as btvec

Drawing = btvec.Drawing
ParameterError = btvec.ParameterError

SQUARE = btvec.Path.rect(0, 0, 10, 10)
LINE = btvec.Path.line((0, 0), (10, 0))
# the same classes, untyped: the wrong types below are what is tested
LOOSE_PATH: Any = SQUARE
LOOSE_DRAWING: Any = Drawing(SQUARE)


def edges(path: btvec.Path) -> tuple[float, float, float, float]:
    """`(left, top, right, bottom)` of the outline."""
    box = path.bounds
    return (box.x, box.y, box.x + box.width, box.y + box.height)


def approx_edges(path: btvec.Path, wanted: tuple[float, float, float, float]) -> None:
    assert edges(path) == pytest.approx(wanted, abs=1e-9)


def area(path: btvec.Path) -> float:
    """The area an outline of straight edges fills: the shoelace sum over its sub-paths, holes counted negative."""
    total = 0.0
    for sub in path.to_svg_path().split("M")[1:]:
        points = [tuple(float(n) for n in point.split()) for point in sub.replace("Z", "").split("L")]
        for (x1, y1), (x2, y2) in zip(points, [*points[1:], points[0]], strict=True):
            total += (x1 * y2 - x2 * y1) / 2
    return abs(total)


# --- Path ----------------------------------------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("join", "corner"),
    [("miter", 4.0), ("bevel", 2.0), ("round", math.pi)],
)
def test_offset_grows_the_region_by_the_distance_and_the_join_shapes_each_corner(join, corner):
    # a 10 square grown by 2: the sides add 80, each of the 4 corners adds a 2 x 2 box, half of it or a quarter disc
    grown = SQUARE.offset(2, join=join)
    approx_edges(grown, (-2, -2, 12, 12))
    assert area(grown) == pytest.approx(180 + 4 * corner, abs=0.1)


def test_offset_shrinks_for_a_negative_distance_and_leaves_an_empty_path_past_nothing():
    approx_edges(SQUARE.offset(-3), (3, 3, 7, 7))
    assert SQUARE.offset(-6).to_svg_path() == ""


@pytest.mark.parametrize(
    ("cap", "painted"),
    [("butt", 40.0), ("square", 56.0), ("round", 40.0 + 4 * math.pi)],
)
def test_outline_paints_the_stroke_and_the_cap_finishes_the_ends(cap, painted):
    # 4 wide along 10: the body is 40, a square end adds 2 x 4 box, a round end a half disc of radius 2
    assert area(LINE.outline(4, cap=cap)) == pytest.approx(painted, abs=0.1)


def test_the_defaults_are_round_joins_and_caps_a_miter_limit_of_4_and_a_tolerance_of_a_hundredth():
    assert SQUARE.offset(2) == SQUARE.offset(2, join="round", miter_limit=4, tolerance=0.01)
    assert LINE.outline(2) == LINE.outline(2, join="round", cap="round", miter_limit=4, tolerance=0.01)


def test_a_miter_sharper_than_the_limit_is_clipped_flat_and_a_looser_limit_keeps_the_point():
    # the corner of a triangle with a 30 degree apex, grown by 1: its point is 1 / sin(15 degrees) = 3.86 from the edge
    thin = btvec.Path.from_svg_path("M0 0 L20 0 L0 5.36 Z")
    pointed = thin.offset(1, join="miter", miter_limit=10)
    clipped = thin.offset(1, join="miter", miter_limit=2)
    assert edges(clipped)[2] < edges(pointed)[2]


@pytest.mark.parametrize(
    ("call", "match"),
    [
        pytest.param(lambda: SQUARE.offset(float("nan")), r"distance", id="offset distance nan"),
        pytest.param(lambda: SQUARE.offset(1e101), r"distance", id="offset distance beyond 1e100"),
        pytest.param(lambda: SQUARE.offset(1, miter_limit=0.5), r"miter_limit", id="offset miter_limit below 1"),
        pytest.param(lambda: SQUARE.offset(1, tolerance=0), r"tolerance", id="offset tolerance zero"),
        pytest.param(lambda: SQUARE.offset(1, join="sharp"), r"join", id="offset join not a word"),
        pytest.param(lambda: SQUARE.outline(0), r"width", id="outline width zero"),
        pytest.param(lambda: SQUARE.outline(-2), r"width", id="outline width negative"),
        pytest.param(lambda: SQUARE.outline(2, miter_limit=0.5), r"miter_limit", id="outline miter_limit below 1"),
        pytest.param(lambda: SQUARE.outline(2, cap="flat"), r"cap", id="outline cap not a word"),
        pytest.param(lambda: SQUARE.outline(2, join="sharp"), r"join", id="outline join not a word"),
        pytest.param(
            lambda: LOOSE_PATH.offset(btvec.Param("d", default=1, min=0, max=2)),
            r"Drawing\(path\)",
            id="a Param on a Path",
        ),
    ],
)
def test_a_value_the_library_refuses_is_a_parameter_error_naming_it(call, match):
    with pytest.raises(ParameterError, match=match):
        call()


@pytest.mark.parametrize(
    "call",
    [
        pytest.param(lambda: LOOSE_PATH.offset(1, "miter"), id="a positional join"),
        pytest.param(lambda: LOOSE_PATH.outline(1, "miter"), id="a positional join for outline"),
        pytest.param(lambda: LOOSE_PATH.offset("2"), id="a distance that is text"),
        pytest.param(lambda: LOOSE_PATH.offset(), id="offset without a distance"),
        pytest.param(lambda: LOOSE_PATH.outline(), id="outline without a width"),
        pytest.param(lambda: LOOSE_PATH.offset(1, cap="butt"), id="offset with a cap"),
    ],
)
def test_a_call_of_the_wrong_shape_on_a_path_is_a_type_error(call):
    with pytest.raises(TypeError):
        call()


# --- Drawing -------------------------------------------------------------------------------------------------------


def test_a_step_is_the_path_method_on_each_shape_and_keeps_its_style_and_attributes():
    red = SQUARE.fill("#ff0000").shapes()[0]
    far = SQUARE.translate(30, 0)
    before = Drawing(red, far).shapes()
    for name, arguments in [("offset", (2,)), ("outline", (3,))]:
        after = getattr(Drawing(red, far), name)(*arguments, join="miter").shapes()
        assert [shape.path for shape in after] == [
            getattr(SQUARE, name)(*arguments, join="miter"),
            getattr(far, name)(*arguments, join="miter"),
        ], name
        assert [(s.style, s.attributes) for s in after] == [(s.style, s.attributes) for s in before], name


def test_a_number_for_each_shape_gives_every_shape_its_own_distance_and_width():
    row = Drawing(SQUARE, SQUARE.translate(30, 0), SQUARE.translate(60, 0))
    grown = [shape.path for shape in row.offset(btvec.by_index(1, 3), join="miter").shapes()]
    approx_edges(grown[0], (-1, -1, 11, 11))
    approx_edges(grown[1], (28, -2, 42, 12))
    approx_edges(grown[2], (57, -3, 73, 13))
    outlined = [shape.path for shape in row.outline(btvec.by_index(2, 6), join="miter", cap="butt").shapes()]
    approx_edges(outlined[0], (-1, -1, 11, 11))
    approx_edges(outlined[2], (57, -3, 73, 13))


def test_a_param_distance_follows_the_value_and_is_described_by_the_drawing():
    grow = btvec.Param("grow", default=0, min=-4, max=4)
    drawing = Drawing(SQUARE).offset(grow, join="miter")
    assert [p.name for p in drawing.parameters()] == ["grow"]
    for value, wanted in [(0, (0, 0, 10, 10)), (3, (-3, -3, 13, 13)), (-4, (4, 4, 6, 6))]:
        (shape,) = drawing.shapes(grow=value)
        approx_edges(shape.path, wanted)


def test_a_shape_shrunk_past_nothing_stays_in_the_list_with_an_empty_path():
    shapes = Drawing(SQUARE, btvec.Path.rect(30, 0, 4, 4)).offset(-3).shapes()
    assert [shape.path.to_svg_path() == "" for shape in shapes] == [False, True]


@pytest.mark.parametrize(
    ("call", "step", "argument"),
    [
        pytest.param(lambda d: d.offset(1, miter_limit=0.5), "offset", "miter_limit", id="offset miter_limit"),
        pytest.param(lambda d: d.offset(1, tolerance=0), "offset", "tolerance", id="offset tolerance"),
        pytest.param(lambda d: d.offset(float("nan")), "offset", "distance", id="offset distance"),
        pytest.param(lambda d: d.outline(0), "outline", "width", id="outline width"),
        pytest.param(lambda d: d.outline(2, miter_limit=0.5), "outline", "miter_limit", id="outline miter_limit"),
    ],
)
def test_a_value_a_step_refuses_is_a_parameter_error_naming_the_step_and_the_argument(call, step, argument):
    with pytest.raises(ParameterError, match=rf"\({step}\)\.{argument}"):
        call(Drawing(SQUARE))


def test_a_word_a_step_does_not_take_is_a_parameter_error():
    for call in (lambda: Drawing(SQUARE).offset(1, join="sharp"), lambda: Drawing(SQUARE).outline(1, cap="flat")):
        with pytest.raises(ParameterError, match=r"join|cap"):
            call()


def test_a_width_read_from_a_param_is_refused_when_drawn():
    width = btvec.Param("w", default=2, min=-5, max=5)
    drawing = Drawing(SQUARE).outline(width)
    with pytest.raises(ParameterError, match=r"\(outline\)\.width"):
        drawing.shapes(w=0)


@pytest.mark.parametrize(
    "call",
    [
        pytest.param(lambda: LOOSE_DRAWING.offset(1, "miter"), id="a positional join"),
        pytest.param(lambda: LOOSE_DRAWING.offset("2"), id="a distance that is text"),
        pytest.param(
            lambda: LOOSE_DRAWING.offset(1, miter_limit=btvec.Param("m", default=2, min=1, max=9)),
            id="a Param miter_limit",
        ),
        pytest.param(lambda: LOOSE_DRAWING.outline(), id="outline without a width"),
        pytest.param(lambda: LOOSE_DRAWING.outline(1, tolerance="fine"), id="a tolerance that is text"),
    ],
)
def test_a_step_called_the_wrong_way_is_a_type_error(call):
    with pytest.raises(TypeError):
        call()


# --- JSON ----------------------------------------------------------------------------------------------------------


def test_both_steps_write_to_json_leaving_the_defaults_out_and_load_back_to_the_same_drawing():
    grow = btvec.Param("grow", default=1, min=0, max=5)
    drawing = (
        Drawing(SQUARE)
        .offset(grow)
        .offset(btvec.by_index(1, 3), join="miter", miter_limit=2, tolerance=0.05)
        .outline(3)
        .outline(1.5, join="bevel", cap="square", miter_limit=6, tolerance=0.25)
    )
    text = drawing.to_json()
    steps = json.loads(text)["root"][1:]
    assert steps[0] == {"offset": {"distance": {"param": "grow"}}}
    assert steps[1]["offset"]["join"] == "miter"
    assert steps[2] == {"outline": {"width": 3}}
    assert steps[3]["outline"] == {"width": 1.5, "join": "bevel", "cap": "square", "miter_limit": 6, "tolerance": 0.25}
    loaded = Drawing.from_json(text)
    assert loaded.to_json() == text
    assert loaded.to_svg(60, 60, grow=2) == drawing.to_svg(60, 60, grow=2)


@pytest.mark.parametrize(
    "call",
    [
        pytest.param('{"offset":{"distance":1,"join":"sharp"}}', id="offset join not a word"),
        pytest.param('{"offset":{}}', id="offset without a distance"),
        pytest.param('{"outline":{"width":0}}', id="outline width zero"),
        pytest.param('{"outline":{"width":1,"cap":"flat"}}', id="outline cap not a word"),
    ],
)
def test_a_document_with_a_wrong_step_is_a_format_error(call):
    text = '{"version":1,"root":[{"path":"M0 0 L4 0 L4 4 L0 4 Z"},' + call + "]}"
    with pytest.raises(btvec.FormatError):
        Drawing.from_json(text)


def test_the_generated_schema_lists_both_steps():
    schema = btvec.drawing_schema()
    for word in ("offset", "outline", "miter_limit"):
        assert f'"{word}"' in schema
