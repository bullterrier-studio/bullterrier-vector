"""`Drawing`, `Shape` and `Style` through the public package: every call shape of docs/api.md for the steps built so
far, the shapes a drawing draws, Params and expressions read when it is drawn, and each error with the place it is
raised at. Expected values are worked out by hand on a 10 by 10 square (the corners are dyadic numbers, so the results
are exact)."""

import re
from collections.abc import Callable, Mapping
from pathlib import Path as FilePath
from typing import Any

import pytest

import bullterrier_vector as btvec

Drawing = btvec.Drawing
Param = btvec.Param
ParameterError = btvec.ParameterError

SQUARE = btvec.Path.rect(0, 0, 10, 10)
LINE = btvec.Path.line((0, 0), (10, 0))
BLACK = "#000000ff"
# the same classes, untyped: the wrong types below are what is tested
LOOSE: Any = btvec.Drawing
FRAMELESS: Any = btvec.Drawing(SQUARE)


def squares(count: int) -> list[btvec.Path]:
    """`count` 5 by 5 squares, the one at index `i` starting at x = 10 * i."""
    return [btvec.Path.rect(10 * i, 0, 5, 5) for i in range(count)]


def drawn(drawing: btvec.Drawing, **values: Any) -> list[str]:
    """The path data of every shape a drawing draws."""
    return [shape.path.to_svg_path() for shape in drawing.shapes(**values)]


def only_shape(drawing: btvec.Drawing, **values: Any) -> btvec.Shape:
    (shape,) = drawing.shapes(**values)
    return shape


def param(name: str, default: float = 0.0, low: float = -100.0, high: float = 100.0, **more: Any) -> btvec.Param:
    return Param(name, default=default, min=low, max=high, **more)


# --- what a drawing of one path draws ---------------------------------------------------------------------------


def test_a_closed_outline_is_drawn_once_filled_black_with_no_attributes_but_its_index():
    shape = only_shape(Drawing(SQUARE))
    assert shape.path == SQUARE
    assert shape.attributes == {"index": 0}
    style = shape.style
    assert (style.fill, style.stroke, style.opacity, style.blend) == (BLACK, None, 1.0, "normal")
    assert (style.stroke_width, style.join, style.cap, style.miter_limit) == (None, None, None, None)


def test_an_open_line_is_drawn_stroked_black_two_pixels_wide():
    style = only_shape(Drawing(LINE)).style
    assert (style.fill, style.stroke) == (None, BLACK)
    assert (style.stroke_width, style.join, style.cap, style.miter_limit) == (2.0, "round", "round", 4.0)


def test_a_drawing_with_no_items_draws_nothing():
    assert Drawing().shapes() == []


def test_a_step_returns_a_new_drawing_and_leaves_the_old_one_alone():
    start = Drawing(SQUARE)
    moved = start.translate(5, 0)
    assert drawn(start) == ["M0 0 L10 0 L10 10 L0 10 Z"]
    assert drawn(moved) == ["M5 0 L15 0 L15 10 L5 10 Z"]


def test_a_drawing_drawn_twice_gives_equal_shapes():
    drawing = Drawing(*squares(3)).translate(1, 2).fill("#336699")
    assert drawing.shapes() == drawing.shapes()


# --- geometry ---------------------------------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("step", "expected"),
    [
        pytest.param(lambda d: d.translate(3, 4), "M3 4 L13 4 L13 14 L3 14 Z", id="translate"),
        pytest.param(lambda d: d.translate(-2.5, 0), "M-2.5 0 L7.5 0 L7.5 10 L-2.5 10 Z", id="translate by a fraction"),
        pytest.param(lambda d: d.rotate(90), "M10 0 L10 10 L0 10 L0 0 Z", id="a quarter turn about the middle"),
        pytest.param(lambda d: d.rotate(360), "M0 0 L10 0 L10 10 L0 10 Z", id="a full turn is no turn"),
        pytest.param(lambda d: d.rotate(450), "M10 0 L10 10 L0 10 L0 0 Z", id="angles wrap"),
        pytest.param(lambda d: d.rotate(90, about=(0, 0)), "M0 0 L0 10 L-10 10 L-10 0 Z", id="about a tuple"),
        pytest.param(
            lambda d: d.rotate(90, about=btvec.Point(0, 0)), "M0 0 L0 10 L-10 10 L-10 0 Z", id="about a Point"
        ),
        pytest.param(lambda d: d.scale(2), "M-5 -5 L15 -5 L15 15 L-5 15 Z", id="scale about the middle"),
        pytest.param(lambda d: d.scale((2, 1)), "M-5 0 L15 0 L15 10 L-5 10 Z", id="scale by a pair"),
        pytest.param(lambda d: d.scale(2, about=(0, 0)), "M0 0 L20 0 L20 20 L0 20 Z", id="scale about a point"),
        pytest.param(lambda d: d.scale(-1, about=(0, 0)), "M0 0 L-10 0 L-10 -10 L0 -10 Z", id="a negative mirrors"),
    ],
)
def test_a_geometry_step_moves_every_shape_as_the_arithmetic_says(step, expected):
    assert drawn(step(Drawing(SQUARE))) == [expected]


