"""`Path`, `Point` and `Rect` through the public package: every call shape of docs/api.md, tuples in and Points out,
the error classes with the values they name, and closed-form oracles written with `math`, not with the library."""

import math
from collections.abc import Callable

import numpy as np
import pytest

import bullterrier_vector as btvec

Path = btvec.Path
Point = btvec.Point
Rect = btvec.Rect

# the unit square, written by hand: SVG path data in the library's canonical form
SQUARE = "M0 0 L10 0 L10 10 L0 10 Z"
# an outline, a hole inside it and an island inside the hole
NESTED = "M0 0 L100 0 L100 100 L0 100 Z M20 20 L20 80 L80 80 L80 20 Z M40 40 L60 40 L60 60 L40 60 Z"


def square() -> btvec.Path:
    return Path.from_svg_path(SQUARE)


def line() -> btvec.Path:
    return Path.line((0, 0), (10, 0))


# --- building ---------------------------------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("built", "written"),
    [
        pytest.param(Path(), "", id="empty"),
        pytest.param(Path().move_to((1, 2)), "M1 2", id="move_to"),
        pytest.param(Path().move_to((0, 0)).line_to((10, 0)), "M0 0 L10 0", id="line_to"),
        pytest.param(Path().move_to((0, 0)).quad_to((5, 5), (10, 0)), "M0 0 Q5 5 10 0", id="quad_to"),
        pytest.param(Path().move_to((0, 0)).curve_to((0, 5), (10, 5), (10, 0)), "M0 0 C0 5 10 5 10 0", id="curve_to"),
        pytest.param(
            Path().move_to((0, 0)).line_to((10, 0)).line_to((10, 10)).close(), "M0 0 L10 0 L10 10 Z", id="close"
        ),
        pytest.param(
            Path().move_to(Point(1.5, 2)).line_to(Point(3, 4)), "M1.5 2 L3 4", id="Points go in as well as tuples"
        ),
        pytest.param(Path().move_to((0, 0)).line_to((1, 0)).move_to((5, 5)), "M0 0 L1 0 M5 5", id="a second sub-path"),
    ],
)
def test_builders_write_the_path_they_describe(built, written):
    assert built.to_svg_path() == written


def test_a_builder_returns_a_new_path_and_leaves_the_old_one_alone():
    start = Path().move_to((0, 0))
    longer = start.line_to((10, 0))
    assert start.to_svg_path() == "M0 0"
    assert longer.to_svg_path() == "M0 0 L10 0"


@pytest.mark.parametrize(
    ("built", "written"),
    [
        pytest.param(Path.line((0, 0), (10, 0)), "M0 0 L10 0", id="line"),
        pytest.param(Path.polyline([(0, 0), (10, 0), (10, 10)]), "M0 0 L10 0 L10 10", id="polyline open"),
        pytest.param(
            Path.polyline([(0, 0), (10, 0), (10, 10)], closed=True), "M0 0 L10 0 L10 10 Z", id="polyline closed"
        ),
        pytest.param(Path.polyline(iter([(0, 0), (1, 0)])), "M0 0 L1 0", id="polyline takes any iterable of points"),
        pytest.param(
            Path.polyline(np.array([(0, 0), (10, 0), (10, 10)], dtype=np.float32)),
            "M0 0 L10 0 L10 10",
            id="polyline takes float32 rows",
        ),
        pytest.param(
            Path.polyline(np.array([(0, 0), (10, 0), (10, 10)], dtype=np.int64), closed=True),
            "M0 0 L10 0 L10 10 Z",
            id="polyline takes int rows",
        ),
        pytest.param(
            Path.polyline([np.array([0.5, 0.0]), [10, 0]]), "M0.5 0 L10 0", id="polyline takes a list of mixed rows"
        ),
        pytest.param(Path().move_to(np.array([1.0, 2.0])).line_to([3, 4]), "M1 2 L3 4", id="a point is any pair"),
        pytest.param(Path.rect(0, 0, 10, 10), SQUARE, id="rect starts at the left end of the top edge"),
        pytest.param(Path.rect(5, 6, 10, 20), "M5 6 L15 6 L15 26 L5 26 Z", id="rect not at the origin"),
    ],
)
def test_constructors_write_the_path_they_describe(built, written):
    assert built.to_svg_path() == written


# --- constructors against closed forms -----------------------------------------------------------------------------


@pytest.mark.parametrize("sides", [3, 4, 5, 12])
def test_a_polygon_has_the_side_length_and_corner_at_the_top_that_geometry_gives(sides):
    radius = 50.0
    polygon = Path.polygon((100, 200), radius, sides)
    assert polygon.length == pytest.approx(sides * 2 * radius * math.sin(math.pi / sides), rel=1e-9)
    assert polygon.point_at(0) == pytest.approx((100, 200 - radius))
    # the first side runs clockwise: its far end is the corner a turn of 2 pi / sides round, on the screen (y down)
    corner = polygon.point_at(1 / sides)
    assert corner.x == pytest.approx(100 + radius * math.sin(2 * math.pi / sides))
    assert corner.y == pytest.approx(200 - radius * math.cos(2 * math.pi / sides))


@pytest.mark.parametrize("radius", [10.0, 100.0, 1234.5])
def test_a_circle_is_round_to_a_few_parts_in_ten_thousand(radius):
    circle = Path.circle((30, -40), radius)
    assert circle.length == pytest.approx(2 * math.pi * radius, rel=1e-3)
    for at in (0, 0.1, 0.25, 0.333, 0.5, 0.77, 0.9):
        point = circle.point_at(at)
        assert math.hypot(point.x - 30, point.y + 40) == pytest.approx(radius, rel=5e-4)
    assert circle.point_at(0) == pytest.approx((30, -40 - radius))


def test_a_circle_bounds_are_its_diameter_square():
    bounds = Path.circle((30, -40), 10).bounds
    assert (bounds.x, bounds.y, bounds.width, bounds.height) == pytest.approx((20, -50, 20, 20))


