"""`twist` and `bulge` through the public package, on `Path` and as steps of a `Drawing`. Expected values are by hand: a
point at distance `r` from the centre turns by `angle * f(r)` degrees (twist, clockwise on the screen as `rotate`) or
goes to `r * (1 + amount * f(r))` along its ray (bulge), `f = 1 - s^2 (3 - 2 s)`, `s = min(r / radius, 1)`."""

import inspect
import json
import math

import pytest

import bullterrier_vector as btvec

ORIGIN = (0, 0)
FRAUNCES = btvec.Font.bundled("fraunces")


def vertices(path: btvec.Path) -> list[tuple[float, float]]:
    """The corners of a path made of straight lines."""
    numbers = path.to_svg_path().replace("M", " ").replace("L", " ").replace("Z", " ").split()
    return [(float(x), float(y)) for x, y in zip(numbers[::2], numbers[1::2], strict=True)]


def test_a_point_at_half_the_radius_turns_by_half_the_angle_by_hand() -> None:
    # s = 0.5, falloff 0.5: 90 degrees turn 45 degrees, clockwise on the screen (y down)
    (x, y), *_ = vertices(btvec.Path.from_svg_path("M50 0").twist(90, radius=100, about=ORIGIN))
    assert (x, y) == pytest.approx((50 / math.sqrt(2),) * 2, abs=1e-9)


def test_a_point_near_the_centre_turns_as_rotate_turns() -> None:
    point = btvec.Path.from_svg_path("M1 0")
    twisted = vertices(point.twist(90, radius=1e6, about=ORIGIN))
    assert twisted[0] == pytest.approx(vertices(point.rotate(90, about=ORIGIN))[0], abs=1e-9)
    assert twisted[0] == pytest.approx((0, 1), abs=1e-9)


def test_the_defaults_are_a_quarter_turn_and_a_half_bulge_about_the_middle_to_the_far_corner() -> None:
    rect = btvec.Path.rect(10, 20, 100, 60)
    assert rect.twist().to_svg_path() == rect.twist(90, radius=math.hypot(50, 30), about=(60, 50)).to_svg_path()
    assert rect.bulge().to_svg_path() == rect.bulge(0.5, radius=math.hypot(50, 30), about=(60, 50)).to_svg_path()
    assert (10.0, 20.0) in vertices(rect.twist())  # the far corner stays


def test_a_bulge_moves_a_point_along_its_ray_by_hand() -> None:
    # (30, 40) is 50 from the centre, half of a radius of 100: falloff 0.5, so amount 1 goes to 1.5 times as far
    ((x, y),) = vertices(btvec.Path.from_svg_path("M30 40").bulge(1, radius=100, about=ORIGIN))
    assert (x, y) == pytest.approx((45, 60), abs=1e-9)
    ((x, y),) = vertices(btvec.Path.from_svg_path("M50 0").bulge(-0.5, radius=100, about=ORIGIN))
    assert (x, y) == pytest.approx((37.5, 0), abs=1e-9)


def test_a_point_at_the_radius_does_not_move_and_a_closed_path_stays_closed() -> None:
    path = btvec.Path.from_svg_path("M0 0 L100 0 L130 40 Z")
    for moved in (path.twist(120, radius=100, about=ORIGIN), path.bulge(1, radius=100, about=ORIGIN)):
        assert moved.to_svg_path().endswith("Z")
        assert (100.0, 0.0) in vertices(moved)
        assert (130.0, 40.0) in vertices(moved)


def test_the_drawing_steps_read_each_argument_per_shape_and_a_param() -> None:
    squares = btvec.Drawing(btvec.Path.rect(0, 0, 100, 100), btvec.Path.rect(300, 0, 100, 100))
    first, second = (s.path.to_svg_path() for s in squares.twist(btvec.by_index(30, 90)).shapes())
    assert first == btvec.Path.rect(0, 0, 100, 100).twist(30).to_svg_path()
    assert second == btvec.Path.rect(300, 0, 100, 100).twist(90).to_svg_path()
    progress = btvec.Param("progress", default=0, min=0, max=1)
    path = btvec.Path.rect(0, 0, 100, 100)
    (half,) = btvec.Drawing(path).twist(progress * 180).shapes(progress=0.5)
    assert half.path.to_svg_path() == path.twist(90).to_svg_path() != path.to_svg_path()
    amount = btvec.Param("amount", default=0, min=-0.9, max=1.4)
    (pinched,) = btvec.Drawing(path).bulge(amount, radius=btvec.by_index(70, 70)).shapes(amount=-0.5)
    assert pinched.path.to_svg_path() == path.bulge(-0.5, radius=70).to_svg_path() != path.to_svg_path()


