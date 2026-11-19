"""`morph` through the public package: an outline carried to another, on `Path` and as a step of a `Drawing`. Expected
values are by hand: squares and lines with the same start morph by the straight lerp of their corners, a hole with no
partner shrinks to the point it has in the other shape's box, shapes pair by index."""

import math

import pytest

import bullterrier_vector as btvec

SQUARE = btvec.Path.rect(0, 0, 10, 10)
DOUBLE = btvec.Path.rect(0, 0, 20, 20)
FONT = btvec.Font.bundled("bricolage_grotesque")


def vertices(path: btvec.Path) -> list[tuple[float, float]]:
    """The corners of a path made of straight lines."""
    numbers = path.to_svg_path().replace("M", " ").replace("L", " ").replace("Z", " ").split()
    return [(float(x), float(y)) for x, y in zip(numbers[::2], numbers[1::2], strict=True)]


def box(path: btvec.Path) -> tuple[float, float, float, float]:
    b = path.bounds
    return (b.x, b.y, b.width, b.height)


def paths(drawing: btvec.Drawing, **values: float) -> list[btvec.Path]:
    return [shape.path for shape in drawing.shapes(**values)]


def two(first: btvec.Path, second: btvec.Path, colours: tuple[str, str]) -> btvec.Drawing:
    return btvec.Drawing(*first.fill(colours[0]).shapes(), *second.fill(colours[1]).shapes())


# --- Path.morph ---------------------------------------------------------------------------------------------------


def test_half_way_from_a_square_to_one_twice_as_wide_is_the_square_of_one_and_a_half() -> None:
    half = SQUARE.morph(DOUBLE, 0.5)
    assert vertices(half) == [(0, 0), (15, 0), (15, 15), (0, 15)]
    assert half.to_svg_path().endswith("Z")


def test_the_ends_are_the_two_outlines_and_outside_them_the_lerp_goes_on() -> None:
    assert box(SQUARE.morph(DOUBLE, 0)) == (0, 0, 10, 10)
    assert box(SQUARE.morph(DOUBLE, 1)) == (0, 0, 20, 20)
    assert box(SQUARE.morph(DOUBLE, 1.5)) == (0, 0, 25, 25)
    assert box(SQUARE.morph(DOUBLE, -0.5)) == (0, 0, 5, 5)


def test_a_square_carried_to_another_place_moves_straight() -> None:
    far = btvec.Path.rect(100, 40, 10, 10)
    assert box(SQUARE.morph(far, 0.25)) == (25, 10, 10, 10)


def test_open_lines_stay_open_and_morph_end_to_end() -> None:
    low = btvec.Path.line((0, 0), (10, 0))
    high = btvec.Path.line((0, 10), (10, 20))
    assert low.morph(high, 0.5).to_svg_path() == "M0 5 L10 10"
    assert low.morph(high, 1).to_svg_path() == "M0 10 L10 20"


def test_a_hole_with_no_partner_shrinks_to_its_place_in_the_other_shape_and_never_paints_outside() -> None:
    ring = btvec.Path.rect(0, 0, 100, 100).subtract(btvec.Path.rect(25, 25, 50, 50))
    solid = btvec.Path.rect(0, 0, 100, 100)
    assert not ring.contains((50, 50))
    half = ring.morph(solid, 0.5)  # the hole is the square of 25 about (50, 50)
    assert not half.contains((50, 50))
    assert half.contains((30, 50))
    assert half.contains((70, 50))
    assert half.contains((50, 30))
    assert box(half) == (0, 0, 100, 100)
    # at 0.9 the hole is the square of 5 and everything else is fill
    nearly = ring.morph(solid, 0.9)
    assert not nearly.contains((50, 50))
    assert nearly.contains((45, 50))
    assert nearly.contains((10, 10))
    assert ring.morph(solid, 1).contains((50, 50))


def test_an_outer_with_no_partner_shrinks_to_its_own_center_when_the_other_is_empty() -> None:
    assert box(SQUARE.morph(btvec.Path(), 0.5)) == (2.5, 2.5, 5, 5)
    assert box(btvec.Path().morph(SQUARE, 0.5)) == (2.5, 2.5, 5, 5)


