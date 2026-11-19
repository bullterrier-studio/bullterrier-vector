"""`Field.map`, `Field.to_map`, `Field.contours`, Params in a `Field` or a `Flow`, a drawing in `Field.distance` and
`Flow.contour`, and `hatch(below=)` through the public package. Expected values are worked out by hand or from the
analytic circle."""

import math
from array import array

import pytest

import bullterrier_vector as btvec

BYTES = bytes([0, 100, 200, 255, 50, 0])  # 3 by 2
BOX = btvec.Path.rect(0, 0, 200, 120)


def lines(path: btvec.Path) -> list[list[tuple[float, float]]]:
    found: list[list[tuple[float, float]]] = []
    for word in path.to_svg_path().replace("M", "|M").replace("Z", "").split("|"):
        numbers = word.replace("M", " ").replace("L", " ").split()
        if numbers:
            found.append([(float(x), float(y)) for x, y in zip(numbers[::2], numbers[1::2], strict=True)])
    return found


def svg(drawing: btvec.Drawing, **values: float) -> str:
    return "|".join(shape.path.to_svg_path() for shape in drawing.shapes(**values))


@pytest.mark.parametrize(
    "data",
    [BYTES, bytearray(BYTES), memoryview(BYTES), array("B", BYTES), list(BYTES)],
    ids=["bytes", "bytearray", "memoryview", "array", "list"],
)
def test_a_map_reads_its_bytes_from_any_buffer_or_list(data: object) -> None:
    field = btvec.Field.map(3, 2, data)  # type: ignore[arg-type]
    assert field.at(0.5, 0.5) == 0.0
    assert field.at(0.5, 1.5) == 1.0
    assert abs(field.at(1.0, 0.5) - 50 / 255) < 1e-12  # half way between 0 and 100
    assert field.at(-50, 9) == 1.0  # the nearest edge value beyond the frame


def test_a_frame_places_a_map_and_equal_maps_are_equal() -> None:
    placed = btvec.Field.map(3, 2, BYTES, frame=(100, 50, 30, 20))
    assert placed.at(105, 65) == 1.0
    assert btvec.Field.map(3, 2, BYTES) == btvec.Field.map(3, 2, bytearray(BYTES))
    assert btvec.Field.map(3, 2, BYTES) != placed
    assert repr(placed) == "Field.map(3, 2, <6 bytes>, frame=(100, 50, 30, 20))"


@pytest.mark.parametrize(
    ("make", "match"),
    [
        (lambda: btvec.Field.map(3, 2, BYTES[:5]), "data"),
        (lambda: btvec.Field.map(0, 2, b""), "width"),
        (lambda: btvec.Field.map(3, 2, BYTES, frame=(0, 0, 0, 1)), "frame"),
        (lambda: btvec.Field.map(3, 2, BYTES, frame=(math.nan, 0, 1, 1)), "frame"),
        (lambda: btvec.Field.map(3, 2, BYTES, name="not a name"), "name"),
        (lambda: btvec.Field.map(1 << 20, 1 << 20, b""), "width"),
    ],
    ids=["short", "zero", "flat", "nan", "name", "huge"],
)
def test_a_map_that_cannot_be_read_raises_a_parameter_error(make: object, match: str) -> None:
    with pytest.raises(btvec.ParameterError, match=match):
        make()  # type: ignore[operator]


def test_what_is_not_bytes_is_a_type_error() -> None:
    with pytest.raises(TypeError, match="data"):
        btvec.Field.map(3, 2, "abcdef")  # type: ignore[arg-type]


def test_a_distance_field_written_out_reads_back_within_half_a_level() -> None:
    circle = btvec.Path.circle((100, 100), 50)
    distance = btvec.Field.distance(circle)
    box = (20, 20, 160, 160)
    data = distance.to_map(64, 64, frame=box, low=-50, high=30)
    assert isinstance(data, bytes)
    assert len(data) == 64 * 64
    back = btvec.Field.map(64, 64, data, frame=box)
    for row, column in [(0, 0), (10, 5), (31, 31), (40, 50), (63, 63)]:
        x, y = 20 + (column + 0.5) * 2.5, 20 + (row + 0.5) * 2.5
        exact = min(1.0, max(0.0, (math.hypot(x - 100, y - 100) - 50 + 50) / 80))
        assert abs(back.at(x, y) - exact) < 0.5 / 255 + 0.002