def test_a_star_is_made_of_the_chords_the_law_of_cosines_gives():
    outer, inner, tips = 100.0, 40.0, 5
    star = Path.star((0, 0), outer, inner, points=tips)
    chord = math.sqrt(outer**2 + inner**2 - 2 * outer * inner * math.cos(math.pi / tips))
    assert star.length == pytest.approx(2 * tips * chord, rel=1e-9)
    assert star.point_at(0) == pytest.approx((0, -outer))


def test_a_star_defaults_to_five_points():
    assert Path.star((0, 0), 100, 40) == Path.star((0, 0), 100, 40, points=5)


def test_a_rounded_rectangle_loses_four_corners_and_gains_four_quarter_circles():
    width, height, radius = 40.0, 30.0, 5.0
    rounded = Path.rect(0, 0, width, height, radius=radius)
    expected = 2 * (width + height) - 8 * radius + 2 * math.pi * radius
    assert rounded.length == pytest.approx(expected, rel=1e-3)


def test_a_radius_of_zero_is_the_plain_rectangle():
    assert Path.rect(0, 0, 10, 10, radius=0) == Path.rect(0, 0, 10, 10)


# --- SVG path data -----------------------------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("data", "written"),
    [
        pytest.param("M0 0 L10 0", "M0 0 L10 0", id="canonical"),
        pytest.param("m1 1 l9 0 v10 h-9 z", "M1 1 L10 1 L10 11 L1 11 Z", id="relative, H and V, lower case"),
        pytest.param("M0,0 10,0 10,10", "M0 0 L10 0 L10 10", id="implicit line_to after M, commas"),
        pytest.param("M0 0 Q5 5 10 0 T20 0", "M0 0 Q5 5 10 0 Q15 -5 20 0", id="T reflects the control point"),
        pytest.param("M0 0 C0 5 10 5 10 0 S20 -5 20 0", "M0 0 C0 5 10 5 10 0 C10 -5 20 -5 20 0", id="S reflects"),
        pytest.param("M1.5.5L-.5 1e1", "M1.5 0.5 L-0.5 10", id="number forms"),
        pytest.param("", "", id="nothing is the empty path"),
    ],
)
def test_svg_path_data_reads_into_the_canonical_path(data, written):
    assert Path.from_svg_path(data).to_svg_path() == written


def test_an_arc_is_a_curve_that_stays_on_its_circle():
    # a half circle of radius 5 from (0,0) to (10,0), the sweep flag choosing the upper half
    half = Path.from_svg_path("M0 0 A5 5 0 0 1 10 0")
    assert half.length == pytest.approx(5 * math.pi, rel=1e-3)
    top = half.point_at(0.5)
    assert top == pytest.approx((5, -5), abs=1e-3)


@pytest.mark.parametrize(
    "path",
    [
        pytest.param(Path.circle((1 / 3, 2 / 3), 0.1 + 0.2), id="circle with long decimals"),
        pytest.param(Path.star((0, 0), 100, 40), id="star"),
        pytest.param(Path.rect(0, 0, 10, 10, radius=3), id="rounded rectangle"),
        pytest.param(Path.from_svg_path(NESTED), id="three sub-paths"),
        pytest.param(Path.from_svg_path("M0 0 Q5 5 10 0 L0 0 M-0 1e-9 L1e21 0"), id="tiny and huge numbers"),
    ],
)
def test_path_data_written_and_read_back_is_the_same_path(path):
    assert Path.from_svg_path(path.to_svg_path()) == path


# --- measuring ---------------------------------------------------------------------------------------------------


def test_length_is_a_property_in_design_pixels():
    assert square().length == 40
    assert Path().length == 0


def test_bounds_is_a_rect_of_the_outline_and_exact_for_curves():
    assert square().bounds == Rect(0, 0, 10, 10)
    # a cubic from (0,0) to (10,0) with both handles at height 10 peaks at 3/4 of 10 (t = 1/2: 3/4 * h)
    arch = Path().move_to((0, 0)).curve_to((0, 10), (10, 10), (10, 0))
    bounds = arch.bounds
    assert (bounds.x, bounds.y, bounds.width) == (0, 0, 10)
    assert bounds.height == pytest.approx(7.5)


def test_point_at_gives_points_along_the_length():
    assert line().point_at(0) == Point(0, 0)
    assert line().point_at(0.25) == Point(2.5, 0)
    assert line().point_at(1) == Point(10, 0)
    assert isinstance(line().point_at(0.5), Point)


def test_point_at_walks_the_sub_paths_in_order():
    two = Path.line((0, 0), (10, 0)).move_to((0, 5)).line_to((10, 5))
    assert two.point_at(0.25) == Point(5, 0)
    assert two.point_at(0.75) == Point(5, 5)


def test_tangent_at_is_the_unit_direction_of_travel():
    assert line().tangent_at(0.5) == Point(1, 0)
    assert Path.line((0, 0), (0, 7)).tangent_at(0.5) == Point(0, 1)
    diagonal = Path.line((0, 0), (3, 4)).tangent_at(0.5)
    assert diagonal == pytest.approx((0.6, 0.8))


def test_tangent_at_a_corner_is_the_direction_leaving_it():
    # the square's first corner (10, 0) is a quarter of the way along; the next side runs down
    assert square().tangent_at(0.25) == Point(0, 1)


def test_locate_is_the_position_of_the_nearest_point():
    assert line().locate((2.5, 100)) == 0.25
    assert line().locate((-5, 3)) == 0
    assert line().locate(Point(50, 0)) == 1


def test_locate_on_a_tie_is_the_smallest_position_and_a_closed_start_is_zero():
    # the point (5, 5) is 5 from every side of the square: the first side, the smallest position, wins
    assert square().locate((5, 5)) == pytest.approx(0.125)
    assert square().locate((0, 0)) == 0


