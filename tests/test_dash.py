"""`dash` through the public package: marching ants and stitching, on `Path` and as a step of a `Drawing`."""

import json

import pytest

import bullterrier_vector as btvec

LINE = btvec.Path.line((0, 0), (64, 0))


def test_a_line_is_cut_into_its_dashes_and_the_phase_moves_them_forward() -> None:
    assert LINE.dash([4, 12]).to_svg_path() == "M0 0 L4 0 M16 0 L20 0 M32 0 L36 0 M48 0 L52 0"
    assert LINE.dash([4, 12], phase=0.25).to_svg_path() == "M4 0 L8 0 M20 0 L24 0 M36 0 L40 0 M52 0 L56 0"
    assert LINE.dash([4, 12], phase=1) == LINE.dash([4, 12])


def test_an_odd_pattern_repeats_itself_and_near_picks_the_start_of_a_closed_path() -> None:
    assert LINE.dash([8]) == LINE.dash([8, 8])
    square = btvec.Path.from_svg_path("M0 0 L10 0 L10 10 L0 10 Z")
    assert square.dash([15, 5], near=(10, 10)).to_svg_path() == "M10 10 L0 10 L0 5 M0 0 L10 0 L10 5"


def test_marching_ants_are_a_phase_param_on_a_drawing() -> None:
    march = btvec.Param("march", default=0, min=0, max=1, unit="cycle")
    ants = btvec.Drawing(LINE).dash([4, 12], phase=march)
    (first,) = ants.shapes(march=0)
    (moved,) = ants.shapes(march=0.25)
    assert first.path.to_svg_path() == LINE.dash([4, 12]).to_svg_path()
    assert moved.path.to_svg_path() == LINE.dash([4, 12], phase=0.25).to_svg_path()
    (every,) = ants.shapes(march=1)
    assert every.path == first.path


def test_a_pattern_entry_may_be_a_param_and_the_step_is_a_call_in_the_document() -> None:
    on = btvec.Param("on", default=4, min=1, max=16)
    (shape,) = btvec.Drawing(LINE).dash([on, 12]).shapes(on=8)
    assert shape.path.to_svg_path() == LINE.dash([8, 12]).to_svg_path()
    steps = json.loads(btvec.Drawing(LINE).dash([4, 12], phase=0.5).to_json())["root"]
    assert steps[-1] == {"dash": {"pattern": [4, 12], "phase": 0.5}}
    again = btvec.Drawing.from_json(btvec.Drawing(LINE).dash([4, 12], phase=0.5).to_json())
    assert again.shapes()[0].path == LINE.dash([4, 12], phase=0.5)


@pytest.mark.parametrize("pattern", [[], [4, 0], [-1], [float("nan")], [float("inf")]])
def test_a_pattern_that_is_empty_or_has_a_length_not_above_zero_is_refused_naming_it(pattern: list[float]) -> None:
    with pytest.raises(btvec.ParameterError, match="pattern"):
        LINE.dash(pattern)
    with pytest.raises(btvec.ParameterError, match="pattern"):
        btvec.Drawing(LINE).dash(pattern).shapes()


def test_a_tiny_pattern_is_refused_not_allocated() -> None:
    with pytest.raises(btvec.ParameterError, match="MAX_SEGMENTS"):
        btvec.Path.line((0, 0), (1e6, 0)).dash([1e-3, 1e-3])
