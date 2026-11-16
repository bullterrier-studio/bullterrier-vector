"""`wave` through the public package: points displaced sideways by a sine, on `Path` and as a step of a `Drawing`.
Expected values are by hand: a point at distance `d` along the direction of travel moves
`amplitude * sin(2 pi (d / wavelength - phase))` sideways (angle 0: down the screen)."""

import json

import pytest

import bullterrier_vector as btvec

LINE = btvec.Path.line((0, 0), (120, 0))


def vertices(path: btvec.Path) -> list[tuple[float, float]]:
    """The corners of a path made of straight lines."""
    numbers = path.to_svg_path().replace("M", " ").replace("L", " ").replace("Z", " ").split()
    return [(float(x), float(y)) for x, y in zip(numbers[::2], numbers[1::2], strict=True)]


def test_a_line_waved_is_a_sine_by_hand() -> None:
    points = vertices(LINE.wave())  # amplitude 8, wavelength 120: pieces of 5
    assert len(points) == 25
    at = dict(points)
    assert at[30.0] == pytest.approx(8, abs=1e-9)
    assert at[90.0] == pytest.approx(-8, abs=1e-9)
    assert at[60.0] == pytest.approx(0, abs=1e-9)


def test_the_angle_turns_the_travel_and_the_phase_moves_it() -> None:
    down = btvec.Path.line((0, 0), (0, 120)).wave(angle=90)
    assert {y: x for x, y in vertices(down)}[30.0] == pytest.approx(-8, abs=1e-9)
    shifted = vertices(btvec.Path.line((0, 0), (120, 0)).wave(angle=90, phase=0.25))
    assert all(x == pytest.approx(i * 5 + 8, abs=1e-9) and y == 0 for i, (x, y) in enumerate(shifted))


def test_a_whole_cycle_of_phase_is_the_same_bits() -> None:
    circle = btvec.Path.circle((60, 60), 40)
    assert circle.wave(phase=1.25, angle=30).to_svg_path() == circle.wave(phase=0.25, angle=30).to_svg_path()


def test_a_closed_path_stays_closed_and_its_corners_stay() -> None:
    waved = btvec.Path.rect(0, 0, 100, 60).wave(amplitude=6, wavelength=70)
    assert waved.to_svg_path().endswith("Z")
    assert (0.0, 0.0) in vertices(waved)  # the corner at x = y = 0 is at distance 0: no displacement


def test_the_drawing_step_reads_each_argument_per_shape_and_a_param() -> None:
    lines = btvec.Drawing(LINE, LINE).wave(amplitude=btvec.by_index(8, 16))
    first, second = (dict(vertices(s.path)) for s in lines.shapes())
    assert first[30.0] == pytest.approx(8, abs=1e-9)
    assert second[30.0] == pytest.approx(16, abs=1e-9)
    phase = btvec.Param("phase", default=0, min=0, max=2)
    (shape,) = btvec.Drawing(LINE).wave(phase=phase).shapes(phase=0.25)
    assert vertices(shape.path)[0] == pytest.approx((0, -8), abs=1e-9)


def test_the_step_is_a_call_in_the_document_and_the_defaults_are_the_documented_ones() -> None:
    steps = json.loads(btvec.Drawing(LINE).wave(angle=30).to_json())["root"]
    assert steps[-1] == {"wave": {"angle": 30}}  # what is left at its default is not written
    loaded = btvec.Drawing.from_json('{"version":1,"root":[{"path":"M0 0 L120 0"},{"wave":{}}]}')
    (shape,) = loaded.shapes()
    assert dict(vertices(shape.path))[30.0] == pytest.approx(8, abs=1e-9)


@pytest.mark.parametrize("wavelength", [0, -1, float("nan"), float("inf"), 1.1e100])
def test_a_wavelength_not_above_zero_is_refused_naming_it(wavelength: float) -> None:
    with pytest.raises(btvec.ParameterError, match="wavelength"):
        LINE.wave(wavelength=wavelength)
    with pytest.raises(btvec.ParameterError, match="wavelength"):
        btvec.Drawing(LINE).wave(wavelength=wavelength).shapes()


@pytest.mark.parametrize("name", ["amplitude", "angle", "phase"])
def test_a_number_that_is_not_finite_is_refused_naming_it(name: str) -> None:
    with pytest.raises(btvec.ParameterError, match=name):
        LINE.wave(**{name: float("nan")})


def test_a_tiny_wavelength_is_refused_not_allocated() -> None:
    with pytest.raises(btvec.ParameterError, match="MAX_SEGMENTS"):
        btvec.Path.line((0, 0), (1000, 0)).wave(wavelength=1e-9)
