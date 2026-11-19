"""`Drawing.trace_levels`, `trace_labels` and `trace_cells` through the public package. Expected values are worked out
by hand (a ramp's edge, a checkerboard's cells) or counted from the bytes."""

import json
import re

import pytest
from jsonschema import Draft202012Validator

import bullterrier_vector as btvec


def area(path: btvec.Path) -> float:
    """The absolute area a path of straight lines fills (shoelace over its sub-paths, holes against outlines)."""
    total = 0.0
    for word in path.to_svg_path().replace("M", "|M").split("|"):
        numbers = [float(n) for n in re.findall(r"-?\d+(?:\.\d+)?(?:e-?\d+)?", word)]
        points = list(zip(numbers[::2], numbers[1::2], strict=True))
        total += sum(x1 * y2 - x2 * y1 for (x1, y1), (x2, y2) in zip(points, points[1:] + points[:1], strict=True)) / 2
    return abs(total)


def ramp_map() -> btvec.Field:
    return btvec.Field.map(256, 4, bytes(x for _ in range(4) for x in range(256)))


def test_a_level_of_a_ramp_is_the_region_past_its_edge_with_the_level_as_an_attribute() -> None:
    drawing = btvec.Drawing.trace_levels(ramp_map(), [0.5], tolerance=0)
    (shape,) = drawing.shapes()
    assert shape.attributes["level"] == 0.5
    inside = shape.path.intersect(btvec.Path.rect(0, 0, 256, 4))
    assert area(inside) == pytest.approx(128 * 4, abs=1e-6)


def test_levels_are_in_the_order_given_and_a_level_the_map_never_reaches_is_left_out() -> None:
    shapes = btvec.Drawing.trace_levels(ramp_map(), [0.75, 0.25, 7]).shapes()
    assert [shape.attributes["level"] for shape in shapes] == [0.75, 0.25]