def test_a_geometry_step_acts_on_every_shape_of_a_stack_about_its_own_middle():
    # two 5 by 5 squares, at x = 0 and x = 10; each turns about its own middle, so a quarter turn leaves each in place
    stack = Drawing(*squares(2))
    assert drawn(stack.rotate(90)) == ["M5 0 L5 5 L0 5 L0 0 Z", "M15 0 L15 5 L10 5 L10 0 Z"]
    assert drawn(stack.scale(2)) == [
        "M-2.5 -2.5 L7.5 -2.5 L7.5 7.5 L-2.5 7.5 Z",
        "M7.5 -2.5 L17.5 -2.5 L17.5 7.5 L7.5 7.5 Z",
    ]


def test_a_stack_turns_as_one_about_a_point_given():
    stack = Drawing(*squares(2))
    assert drawn(stack.rotate(180, about=(10, 0))) == [
        "M20 0 L15 0 L15 -5 L20 -5 Z",
        "M10 0 L5 0 L5 -5 L10 -5 Z",
    ]


# --- style ------------------------------------------------------------------------------------------------------


def test_fill_paints_only_the_fill_once_set():
    style = only_shape(Drawing(SQUARE).fill("#FF4F9A")).style
    assert (style.fill, style.stroke) == ("#ff4f9aff", None)


def test_fill_none_counts_as_set_and_paints_nothing():
    style = only_shape(Drawing(SQUARE).fill("none")).style
    assert (style.fill, style.stroke) == (None, None)


def test_a_colour_with_alpha_is_kept():
    assert only_shape(Drawing(SQUARE).fill("#11223344")).style.fill == "#11223344"


def test_stroke_with_only_a_colour_takes_the_defaults_and_leaves_the_fill_off():
    style = only_shape(Drawing(SQUARE).stroke("#112233")).style
    assert (style.fill, style.stroke) == (None, "#112233ff")
    assert (style.stroke_width, style.join, style.cap, style.miter_limit) == (2.0, "round", "round", 4.0)


def test_stroke_takes_each_option():
    style = only_shape(
        Drawing(SQUARE).fill("#ffcc33").stroke("#1a1a1a", width=4.5, join="miter", cap="square", miter_limit=2)
    ).style
    assert (style.fill, style.stroke) == ("#ffcc33ff", "#1a1a1aff")
    assert (style.stroke_width, style.join, style.cap, style.miter_limit) == (4.5, "miter", "square", 2.0)


def test_each_option_of_stroke_is_independent():
    assert only_shape(Drawing(LINE).stroke("#000000", cap="butt")).style.cap == "butt"
    assert only_shape(Drawing(LINE).stroke("#000000", join="bevel")).style.join == "bevel"
    assert only_shape(Drawing(LINE).stroke("#000000", miter_limit=1)).style.miter_limit == 1.0
    assert only_shape(Drawing(LINE).stroke("#000000", width=1e-3)).style.stroke_width == 1e-3


def test_opacity_multiplies_and_blend_replaces():
    style = only_shape(
        Drawing(SQUARE).opacity(0.5).opacity(0.5).blend("multiply").blend("normal").blend("multiply")
    ).style
    assert (style.opacity, style.blend) == (0.25, "multiply")


@pytest.mark.parametrize("alpha", [0, 1, 0.5])
def test_opacity_accepts_the_ends_of_its_range(alpha):
    assert only_shape(Drawing(SQUARE).opacity(alpha)).style.opacity == alpha


def test_a_style_step_called_on_a_path_returns_a_drawing_of_that_path():
    for drawing, field, expected in [
        (SQUARE.fill("#ff0000"), "fill", "#ff0000ff"),
        (SQUARE.stroke("#00ff00", width=3), "stroke", "#00ff00ff"),
        (SQUARE.opacity(0.5), "opacity", 0.5),
        (SQUARE.blend("multiply"), "blend", "multiply"),
    ]:
        assert isinstance(drawing, Drawing)
        shape = only_shape(drawing)
        assert shape.path == SQUARE
        assert getattr(shape.style, field) == expected


def test_a_style_on_a_path_is_the_same_as_the_style_on_its_drawing():
    assert SQUARE.stroke("#00ff00", width=3).shapes() == Drawing(SQUARE).stroke("#00ff00", width=3).shapes()


def test_styles_with_equal_fields_are_equal_and_hash_alike():
    first, second = only_shape(SQUARE.fill("#ff0000")).style, only_shape(Drawing(SQUARE).fill("#ff0000")).style
    assert first == second
    assert hash(first) == hash(second)
    assert first != only_shape(SQUARE.fill("#ff0001")).style


