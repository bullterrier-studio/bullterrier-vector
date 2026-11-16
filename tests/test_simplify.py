"""`simplify` through the public package: a dense polyline as a few smooth curves, on `Path` and a `Drawing`."""

import json

import pytest

import bullterrier_vector as btvec

DENSE = btvec.Path.polyline([(i * 2, 0) for i in range(50)])
BUMP = btvec.Path.polyline([(0, 0), (10, 1), (20, 0)])


def test_a_dense_polyline_simplified_is_one_line() -> None:
    assert DENSE.simplify(tolerance=0.5).to_svg_path() == "M0 0 L98 0"
    (shape,) = btvec.Drawing(DENSE).simplify(tolerance=0.5).shapes()
    assert shape.path.to_svg_path() == "M0 0 L98 0"


def test_the_tolerance_defaults_to_half_a_pixel_and_is_keyword_only() -> None:
    assert DENSE.simplify().to_svg_path() == "M0 0 L98 0"
    assert btvec.Drawing(DENSE).simplify().shapes()[0].path.to_svg_path() == "M0 0 L98 0"
    with pytest.raises(TypeError):
        DENSE.simplify(0.5)  # type: ignore[misc]


def test_sharp_corners_are_kept_and_a_smooth_run_becomes_a_curve() -> None:
    square = btvec.Path.polyline([(x, 0) for x in range(0, 100, 4)] + [(100, y) for y in range(0, 101, 4)])
    assert square.simplify().to_svg_path() == "M0 0 L100 0 L100 100"
    assert "C" in BUMP.simplify(tolerance=0.25).to_svg_path()
    assert BUMP.simplify(tolerance=2).to_svg_path() == "M0 0 L20 0"


def test_each_shape_reads_its_own_tolerance_and_a_param() -> None:
    drawing = btvec.Drawing(BUMP, BUMP).simplify(tolerance=btvec.by_index(2, 0.25))
    found = [s.path.to_svg_path() for s in drawing.shapes()]
    assert found[0] == "M0 0 L20 0"
    assert found[1].startswith("M0 0 C")
    wide = btvec.Param("wide", default=0.25, min=0.1, max=5)
    (shape,) = btvec.Drawing(BUMP).simplify(tolerance=wide).shapes(wide=2)
    assert shape.path.to_svg_path() == "M0 0 L20 0"


def test_the_step_is_a_call_in_the_document() -> None:
    steps = json.loads(btvec.Drawing(DENSE).simplify(tolerance=2).to_json())["root"]
    assert steps[-1] == {"simplify": {"tolerance": 2}}


@pytest.mark.parametrize("tolerance", [0, -1, float("nan"), float("inf"), 1.0000001e100])
def test_a_tolerance_not_above_zero_is_refused_naming_it(tolerance: float) -> None:
    with pytest.raises(btvec.ParameterError, match="tolerance"):
        DENSE.simplify(tolerance=tolerance)
    with pytest.raises(btvec.ParameterError, match="tolerance"):
        btvec.Drawing(DENSE).simplify(tolerance=tolerance).shapes()