def test_a_checkerboard_of_cells_is_traced_exactly_to_rectangles_that_fill_the_map() -> None:
    cells = bytes(((x // 2) + (y // 2)) % 2 for y in range(8) for x in range(8))
    drawing = btvec.Drawing.trace_cells(btvec.Field.map(8, 8, cells), 2)
    shapes = drawing.shapes()
    assert [shape.attributes["label"] for shape in shapes] == [0, 1]
    assert [area(shape.path) for shape in shapes] == [32, 32]
    assert shapes[0].path.to_svg_path().count("M") == 8


def test_labels_make_one_shape_each_and_more_than_max_colors_raises() -> None:
    labels = btvec.Field.map(4, 2, bytes([2, 2, 9, 9, 2, 2, 9, 9]))
    shapes = btvec.Drawing.trace_labels(labels, tolerance=0).shapes()
    assert [shape.attributes["label"] for shape in shapes] == [2, 9]
    with pytest.raises(btvec.ParameterError, match="max_colors"):
        btvec.Drawing.trace_labels(labels, max_colors=1).shapes()


@pytest.mark.parametrize(
    ("call", "argument"),
    [
        (lambda: btvec.Drawing.trace_levels(ramp_map(), [0.5], smooth=-1), "smooth"),
        (lambda: btvec.Drawing.trace_levels(ramp_map(), [], tolerance=0), "levels"),
        (lambda: btvec.Drawing.trace_levels(ramp_map(), [float("nan")]), "levels"),
        (lambda: btvec.Drawing.trace_levels(ramp_map(), [0.5], open=float("nan")), "open"),
        (lambda: btvec.Drawing.trace_cells(ramp_map(), 0), "cell"),
        (lambda: btvec.Drawing.trace_cells(ramp_map(), 3).shapes(), "cell"),
        (lambda: btvec.Drawing.trace_cells(ramp_map(), -1), "cell"),
        (lambda: btvec.Drawing.trace_cells(ramp_map(), 2.5), "cell"),  # type: ignore[arg-type]
        (lambda: btvec.Drawing.trace_cells(ramp_map(), 2, max_colors=-3), "max_colors"),
        (lambda: btvec.Drawing.trace_labels(ramp_map(), max_colors=1.5), "max_colors"),  # type: ignore[arg-type]
        (lambda: btvec.Drawing.trace_levels(btvec.Field.ramp((0, 0), (1, 0)), [0.5]).shapes(), "map"),
    ],
)
def test_a_refused_number_names_its_argument(call: object, argument: str) -> None:
    with pytest.raises(btvec.ParameterError, match=argument):
        call()  # type: ignore[operator]


def test_a_refused_negative_number_reports_the_given_value() -> None:
    with pytest.raises(btvec.ParameterError, match="got -1"):
        btvec.Drawing.trace_cells(ramp_map(), -1)
    with pytest.raises(btvec.ParameterError, match="got -3"):
        btvec.Drawing.trace_cells(ramp_map(), 2, max_colors=-3)


def test_a_trace_is_a_document_that_loads_back_to_the_same_shapes() -> None:
    data = bytes(range(64))
    drawing = btvec.Drawing.trace_levels(btvec.Field.map(8, 8, data, name="photo"), [0.3, 0.6], smooth=0.5, min_area=2)
    text = drawing.to_json()
    back = btvec.Drawing.from_json(text, inputs={"photo": data})
    assert [s.path.to_svg_path() for s in back.shapes()] == [s.path.to_svg_path() for s in drawing.shapes()]
    assert back.to_json() == text


def test_a_whole_number_float_is_the_whole_number_it_reads_as() -> None:
    cells = bytes(((x // 2) + (y // 2)) % 2 for y in range(8) for x in range(8))
    shapes = btvec.Drawing.trace_cells(btvec.Field.map(8, 8, cells), 2.0).shapes()  # type: ignore[arg-type]
    assert [shape.attributes["label"] for shape in shapes] == [0, 1]


def test_a_param_feeds_a_tidying_number() -> None:
    cells = bytes(255 if 4 <= x < 7 and 4 <= y < 7 else 0 for y in range(12) for x in range(12))
    grow = btvec.Param("grow", default=0, min=0, max=3)
    drawing = btvec.Drawing.trace_levels(btvec.Field.map(12, 12, cells), [0.5], tolerance=0, close=grow)
    plain, grown = (drawing.shapes(grow=grow)[0].path.to_svg_path() for grow in (0, 1))
    assert plain != grown


def test_the_generated_schema_accepts_a_trace_document_and_refuses_a_bad_one() -> None:
    validator = Draft202012Validator(json.loads(btvec.drawing_schema()))
    inputs = {"photo": {"kind": "map"}}
    photo = {"Field.map": {"width": 8, "height": 8, "data": "photo"}}
    good = {"version": 1, "inputs": inputs, "root": [{"trace_levels": {"map": photo, "levels": [0.3], "open": 1}}]}
    assert validator.is_valid(good)
    assert validator.is_valid({**good, "root": [{"trace_cells": {"map": photo, "cell": 2, "max_colors": 8}}]})
    assert not validator.is_valid({**good, "root": [{"trace_levels": {"map": photo, "levels": "dark"}}]})
    assert not validator.is_valid({**good, "root": [{"trace_levels": {"map": photo, "levels": [0.3], "smooth": -1}}]})
    assert not validator.is_valid({**good, "root": [{"trace_labels": {"map": photo, "max_colors": 300}}]})
    # a count is a whole number: the schema says so rather than leave the draw to refuse it
    assert not validator.is_valid({**good, "root": [{"trace_cells": {"map": photo, "cell": 1.5}}]})
    assert not validator.is_valid({**good, "root": [{"trace_labels": {"map": photo, "max_colors": 2.5}}]})
    assert validator.is_valid({**good, "root": [{"trace_labels": {"map": photo, "max_colors": 2.0}}]})


def test_labels_cropped_to_the_frame_still_tile_it() -> None:
    labels = btvec.Field.map(8, 8, bytes((x // 3 + y // 2) % 3 for y in range(8) for x in range(8)))
    shapes = btvec.Drawing.trace_labels(labels, smooth=0.6, close=1).intersect(btvec.Path.rect(0, 0, 8, 8)).shapes()
    assert sum(area(shape.path) for shape in shapes) == pytest.approx(64, abs=1e-6)
    for shape in shapes:
        bounds = shape.path.bounds
        assert bounds.x >= -1e-9
        assert bounds.y >= -1e-9
        assert bounds.x + bounds.width <= 8 + 1e-9
        assert bounds.y + bounds.height <= 8 + 1e-9


def test_levels_may_be_params() -> None:
    level = btvec.Param("level", default=0.25, min=0, max=1)
    drawing = btvec.Drawing.trace_levels(ramp_map(), [level, 0.75], tolerance=0)
    assert [s.attributes["level"] for s in drawing.shapes(level=0.5)] == [0.5, 0.75]
    assert [s.attributes["level"] for s in drawing.shapes()] == [0.25, 0.75]
    assert btvec.Drawing.trace_levels(ramp_map(), level).shapes(level=0.5)[0].attributes["level"] == 0.5


def test_any_field_is_traced_over_its_box_and_a_disc_comes_out_at_its_radius() -> None:
    inside = -btvec.Field.distance(btvec.Path.circle((100, 100), 60))  # at least 15 inside the circle of radius 45
    drawing = btvec.Drawing.trace_levels(inside, 15, frame=(0, 0, 200, 200), cell=2, tolerance=0)
    (shape,) = drawing.shapes()
    assert shape.attributes["level"] == 15
    assert area(shape.path) == pytest.approx(3.14159265 * 45**2, rel=0.01)
    back = btvec.Drawing.from_json(drawing.to_json())
    assert back.to_svg(200, 200) == drawing.to_svg(200, 200)


@pytest.mark.parametrize(
    ("given", "name"),
    [
        pytest.param({"cell": 2}, "frame", id="no frame"),
        pytest.param({"frame": (0, 0, 200, 200)}, "cell", id="no cell"),
    ],
)
def test_a_field_that_is_not_a_map_needs_a_box_and_a_cell(given: dict[str, object], name: str) -> None:
    drawing = btvec.Drawing.trace_levels(btvec.Field.ramp((0, 0), (100, 0)), 0.5, **given)  # type: ignore[arg-type]
    with pytest.raises(btvec.ParameterError, match=name):
        drawing.shapes()