def test_near_moves_the_start_of_a_closed_outline_to_its_point_nearest_the_given_one():
    assert square().point_at(0) == Point(0, 0)
    assert square().point_at(0, near=(11, 11)) == Point(10, 10)
    assert square().point_at(0, near=Point(11, 11)) == Point(10, 10)
    assert square().tangent_at(0, near=(11, 11)) == Point(-1, 0)
    assert square().locate((10, 10), near=(11, 11)) == 0
    assert square().slice(0, 0.25, near=(11, 11)).to_svg_path() == "M10 10 L0 10"


# --- hit-testing and parts -----------------------------------------------------------------------------------------


def test_contains_agrees_with_the_distance_to_the_centre_of_a_circle():
    centre, radius = (3.0, -2.0), 10.0
    circle = Path.circle(centre, radius)
    checked = 0
    for i in range(-14, 15):
        for j in range(-14, 15):
            point = (centre[0] + i * 0.97, centre[1] + j * 0.91)
            distance = math.hypot(point[0] - centre[0], point[1] - centre[1])
            if abs(distance - radius) > 0.1:
                assert circle.contains(point) == (distance < radius), point
                checked += 1
    assert checked > 500


def test_contains_follows_the_nonzero_rule_so_a_hole_is_outside():
    nested = Path.from_svg_path("M0 0 L100 0 L100 100 L0 100 Z M20 20 L20 80 L80 80 L80 20 Z")
    assert nested.contains((10, 10))
    assert not nested.contains((50, 50))
    assert not nested.contains(Point(200, 50))


def test_split_gives_each_outline_with_its_holes_and_nests_by_parity():
    parts = Path.from_svg_path(NESTED).split()
    assert isinstance(parts, list)
    assert [part.length for part in parts] == [400 + 240, 80]
    assert parts[0].bounds == Rect(0, 0, 100, 100)
    assert parts[1].bounds == Rect(40, 40, 20, 20)


def test_split_gives_each_open_line_alone_and_an_empty_path_no_parts():
    two_lines = line().move_to((0, 5)).line_to((10, 5))
    assert [part.to_svg_path() for part in two_lines.split()] == ["M0 0 L10 0", "M0 5 L10 5"]
    assert Path().split() == []


# --- cutting and turning -------------------------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("start", "end", "written"),
    [
        pytest.param(0.25, 0.75, "M2.5 0 L7.5 0", id="the middle"),
        pytest.param(0, 1, "M0 0 L10 0", id="all of it"),
        pytest.param(0.5, 0.5, "M5 0", id="start equal to end is a lone start"),
    ],
)
def test_slice_keeps_the_piece_between_two_positions(start, end, written):
    assert line().slice(start, end).to_svg_path() == written


def test_slice_of_a_whole_closed_outline_is_closed_again():
    assert square().slice(0, 1) == square()


def test_slice_keeps_curves_as_curves_and_has_the_length_the_positions_say():
    circle = Path.circle((0, 0), 10)
    piece = circle.slice(0.2, 0.7)
    assert "C" in piece.to_svg_path()
    assert piece.length == pytest.approx(0.5 * circle.length, rel=1e-3)


def test_reverse_runs_the_path_the_other_way():
    assert line().reverse().to_svg_path() == "M10 0 L0 0"
    assert square().reverse().reverse() == square()
    arch = Path.circle((0, 0), 10).slice(0.1, 0.8)
    start, end = arch.point_at(0.3), arch.reverse().point_at(0.7)
    assert start == pytest.approx(tuple(end), abs=1e-9)


def test_translate_moves_by_dx_and_dy():
    assert line().translate(5, -2).to_svg_path() == "M5 -2 L15 -2"
    assert line().translate(dx=1, dy=1).to_svg_path() == "M1 1 L11 1"


@pytest.mark.parametrize(
    ("angle", "about", "written"),
    [
        pytest.param(90, (0, 0), "M0 0 L0 10", id="a quarter turn clockwise on the screen is exact"),
        pytest.param(180, (5, 0), "M10 0 L0 0", id="a half turn about a point"),
        pytest.param(-90, (0, 0), "M0 0 L0 -10", id="negative turns the other way"),
        pytest.param(450, (0, 0), "M0 0 L0 10", id="more than a turn wraps"),
        pytest.param(90, None, "M5 -5 L5 5", id="about the middle of the bounds by default"),
        pytest.param(90, Point(0, 0), "M0 0 L0 10", id="about takes a Point"),
    ],
)
def test_rotate_turns_clockwise_in_degrees(angle, about, written):
    turned = line().rotate(angle) if about is None else line().rotate(angle, about=about)
    assert turned.to_svg_path() == written


def test_rotating_by_an_angle_and_back_returns_the_path():
    turned = Path.star((10, 20), 50, 20).rotate(37.5, about=(10, 20)).rotate(-37.5, about=(10, 20))
    original = Path.star((10, 20), 50, 20)
    assert turned.length == pytest.approx(original.length, rel=1e-9)
    assert turned.bounds.width == pytest.approx(original.bounds.width, abs=1e-9)
    assert turned.point_at(0.3) == pytest.approx(tuple(original.point_at(0.3)), abs=1e-9)


@pytest.mark.parametrize(
    ("by", "about", "written"),
    [
        pytest.param(2, (0, 0), "M0 0 L20 0", id="a number"),
        pytest.param(2, None, "M-5 0 L15 0", id="about the middle by default"),
        pytest.param((2, 3), (0, 0), "M0 0 L20 0", id="an (x, y) pair"),
        pytest.param(Point(1, -1), (0, 5), "M0 10 L10 10", id="a Point pair mirrors where negative"),
        pytest.param(-1, (5, 0), "M10 0 L0 0", id="a negative number mirrors"),
    ],
)
def test_scale_by_a_number_or_a_pair_about_a_point(by, about, written):
    scaled = line().scale(by) if about is None else line().scale(by, about=about)
    assert scaled.to_svg_path() == written


def test_scale_by_a_pair_scales_each_direction_on_its_own():
    assert square().scale((2, 0.5), about=(0, 0)).bounds == Rect(0, 0, 20, 5)