def test_a_shape_with_an_origin_twists_and_bulges_about_it_not_the_middle_of_its_bounds() -> None:
    (shape,) = btvec.text("o", font=FRAUNCES, size=200).shapes()
    origin = shape.attributes["origin"]
    assert isinstance(origin, btvec.Point)
    middle = shape.path.bounds
    assert abs(middle.y + middle.height / 2 - origin.y) > 1  # the premise
    for verb in ("twist", "bulge"):
        (drawn,) = getattr(btvec.text("o", font=FRAUNCES, size=200), verb)().shapes()
        about_origin = getattr(shape.path, verb)(about=origin).to_svg_path()
        assert drawn.path.to_svg_path() == about_origin != getattr(shape.path, verb)().to_svg_path()


def test_the_steps_are_calls_in_the_document_and_leave_the_defaults_out() -> None:
    line = btvec.Path.line((0, 0), (120, 0))
    steps = json.loads(btvec.Drawing(line).twist(radius=50, about=(10, 20)).bulge(-0.4).to_json())["root"]
    assert steps[-2] == {"twist": {"radius": 50, "about": [10, 20]}}
    assert steps[-1] == {"bulge": {"amount": -0.4}}
    loaded = btvec.Drawing.from_json('{"version":1,"root":[{"path":"M0 0 L120 0"},{"twist":{}},{"bulge":{}}]}')
    assert loaded.shapes()[0].path.to_svg_path() == btvec.Drawing(line).twist().bulge().shapes()[0].path.to_svg_path()


def test_the_signatures_show_the_real_defaults() -> None:
    assert str(inspect.signature(btvec.Drawing.twist)) == "(self, /, angle=90, *, radius=None, about=None)"
    assert str(inspect.signature(btvec.Drawing.bulge)) == "(self, /, amount=0.5, *, radius=None, about=None)"


@pytest.mark.parametrize("amount", [-1, 16 / 11, 2, float("nan"), float("inf")])
def test_an_amount_that_would_fold_the_shape_is_refused_naming_it(amount: float) -> None:
    line = btvec.Path.line((0, 0), (10, 0))
    with pytest.raises(btvec.ParameterError, match="amount"):
        line.bulge(amount)
    with pytest.raises(btvec.ParameterError, match="amount"):
        btvec.Drawing(line).bulge(amount).shapes()


def test_a_bulge_amount_is_refused_in_the_same_words_on_a_path_and_a_drawing() -> None:
    line = btvec.Path.line((0, 0), (10, 0))
    rule = "must be above -1 and below 16/11 (1.4545), so that the shape does not fold over, got 1.5"
    with pytest.raises(btvec.ParameterError) as on_path:
        line.bulge(1.5)
    with pytest.raises(btvec.ParameterError) as on_drawing:
        btvec.Drawing(line).bulge(1.5).shapes()
    assert rule in str(on_path.value)
    assert rule in str(on_drawing.value)


@pytest.mark.parametrize("radius", [0, -1, float("nan"), float("inf"), 1.1e100])
def test_a_radius_not_above_zero_is_refused_naming_it(radius: float) -> None:
    line = btvec.Path.line((0, 0), (10, 0))
    for verb in ("twist", "bulge"):
        with pytest.raises(btvec.ParameterError, match="radius"):
            getattr(line, verb)(radius=radius)
        with pytest.raises(btvec.ParameterError, match="radius"):
            getattr(btvec.Drawing(line), verb)(radius=radius).shapes()


def test_an_angle_that_is_not_finite_is_refused_naming_it() -> None:
    with pytest.raises(btvec.ParameterError, match="angle"):
        btvec.Path.line((0, 0), (10, 0)).twist(float("nan"))


def test_a_tiny_radius_is_refused_not_allocated() -> None:
    line = btvec.Path.line((0, 0), (1000, 0))
    for verb in ("twist", "bulge"):
        with pytest.raises(btvec.ParameterError, match="MAX_SEGMENTS"):
            getattr(line, verb)(radius=1e-9)