# --- Params and expressions, read when the drawing is drawn ---------------------------------------------------------


def test_a_param_is_read_at_its_default_and_at_the_value_given():
    dx = param("dx", default=2)
    moved = Drawing(SQUARE).translate(dx, 0)
    assert drawn(moved) == ["M2 0 L12 0 L12 10 L2 10 Z"]
    assert drawn(moved, dx=7) == ["M7 0 L17 0 L17 10 L7 10 Z"]


def test_an_expression_is_worked_out_from_the_values():
    dx = param("dx")
    moved = Drawing(SQUARE).translate(2 * dx + 1, 10 - dx)
    assert drawn(moved, dx=3) == ["M7 7 L17 7 L17 17 L7 17 Z"]


def test_a_param_in_a_pair_and_in_a_pivot_is_read_component_by_component():
    kx, cx = param("kx", default=1, low=-4, high=4), param("cx")
    scaled = Drawing(SQUARE).scale((kx, 1), about=(cx, 0))
    assert drawn(scaled, kx=2, cx=0) == ["M0 0 L20 0 L20 10 L0 10 Z"]
    assert drawn(scaled, kx=2, cx=10) == ["M-10 0 L10 0 L10 10 L-10 10 Z"]


def test_a_param_as_a_single_scale_factor_and_a_turn():
    k = param("k", default=1, low=-4, high=4)
    turn = Param("turn", default=0, min=0, max=360, unit="deg")
    drawing = Drawing(SQUARE).scale(k, about=(0, 0)).rotate(turn, about=(0, 0))
    assert drawn(drawing, k=2, turn=90) == ["M0 0 L0 20 L-20 20 L-20 0 Z"]


def test_a_cyclic_param_wraps_any_value_into_its_turn():
    turn = Param("turn", default=0, min=0, max=360, unit="deg")
    quarter = Drawing(SQUARE).rotate(turn)
    assert drawn(quarter, turn=90) == drawn(quarter, turn=450) == drawn(quarter, turn=-270)


def test_a_param_in_a_style_is_read_in_the_style():
    alpha = param("alpha", default=1, low=0, high=1)
    width = param("width", default=2, low=1, high=9)
    drawing = Drawing(SQUARE).opacity(alpha).stroke("#000000", width=width * 2, miter_limit=width)
    style = only_shape(drawing, alpha=0.25, width=3).style
    assert (style.opacity, style.stroke_width, style.miter_limit) == (0.25, 6.0, 3.0)


def test_a_list_gives_each_shape_its_own_value():
    dx = param("dx")
    moved = Drawing(*squares(3)).translate(dx, 0)
    assert drawn(moved, dx=[1, 2, 3]) == [
        "M1 0 L6 0 L6 5 L1 5 Z",
        "M12 0 L17 0 L17 5 L12 5 Z",
        "M23 0 L28 0 L28 5 L23 5 Z",
    ]


def test_a_value_may_be_a_tuple_or_any_sequence_of_numbers():
    dx = param("dx")
    assert drawn(Drawing(*squares(2)).translate(dx, 0), dx=(1, 2)) == drawn(
        Drawing(*squares(2)).translate(dx, 0), dx=[1, 2]
    )


def test_each_call_of_shapes_reads_its_own_values():
    dx = param("dx")
    moved = Drawing(SQUARE).translate(dx, 0)
    first, second, again = drawn(moved, dx=1), drawn(moved, dx=2), drawn(moved, dx=1)
    assert first == again
    assert first != second


def test_parameters_lists_each_param_in_the_order_first_used_once():
    dx, dy, turn = param("dx"), param("dy"), Param("turn", default=0, min=0, max=360, unit="deg")
    drawing = Drawing(SQUARE).translate(dy, dx).rotate(turn).translate(dx, 0)
    assert drawing.parameters() == [dy, dx, turn]


def test_parameters_gives_back_every_field():
    dx = Param("dx", default=2, min=-5, max=9, unit="px", meaning="how far right")
    (found,) = Drawing(SQUARE).translate(dx, 0).parameters()
    assert (found.name, found.default, found.min, found.max, found.unit, found.meaning) == (
        "dx",
        2.0,
        -5.0,
        9.0,
        "px",
        "how far right",
    )


def test_a_drawing_that_reads_no_param_has_no_parameters():
    assert Drawing(SQUARE).translate(1, 2).parameters() == []


def test_equal_params_used_twice_are_one_parameter():
    first = Param("p", default=1, min=0, max=2)
    again = Param("p", default=1, min=0, max=2)
    assert Drawing(SQUARE).translate(first, 0).translate(again, 0).parameters() == [first]