# --- map_points --------------------------------------------------------------------------------------------------


EAST = Path.line((10, 20), (110, 20))  # 100 long, running east along y = 20
RING = Path.circle((0, 0), 100)  # clockwise from the top


@pytest.mark.parametrize(
    ("guide", "start", "shape", "written"),
    [
        pytest.param(EAST, 0, "M0 0 L10 0", "M10 20 L20 20", id="a guide's own start"),
        pytest.param(EAST, 0.25, "M0 0 L10 0", "M35 20 L45 20", id="start is a fraction of the length"),
        pytest.param(EAST, 0, "M0 0 L0 5", "M10 20 L10 25", id="y goes to the right of the way, down for east"),
        pytest.param(EAST, 0, "M0 0 L0 -5", "M10 20 L10 15", id="a negative y goes left"),
        pytest.param(Path.line((7, 1), (7, 129)), 0.5, "M0 0 L16 -2", "M7 65 L9 81", id="south, whose right is west"),
        pytest.param(EAST, -0.5, "M0 0 L10 0", "M-40 20 L-30 20", id="before the start of an open guide, straight on"),
        pytest.param(EAST, 1.5, "M0 0 L10 0", "M160 20 L170 20", id="past the end of an open guide, straight on"),
        pytest.param(EAST, 0, "M0 0 L10 0 Z", "M10 20 L20 20 Z", id="a closed shape stays closed"),
    ],
)
def test_along_carries_x_along_the_guide_and_y_to_its_right(guide, start, shape, written):
    assert Path.from_svg_path(shape).along(guide, start=start).to_svg_path() == written


def test_along_a_circle_puts_a_point_at_the_angle_the_distance_makes_and_y_moves_the_radius():
    # a clockwise circle of 100 from the top: 157.08 along is a quarter turn, at (100, 0); a y of 10 is 10 nearer
    # the centre
    bent = Path.line((0, 10), (100, 10)).along(RING, start=0.25)
    for at, angle in [(0, math.pi / 2), (1, math.pi / 2 + 1.0)]:
        point = bent.point_at(at)
        assert (point.x, point.y) == pytest.approx((90 * math.sin(angle), -90 * math.cos(angle)), abs=0.1)


def test_along_a_closed_guide_wraps_and_near_moves_where_it_starts():
    shape = Path.line((0, 0), (30, 0))
    start = shape.along(RING, start=0.25).point_at(0)
    for wrapped in (1.25, -0.75):
        assert shape.along(RING, start=wrapped).point_at(0) == pytest.approx((start.x, start.y), abs=1e-6)
    # nearest the bottom, it starts at the bottom
    bottom = shape.along(RING, near=(0, 1000)).point_at(0)
    assert (bottom.x, bottom.y) == pytest.approx((0, 100), abs=0.1)
    assert shape.along(RING, near=Point(0, 1000)).point_at(0) == bottom


# --- booleans ---------------------------------------------------------------------------------------------------

BOX = Path.rect(0, 0, 10, 10)
OVERLAPPING = Path.rect(5, 5, 10, 10)  # shares a 5 by 5 corner with BOX
DISJOINT = Path.rect(50, 50, 1, 1)
# written by hand in the canonical form: outers clockwise, holes counter-clockwise, from the topmost-leftmost point
BOOLEANS = {
    "union": "M0 0 L10 0 L10 5 L15 5 L15 15 L5 15 L5 10 L0 10 Z",
    "subtract": "M0 0 L10 0 L10 5 L5 5 L5 10 L0 10 Z",
    "intersect": "M5 5 L10 5 L10 10 L5 10 Z",
    "exclude": "M0 0 L10 0 L10 5 L5 5 L5 10 L0 10 Z M10 5 L15 5 L15 15 L5 15 L5 10 L10 10 Z",
}


def area(path: btvec.Path) -> float:
    """The filled area of a path of straight edges (shoelace over each closed contour: outers add, holes subtract)."""
    total = 0.0
    for contour in path.to_svg_path().split("M")[1:]:
        pairs = [pair.lstrip("L").split() for pair in contour.replace("Z", "").split(" L")]
        points = [(float(x), float(y)) for x, y in pairs]
        total += sum(x0 * y1 - x1 * y0 for (x0, y0), (x1, y1) in zip(points, points[1:] + points[:1], strict=True)) / 2
    return total


@pytest.mark.parametrize("verb", sorted(BOOLEANS))
def test_a_boolean_of_two_overlapping_squares_is_the_canonical_outline_worked_out_by_hand(verb):
    assert getattr(BOX, verb)(OVERLAPPING).to_svg_path() == BOOLEANS[verb]


@pytest.mark.parametrize(
    ("verb", "filled"),
    [("union", 175), ("subtract", 75), ("intersect", 25), ("exclude", 150)],
)
def test_the_area_of_a_boolean_follows_the_set_rule(verb, filled):
    # 100 + 100 square units, 25 of them shared
    assert area(getattr(BOX, verb)(OVERLAPPING)) == filled


def test_subtracting_a_square_inside_leaves_a_hole_that_is_not_filled():
    ring = Path.rect(0, 0, 100, 100).subtract(Path.rect(25, 25, 50, 50))
    assert ring.to_svg_path() == "M0 0 L100 0 L100 100 L0 100 Z M25 25 L25 75 L75 75 L75 25 Z"
    assert area(ring) == 7500
    assert (ring.contains((10, 10)), ring.contains((50, 50))) == (True, False)


def test_an_intersection_that_meets_nowhere_is_an_empty_path_and_a_union_keeps_both_pieces():
    assert BOX.intersect(DISJOINT) == Path()
    assert BOX.union(DISJOINT).to_svg_path() == f"{SQUARE} M50 50 L51 50 L51 51 L50 51 Z"


