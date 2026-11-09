"""Gradient paints through the public package: `linear_gradient` and `radial_gradient`, a fill or stroke that takes one,
what `Style` reads back, the document, the pixels (worked out by hand) and what is refused."""

import pytest

import bullterrier_vector as btvec

SQUARE = btvec.Path.rect(0, 0, 100, 100)
FADE = btvec.linear_gradient([(0, "#ff0000"), (1, "#0000ff")], start=(0, 0), end=(100, 0))
GLOW = btvec.radial_gradient([(0, "#ffffff"), (1, "#00000000")], center=(50, 50), radius=40)


def style_of(drawing: btvec.Drawing) -> btvec.Style:
    (shape,) = drawing.shapes()
    return shape.style


def test_a_linear_gradient_reads_back_its_line_and_stops():
    assert FADE.kind == "linear"
    assert FADE.stops == [(0.0, "#ff0000ff"), (1.0, "#0000ffff")]
    assert (FADE.start, FADE.end) == (btvec.Point(0, 0), btvec.Point(100, 0))
    assert (FADE.center, FADE.radius) == (None, None)
    assert repr(FADE) == "linear_gradient([(0.0, '#ff0000ff'), (1.0, '#0000ffff')], start=(0.0, 0.0), end=(100.0, 0.0))"


def test_a_radial_gradient_reads_back_its_centre_and_radius():
    assert GLOW.kind == "radial"
    assert (GLOW.center, GLOW.radius) == (btvec.Point(50, 50), 40.0)
    assert (GLOW.start, GLOW.end) == (None, None)


def test_gradients_compare_and_hash_by_their_parts():
    same = btvec.linear_gradient([(0, "#ff0000"), (1, "#0000ff")], start=(0, 0), end=(100, 0))
    assert same == FADE
    assert hash(same) == hash(FADE)
    moved = btvec.linear_gradient([(0, "#ff0000"), (1, "#0000ff")], start=(0, 0), end=(100, 1))
    assert moved != FADE
    assert FADE != GLOW


def test_a_fill_and_a_stroke_take_a_gradient_and_the_style_gives_it_back():
    style = style_of(SQUARE.fill(FADE).stroke(GLOW, width=4))
    assert style.fill == FADE
    assert style.stroke == GLOW
    assert style.stroke_width == 4
    assert "linear_gradient(" in repr(style)
    assert hash(style) == hash(style_of(SQUARE.fill(FADE).stroke(GLOW, width=4)))
    assert style != style_of(SQUARE.fill("#ff0000").stroke(GLOW, width=4))


def test_a_colour_after_a_gradient_replaces_it_and_a_colour_is_still_a_text():
    assert style_of(SQUARE.fill(FADE).fill("#336699")).fill == "#336699ff"
    assert style_of(btvec.Drawing(SQUARE).fill(FADE)).fill == FADE
    assert style_of(btvec.Drawing(SQUARE).stroke("#000000")).stroke == "#000000ff"


def test_a_shape_goes_back_into_a_drawing_and_through_json_with_its_gradient():
    drawing = btvec.Drawing(SQUARE).fill(FADE).stroke(GLOW)
    shapes = drawing.shapes()
    assert btvec.Drawing(*shapes).shapes() == shapes
    again = btvec.Drawing.from_json(drawing.to_json())
    assert again.shapes() == shapes
    assert '"gradient":"linear"' in drawing.to_json()
    assert '"radius":40' in drawing.to_json()


def test_pixels_are_the_stops_at_the_projection_of_the_pixel_on_the_line():
    image = btvec.Drawing(SQUARE).fill(FADE).to_rgba(100, 100)

    def at(x: int, y: int) -> tuple[int, ...]:
        start = (y * 100 + x) * 4
        return tuple(image[start : start + 4])

    # pixel centre x + 0.5 of 100 along the line: the mean of the stops weighted by that share
    for x in (10, 50, 90):
        share = (x + 0.5) / 100
        red, green, blue, alpha = at(x, 40)
        assert (red, green, blue, alpha) == pytest.approx((255 * (1 - share), 0, 255 * share, 255), abs=3)


def test_the_svg_names_the_gradient_and_the_mesh_refuses_it():
    svg = btvec.Drawing(SQUARE).fill(FADE).to_svg(100, 100)
    assert "<linearGradient" in svg
    assert 'fill="url(#gradient-0-fill)"' in svg
    with pytest.raises(btvec.ParameterError, match="gradient"):
        btvec.Drawing(SQUARE).fill(FADE).to_mesh(100, 100)


@pytest.mark.parametrize(
    ("make", "argument"),
    [
        pytest.param(lambda: btvec.linear_gradient([(0, "#ff0000")], start=(0, 0), end=(1, 0)), "stops", id="one stop"),
        pytest.param(
            lambda: btvec.linear_gradient([(0, "#ff0000"), (2, "#0000ff")], start=(0, 0), end=(1, 0)),
            "stops",
            id="an offset above 1",
        ),
        pytest.param(
            lambda: btvec.linear_gradient([(0.5, "#ff0000"), (0.2, "#0000ff")], start=(0, 0), end=(1, 0)),
            "stops",
            id="offsets that decrease",
        ),
        pytest.param(
            lambda: btvec.linear_gradient([(0, "#ff0000"), (1, "none")], start=(0, 0), end=(1, 0)),
            "stops",
            id="a stop of none",
        ),
        pytest.param(
            lambda: btvec.linear_gradient([(0, "#ff0000"), (1, "red")], start=(0, 0), end=(1, 0)),
            "color",
            id="a stop colour by name",
        ),
        pytest.param(
            lambda: btvec.linear_gradient([(0, "#ff0000"), (1, "#0000ff")], start=(3, 3), end=(3, 3)),
            "end",
            id="a line of no length",
        ),
        pytest.param(
            lambda: btvec.radial_gradient([(0, "#ff0000"), (1, "#0000ff")], center=(0, 0), radius=0),
            "radius",
            id="a radius of 0",
        ),
    ],
)
def test_what_a_gradient_cannot_paint_is_a_parameter_error(make, argument):
    with pytest.raises(btvec.ParameterError, match=argument):
        make()


def test_a_fill_that_is_neither_a_colour_nor_a_gradient_is_a_type_error():
    with pytest.raises(TypeError, match="Gradient"):
        btvec.Drawing(SQUARE).fill(3)  # type: ignore[arg-type]  # the wrong type is what is tested
    with pytest.raises(TypeError, match="Gradient"):
        SQUARE.stroke(None)  # type: ignore[arg-type]  # the wrong type is what is tested
