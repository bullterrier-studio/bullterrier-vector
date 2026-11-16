"""`reverse` through the public package: the same outline the other way, on `Path` and as a `Drawing` step."""

import json

import bullterrier_vector as btvec

ELL = btvec.Path.from_svg_path("M0 0 L10 0 L10 10")


def test_a_drawing_reverse_runs_every_shape_the_other_way() -> None:
    (shape,) = btvec.Drawing(ELL).reverse().shapes()
    assert shape.path.to_svg_path() == "M10 10 L10 0 L0 0"
    assert btvec.Drawing(ELL).reverse().reverse().shapes()[0].path == ELL


def test_a_draw_on_the_other_way_round_starts_at_the_far_end() -> None:
    (shape,) = btvec.Drawing(ELL).reverse().slice(0, 0.25).shapes()
    assert shape.path.to_svg_path() == "M10 10 L10 5"


def test_the_step_round_trips_through_a_document() -> None:
    drawing = btvec.Drawing(ELL).reverse()
    assert {"reverse": {}} in json.loads(drawing.to_json())["root"]
    again = btvec.Drawing.from_json(drawing.to_json())
    assert again.shapes()[0].path.to_svg_path() == "M10 10 L10 0 L0 0"