def test_resolve_cuts_a_self_crossing_outline_into_the_pieces_it_encloses():
    bow_tie = Path.from_svg_path("M0 0 L10 10 L10 0 L0 10 Z")
    assert bow_tie.resolve().to_svg_path() == "M0 0 L5 5 L0 10 Z M10 0 L10 10 L5 5 Z"


def test_resolve_of_a_resolved_path_is_the_same_path():
    once = Path.circle((0, 0), 20).union(Path.circle((15, 0), 20)).resolve()
    assert once.resolve() == once


@pytest.mark.parametrize("verb", ["resolve", "union", "subtract", "intersect", "exclude"])
def test_the_tolerance_keyword_sets_how_many_straight_edges_a_curve_becomes_and_defaults_to_a_hundredth(verb):
    circle = Path.circle((0, 0), 50)
    other = Path.rect(-100, -100, 200, 200) if verb == "intersect" else Path.rect(500, 500, 1, 1)  # keeps the circle
    arguments = () if verb == "resolve" else (other,)

    def edges(**keyword):
        return getattr(circle, verb)(*arguments, **keyword).to_svg_path().count("L")

    assert edges() == edges(tolerance=0.01) > 4 * edges(tolerance=1)


def test_the_edges_of_a_resolved_circle_stray_from_it_by_at_most_the_tolerance():
    circle = Path.circle((0, 0), 50)
    fine, coarse = circle.resolve(tolerance=0.01), circle.resolve(tolerance=1)
    # the polygon's area is within tolerance times the length
    assert area(coarse) == pytest.approx(math.pi * 50**2, abs=1 * circle.length)
    assert area(fine) == pytest.approx(math.pi * 50**2, abs=0.01 * circle.length)


def test_a_ring_cut_from_two_circles_has_the_area_of_the_annulus_within_the_tolerance():
    outer, inner = Path.circle((0, 0), 50), Path.circle((0, 0), 30)
    ring = outer.subtract(inner)
    assert area(ring) == pytest.approx(math.pi * (50**2 - 30**2), abs=0.01 * (outer.length + inner.length))
    assert (ring.contains((40, 0)), ring.contains((0, 0)), ring.contains((60, 0))) == (True, False, False)


@pytest.mark.parametrize(
    ("tolerance", "shown"),
    [
        pytest.param(0, "above 0", id="zero"),
        pytest.param(-1, "-1", id="negative"),
        pytest.param(math.nan, "nan", id="NaN"),
        pytest.param(math.inf, "inf", id="infinite"),
    ],
)
@pytest.mark.parametrize("verb", ["resolve", "union", "subtract", "intersect", "exclude"])
def test_a_tolerance_that_is_not_a_positive_number_is_a_parameter_error_naming_it(verb, tolerance, shown):
    arguments = () if verb == "resolve" else (OVERLAPPING,)
    with pytest.raises(btvec.ParameterError, match=r"tolerance") as raised:
        getattr(BOX, verb)(*arguments, tolerance=tolerance)
    assert shown in str(raised.value).lower()


@pytest.mark.parametrize(
    "call",
    [
        pytest.param(lambda: BOX.union("square"), id="other is a string"),  # type: ignore[arg-type]  # the misuse under test
        pytest.param(lambda: BOX.subtract(None), id="other is None"),  # type: ignore[arg-type]  # the misuse under test
        pytest.param(lambda: BOX.intersect(OVERLAPPING, 0.5), id="a positional tolerance"),  # type: ignore[call-arg]  # the misuse under test
        pytest.param(lambda: BOX.resolve(0.5), id="resolve with a positional tolerance"),  # type: ignore[call-arg]  # the misuse under test
        pytest.param(lambda: BOX.exclude(tolerance=0.5), id="exclude without other"),  # type: ignore[call-arg]  # the misuse under test
    ],
)
def test_a_boolean_called_with_the_wrong_kind_of_argument_is_a_type_error(call):
    with pytest.raises(TypeError):
        call()


def test_along_keeps_a_curve_as_a_smooth_bend_not_a_few_corners():
    # a half circle of radius 20 laid along a straight guide 100 down is the same half circle, moved
    arch = Path.circle((50, 0), 20)
    flat = arch.translate(0, 100)
    bent = arch.along(Path.line((0, 100), (500, 100)))
    assert bent.bounds.y == pytest.approx(flat.bounds.y, abs=0.03)
    assert bent.bounds.height == pytest.approx(flat.bounds.height, abs=0.03)
    assert bent.length == pytest.approx(flat.length, rel=1e-3)


def test_map_points_passes_every_point_on_and_off_the_curve_as_a_point():
    seen: list[btvec.Point] = []

    def record(point: btvec.Point) -> btvec.Point:
        seen.append(point)
        return point

    path = Path().move_to((0, 0)).curve_to((1, 2), (3, 4), (5, 6)).quad_to((7, 8), (9, 10))
    assert path.map_points(record) == path
    assert all(isinstance(point, Point) for point in seen)
    assert [tuple(point) for point in seen] == [(0, 0), (1, 2), (3, 4), (5, 6), (7, 8), (9, 10)]


def test_map_points_uses_what_the_function_returns_tuples_included():
    assert line().map_points(lambda p: (p.x, p.y + 1)).to_svg_path() == "M0 1 L10 1"
    assert line().map_points(lambda p: p * 2).to_svg_path() == "M0 0 L20 0"


def test_map_points_lets_the_exception_of_the_function_through_unchanged():
    class Boom(Exception):
        pass

    boom = Boom("from the callback")

    def fail(point: btvec.Point) -> btvec.Point:
        raise boom

    with pytest.raises(Boom) as raised:
        line().map_points(fail)
    assert raised.value is boom


def test_map_points_stops_at_the_first_failure():
    calls = []

    def fail_on_the_second(point: btvec.Point) -> btvec.Point:
        calls.append(point)
        if len(calls) == 2:
            raise ArithmeticError("second")
        return point

    with pytest.raises(ArithmeticError, match="second"):
        square().map_points(fail_on_the_second)
    assert len(calls) == 2