def test_with_values_fixes_a_param_and_removes_it_from_parameters():
    dx, dy = param("dx"), param("dy")
    drawing = Drawing(SQUARE).translate(dx, dy)
    fixed = drawing.with_values(dx=5)
    assert fixed.parameters() == [dy]
    assert drawn(fixed, dy=1) == drawn(drawing, dx=5, dy=1)


def test_with_values_accepts_a_list_per_shape():
    dx = param("dx")
    drawing = Drawing(*squares(2)).translate(dx, 0)
    assert drawn(drawing.with_values(dx=[1, 2])) == drawn(drawing, dx=[1, 2])


def test_a_step_after_with_values_reads_the_param_again():
    dx = param("dx")
    drawing = Drawing(SQUARE).translate(dx, 0).with_values(dx=5).translate(dx, 0)
    assert drawing.parameters() == [dx]
    assert drawn(drawing, dx=1) == ["M6 0 L16 0 L16 10 L6 10 Z"]


# --- only and without ---------------------------------------------------------------------------------------------


def test_only_keeps_the_shapes_whose_index_is_asked_for_and_renumbers():
    kept = Drawing(*squares(4)).only(index=2)
    assert drawn(kept) == ["M20 0 L25 0 L25 5 L20 5 Z"]
    assert [shape.attributes["index"] for shape in kept.shapes()] == [0]


def test_a_list_means_any_of():
    kept = Drawing(*squares(4)).only(index=[1, 3])
    assert drawn(kept) == ["M10 0 L15 0 L15 5 L10 5 Z", "M30 0 L35 0 L35 5 L30 5 Z"]
    assert [shape.attributes["index"] for shape in kept.shapes()] == [0, 1]


def test_without_keeps_the_others():
    kept = Drawing(*squares(4)).without(index=[0, 2])
    assert drawn(kept) == ["M10 0 L15 0 L15 5 L10 5 Z", "M30 0 L35 0 L35 5 L30 5 Z"]


def test_without_renumbers_so_the_next_selection_reads_the_new_index():
    kept = Drawing(*squares(4)).without(index=0).only(index=0)
    assert drawn(kept) == ["M10 0 L15 0 L15 5 L10 5 Z"]


def test_only_reads_an_attribute_a_stack_wrote():
    # the order of four shapes runs 0, 1/3, 2/3, 1
    assert drawn(Drawing(*squares(4)).only(order=1)) == ["M30 0 L35 0 L35 5 L30 5 Z"]
    assert drawn(Drawing(*squares(5)).only(order=0.5)) == ["M20 0 L25 0 L25 5 L20 5 Z"]


def test_two_attributes_must_both_match():
    drawing = Drawing(*squares(3))
    assert drawn(drawing.only(index=1, order=0.5)) == ["M10 0 L15 0 L15 5 L10 5 Z"]
    assert drawn(drawing.only(index=1, order=1)) == []


def test_only_and_without_split_a_list_into_two_that_stack_back():
    # series_bars.py in miniature: the tallest in a second ink
    bars = Drawing(*squares(4))
    chart = Drawing(bars.without(index=2).fill("#15161a"), bars.only(index=2).fill("#e2412e"))
    assert [shape.style.fill for shape in chart.shapes()] == ["#15161aff"] * 3 + ["#e2412eff"]


# --- shapes ---------------------------------------------------------------------------------------------------


def test_a_stack_gives_each_shape_that_carries_none_an_order_from_0_to_1():
    shapes = Drawing(*squares(3)).shapes()
    assert [dict(shape.attributes) for shape in shapes] == [
        {"index": 0, "order": 0.0},
        {"index": 1, "order": 0.5},
        {"index": 2, "order": 1.0},
    ]
    assert [shape.attributes["index"] for shape in shapes] == [0, 1, 2]


def test_a_shape_keeps_the_index_first_in_its_attributes():
    attributes = Drawing(*squares(2)).shapes()[1].attributes
    assert list(attributes) == ["index", "order"]
    assert list(attributes.items()) == [("index", 1), ("order", 1.0)]
    assert isinstance(attributes["index"], int)
    assert isinstance(attributes["order"], float)


def test_attributes_are_a_read_only_mapping():
    attributes = only_shape(Drawing(SQUARE)).attributes
    assert isinstance(attributes, Mapping)
    with pytest.raises(TypeError):
        attributes["index"] = 3  # type: ignore[index]
    with pytest.raises(KeyError):
        attributes["glyph"]


def test_a_shape_cannot_be_changed():
    shape = only_shape(Drawing(SQUARE))
    with pytest.raises(AttributeError):
        shape.path = LINE  # type: ignore[misc]


def test_shapes_put_back_into_a_drawing_draw_as_they_were():
    original = Drawing(*squares(3)).translate(1, 2).rotate(45).fill("#336699").stroke("#112233", width=3).opacity(0.5)
    shapes = original.shapes()
    assert Drawing(*shapes).shapes() == shapes


