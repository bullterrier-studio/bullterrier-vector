"""`halftone` through the public package: the tone as a number or a `Field`, the shape of a dot, the screen it lies on,
what a drawn shape says it carries, the document it is saved as, and what it refuses. The dots themselves (areas, radii,
angles) are measured against hand-worked values in the Rust tests (`halftone_screens.rs`); here the pixels the Python
call draws are counted against the tone."""

from typing import Any

import numpy as np
import pytest

import bullterrier_vector as btvec

Drawing = btvec.Drawing
ParameterError = btvec.ParameterError
INK = "#000000"
# 5 pixels in from the edge of a 100 by 100 canvas: the dots of a lattice of 10 at 10, 20 .. 90 lie whole inside it
INSET = btvec.Path.rect(5, 5, 90, 90)
TRIANGLE = btvec.Path.from_svg_path("M0 0 L4 0 L0 2 Z")


def coverage(drawing: btvec.Drawing, side: int = 100) -> float:
    """The ink the pixels hold, in pixels of the canvas: the sum of alpha over 255."""
    image = np.frombuffer(drawing.to_rgba(side, side), dtype=np.uint8).reshape(side, side, 4)
    return float(image[:, :, 3].sum()) / 255.0


def the_halftone(drawing: btvec.Drawing, **values: Any) -> btvec.Halftone:
    (shape,) = drawing.shapes(**values)
    assert shape.style.halftone is not None
    return shape.style.halftone


def test_dots_of_a_number_cover_one_minus_the_tone_of_their_cells():
    # 81 squares of 5 by 5 at tone 0.75, on a lattice turned by 0: 25 of each cell's 100 pixels
    drawing = Drawing(INSET).halftone(INK, tone=0.75, spacing=10, angle=0, shape="square")
    assert coverage(drawing) == pytest.approx(81 * 25, rel=0.002)


def test_a_field_tone_is_read_at_the_center_of_each_dot():
    # the ramp is the tone x / 100: a dot at x = 10, 20 .. 90 covers 1 - x / 100 of its cell, nine dots to a column
    ramp = btvec.Field.ramp((0, 0), (100, 0))
    drawing = Drawing(INSET).halftone(INK, tone=ramp, spacing=10, angle=0, shape="square")
    assert coverage(drawing) == pytest.approx(9 * sum((1 - x / 100) * 100 for x in range(10, 100, 10)), rel=0.002)


def test_a_map_is_a_tone_too():
    # a map white on the left half and black on the right: bare cells on the left, solid ones on the right
    grey = bytes([255] * 8 + [0] * 8) * 16
    photo = btvec.Field.map(16, 16, grey, frame=(0, 0, 100, 100))
    drawing = Drawing(INSET).halftone(INK, tone=photo, spacing=10, angle=0, shape="square")
    # x = 10 .. 40 are bare, 60 .. 90 are tone 0 (full cells of 100), x = 50 is the edge of the map's halves
    full = 9 * 100 * 4
    assert full <= coverage(drawing) <= full + 9 * 100 * 1


def test_a_number_per_shape_still_reads_the_shape_it_is_for():
    drawing = Drawing(btvec.Path.rect(0, 0, 40, 40), btvec.Path.rect(50, 0, 40, 40)).halftone(
        INK, tone=btvec.by_index(0.2, 0.8), spacing=10, angle=0, shape="square"
    )
    first, second = (shape.style.halftone for shape in drawing.shapes())
    assert first is not None
    assert second is not None
    assert (first.tone, second.tone) == (0.2, 0.8)


def test_a_drawn_shape_says_what_its_halftone_is():
    ramp = btvec.Field.ramp((0, 0), (0, 10))
    halftone = the_halftone(
        Drawing(btvec.Path.rect(0, 0, 10, 10)).halftone(
            INK, tone=ramp, spacing=6, shape="ellipse", aspect=2, family="rings", center=(3, 4)
        )
    )
    assert isinstance(halftone.tone, btvec.Field)
    assert halftone.tone == ramp
    assert (halftone.spacing, halftone.shape, halftone.aspect, halftone.family) == (6.0, "ellipse", 2.0, "rings")
    assert halftone.center == btvec.Point(3, 4)
    # a wave left at its defaults is 8 design pixels high and 48 long, whatever the spacing
    assert (halftone.amplitude, halftone.wavelength) == (8.0, 48.0)
    plain = the_halftone(Drawing(btvec.Path.rect(0, 0, 10, 10)).halftone(INK, tone=0.5))
    assert (plain.tone, plain.shape, plain.family, plain.center, plain.aspect) == (0.5, "dot", "lines", None, 1.0)
    assert repr(plain) == (
        "Halftone(color='#000000ff', tone=0.5, spacing=8.0, angle=45.0, shape='dot', aspect=1.0, family='lines', "
        "center=None, amplitude=8.0, wavelength=48.0)"
    )


def test_a_caller_outline_is_the_shape_and_comes_back_as_a_path():
    halftone = the_halftone(Drawing(INSET).halftone(INK, tone=0.5, shape=TRIANGLE))
    assert halftone.shape == TRIANGLE


