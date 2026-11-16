"""`resample` through the public package: dots spaced evenly along a path, on `Path` and as a step of a `Drawing`."""

import json

import pytest

import bullterrier_vector as btvec

LINE = btvec.Path.line((0, 0), (30, 0))


def test_a_line_resampled_to_twelve_pixels_has_a_corner_every_twelve() -> None:
    assert LINE.resample(12).to_svg_path() == "M0 0 L12 0 L24 0 L30 0"
    (shape,) = btvec.Drawing(LINE).resample(12).shapes()
    assert shape.path.to_svg_path() == "M0 0 L12 0 L24 0 L30 0"


def test_a_closed_path_stays_closed() -> None:
    assert btvec.Path.rect(0, 0, 10, 10).resample(15).to_svg_path() == "M0 0 L10 5 L0 10 Z"


def test_each_shape_reads_its_own_spacing_and_a_param() -> None:
    lines = btvec.Drawing(LINE, LINE.translate(0, 5)).resample(btvec.by_index(10, 30))
    assert [s.path.to_svg_path() for s in lines.shapes()] == ["M0 0 L10 0 L20 0 L30 0", "M0 5 L30 5"]
    gap = btvec.Param("gap", default=10, min=1, max=40)
    (shape,) = btvec.Drawing(LINE).resample(gap).shapes(gap=15)
    assert shape.path.to_svg_path() == "M0 0 L15 0 L30 0"


def test_the_step_is_a_call_in_the_document() -> None:
    steps = json.loads(btvec.Drawing(LINE).resample(12).to_json())["root"]
    assert steps[-1] == {"resample": {"spacing": 12}}


@pytest.mark.parametrize("spacing", [0, -1, float("nan"), float("inf")])
def test_a_spacing_not_above_zero_is_refused_naming_it(spacing: float) -> None:
    with pytest.raises(btvec.ParameterError, match="spacing"):
        LINE.resample(spacing)
    with pytest.raises(btvec.ParameterError, match="spacing"):
        btvec.Drawing(LINE).resample(spacing).shapes()


def test_a_tiny_spacing_is_refused_not_allocated() -> None:
    with pytest.raises(btvec.ParameterError, match="MAX_SEGMENTS"):
        btvec.Path.line((0, 0), (1e6, 0)).resample(1e-3)
