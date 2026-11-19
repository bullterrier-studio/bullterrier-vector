"""`Field.noise` and arithmetic on fields through the public package: the operators, `min`, `max`, `abs`, numbers and
Params as operands, and the errors. Expected values are the operands' own readings combined here."""

import pytest

import bullterrier_vector as btvec

NOISE = btvec.Field.noise(wavelength=40, seed=3, octaves=3)
RAMP = btvec.Field.ramp((0, 0), (100, 0))
TWO = btvec.Param("two", default=2, min=0, max=4)
POINTS = [(0.0, 0.0), (13.5, -7.25), (71.0, 40.0), (250.0, 3.0)]


@pytest.mark.parametrize(
    ("made", "want"),
    [
        pytest.param(lambda: NOISE + RAMP, lambda a, b: a + b, id="field + field"),
        pytest.param(lambda: NOISE + 2, lambda a, b: a + 2, id="field + number"),
        pytest.param(lambda: 2 + NOISE, lambda a, b: 2 + a, id="number + field"),
        pytest.param(lambda: NOISE - RAMP, lambda a, b: a - b, id="field - field"),
        pytest.param(lambda: 5 - NOISE, lambda a, b: 5 - a, id="number - field"),
        pytest.param(lambda: NOISE * RAMP, lambda a, b: a * b, id="field * field"),
        pytest.param(lambda: 14 * NOISE, lambda a, b: 14 * a, id="number * field"),
        pytest.param(lambda: TWO * NOISE, lambda a, b: 2 * a, id="Param * field"),
        pytest.param(lambda: NOISE + TWO * 3, lambda a, b: a + 6, id="field + expression"),
        pytest.param(lambda: -NOISE, lambda a, b: -a, id="negated"),
        pytest.param(lambda: abs(NOISE), lambda a, b: abs(a), id="abs"),
        pytest.param(lambda: NOISE.min(RAMP), lambda a, b: min(a, b), id="min"),
        pytest.param(lambda: NOISE.max(0.25), lambda a, b: max(a, 0.25), id="max of a number"),
    ],
)
def test_an_operation_reads_as_its_operands_combined(made: object, want: object) -> None:
    field = made()  # type: ignore[operator]
    for x, y in POINTS:
        a, b = NOISE.at(x, y), RAMP.at(x, y)
        assert field.at(x, y) == pytest.approx(want(a, b), abs=1e-12)  # type: ignore[operator]


def test_noise_stays_within_one_and_one_cycle_of_phase_is_the_same_field() -> None:
    for octaves in range(1, 9):
        field = btvec.Field.noise(wavelength=17, phase=0.25, seed=-4, octaves=octaves)
        again = btvec.Field.noise(wavelength=17, phase=1.25, seed=-4, octaves=octaves)
        for i in range(50):
            x, y = i * 7.3, i * -3.1
            assert -1 <= field.at(x, y) <= 1
            assert field.at(x, y) == again.at(x, y)
    assert btvec.Field.noise(wavelength=40, seed=3) != btvec.Field.noise(wavelength=40, seed=4)


def test_a_param_in_an_operand_is_read_when_the_step_draws() -> None:
    depth = btvec.Param("depth", default=14, min=0, max=40)
    edge = btvec.Field.distance(btvec.Path.circle((100, 60), 40))
    tone = abs(edge + NOISE * depth).min(30) * (1 / 30)
    drawing = btvec.Drawing(btvec.Path.rect(0, 0, 200, 120)).hatch((3, 9), tone=tone)
    assert [param.name for param in drawing.parameters()] == ["depth"]
    shallow, deep = drawing.shapes(depth=0), drawing.shapes(depth=40)
    assert shallow[0].path.to_svg_path() != deep[0].path.to_svg_path()
    back = btvec.Drawing.from_json(drawing.to_json())
    assert back.to_json() == drawing.to_json()


def test_a_number_operand_is_written_first() -> None:
    assert repr(NOISE * 14) == "14 * Field.noise(wavelength=40, phase=0, seed=3, octaves=3)"


def test_field_noise_takes_the_arguments_of_flow_noise_and_the_same_default_wavelength() -> None:
    with pytest.raises(TypeError):
        btvec.Field.noise(40)  # type: ignore[call-arg]  # positional on purpose: refused
    # the default wavelength is Flow.noise's: the two read one kernel, the flow at 180 degrees times the field
    field, flow = btvec.Field.noise(), btvec.Flow.noise()
    for x, y in POINTS:
        assert flow.at(x, y) == pytest.approx((field.at(x, y) * 180) % 180, abs=1e-9)


@pytest.mark.parametrize(
    ("make", "name"),
    [
        pytest.param(lambda: btvec.Field.noise(wavelength=0), "wavelength", id="a wavelength of 0"),
        pytest.param(lambda: btvec.Field.noise(wavelength=1e-300), "wavelength", id="a wavelength that overflows"),
        pytest.param(lambda: btvec.Field.noise(octaves=0), "octaves", id="no octave"),
        pytest.param(lambda: btvec.Field.noise(octaves=9), "octaves", id="nine octaves"),
        pytest.param(lambda: btvec.Field.noise(octaves=10**30), "octaves", id="far too many octaves"),
        pytest.param(
            lambda: btvec.Field.noise(octaves=2.5),  # type: ignore[arg-type]  # a float on purpose
            "octaves",
            id="octaves with a fraction",
        ),
        pytest.param(lambda: btvec.Field.noise(octaves=-1), "octaves", id="negative octaves"),
        pytest.param(lambda: btvec.Field.noise(seed=0.5), "seed", id="a seed with a fraction"),
        pytest.param(lambda: NOISE + btvec.by_index(0, 1), "value", id="a per-item number"),
    ],
)
def test_a_bad_argument_is_a_parameter_error_naming_it(make: object, name: str) -> None:
    with pytest.raises(btvec.ParameterError, match=name):
        make()  # type: ignore[operator]


@pytest.mark.parametrize(
    "make",
    [
        pytest.param(lambda: btvec.Field.noise(octaves=True), id="octaves"),
        pytest.param(
            lambda: btvec.Drawing.trace_labels(btvec.Field.map(2, 1, b"\x00\xff"), max_colors=True), id="max_colors"
        ),
    ],
)
def test_a_whole_number_argument_refuses_a_bool(make: object) -> None:
    with pytest.raises(TypeError, match="expected a whole number"):
        make()  # type: ignore[operator]


def test_a_tree_deeper_than_sixteen_is_refused() -> None:
    tree = NOISE
    for _ in range(16):
        tree = tree + NOISE
    with pytest.raises(btvec.ParameterError, match="other"):
        tree + NOISE


def test_an_operand_that_is_no_field_or_number_is_a_type_error() -> None:
    with pytest.raises(TypeError):
        NOISE + "1"  # type: ignore[operator]