def test_map_points_rejects_a_function_that_returns_no_point():
    with pytest.raises(TypeError):
        line().map_points(lambda point: "not a point")  # type: ignore[arg-type, return-value]  # the misuse under test


def test_map_points_rejects_a_function_that_returns_a_non_finite_point_naming_fn():
    with pytest.raises(btvec.ParameterError, match=r"(?i)fn.*nan"):
        line().map_points(lambda point: (math.nan, 0))


# --- errors ----------------------------------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("call", "error", "names"),
    [
        pytest.param(lambda: Path().line_to((1, 1)), btvec.ParameterError, ["line_to", "move_to"], id="line_to first"),
        pytest.param(
            lambda: Path().curve_to((0, 0), (1, 1), (2, 2)), btvec.ParameterError, ["curve_to"], id="curve_to first"
        ),
        pytest.param(lambda: Path().quad_to((0, 0), (2, 2)), btvec.ParameterError, ["quad_to"], id="quad_to first"),
        pytest.param(lambda: Path().close(), btvec.ParameterError, ["close"], id="close first"),
        pytest.param(lambda: Path().move_to((math.nan, 0)), btvec.ParameterError, ["point", "nan"], id="NaN point"),
        pytest.param(
            lambda: Path().move_to((0, 0)).line_to((math.inf, 0)),
            btvec.ParameterError,
            ["point", "inf"],
            id="infinite point",
        ),
        pytest.param(lambda: Path.polyline([(0, 0)]), btvec.ParameterError, ["points", "1"], id="one point"),
        pytest.param(
            lambda: Path.polyline(np.array([(0.0, 0.0), (1.0, math.nan)])),
            btvec.ParameterError,
            ["point", "nan"],
            id="NaN in a numpy row",
        ),
        pytest.param(
            lambda: Path.polyline(np.zeros((3, 3))), TypeError, ["expected a Point", "0."], id="three numbers in a row"
        ),
        pytest.param(
            lambda: Path.polyline(np.zeros((3, 1))), TypeError, ["expected a Point", "0."], id="one number in a row"
        ),
        pytest.param(
            lambda: Path.polyline(np.zeros(4)), TypeError, ["expected a Point", "0."], id="a flat array is not points"
        ),
        pytest.param(lambda: Path.rect(0, 0, -3, 5), btvec.ParameterError, ["width", "-3"], id="negative width"),
        pytest.param(
            lambda: Path.rect(0, 0, 10, 10, radius=6), btvec.ParameterError, ["radius", "6"], id="radius over half"
        ),
        pytest.param(lambda: Path.circle((0, 0), 0), btvec.ParameterError, ["radius", "0"], id="circle radius 0"),
        pytest.param(lambda: Path.polygon((0, 0), 10, 2), btvec.ParameterError, ["sides", "2"], id="polygon 2 sides"),
        pytest.param(
            lambda: Path.polygon((0, 0), 10, 100_001),
            btvec.ParameterError,
            ["sides", "100001"],
            id="polygon over the cap",
        ),
        pytest.param(lambda: Path.polygon((0, 0), -1, 5), btvec.ParameterError, ["radius", "-1"], id="polygon radius"),
        pytest.param(
            lambda: Path.star((0, 0), 10, 5, points=1), btvec.ParameterError, ["points", "1"], id="star 1 point"
        ),
        pytest.param(
            lambda: Path.star((0, 0), 10, 0, points=5), btvec.ParameterError, ["inner", "0"], id="star inner radius"
        ),
        pytest.param(
            lambda: Path.from_svg_path("M0 0 L10"), btvec.ParameterError, ["data", "L"], id="malformed path data"
        ),
        pytest.param(
            lambda: Path.from_svg_path("L10 10"), btvec.ParameterError, ["data", "M"], id="data not starting with M"
        ),
        pytest.param(lambda: line().point_at(1.5), btvec.ParameterError, ["at", "1.5"], id="point_at above 1"),
        pytest.param(lambda: line().point_at(math.nan), btvec.ParameterError, ["at", "nan"], id="point_at NaN"),
        pytest.param(lambda: line().tangent_at(-0.1), btvec.ParameterError, ["at", "-0.1"], id="tangent_at below 0"),
        pytest.param(lambda: line().locate((math.nan, 0)), btvec.ParameterError, ["point", "nan"], id="locate NaN"),
        pytest.param(
            lambda: line().contains((0, math.inf)), btvec.ParameterError, ["point", "inf"], id="contains non-finite"
        ),
        pytest.param(
            lambda: line().point_at(0.5, near=(math.nan, 0)), btvec.ParameterError, ["near", "nan"], id="near NaN"
        ),
        pytest.param(
            lambda: line().slice(0.8, 0.2), btvec.ParameterError, ["start", "0.8", "0.2"], id="slice start above end"
        ),
        pytest.param(lambda: line().slice(0, 2), btvec.ParameterError, ["end", "2"], id="slice end above 1"),
        pytest.param(lambda: line().translate(math.nan, 0), btvec.ParameterError, ["dx", "nan"], id="translate NaN"),
        pytest.param(lambda: line().rotate(math.inf), btvec.ParameterError, ["angle", "inf"], id="rotate infinite"),
        pytest.param(
            lambda: line().along(EAST, start=math.nan), btvec.ParameterError, ["start", "nan"], id="along NaN"
        ),
        pytest.param(
            lambda: line().along(EAST, start=1e300), btvec.ParameterError, ["start", "too large"], id="along too far"
        ),
        pytest.param(
            lambda: line().along(RING, near=(math.nan, 0)), btvec.ParameterError, ["near", "nan"], id="along near NaN"
        ),
        pytest.param(lambda: line().scale(0), btvec.ParameterError, ["by", "0"], id="scale by 0"),
        pytest.param(lambda: line().scale((2, 0)), btvec.ParameterError, ["by", "0"], id="scale by a pair with a 0"),
        pytest.param(lambda: Path().bounds, btvec.GeometryError, ["bounds", "empty"], id="bounds of an empty path"),
        pytest.param(
            lambda: Path().point_at(0), btvec.GeometryError, ["point_at", "empty"], id="point_at on an empty path"
        ),
        pytest.param(
            lambda: Path().tangent_at(0), btvec.GeometryError, ["tangent_at", "empty"], id="tangent_at on an empty path"
        ),
        pytest.param(
            lambda: Path().locate((0, 0)), btvec.GeometryError, ["locate", "empty"], id="locate on an empty path"
        ),
        pytest.param(lambda: Path().slice(0, 1), btvec.GeometryError, ["slice", "empty"], id="slice of an empty path"),
        pytest.param(lambda: line().along(Path()), btvec.GeometryError, ["along", "empty"], id="along an empty guide"),
        pytest.param(
            lambda: line().along(Path().move_to((1, 1))),
            btvec.GeometryError,
            ["along", "no length"],
            id="along a guide with no length",
        ),
        pytest.param(
            lambda: Path().move_to((1, 1)).tangent_at(0),
            btvec.GeometryError,
            ["tangent_at", "direction"],
            id="no direction",
        ),
    ],
)
def test_a_bad_call_raises_its_error_class_naming_the_argument_and_the_value(
    call: Callable[[], object], error: type[Exception], names: list[str]
):
    with pytest.raises(error) as raised:
        call()
    message = str(raised.value).lower()
    for name in names:
        assert name.lower() in message, message
    assert type(raised.value) is error