def test_a_shape_put_back_alone_draws_as_it_was():
    (shape,) = SQUARE.stroke("#112233", join="bevel").blend("multiply").shapes()
    assert Drawing(shape).shapes() == [shape]


def test_shapes_with_the_same_path_style_and_attributes_are_equal():
    assert Drawing(SQUARE).shapes() == Drawing(SQUARE).shapes()
    assert Drawing(SQUARE).shapes() != Drawing(LINE).shapes()
    assert Drawing(SQUARE).shapes() != SQUARE.fill("#ff0000").shapes()


def test_drawings_of_drawings_and_paths_and_shapes_stack_in_order():
    inner = Drawing(*squares(2))
    (shape,) = SQUARE.fill("#ff0000").shapes()
    stack = Drawing(inner, LINE, shape)
    assert [(s.attributes["index"], s.path.to_svg_path()) for s in stack.shapes()] == [
        (0, "M0 0 L5 0 L5 5 L0 5 Z"),
        (1, "M10 0 L15 0 L15 5 L10 5 Z"),
        (2, "M0 0 L10 0"),
        (3, "M0 0 L10 0 L10 10 L0 10 Z"),
    ]


def test_one_item_alone_draws_as_that_item():
    inner = Drawing(*squares(2)).translate(1, 1)
    assert Drawing(inner).shapes() == inner.shapes()


# --- the design frame -------------------------------------------------------------------------------------------


def test_a_drawing_has_a_size_only_when_it_or_an_item_was_given_one():
    assert Drawing(SQUARE).size is None
    assert Drawing(SQUARE, size=(400, 300)).size == (400.0, 300.0)
    assert Drawing(Drawing(SQUARE, size=(10, 20)), LINE).size == (10.0, 20.0)


def test_a_step_keeps_the_size():
    assert Drawing(SQUARE, size=(400, 300)).translate(1, 1).fill("#000000").size == (400.0, 300.0)


def test_a_stack_of_items_that_agree_on_a_size_has_it():
    framed = Drawing(SQUARE, size=(10, 10))
    assert Drawing(framed, framed.translate(1, 0)).size == (10.0, 10.0)


def test_a_size_given_settles_items_that_disagree():
    first, second = Drawing(SQUARE, size=(10, 10)), Drawing(SQUARE, size=(20, 20))
    assert Drawing(first, second, size=(30, 40)).size == (30.0, 40.0)


@pytest.mark.parametrize("size", [(100000, 100000), (0.5, 1)])
def test_a_size_at_the_ends_of_its_range_is_accepted(size):
    assert Drawing(SQUARE, size=size).size == size


# --- errors: kind, what they name, and the call that raises them -------------------------------------------------


def a_param() -> btvec.Param:
    return param("dx")


