"""`rows`, `bands`, `stipple`, `within`, `Field.contours(smooth=, min_length=)` and `Drawing.to_plot_svg` through the
public package. Expected values are worked out by hand (heights of rows, the width of a band at a tone) or measured on
the output."""

import math
import re

import pytest

import bullterrier_vector as btvec

BOX = btvec.Path.rect(0, 0, 100, 40)
RAMP = btvec.Field.ramp((0, 0), (100, 0))


def lines(path: btvec.Path) -> list[list[tuple[float, float]]]:
    found: list[list[tuple[float, float]]] = []
    for word in path.to_svg_path().replace("M", "|M").replace("Z", "").split("|"):
        numbers = word.replace("M", " ").replace("L", " ").split()
        if numbers:
            found.append([(float(x), float(y)) for x, y in zip(numbers[::2], numbers[1::2], strict=True)])
    return found


def svg(drawing: btvec.Drawing, **values: float) -> str:
    return "|".join(shape.path.to_svg_path() for shape in drawing.shapes(**values))


def test_rows_are_a_spacing_apart_and_lifted_by_the_tone() -> None:
    flat = lines(BOX.rows(10, tone=0, amplitude=6))
    assert [row[0][1] for row in flat] == [5, 15, 25, 35]
    lifted = lines(BOX.rows(10, tone=0.5, amplitude=6))
    assert [row[0][1] for row in lifted] == [2, 12, 22, 32]


def test_a_band_is_as_wide_as_the_tone_says() -> None:
    (outline,) = lines(btvec.Path.rect(0, 0, 100, 10).bands(10, tone=RAMP, width=(1, 5)))
    for x in {p[0] for p in outline}:
        ys = [p[1] for p in outline if p[0] == x]
        assert len(ys) == 2
        assert abs(abs(ys[0] - ys[1]) - (1 + 4 * x / 100)) < 1e-9


@pytest.mark.parametrize(
    "call",
    [
        {"family": "waves", "wavelength": 30, "amplitude": 3},
        {"family": "rings"},
        {"family": "spiral", "centre": (50, 20)},
        {"family": "rays", "angle": 15},
    ],
    ids=["waves", "rings", "spiral", "rays"],
)
def test_every_family_makes_outlines_and_a_drawing_makes_the_same(call: dict[str, object]) -> None:
    path = BOX.bands(8, tone=RAMP, width=(1, 3), **call)  # type: ignore[arg-type]
    assert path.to_svg_path()
    drawing = btvec.Drawing(BOX).bands(8, tone=RAMP, width=(1, 3), **call)  # type: ignore[arg-type]
    assert svg(drawing) == path.to_svg_path()


def test_stipple_density_follows_the_tone_and_a_seed_moves_the_dots() -> None:
    region = btvec.Path.rect(0, 0, 400, 100)
    tone = btvec.Field.ramp((0, 0), (400, 0))
    dots = region.stipple(4, tone=tone, radius=0.8).to_svg_path().count("M")
    assert abs(dots - 1250) < 0.06 * 2500  # a cell is 4 by 4, density 1 - tone: half of 2500 cells
    assert (
        region.stipple(4, tone=tone, radius=0.8, seed=3).to_svg_path()
        != region.stipple(4, tone=tone, radius=0.8).to_svg_path()
    )


def test_a_param_in_a_step_is_read_with_the_values_of_the_draw() -> None:
    lift = btvec.Param("lift", default=0, min=0, max=30)
    drawing = btvec.Drawing(BOX).rows(10, tone=RAMP, amplitude=lift)
    assert svg(drawing) == svg(drawing, lift=0)
    assert svg(drawing, lift=20) != svg(drawing, lift=0)
    assert btvec.Drawing.from_json(drawing.to_json()).to_json() == drawing.to_json()


def test_within_cuts_a_line_at_the_region() -> None:
    (piece,) = lines(btvec.Path.polyline([(0, 50), (200, 50)]).within(btvec.Path.circle((100, 50), 30)))
    assert abs(piece[0][0] - 70) < 0.05
    assert abs(piece[-1][0] - 130) < 0.05