# A coordinate beyond ±1e100 is refused where it enters: the value written out in full is in the message.
_FAR = 1e200
_FAR_TEXT = "1000000000000000"


@pytest.mark.parametrize(
    ("call", "name"),
    [
        pytest.param(lambda: Path().move_to((_FAR, 0)), "point", id="move_to"),
        pytest.param(lambda: Path().move_to((0, 0)).line_to((0, -_FAR)), "point", id="line_to"),
        pytest.param(lambda: Path().move_to((0, 0)).quad_to((_FAR, 0), (1, 1)), "control", id="quad_to control"),
        pytest.param(
            lambda: Path().move_to((0, 0)).curve_to((1, 1), (0, _FAR), (2, 2)), "control2", id="curve_to control2"
        ),
        pytest.param(lambda: Path.line((0, 0), (_FAR, 0)), "end", id="line"),
        pytest.param(lambda: Path.polyline([(0, 0), (_FAR, _FAR)]), "points", id="polyline"),
        pytest.param(lambda: Path.rect(_FAR, 0, 1, 1), "x", id="rect x"),
        pytest.param(lambda: Path.circle((0, 0), _FAR), "radius", id="circle radius"),
        pytest.param(lambda: Path.polygon((_FAR, 0), 10, 5), "center", id="polygon center"),
        pytest.param(lambda: Path.star((0, 0), 10, _FAR, points=5), "inner", id="star inner"),
        pytest.param(lambda: Path.from_svg_path("M0 0 M1 0 L1e200 0"), "data", id="path data"),
        pytest.param(lambda: line().translate(_FAR, 0), "dx", id="translate"),
        pytest.param(lambda: line().rotate(30, about=(_FAR, 0)), "about", id="rotate about"),
        pytest.param(lambda: line().scale(2, about=(0, _FAR)), "about", id="scale about"),
        pytest.param(lambda: line().map_points(lambda point: (_FAR, 0)), "fn", id="map_points"),
        pytest.param(lambda: line().locate((_FAR, 0)), "point", id="locate"),
        pytest.param(lambda: line().contains((0, _FAR)), "point", id="contains"),
        pytest.param(lambda: line().point_at(0.5, near=(_FAR, 0)), "near", id="point_at near"),
        pytest.param(lambda: line().tangent_at(0.5, near=(_FAR, 0)), "near", id="tangent_at near"),
        pytest.param(lambda: line().along(RING, near=(_FAR, 0)), "near", id="along near"),
        pytest.param(lambda: line().slice(0, 0.5, near=(_FAR, 0)), "near", id="slice near"),
        pytest.param(lambda: Path.line((0, 0), (9e99, 0)).translate(9e99, 0), "dx", id="translate past the limit"),
    ],
)
def test_a_coordinate_beyond_the_limit_is_a_parameter_error_naming_it(call: Callable[[], object], name: str):
    with pytest.raises(btvec.ParameterError) as raised:
        call()
    message = str(raised.value)
    assert name in message, message
    assert "1e100" in message or "too large" in message, message


def test_a_path_within_the_limit_measures_like_any_other():
    big = Path.line((-1e99, 0), (1e99, 0))
    assert big.length == pytest.approx(2e99)
    assert big.point_at(0) == Point(-1e99, 0)
    assert big.slice(0, 0.5).point_at(1).x == pytest.approx(0, abs=1e87)


def test_both_errors_are_catchable_as_the_package_error():
    with pytest.raises(btvec.BullterrierVectorError):
        line().point_at(2)
    with pytest.raises(btvec.BullterrierVectorError):
        _ = Path().bounds


def test_an_empty_path_has_no_length_and_transforms_return_it_unchanged():
    empty = Path()
    assert empty.length == 0
    assert empty.translate(3, 4) == empty
    assert empty.rotate(30) == empty
    assert empty.scale(2) == empty
    assert empty.along(EAST, start=0.5) == empty
    assert empty.reverse() == empty