@pytest.mark.parametrize(
    ("call", "names"),
    [
        pytest.param(lambda: Drawing(SQUARE).fill("red"), "color", id="fill: a colour name"),
        pytest.param(lambda: Drawing(SQUARE).fill("#fff"), "color", id="fill: three digits"),
        pytest.param(lambda: Drawing(SQUARE).fill(""), "color", id="fill: empty"),
        pytest.param(lambda: Drawing(SQUARE).stroke("#12"), "color", id="stroke: a short colour"),
        pytest.param(lambda: Drawing(SQUARE).stroke("none"), "color", id="stroke: none paints nothing"),
        pytest.param(lambda: Drawing(SQUARE).stroke("#000000", width=0), "width", id="stroke: width zero"),
        pytest.param(lambda: Drawing(SQUARE).stroke("#000000", width=-1), "width", id="stroke: negative width"),
        pytest.param(
            lambda: Drawing(SQUARE).stroke("#000000", miter_limit=0.5), "miter_limit", id="stroke: miter below 1"
        ),
        pytest.param(lambda: Drawing(SQUARE).stroke("#000000", join="sharp"), "join", id="stroke: another join"),
        pytest.param(lambda: Drawing(SQUARE).stroke("#000000", cap="flat"), "cap", id="stroke: another cap"),
        pytest.param(lambda: Drawing(SQUARE).opacity(1.5), "alpha", id="opacity: above 1"),
        pytest.param(lambda: Drawing(SQUARE).opacity(-0.1), "alpha", id="opacity: below 0"),
        pytest.param(lambda: Drawing(SQUARE).blend("screen"), "mode", id="blend: another mode"),
        pytest.param(lambda: Drawing(SQUARE).translate(float("nan"), 0), "dx", id="translate: NaN"),
        pytest.param(lambda: Drawing(SQUARE).translate(0, float("inf")), "dy", id="translate: infinite"),
        pytest.param(lambda: Drawing(SQUARE).translate(1e101, 0), "dx", id="translate: beyond 1e100"),
        pytest.param(lambda: Drawing(SQUARE).rotate(float("nan")), "angle", id="rotate: NaN"),
        pytest.param(lambda: Drawing(SQUARE).scale(0), "by", id="scale: zero"),
        pytest.param(lambda: Drawing(SQUARE).scale((2, 0)), "by", id="scale: a zero component"),
        pytest.param(lambda: Drawing(SQUARE).only(), "attributes", id="only: nothing asked"),
        pytest.param(lambda: Drawing(SQUARE).without(), "attributes", id="without: nothing asked"),
        pytest.param(lambda: Drawing(SQUARE).only(colour=1), "colour", id="only: not an attribute"),
        pytest.param(lambda: Drawing(SQUARE).only(index=-1), "index", id="only: a negative index"),
        pytest.param(lambda: Drawing(SQUARE).only(index=1.5), "index", id="only: a fractional index"),
        pytest.param(lambda: Drawing(SQUARE).only(index="a"), "index", id="only: a character for a number"),
        pytest.param(lambda: Drawing(SQUARE).only(char="ab"), "char", id="only: two characters"),
        pytest.param(lambda: Drawing(SQUARE).with_values(nope=1), "nope", id="with_values: not a Param"),
        pytest.param(
            lambda: Drawing(SQUARE).translate(a_param(), 0).with_values(dx=1000), "dx", id="with_values: out of range"
        ),
        pytest.param(lambda: Drawing(SQUARE, size=(0, 10)), "size", id="size: zero"),
        pytest.param(lambda: Drawing(SQUARE, size=(10, -1)), "size", id="size: negative"),
        pytest.param(lambda: Drawing(SQUARE, size=(100001, 10)), "size", id="size: above 100,000"),
        pytest.param(lambda: Drawing(SQUARE, size=(float("nan"), 10)), "size", id="size: NaN"),
        pytest.param(
            lambda: Drawing(Drawing(SQUARE, size=(10, 10)), Drawing(SQUARE, size=(20, 20))),
            "size",
            id="items that disagree on a size",
        ),
    ],
)
def test_an_argument_outside_what_a_step_accepts_raises_a_parameter_error_at_the_call(call, names):
    with pytest.raises(ParameterError, match=names):
        call()


def test_the_first_error_of_a_chain_is_raised_by_the_call_that_made_it():
    drawing = Drawing(SQUARE)
    with pytest.raises(ParameterError, match="color"):
        drawing.fill("red")
    # the drawing the call was made on is still good, and the later steps were never reached
    assert drawn(drawing) == ["M0 0 L10 0 L10 10 L0 10 Z"]


def test_a_colour_error_shows_the_text_given():
    with pytest.raises(ParameterError, match=r"'red'|\"red\""):
        Drawing(SQUARE).fill("red")


@pytest.mark.parametrize(
    "call",
    [
        pytest.param(lambda: SQUARE.fill("red"), id="fill"),
        pytest.param(lambda: SQUARE.stroke("none"), id="stroke colour"),
        pytest.param(lambda: SQUARE.stroke("#000000", width=0), id="stroke width"),
        pytest.param(lambda: SQUARE.opacity(2), id="opacity"),
        pytest.param(lambda: SQUARE.blend("screen"), id="blend"),
    ],
)
def test_a_style_step_called_on_a_path_raises_at_the_call_too(call):
    with pytest.raises(ParameterError):
        call()


@pytest.mark.parametrize(
    "call",
    [
        pytest.param(lambda p: SQUARE.translate(p, 0), id="translate dx"),
        pytest.param(lambda p: SQUARE.translate(0, p), id="translate dy"),
        pytest.param(lambda p: SQUARE.translate(2 * p + 1, 0), id="translate with an expression"),
        pytest.param(lambda p: SQUARE.rotate(p), id="rotate"),
        pytest.param(lambda p: SQUARE.rotate(30, about=(p, 0)), id="rotate about"),
        pytest.param(lambda p: SQUARE.scale(p), id="scale"),
        pytest.param(lambda p: SQUARE.scale((p, 1)), id="scale by a pair"),
        pytest.param(lambda p: SQUARE.scale(2, about=(0, p)), id="scale about"),
    ],
)
def test_a_path_verb_given_a_param_says_to_wrap_the_path_in_a_drawing(call):
    with pytest.raises(ParameterError, match="Drawing"):
        call(a_param())


def test_a_path_verb_still_takes_plain_numbers():
    assert SQUARE.translate(1, 2).to_svg_path() == "M1 2 L11 2 L11 12 L1 12 Z"
    assert SQUARE.rotate(90).to_svg_path() == "M10 0 L10 10 L0 10 L0 0 Z"
    assert SQUARE.scale(2).to_svg_path() == "M-5 -5 L15 -5 L15 15 L-5 15 Z"