def test_contours_can_be_smoothed_and_short_lines_dropped() -> None:
    data = bytearray(60 * 60)
    for y in range(10, 40):
        for x in range(10, 40):
            data[y * 60 + x] = 255
    data[50 * 60 + 50] = 255
    field = btvec.Field.map(60, 60, bytes(data))
    centres = (0.5, 0.5, 59, 59)
    assert len(lines(field.contours(0.5, box=centres, cell=1))) == 2
    assert len(lines(field.contours(0.5, box=centres, cell=1, smooth=2))) == 1
    assert len(lines(field.contours(0.5, box=centres, cell=1, min_length=30))) == 1


def test_a_plot_has_a_layer_per_pen_and_no_fill() -> None:
    art = btvec.Drawing(
        btvec.Path.rect(5, 5, 30, 20).fill("#ff0000"),
        btvec.Path.polyline([(40, 5), (90, 30)]).stroke("#0000ff", width=3),
        size=(100, 40),
    )
    text = art.to_plot_svg(200, 80, pen_width=1)
    assert re.findall(r'inkscape:label="pen (\d) (#[0-9a-f]{6})"', text) == [("1", "#ff0000"), ("2", "#0000ff")]
    assert 'fill="none"' in text
    assert 'stroke-width="2"' in text
    assert text == art.to_plot_svg(200, 80, pen_width=1)


@pytest.mark.parametrize(
    "call",
    [
        lambda: BOX.rows(0, tone=0, amplitude=1),
        lambda: BOX.rows(5, tone=0, amplitude=math.nan),
        lambda: BOX.rows(0.001, tone=0, amplitude=1),
        lambda: btvec.Path.rect(0, 0, 1e6, 1e6).rows(0.001, tone=0, amplitude=1),
        lambda: BOX.bands(5, tone=0, width=(-1, 2)),
        lambda: BOX.bands(5, tone=0, width=(1, 2), family="zigzag"),
        lambda: BOX.bands(5, tone=0, width=(1, 2), family="waves"),
        lambda: BOX.stipple(5, tone=0, radius=0),
        lambda: BOX.stipple(5, tone=0, radius=1, density=(0, 2)),
        lambda: btvec.Path.rect(0, 0, 1e6, 1e6).stipple(1, tone=0, radius=0.4),
        lambda: btvec.Drawing(BOX).to_plot_svg(10, 10, pen_width=0),
        lambda: btvec.Drawing(BOX.fill("#f00")).to_plot_svg(10, 10, pen_width=1, tolerance=-1),
    ],
)
def test_what_cannot_be_made_is_a_parameter_error(call: object) -> None:
    with pytest.raises(btvec.ParameterError):
        call()  # type: ignore[operator]


def test_a_width_of_nothing_is_the_same_in_a_path_a_step_and_a_document() -> None:
    drawing = btvec.Drawing(BOX).bands(10, tone=RAMP, width=(0, 4))
    assert svg(drawing) == BOX.bands(10, tone=RAMP, width=(0, 4)).to_svg_path()
    back = btvec.Drawing.from_json(drawing.to_json())
    assert svg(back) == svg(drawing)


@pytest.mark.parametrize(
    ("call", "argument", "family"),
    [
        ({"family": "lines", "wavelength": 30}, "wavelength", "lines"),
        ({"family": "rays", "amplitude": 2}, "amplitude", "rays"),
        ({"family": "lines", "centre": (5, 5)}, "centre", "lines"),
        ({"family": "waves", "wavelength": 30, "amplitude": 3, "centre": (5, 5)}, "centre", "waves"),
        ({"family": "rings", "angle": 10}, "angle", "rings"),
    ],
)
def test_an_argument_the_family_does_not_read_is_refused_naming_it_and_the_family(
    call: dict[str, object], argument: str, family: str
) -> None:
    for target in (BOX, btvec.Drawing(BOX)):
        with pytest.raises(btvec.ParameterError, match=rf"{argument}.*{family}"):
            target.bands(10, tone=RAMP, width=(1, 2), **call)  # type: ignore[arg-type]
