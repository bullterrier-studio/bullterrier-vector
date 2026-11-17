"""`hatch`, `Flow` and `Field` through the public package. Expected values are by hand or measured on the output:
the gap of parallel lines against the spacing asked, the analytic distance to a circle, the doubled-angle mean."""

import math
from itertools import pairwise

import pytest

import bullterrier_vector as btvec

BOX = btvec.Path.rect(0, 0, 200, 100)
CIRCLE = btvec.Path.circle((100, 100), 50)


def lines(path: btvec.Path) -> list[list[tuple[float, float]]]:
    """The sub-paths of a path made of straight lines, each as its vertices."""
    found: list[list[tuple[float, float]]] = []
    for word in path.to_svg_path().replace("M", "|M").replace("Z", "").split("|"):
        numbers = word.replace("M", " ").replace("L", " ").split()
        if numbers:
            found.append([(float(x), float(y)) for x, y in zip(numbers[::2], numbers[1::2], strict=True)])
    return found


def apart(a: float, b: float) -> float:
    """The distance between two directions without sign, in degrees."""
    d = (a - b) % 180
    return min(d, 180 - d)


def test_parallel_lines_are_one_spacing_apart() -> None:
    heights = sorted(line[0][1] for line in lines(BOX.hatch(10, flow=btvec.Flow.angle(0))))
    assert len(heights) >= 9
    assert all(abs(b - a - 10) < 1e-6 for a, b in pairwise(heights))


def test_a_drawing_hatches_each_shape_and_keeps_its_style() -> None:
    drawing = btvec.Drawing(BOX).fill("#ff0000").hatch(10, flow=btvec.Flow.angle(0))
    (shape,) = drawing.shapes()
    assert shape.path.to_svg_path() == BOX.hatch(10, flow=btvec.Flow.angle(0)).to_svg_path()
    assert shape.style.fill == "#ff0000ff"


def test_the_default_flow_is_45_degrees_and_a_number_is_the_spacing_everywhere() -> None:
    (line, *_) = lines(BOX.hatch(20))
    (x0, y0), (x1, y1) = line[0], line[-1]
    assert apart(math.degrees(math.atan2(y1 - y0, x1 - x0)), 45) < 0.5


def test_avoid_keeps_lines_out_of_a_path_a_shape_or_a_drawing() -> None:
    inside = btvec.Path.circle((100, 50), 30)
    for avoid in (inside, btvec.Drawing(inside)):
        result = btvec.Drawing(BOX).hatch(8, flow=btvec.Flow.angle(10), avoid=avoid)
        for shape in result.shapes():
            for line in lines(shape.path):
                assert not any(inside.contains(p) for p in line)


def test_a_tone_field_sets_the_spacing_between_the_pair() -> None:
    tone = btvec.Field.ramp((0, 0), (200, 0))
    xs = sorted(line[0][0] for line in lines(BOX.hatch((5, 15), tone=tone, flow=btvec.Flow.angle(90))))
    gaps = [b - a for a, b in pairwise(xs)]
    assert gaps[0] < 7
    assert gaps[-1] > 13


def test_a_pair_of_spacings_needs_a_tone_and_a_spacing_must_be_above_zero() -> None:
    with pytest.raises(btvec.ParameterError, match="tone"):
        BOX.hatch((5, 15))
    with pytest.raises(btvec.ParameterError, match="spacing"):
        BOX.hatch(0)
    with pytest.raises(TypeError):
        BOX.hatch("wide")  # type: ignore[arg-type]


def test_the_distance_to_a_circle_is_the_analytic_one() -> None:
    field = btvec.Field.distance(CIRCLE)
    for x, y in [(100, 100), (160, 100), (10, 10)]:
        assert field.at(x, y) == pytest.approx(math.hypot(x - 100, y - 100) - 50, abs=0.05)
    assert btvec.Field.distance(CIRCLE, normalized=True).at(100, 100) == pytest.approx(1, abs=0.02)
    with pytest.raises(btvec.GeometryError):
        btvec.Field.distance(btvec.Path())


def test_a_direction_and_its_opposite_mix_to_the_same_direction() -> None:
    forward = btvec.Flow.contour(CIRCLE)
    backward = btvec.Flow.contour(CIRCLE.reverse())
    mixed = forward.mix(backward, 0.5)
    for x, y in [(143.7, 101.3), (97.1, 142.9), (62.3, 71.7)]:
        assert apart(mixed.at(x, y), forward.at(x, y)) < 1e-6
    mean = btvec.Flow.angle(10).mix(btvec.Flow.angle(170), 0.5)
    assert apart(mean.at(0, 0), 0) < 1e-6
    with pytest.raises(btvec.ParameterError):
        forward.mix(backward, 2)
    with pytest.raises(btvec.ParameterError):
        btvec.Flow.noise(wavelength=0)


def test_a_hatched_drawing_saves_to_json_and_loads_back_to_the_same_lines() -> None:
    flow = btvec.Flow.contour(CIRCLE).mix(btvec.Flow.noise(wavelength=90, seed=4), 0.4)
    drawing = btvec.Drawing(BOX).hatch((6, 12), tone=btvec.Field.distance(CIRCLE, normalized=True), flow=flow)
    text = drawing.to_json()
    again = btvec.Drawing.from_json(text)
    assert again.to_json() == text
    assert [s.path.to_svg_path() for s in again.shapes()] == [s.path.to_svg_path() for s in drawing.shapes()]


@pytest.mark.parametrize("points", [[(0, 0), (100, 100)], [(0, 0), (100, 100), (0, 0)]])
def test_a_path_of_two_points_encloses_nothing(points: list[tuple[int, int]]) -> None:
    line = btvec.Path.polyline(points)
    assert line.hatch(50).to_svg_path() == ""
    assert btvec.Field.distance(line).at(10, 50) > 0


def test_the_angle_is_named_angle_as_in_the_document() -> None:
    assert btvec.Flow.angle(angle=45) == btvec.Flow.angle(45)


def test_a_field_and_a_flow_print_as_their_call_and_equal_when_they_are_the_same_call() -> None:
    assert repr(btvec.Flow.angle(45)) == "Flow.angle(45)"
    assert repr(btvec.Field.ramp((0, 0), (200, 0))) == "Field.ramp((0, 0), (200, 0))"
    mixed = btvec.Flow.angle(0).mix(btvec.Flow.noise(wavelength=90, seed=4), 0.5)
    assert repr(mixed) == "Flow.angle(0).mix(Flow.noise(wavelength=90, phase=0, seed=4), 0.5)"
    assert btvec.Field.distance(CIRCLE) == btvec.Field.distance(CIRCLE)
    assert btvec.Field.distance(CIRCLE) != btvec.Field.distance(CIRCLE, normalized=True)
    assert btvec.Flow.angle(45) != btvec.Flow.angle(46)
    assert btvec.Flow.angle(45) != btvec.Field.ramp((0, 0), (1, 1))