def test_contours_of_a_circles_distance_are_circles_and_a_ramp_gives_a_straight_line() -> None:
    distance = btvec.Field.distance(btvec.Path.circle((100, 100), 50))
    found = lines(distance.contours([-20, 15], frame=(0, 0, 200, 200), cell=2))
    assert len(found) == 2
    for line, radius in zip(found, (30, 65), strict=True):
        assert all(abs(math.hypot(x - 100, y - 100) - radius) < 0.05 for x, y in line)
    (line,) = lines(btvec.Field.ramp((0, 0), (100, 0)).contours(0.37, frame=(0, 0, 100, 50), cell=5))
    assert all(abs(x - 37) < 1e-9 for x, _ in line)


def test_contours_refuse_what_they_cannot_trace() -> None:
    ramp = btvec.Field.ramp((0, 0), (100, 0))
    for call in (
        lambda: ramp.contours(math.nan, frame=(0, 0, 10, 10), cell=1),
        lambda: ramp.contours(0.5, frame=(0, 0, 10, 10), cell=0),
        lambda: ramp.contours(0.5, frame=(0, 0, 10, 10), cell=1e-9),
        lambda: ramp.contours([0.5] * 300, frame=(0, 0, 10, 10), cell=1),
    ):
        with pytest.raises(btvec.ParameterError):
            call()


def test_a_param_moves_a_noise_flow_and_one_cycle_on_is_the_same_flow() -> None:
    phase = btvec.Param("phase", default=0, min=0, max=4)
    drawing = btvec.Drawing(BOX).hatch(8, flow=btvec.Flow.noise(wavelength=120, phase=phase, seed=3))
    assert svg(drawing, phase=1) == svg(drawing, phase=0)
    assert svg(drawing, phase=0.5) != svg(drawing, phase=0)
    assert [p.name for p in drawing.parameters()] == ["phase"]
    mixed = btvec.Flow.angle(0).mix(btvec.Flow.angle(90), btvec.Param("share", default=0, min=0, max=1))
    horizontal = btvec.Drawing(BOX).hatch(10, flow=mixed)
    assert all(len({y for _, y in line}) == 1 for line in lines(horizontal.shapes(share=0)[0].path))
    assert all(len({x for x, _ in line}) == 1 for line in lines(horizontal.shapes(share=1)[0].path))


def test_a_drawing_in_a_distance_field_follows_the_values_the_step_draws_with() -> None:
    font = btvec.Font.bundled("bricolage_grotesque")
    weight = btvec.Param("weight", default=800, min=200, max=800)
    word = btvec.text("HI", font=font, size=180, axes={"wght": weight}, anchor=(20, 160)).resolve()
    hatched = word.hatch((3, 8), tone=btvec.Field.distance(word, normalized=True), flow=btvec.Flow.contour(word))
    assert [p.name for p in hatched.parameters()] == ["weight"]
    for w in (800, 250):
        letters = word.shapes(weight=w)
        whole = btvec.Path.from_svg_path(" ".join(s.path.to_svg_path() for s in letters))
        expected = [
            s.path.hatch((3, 8), tone=btvec.Field.distance(whole, normalized=True), flow=btvec.Flow.contour(whole))
            for s in letters
        ]
        assert [s.path.to_svg_path() for s in hatched.shapes(weight=w)] == [e.to_svg_path() for e in expected]


def test_a_path_a_shape_or_a_drawing_goes_where_a_drawing_is_asked_for() -> None:
    circle = btvec.Path.circle((100, 60), 40)
    for item in (circle, btvec.Drawing(circle), btvec.Drawing(circle).shapes()[0]):
        assert btvec.Field.distance(item).at(100, 60) == pytest.approx(-40, abs=0.05)
        btvec.Flow.contour(item)
    assert repr(btvec.Flow.contour(circle)).startswith("Flow.contour(Path.from_svg_path('")
    assert btvec.Flow.contour(circle) == btvec.Flow.contour(btvec.Drawing(circle))
    assert btvec.Field.distance(circle) == btvec.Field.distance(btvec.Drawing(circle))