@pytest.mark.parametrize(
    "call",
    [
        pytest.param(lambda: Path().move_to((1, 2, 3)), id="a triple"),  # the misuse under test
        pytest.param(lambda: Path().move_to("ab"), id="a string"),  # type: ignore[arg-type]  # the misuse under test
        pytest.param(lambda: Path().move_to((1, "a")), id="a pair that is not numbers"),  # type: ignore[arg-type]  # the misuse under test
        pytest.param(lambda: Path().move_to(5), id="a number"),  # type: ignore[arg-type]  # the misuse under test
        pytest.param(lambda: Path.polygon((0, 0), 10, 5.5), id="a float for sides"),  # type: ignore[arg-type]  # the misuse under test
        pytest.param(lambda: line().scale("big"), id="scale by a string"),  # type: ignore[arg-type]  # the misuse under test
        pytest.param(lambda: line().map_points(None), id="map_points without a function"),  # type: ignore[arg-type]  # the misuse under test
        pytest.param(lambda: line().along("circle"), id="along a string"),  # type: ignore[arg-type]  # the misuse under test
        pytest.param(lambda: line().along(EAST, 0.5), id="along with a positional start"),  # type: ignore[call-arg]  # the misuse under test
    ],
)
def test_an_argument_of_the_wrong_kind_is_a_type_error_not_a_parameter_error(call):
    with pytest.raises(TypeError):
        call()


# --- values ----------------------------------------------------------------------------------------------------


def test_paths_with_the_same_outline_are_equal_and_hash_alike():
    first, second = Path.circle((0, 0), 5), Path.circle((0, 0), 5)
    assert first == second
    assert hash(first) == hash(second)
    assert {first: "kept"}[second] == "kept"


@pytest.mark.parametrize(
    "other",
    [
        pytest.param(Path.circle((0, 0), 6), id="another radius"),
        pytest.param(Path.circle((0, 0), 5).move_to((1, 1)), id="one more sub-path"),
        pytest.param(Path.circle((0, 0), 5).to_svg_path(), id="its path data is not itself"),
        pytest.param(None, id="None"),
    ],
)
def test_a_different_path_or_another_kind_of_value_is_not_equal(other):
    assert Path.circle((0, 0), 5) != other


def test_a_closed_and_an_open_outline_through_the_same_points_differ():
    open_ = Path.polyline([(0, 0), (10, 0), (10, 10)])
    assert open_ != Path.polyline([(0, 0), (10, 0), (10, 10)], closed=True)


def test_a_path_cannot_be_changed_in_place():
    path = line()
    with pytest.raises(AttributeError):
        path.length = 3  # type: ignore[misc]  # the misuse under test
    with pytest.raises(AttributeError):
        path.anything = 3  # type: ignore[attr-defined]  # the misuse under test


def test_repr_shows_the_path_data():
    assert repr(line()) == "Path.from_svg_path('M0 0 L10 0')"
    assert repr(Path()) == "Path()"


# --- Point and Rect ------------------------------------------------------------------------------------------------


def test_a_point_has_x_and_y_and_unpacks():
    point = Point(1.5, -2)
    assert (point.x, point.y) == (1.5, -2)
    x, y = point
    assert (x, y) == (1.5, -2)
    assert tuple(point) == (1.5, -2)
    assert list(point) == [1.5, -2]
    assert len(point) == 2
    assert point == pytest.approx((1.5, -2.0000001))


def test_points_add_and_subtract_and_scale_by_a_number():
    assert Point(1, 2) + Point(10, 20) == Point(11, 22)
    assert Point(1, 2) + (10, 20) == Point(11, 22)  # noqa: RUF005  a Point is not a tuple
    assert (10, 20) + Point(1, 2) == Point(11, 22)  # noqa: RUF005  a Point is not a tuple
    assert Point(10, 20) - Point(1, 2) == Point(9, 18)
    assert Point(10, 20) - (1, 2) == Point(9, 18)
    assert (10, 20) - Point(1, 2) == Point(9, 18)
    assert Point(1, 2) * 3 == Point(3, 6)
    assert 3 * Point(1, 2) == Point(3, 6)
    assert Point(1, 2) * 0.5 == Point(0.5, 1)
    assert isinstance(Point(1, 2) + Point(1, 1), Point)


@pytest.mark.parametrize(
    "expression",
    [
        pytest.param(lambda: Point(1, 2) * Point(1, 2), id="point times point"),  # type: ignore[operator]  # the misuse under test
        pytest.param(lambda: Point(1, 2) * "a", id="point times a string"),  # type: ignore[operator]  # the misuse under test
        pytest.param(lambda: Point(1, 2) + 1, id="point plus a number"),  # type: ignore[operator]  # the misuse under test
        pytest.param(
            lambda: Point(1, 2) + (1, 2, 3),  # noqa: RUF005  the misuse under test: a Point is not a tuple
            id="point plus a triple",
        ),
    ],
)
def test_arithmetic_a_point_cannot_do_is_a_type_error(expression):
    with pytest.raises(TypeError):
        expression()


def test_a_point_equals_the_tuple_of_its_coordinates_and_hashes_like_it():
    assert Point(1, 2) == (1, 2)
    assert Point(1, 2) == Point(1.0, 2.0)
    assert Point(1, 2) != Point(2, 1)
    assert Point(1, 2) != (1, 2, 3)
    assert Point(1, 2) != "ab"
    assert hash(Point(1, 2)) == hash((1, 2))
    assert {Point(1, 2): "kept"}[(1, 2)] == "kept"  # type: ignore[index]  # the misuse under test


def test_a_point_is_a_value():
    point = Point(1, 2)
    with pytest.raises(AttributeError):
        point.x = 5  # type: ignore[misc]  # the misuse under test
    assert repr(Point(1.5, -2)) == "Point(1.5, -2.0)"


def test_a_rect_has_its_corner_and_size_and_is_a_value():
    rect = Rect(1, 2, 30, 40)
    assert (rect.x, rect.y, rect.width, rect.height) == (1, 2, 30, 40)
    assert rect == Rect(1.0, 2.0, 30.0, 40.0)
    assert rect != Rect(1, 2, 30, 41)
    assert rect != (1, 2, 30, 40)
    assert hash(rect) == hash(Rect(1, 2, 30, 40))
    assert repr(rect) == "Rect(x=1.0, y=2.0, width=30.0, height=40.0)"
    with pytest.raises(AttributeError):
        rect.width = 5  # type: ignore[misc]  # the misuse under test