def test_morphing_back_from_the_other_end_fills_the_same_region() -> None:
    forward = SQUARE.morph(DOUBLE, 0.3)
    back = DOUBLE.morph(SQUARE, 0.7)
    assert box(forward) == pytest.approx(box(back))
    assert [c for xy in sorted(vertices(forward)) for c in xy] == pytest.approx(
        [c for xy in sorted(vertices(back)) for c in xy]
    )


@pytest.mark.parametrize("fraction", [math.nan, math.inf, -math.inf])
def test_a_fraction_that_is_not_a_number_is_refused_naming_it(fraction: float) -> None:
    with pytest.raises(btvec.ParameterError, match="fraction"):
        SQUARE.morph(DOUBLE, fraction)


def test_a_result_beyond_the_limit_of_a_coordinate_is_refused_naming_the_fraction() -> None:
    with pytest.raises(btvec.ParameterError, match="fraction"):
        SQUARE.morph(DOUBLE, 1e300)


def test_both_arguments_are_required_and_the_target_is_a_path() -> None:
    with pytest.raises(TypeError):
        SQUARE.morph(DOUBLE)  # type: ignore[call-arg]  # the misuse under test
    with pytest.raises(TypeError):
        SQUARE.morph(None, 0.5)  # type: ignore[arg-type]  # the misuse under test


# --- Drawing.morph ------------------------------------------------------------------------------------------------

LEFT, RIGHT = btvec.Path.rect(0, 0, 10, 10), btvec.Path.rect(100, 0, 10, 10)
SOURCE = two(LEFT, RIGHT, ("#ff0000", "#0000ff"))
TARGET = two(btvec.Path.rect(0, 0, 20, 20), btvec.Path.rect(100, 0, 30, 10), ("#00ff00", "#00ff00"))


def test_shapes_pair_by_index_and_the_result_keeps_the_style_of_the_source() -> None:
    moved = SOURCE.morph(TARGET, 0.5).shapes()
    assert [box(shape.path) for shape in moved] == [(0, 0, 15, 15), (100, 0, 20, 10)]
    assert [shape.style.fill for shape in moved] == ["#ff0000ff", "#0000ffff"]


def test_fraction_0_is_the_source_and_1_the_target() -> None:
    assert [box(p) for p in paths(SOURCE.morph(TARGET, 0))] == [(0, 0, 10, 10), (100, 0, 10, 10)]
    assert [box(p) for p in paths(SOURCE.morph(TARGET, 1))] == [(0, 0, 20, 20), (100, 0, 30, 10)]
    assert box(paths(SOURCE.morph(TARGET, 2))[0]) == (0, 0, 30, 30)  # the lerp of each point, beyond 1 as well


def test_the_target_may_be_a_path_or_a_shape() -> None:
    (from_path,) = paths(btvec.Drawing(SQUARE).morph(DOUBLE, 0.5))
    (from_shape,) = paths(btvec.Drawing(SQUARE).morph(DOUBLE.fill("#00ff00").shapes()[0], 0.5))
    assert box(from_path) == box(from_shape) == (0, 0, 15, 15)


def test_a_surplus_source_shape_shrinks_to_its_own_center_and_keeps_its_style() -> None:
    shapes = SOURCE.morph(TARGET.only(index=0), 0.5).shapes()
    assert len(shapes) == 2
    assert box(shapes[1].path) == (102.5, 2.5, 5, 5)
    assert shapes[1].style.fill == "#0000ffff"
    assert box(SOURCE.morph(TARGET.only(index=0), 1).shapes()[1].path) == (105, 5, 0, 0)


def test_a_surplus_target_shape_grows_from_its_own_center_with_its_own_style() -> None:
    shapes = SOURCE.only(index=0).morph(TARGET, 0.5).shapes()
    assert len(shapes) == 2
    assert box(shapes[1].path) == (107.5, 2.5, 15, 5)  # the target's (100, 0, 30, 10), half the size about (115, 5)
    assert shapes[1].style.fill == "#00ff00ff"
    assert box(SOURCE.only(index=0).morph(TARGET, 0).shapes()[1].path) == (115, 5, 0, 0)