@pytest.mark.parametrize(
    "call",
    [
        pytest.param(lambda: LOOSE(5), id="a number as an item"),
        pytest.param(lambda: LOOSE("M0 0"), id="text as an item"),
        pytest.param(lambda: LOOSE([SQUARE]), id="a list as an item"),
        pytest.param(lambda: LOOSE(SQUARE, size=10), id="a size that is not a pair"),
        pytest.param(lambda: FRAMELESS.fill(3), id="a colour that is a number"),
        pytest.param(lambda: FRAMELESS.stroke(None), id="a stroke colour that is None"),
        pytest.param(lambda: FRAMELESS.translate("a", 0), id="a number that is text"),
        pytest.param(lambda: FRAMELESS.translate(None, 0), id="a number that is None"),
        pytest.param(lambda: FRAMELESS.rotate(30, about=5), id="a pivot that is a number"),
        pytest.param(lambda: FRAMELESS.scale("2"), id="a scale that is text"),
        pytest.param(lambda: FRAMELESS.only(index=None), id="a wanted value that is None"),
        pytest.param(lambda: FRAMELESS.with_values(dx="a"), id="a value that is text"),
        pytest.param(lambda: FRAMELESS.shapes(dx="a"), id="a drawn value that is text"),
        pytest.param(lambda: FRAMELESS.shapes(dx=["a"]), id="a drawn list of text"),
    ],
)
def test_an_argument_of_the_wrong_type_is_a_type_error(call):
    with pytest.raises(TypeError):
        call()


@pytest.mark.parametrize(
    ("build", "values", "names"),
    [
        pytest.param(lambda p: Drawing(SQUARE).translate(p, 0), {"dx": 1000}, "dx", id="a value above the range"),
        pytest.param(lambda p: Drawing(SQUARE).translate(p, 0), {"dx": float("nan")}, "dx", id="a value that is NaN"),
        pytest.param(lambda p: Drawing(SQUARE).translate(p, 0), {"dz": 1}, "dz", id="a name no Param has"),
        pytest.param(
            lambda p: Drawing(SQUARE).translate(p, 0), {"dx": [1, 2]}, "dx", id="a list for one shape too long"
        ),
        pytest.param(lambda p: Drawing(*squares(3)).translate(p, 0), {"dx": [1, 2]}, "dx", id="a list too short"),
        pytest.param(lambda p: Drawing(SQUARE).translate(p, 0), {"dx": []}, "dx", id="an empty list"),
        pytest.param(
            lambda p: Drawing(*squares(2)).translate(p, 0), {"dx": [1, 1000]}, "dx", id="a list entry out of range"
        ),
        pytest.param(lambda p: Drawing(SQUARE).translate(1 / (p - p), 0), {}, "dx", id="division by zero"),
        pytest.param(
            lambda p: Drawing(SQUARE).opacity(p), {"dx": 5}, "alpha", id="an expression outside the argument's range"
        ),
        pytest.param(lambda p: Drawing(SQUARE).scale(p), {"dx": 0}, "by", id="a Param that makes a scale zero"),
    ],
)
def test_a_value_outside_what_is_accepted_raises_a_parameter_error_when_drawn(build, values, names):
    drawing = build(a_param())
    with pytest.raises(ParameterError, match=names):
        drawing.shapes(**values)


def test_an_error_while_drawing_names_the_step_and_the_argument():
    dx = param("dx", high=10)
    drawing = Drawing(SQUARE).fill("#000000").translate(dx, 0)
    with pytest.raises(ParameterError, match=r"dx.*(10|range)"):
        drawing.shapes(dx=50)
    # a number a Param makes outside what the step accepts names the step and the argument
    with pytest.raises(ParameterError, match=r"step 2 \(opacity\)\.alpha"):
        Drawing(SQUARE).fill("#000000").opacity(dx).shapes(dx=2)


@pytest.mark.parametrize("bad", [1000, float("nan"), float("inf")])
def test_a_value_out_of_range_is_refused_even_where_the_step_that_reads_it_sees_no_shapes(bad):
    dx = param("dx", high=10)
    nothing = Drawing(*squares(2)).only(index=5).translate(dx, 0)
    assert nothing.shapes() == []
    with pytest.raises(ParameterError, match="dx"):
        nothing.shapes(dx=bad)
    with pytest.raises(ParameterError, match="dx"):
        nothing.shapes(dx=[1, bad])


def test_two_params_of_one_name_that_differ_raise_a_parameter_error():
    first = Param("p", default=1, min=0, max=2)
    other = Param("p", default=1, min=0, max=3)
    with pytest.raises(ParameterError, match="p"):
        Drawing(SQUARE).translate(first, 0).translate(other, 0).parameters()


def test_a_parameter_error_from_a_step_is_a_value_error_and_a_package_error():
    with pytest.raises(btvec.BullterrierVectorError) as raised:
        Drawing(SQUARE).fill("red")
    assert isinstance(raised.value, ValueError)


