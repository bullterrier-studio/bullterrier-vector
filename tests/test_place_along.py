"""`place_along` through the public package: copies of a drawing stamped at even stops along a path, turned to its
direction, not bent. Expected places are by hand (a straight line, a circle's quarter points)."""

import json

import pytest

import bullterrier_vector as btvec

LINE = btvec.Path.line((0, 0), (100, 0))
RING = btvec.Path.circle((0, 0), 100)
BEAD = btvec.Drawing(btvec.Path.rect(-1, -1, 2, 2))


def middles(drawing: btvec.Drawing, **values: float) -> list[float]:
    """The x and y of every shape's middle, flattened (pytest.approx takes no nested rows)."""
    out: list[float] = []
    for shape in drawing.shapes(**values):
        box = shape.path.bounds
        out += [box.x + box.width / 2, box.y + box.height / 2]
    return out


def test_a_count_on_an_open_path_includes_both_ends_and_a_spacing_stops_at_the_end() -> None:
    assert middles(BEAD.place_along(LINE, count=3)) == pytest.approx([0, 0, 50, 0, 100, 0])
    assert middles(BEAD.place_along(LINE, spacing=40)) == pytest.approx([0, 0, 40, 0, 80, 0])
    assert middles(BEAD.place_along(LINE, spacing=50)) == pytest.approx([0, 0, 50, 0, 100, 0])


def test_a_closed_path_has_no_copy_at_its_end_and_start_wraps() -> None:
    quarter = [0, -100, 100, 0, 0, 100, -100, 0]
    assert middles(BEAD.place_along(RING, count=4)) == pytest.approx(quarter, abs=0.05)
    moved = middles(BEAD.place_along(RING, count=4, start=1.25))
    assert moved == pytest.approx([100, 0, 0, 100, -100, 0, 0, -100], abs=0.05)


def test_start_is_a_param_and_the_copies_stay_the_same_number() -> None:
    march = btvec.Param("march", default=0, min=0, max=1)
    drawing = BEAD.place_along(RING, count=4, start=march)
    assert middles(drawing, march=0.25) == pytest.approx([100, 0, 0, 100, -100, 0, 0, -100], abs=0.05)
    assert len(drawing.shapes(march=0.7)) == 4


def test_copies_turn_to_the_path_unless_turn_is_false() -> None:
    bar = btvec.Drawing(btvec.Path.rect(-10, -1, 20, 2))
    down = btvec.Path.line((5, 0), (5, 90))
    for shape in bar.place_along(down, count=3).shapes():
        assert (shape.path.bounds.width, shape.path.bounds.height) == pytest.approx((2, 20))
    for shape in bar.place_along(down, count=3, turn=False).shapes():
        assert (shape.path.bounds.width, shape.path.bounds.height) == pytest.approx((20, 2))


def test_a_stamp_of_two_shapes_keeps_its_layout_and_each_shape_numbers_its_copy() -> None:
    arrow = btvec.Drawing(btvec.Path.rect(0, -1, 30, 2), btvec.Path.rect(30, -5, 10, 10))
    shapes = arrow.place_along(LINE, spacing=50).shapes()
    assert [shape.attributes["copy"] for shape in shapes] == [0, 0, 1, 1, 2, 2]
    centres = middles(arrow.place_along(LINE, spacing=50))
    # the arrow's middle (20, 0) goes to 0, 50 and 100: the shaft is 5 left of it, the head 15 right
    assert centres == pytest.approx([-5, 0, 15, 0, 45, 0, 65, 0, 95, 0, 115, 0])


def test_a_lone_shape_gets_the_order_of_its_copy() -> None:
    orders = [shape.attributes["order"] for shape in BEAD.place_along(LINE, count=5).shapes()]
    assert orders == [0, 0.25, 0.5, 0.75, 1]


def test_the_step_is_a_call_in_the_document_and_loads_back() -> None:
    drawing = BEAD.place_along(RING, spacing=30, start=0.5, turn=False)
    steps = json.loads(drawing.to_json())["root"]
    assert steps[-1]["place_along"] == {"path": RING.to_svg_path(), "spacing": 30, "start": 0.5, "turn": "none"}
    again = btvec.Drawing.from_json(drawing.to_json())
    assert again.shapes() == drawing.shapes()


@pytest.mark.parametrize(
    ("arguments", "name"),
    [
        ({}, "count"),
        ({"count": 2, "spacing": 5}, "count"),
        ({"count": 0}, "count"),
        ({"count": -3}, "count"),
        ({"spacing": 0}, "spacing"),
        ({"spacing": float("nan")}, "spacing"),
        ({"spacing": 1.000_000_1e100}, "spacing"),
    ],
)
def test_a_count_or_spacing_that_cannot_place_is_refused_naming_it(arguments: dict[str, float], name: str) -> None:
    with pytest.raises(btvec.ParameterError, match=name):
        BEAD.place_along(LINE, **arguments).shapes()  # type: ignore[arg-type]


def test_an_empty_path_is_a_geometry_error_and_too_many_copies_are_refused_not_allocated() -> None:
    with pytest.raises(btvec.GeometryError):
        BEAD.place_along(btvec.Path(), count=2).shapes()
    with pytest.raises(btvec.ParameterError, match="MAX_"):
        BEAD.place_along(btvec.Path.line((0, 0), (1e6, 0)), spacing=1e-3).shapes()