def test_equal_screens_are_equal_and_hash_alike():
    first = the_halftone(Drawing(INSET).halftone(INK, tone=0.3, family="waves", amplitude=2))
    second = the_halftone(Drawing(INSET).halftone(INK, tone=0.3, family="waves", amplitude=2))
    assert first == second
    assert hash(first) == hash(second)
    assert first != the_halftone(Drawing(INSET).halftone(INK, tone=0.3, family="waves", amplitude=3))
    # a shape put back into a drawing is the same shape, a field tone and a caller's outline included
    ramp = btvec.Field.ramp((0, 0), (0, 10))
    shapes = Drawing(INSET).halftone(INK, tone=ramp, shape=TRIANGLE).shapes()
    assert Drawing(*shapes).shapes() == shapes


@pytest.mark.parametrize("family", ["lines", "waves", "rings", "spiral", "rays"])
@pytest.mark.parametrize("shape", ["dot", "square", "ellipse", "diamond", "euclidean", TRIANGLE])
def test_every_shape_on_every_family_draws_in_svg_and_pixels(family: str, shape: Any):
    options: dict[str, Any] = {"aspect": 1.5} if shape == "ellipse" else {}
    drawing = Drawing(INSET).halftone(INK, tone=0.6, spacing=9, shape=shape, family=family, **options)
    assert "<path" in drawing.to_svg(100, 100)
    assert coverage(drawing) > 100


def test_a_document_saves_and_reads_back_every_argument():
    ramp = btvec.Field.ramp((0, 0), (0, 100))
    for drawing in (
        Drawing(INSET).halftone(INK, tone=0.4, spacing=5, shape="ellipse", aspect=2, family="rings", center=(10, 20)),
        Drawing(INSET).halftone(INK, tone=ramp, shape=TRIANGLE, family="spiral"),
        Drawing(INSET).halftone(INK, tone=0.4, family="waves", amplitude=2, wavelength=9),
    ):
        text = drawing.to_json()
        again = Drawing.from_json(text)
        assert again.to_json() == text
        assert again.shapes() == drawing.shapes()
    text = Drawing(INSET).halftone(INK, tone=0.4, shape=TRIANGLE).to_json()
    assert '"shape":{"path":"M0 0 L4 0 L0 2 Z"}' in text


@pytest.mark.parametrize(
    ("make", "argument"),
    [
        pytest.param(lambda: Drawing(INSET).halftone(INK, tone=0.5, shape="star"), "shape", id="an unknown shape"),
        pytest.param(lambda: Drawing(INSET).halftone(INK, tone=0.5, family="zigzag"), "family", id="an unknown family"),
        pytest.param(
            lambda: Drawing(INSET).halftone(INK, tone=0.5, shape="ellipse", aspect=0.5), "aspect", id="aspect < 1"
        ),
        pytest.param(lambda: Drawing(INSET).halftone(INK, tone=0.5, center=(1, 1)), "center", id="a center on a grid"),
        pytest.param(
            lambda: Drawing(INSET).halftone(INK, tone=0.5, amplitude=3), "amplitude", id="an amplitude on a grid"
        ),
        pytest.param(lambda: Drawing(INSET).halftone(INK, tone=0.5, aspect=3), "aspect", id="an aspect of a dot"),
        pytest.param(
            lambda: Drawing(INSET).halftone(INK, tone=0.5, family="rings", angle=10), "angle", id="an angle on rings"
        ),
        pytest.param(
            lambda: Drawing(INSET).halftone(INK, tone=0.5, family="waves", wavelength=0),
            "wavelength",
            id="a wavelength of 0",
        ),
        pytest.param(
            lambda: Drawing(INSET).halftone(INK, tone=0.5, shape=btvec.Path.line((0, 0), (5, 5))),
            "shape",
            id="an open outline",
        ),
        pytest.param(
            lambda: Drawing(INSET).halftone(INK, tone=0.5, shape=btvec.Path.from_svg_path("M0 0 L4 0 L8 0 Z")),
            "shape",
            id="no area",
        ),
    ],
)
def test_what_it_refuses_names_the_argument(make: Any, argument: str):
    with pytest.raises(ParameterError, match=argument):
        make()


def test_an_unknown_shape_lists_the_valid_ones():
    with pytest.raises(ParameterError, match=r'"dot", "square", "ellipse", "diamond", "euclidean"'):
        Drawing(INSET).halftone(INK, tone=0.5, shape="star")


def test_a_tone_of_the_wrong_type_is_a_type_error():
    loose: Any = Drawing(INSET).halftone
    with pytest.raises(TypeError, match="Field or a number"):
        loose(INK, tone="dark")
    with pytest.raises(TypeError, match="shape or a Path"):
        loose(INK, tone=0.5, shape=3)


def test_a_spacing_too_small_for_the_shape_is_refused_when_drawn():
    drawing = Drawing(btvec.Path.rect(0, 0, 1000, 1000)).halftone(INK, tone=0.5, spacing=0.001, family="rings")
    with pytest.raises(ParameterError, match="spacing"):
        drawing.to_svg(100, 100)