def test_hatch_below_keeps_lines_where_the_tone_is_dark() -> None:
    tone = btvec.Field.ramp((0, 0), (200, 0))
    dark = BOX.hatch((6, 6), tone=tone, flow=btvec.Flow.angle(45), below=0.3)
    xs = [x for line in lines(dark) for x, _ in line]
    assert xs
    assert max(xs) <= 0.3 * 200 + 1e-9
    stepped = btvec.Drawing(BOX).hatch((6, 6), tone=tone, flow=btvec.Flow.angle(45), below=0.3)
    assert stepped.shapes()[0].path.to_svg_path() == dark.to_svg_path()
    with pytest.raises(btvec.ParameterError, match="below"):
        BOX.hatch(6, below=0.5)
    level = btvec.Param("level", default=0.3, min=0, max=1)
    drawing = btvec.Drawing(BOX).hatch((6, 6), tone=tone, below=level)
    assert svg(drawing, level=0.6) != svg(drawing, level=0.3)


def test_a_map_is_an_input_of_the_saved_drawing() -> None:
    field = btvec.Field.map(3, 2, BYTES, name="photo")
    drawing = btvec.Drawing(btvec.Path.rect(0, 0, 3, 2)).hatch((0.5, 0.5), tone=field, below=0.5)
    text = drawing.to_json()
    assert '"photo":{"kind":"map"' in text
    back = btvec.Drawing.from_json(text, inputs={"photo": BYTES})
    assert svg(back) == svg(drawing)
    assert back.to_json() == text
    with pytest.raises(ValueError, match="photo"):
        btvec.Drawing.from_json(text)
    with pytest.raises(ValueError, match="photo"):
        btvec.Drawing.from_json(text, inputs={"photo": bytes(6)})


def test_two_names_for_equal_bytes_cannot_be_saved_and_make_maps_differ() -> None:
    first = btvec.Field.map(3, 2, BYTES, name="a")
    second = btvec.Field.map(3, 2, BYTES, name="b")
    assert first != second
    assert first == btvec.Field.map(3, 2, BYTES, name="a")
    drawing = btvec.Drawing(btvec.Path.rect(0, 0, 3, 2))
    drawing = drawing.hatch((0.5, 0.5), tone=first, below=0.5).hatch((0.5, 0.5), tone=second, below=0.5)
    with pytest.raises(btvec.ParameterError, match="to_json"):
        drawing.to_json()


def test_the_recipe_for_tracing_a_map_along_its_pixels_gives_one_node_a_pixel() -> None:
    size, pixels = 1000, 3
    p = size / pixels
    ramp = btvec.Field.ramp((0, 0), (size, 0))
    (line,) = lines(ramp.contours(0.37, frame=(p / 2, p / 2, size - p, size - p), cell=p))
    assert len(line) == pixels


def test_errors_name_the_argument_the_caller_wrote() -> None:
    ramp = btvec.Field.ramp((0, 0), (100, 0))
    with pytest.raises(btvec.ParameterError, match=r"frame: .*at most 1e100"):
        ramp.contours(0.5, frame=(0, 0, 1e200, 2), cell=1)
    with pytest.raises(btvec.ParameterError, match=r"frame: width must be above 0"):
        ramp.contours(0.5, frame=(0, 0, 0, 2), cell=1)
    with pytest.raises(btvec.ParameterError, match=r"width: must be from 1, as must `height`"):
        btvec.Field.map(0, 0, b"")


def test_a_field_too_costly_to_read_is_refused_before_it_starts() -> None:
    wiggle = btvec.Path.from_svg_path(
        "M"
        + " L".join(
            f"{500 + 400 * math.cos(i * 0.01):.3f} {500 + 400 * math.sin(i * 0.0101):.3f}" for i in range(20_000)
        )
    )
    field = btvec.Field.distance(wiggle)
    # 20,000 edges: a read costs 143 units, the limit is 2e8: 90,000 reads pass, 2.8 million do not
    assert len(field.to_map(300, 300, frame=(0, 0, 1000, 1000))) == 90_000
    with pytest.raises(btvec.ParameterError, match=r"cell: contours would read"):
        field.contours(10, frame=(0, 0, 1000, 1000), cell=0.6)
