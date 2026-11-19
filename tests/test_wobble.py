"""`wobble` through the public package: points displaced by smooth seeded noise, on `Path` and as a step of a
`Drawing`. The noise is not recomputed here: its promises (bounds, smoothness, periodic phase) are checked in Rust."""

import json

import pytest

import bullterrier_vector as btvec

LINE = btvec.Path.line((0, 0), (200, 0))


def vertices(path: btvec.Path) -> list[tuple[float, float]]:
    """The corners of a path made of straight lines."""
    numbers = path.to_svg_path().replace("M", " ").replace("L", " ").replace("Z", " ").split()
    return [(float(x), float(y)) for x, y in zip(numbers[::2], numbers[1::2], strict=True)]


def test_the_displacement_stays_within_the_amplitude_and_the_seed_chooses_it() -> None:
    points = vertices(LINE.wobble(amplitude=5, wavelength=50))
    assert len(points) > 20
    assert all(abs(y) <= 5 + 1e-9 and abs(x - round(x / (50 / 24)) * 50 / 24) <= 5 + 1e-9 for x, y in points)
    assert max(abs(y) for _, y in points) > 0.5  # it does move
    assert LINE.wobble(seed=1).to_svg_path() == LINE.wobble(seed=1).to_svg_path()
    assert LINE.wobble(seed=1).to_svg_path() != LINE.wobble(seed=2).to_svg_path()


def test_a_whole_cycle_of_phase_is_the_same_bits() -> None:
    assert LINE.wobble(phase=1.25).to_svg_path() == LINE.wobble(phase=0.25).to_svg_path()
    assert LINE.wobble(phase=0.25).to_svg_path() != LINE.wobble(phase=0).to_svg_path()


def test_the_drawing_step_reads_each_argument_per_shape_and_a_param() -> None:
    lines = btvec.Drawing(LINE, LINE).wobble(amplitude=btvec.by_index(0, 10))
    first, second = (s.path for s in lines.shapes())
    assert first.to_svg_path() == LINE.wobble(amplitude=0).to_svg_path()
    assert second.to_svg_path() == LINE.wobble(amplitude=10).to_svg_path()
    phase = btvec.Param("phase", default=0, min=0, max=2)
    (shape,) = btvec.Drawing(LINE).wobble(phase=phase).shapes(phase=0.25)
    assert shape.path.to_svg_path() == LINE.wobble(phase=0.25).to_svg_path()


def test_the_step_is_a_call_in_the_document_and_the_defaults_are_the_documented_ones() -> None:
    steps = json.loads(btvec.Drawing(LINE).wobble(seed=3).to_json())["root"]
    assert steps[-1] == {"wobble": {"seed": 3}}
    loaded = btvec.Drawing.from_json('{"version":1,"root":[{"path":"M0 0 L200 0"},{"wobble":{}}]}')
    (shape,) = loaded.shapes()
    assert shape.path.to_svg_path() == LINE.wobble().to_svg_path()


@pytest.mark.parametrize("wavelength", [0, -1, float("nan"), float("inf"), 1.1e100])
def test_a_wavelength_not_above_zero_is_refused_naming_it(wavelength: float) -> None:
    with pytest.raises(btvec.ParameterError, match="wavelength"):
        LINE.wobble(wavelength=wavelength)
    with pytest.raises(btvec.ParameterError, match="wavelength"):
        btvec.Drawing(LINE).wobble(wavelength=wavelength).shapes()


@pytest.mark.parametrize("name", ["amplitude", "phase", "seed"])
def test_a_number_the_verb_cannot_use_is_refused_naming_it(name: str) -> None:
    with pytest.raises(btvec.ParameterError, match=name):
        LINE.wobble(**{name: float("nan")})
    with pytest.raises(btvec.ParameterError, match="seed"):
        LINE.wobble(seed=0.5)


def test_a_tiny_wavelength_is_refused_not_allocated() -> None:
    with pytest.raises(btvec.ParameterError, match="MAX_SEGMENTS"):
        btvec.Path.line((0, 0), (1000, 0)).wobble(wavelength=1e-9)


def test_a_wobble_left_unset_is_4_high_80_long_at_phase_0_and_seed_0_in_a_path_a_step_and_a_document() -> None:
    word = btvec.Path.rect(0, 0, 300, 120)
    written = word.wobble(amplitude=4, wavelength=80, phase=0, seed=0).to_svg_path()
    assert word.wobble().to_svg_path() == written
    assert btvec.Drawing(word).wobble().shapes()[0].path.to_svg_path() == written
    document = {"version": 1, "root": [{"path": word.to_svg_path()}, {"wobble": {}}]}
    assert btvec.Drawing.from_json(json.dumps(document)).shapes()[0].path.to_svg_path() == written