def test_a_word_turns_into_a_longer_and_a_shorter_one_glyph_by_glyph() -> None:
    cat, horse = btvec.text("cat", font=FONT, size=200), btvec.text("horse", font=FONT, size=200)
    longer, shorter = cat.morph(horse, 1), horse.morph(cat, 1)
    assert len(longer.shapes()) == 5
    assert len(shorter.shapes()) == 5  # the surplus glyphs shrink to a point, they do not go
    for found, wanted in zip(paths(longer), paths(horse), strict=True):
        assert box(found) == pytest.approx(box(wanted), abs=0.01)
    for found, wanted in zip(paths(shorter)[:3], paths(cat), strict=True):
        assert box(found) == pytest.approx(box(wanted), abs=0.01)
    assert all(box(p)[2:] == (0, 0) for p in paths(shorter)[3:])


# --- per-item fractions -------------------------------------------------------------------------------------------


def test_the_fraction_is_a_param_read_when_the_drawing_is_drawn() -> None:
    t = btvec.Param("t", default=0, min=0, max=1)
    drawing = SOURCE.morph(TARGET, t)
    assert box(paths(drawing)[0]) == (0, 0, 10, 10)
    assert box(paths(drawing, t=0.5)[0]) == (0, 0, 15, 15)
    assert box(paths(drawing, t=1)[1]) == (100, 0, 30, 10)


def test_stagger_gives_each_shape_its_own_fraction() -> None:
    progress = btvec.Param("progress", default=0, min=0, max=1)
    # two shapes: the first runs for progress 0 to 0.5, the second for 0.5 to 1
    drawing = SOURCE.morph(TARGET, btvec.stagger(progress))
    assert [box(p) for p in paths(drawing, progress=0.25)] == [(0, 0, 15, 15), (100, 0, 10, 10)]
    assert [box(p) for p in paths(drawing, progress=0.5)] == [(0, 0, 20, 20), (100, 0, 10, 10)]
    assert [box(p) for p in paths(drawing, progress=1)] == [(0, 0, 20, 20), (100, 0, 30, 10)]


def test_by_index_gives_the_shapes_their_numbers() -> None:
    moved = SOURCE.morph(TARGET, btvec.by_index(0, 1))
    assert [box(p) for p in paths(moved)] == [(0, 0, 10, 10), (100, 0, 30, 10)]


@pytest.mark.parametrize("fraction", [math.nan, math.inf])
def test_a_drawing_refuses_a_fraction_that_is_not_a_number_naming_it(fraction: float) -> None:
    with pytest.raises(btvec.ParameterError, match="fraction"):
        SOURCE.morph(TARGET, fraction)


def test_a_param_fraction_that_sends_a_point_too_far_is_refused_when_drawn() -> None:
    t = btvec.Param("t", default=0, min=0, max=1e300)
    drawing = SOURCE.morph(TARGET, t)
    with pytest.raises(btvec.ParameterError, match="fraction"):
        drawing.shapes(t=1e300)


def test_a_target_that_is_not_a_drawing_is_a_type_error() -> None:
    with pytest.raises(TypeError):
        SOURCE.morph(None, 0.5)  # type: ignore[arg-type]  # the misuse under test


# --- documents ----------------------------------------------------------------------------------------------------


def test_a_morph_round_trips_through_json_target_param_and_stagger_included() -> None:
    progress = btvec.Param("progress", default=0.5, min=0, max=1)
    drawing = SOURCE.morph(TARGET, btvec.stagger(progress))
    text = drawing.to_json()
    assert '"morph"' in text
    loaded = btvec.Drawing.from_json(text)
    assert loaded.to_json() == text
    for p in (0, 0.25, 0.5, 1):
        assert [x.to_svg_path() for x in paths(loaded, progress=p)] == [
            x.to_svg_path() for x in paths(drawing, progress=p)
        ]