# --- every registered verb has a row --------------------------------------------------------------------------------

ROOT = FilePath(__file__).resolve().parents[1]


def registered_verbs() -> set[str]:
    """The names of the verbs the Rust core registers, read from their declarations (`static X: Verb = Verb {..}`)."""
    found: set[str] = set()
    for source in (ROOT / "crates" / "core" / "src" / "node").glob("*.rs"):
        found |= set(re.findall(r'static \w+: Verb = Verb \{\s*name: "(\w+)"', source.read_text(encoding="utf-8")))
    return found


def summary(drawing: btvec.Drawing, **values: Any) -> list[tuple[str, str | None, str | None, float, str]]:
    """What each shape shows: path data, fill, stroke, opacity, blend."""
    return [
        (shape.path.to_svg_path(), shape.style.fill, shape.style.stroke, shape.style.opacity, shape.style.blend)
        for shape in drawing.shapes(**values)
    ]


RECT = "M0 0 L10 0 L10 10 L0 10 Z"
SMALL = ("M0 0 L5 0 L5 5 L0 5 Z", "M10 0 L15 0 L15 5 L10 5 Z")
OPEN = "M0 0 L10 0"


def row(path: str, fill: str | None = BLACK, stroke: str | None = None, opacity: float = 1.0, blend: str = "normal"):
    return (path, fill, stroke, opacity, blend)


VERB_ROWS: dict[str, tuple[Callable[[], btvec.Drawing], dict[str, Any], list[Any]]] = {
    "path": (lambda: Drawing(SQUARE), {}, [row(RECT)]),
    "shape": (lambda: Drawing(*SQUARE.fill("#ff0000").shapes()), {}, [row(RECT, "#ff0000ff")]),
    "stack": (lambda: Drawing(*squares(2)), {}, [row(SMALL[0]), row(SMALL[1])]),
    "translate": (lambda: Drawing(SQUARE).translate(1, 2), {}, [row("M1 2 L11 2 L11 12 L1 12 Z")]),
    "rotate": (lambda: Drawing(SQUARE).rotate(90), {}, [row("M10 0 L10 10 L0 10 L0 0 Z")]),
    "scale": (lambda: Drawing(SQUARE).scale(2), {}, [row("M-5 -5 L15 -5 L15 15 L-5 15 Z")]),
    "fill": (lambda: Drawing(SQUARE).fill("#0000ff"), {}, [row(RECT, "#0000ffff")]),
    "stroke": (lambda: Drawing(SQUARE).stroke("#00ff00"), {}, [row(RECT, None, "#00ff00ff")]),
    "opacity": (lambda: Drawing(SQUARE).opacity(0.5), {}, [row(RECT, opacity=0.5)]),
    "blend": (lambda: Drawing(SQUARE).blend("multiply"), {}, [row(RECT, blend="multiply")]),
    "only": (lambda: Drawing(*squares(2)).only(index=1), {}, [row(SMALL[1])]),
    "without": (lambda: Drawing(*squares(2)).without(index=1), {}, [row(SMALL[0])]),
    "with_values": (
        lambda: Drawing(SQUARE).translate(param("dx"), 0).with_values(dx=3),
        {},
        [row("M3 0 L13 0 L13 10 L3 10 Z")],
    ),
}


def test_the_table_has_a_row_for_every_registered_verb_and_no_other():
    # a verb added to the core without a row here fails by its name
    assert set(VERB_ROWS) == registered_verbs()


@pytest.mark.parametrize("verb", sorted(VERB_ROWS))
def test_each_verb_draws_what_its_row_says(verb):
    build, values, expected = VERB_ROWS[verb]
    assert summary(build(), **values) == expected


def test_drawing_shape_and_style_read_in_a_repr():
    drawing = Drawing(SQUARE, size=(40, 30))
    assert repr(drawing) == "Drawing(size=(40.0, 30.0))"
    assert repr(Drawing(SQUARE)) == "Drawing(size=None)"
    shape = drawing.fill("#ff0000").shapes()[0]
    assert repr(shape.style) == "Style(fill='#ff0000ff', stroke=None, stroke_width=None, opacity=1.0, blend='normal')"
    assert repr(shape) == f"Shape(index=0, path={shape.path!r}, style={shape.style!r})"


def test_a_refused_selection_value_reads_as_the_caller_wrote_it():
    with pytest.raises(btvec.ParameterError, match=r"got 'a'"):
        Drawing(SQUARE).only(index="a")
    with pytest.raises(btvec.ParameterError, match=r"one character, got 5"):
        Drawing(SQUARE).only(char=5)


def test_a_value_for_a_param_that_was_already_fixed_says_so():
    drawing = Drawing(SQUARE).translate(param("x"), 0).with_values(x=1)
    with pytest.raises(btvec.ParameterError, match=r"with_values.*already fixed"):
        drawing.with_values(x=2)
