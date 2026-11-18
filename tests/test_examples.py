"""The example programs in examples/ run as far as they can draw and draw what their comments say. Each runs with
`runpy` in an empty folder, which it must leave empty: the picture stays in a variable. Expected values are worked out
by hand: the star's from the geometry of a five-pointed star, the bars' from the series printed in the program."""

import runpy
import xml.etree.ElementTree as ET
from functools import reduce
from itertools import pairwise
from pathlib import Path

import pytest

import bullterrier_vector as btvec

EXAMPLES = Path(__file__).resolve().parents[1] / "examples"


@pytest.fixture
def run(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    """Run an example in an empty folder; gives the variables it left."""
    monkeypatch.chdir(tmp_path)

    def run_example(name: str) -> dict[str, object]:
        found = runpy.run_path(str(EXAMPLES / f"{name}.py"), run_name="__main__")
        assert list(tmp_path.iterdir()) == []
        return found

    return run_example


def test_star_draws_the_star_a_dot_on_its_top_tip_and_a_line_from_its_far_side(run):
    star, dot, line = run("star")["shapes"]
    assert [shape.attributes["index"] for shape in (star, dot, line)] == [0, 1, 2]
    assert [shape.attributes["order"] for shape in (star, dot, line)] == [0.0, 0.5, 1.0]

    # the star is painted: filled yellow, stroked 4 pixels dark
    assert (star.style.fill, star.style.stroke, star.style.stroke_width) == ("#ffcc33ff", "#1a1a1aff", 4.0)
    # five tips on a circle of 150 about (200, 200): the top tip is at (200, 50)
    assert star.path.bounds.y == pytest.approx(50)
    assert star.path.contains((200, 200))

    # the dot: a circle of radius 8 on the top tip, filled red only
    assert (dot.style.fill, dot.style.stroke) == ("#e2412eff", None)
    assert dot.path.bounds == btvec.Rect(192, 42, 16, 16)

    # the line starts half the outline further on, which is the inner corner at the bottom, (200, 260), and is 40 long
    assert (line.style.fill, line.style.stroke) == (None, "#e2412eff")
    assert line.path.length == pytest.approx(40)
    assert line.path.point_at(0).x == pytest.approx(200)
    assert line.path.point_at(0).y == pytest.approx(260)


def test_star_writes_its_svg_at_the_size_of_its_frame_one_path_a_shape(run):
    found = run("star")
    star, dot, line = found["shapes"]
    svg = found["svg"]
    assert isinstance(svg, str)
    root = ET.fromstring(svg)  # noqa: S314  our own output
    assert (root.attrib["width"], root.attrib["height"], root.attrib["viewBox"]) == ("400", "400", "0 0 400 400")
    elements = root.findall("{http://www.w3.org/2000/svg}path")
    assert len(elements) == 3

    # the frame is the output, so the data is the shapes' own, and the paint as the comments of the example say
    assert [element.attrib["d"] for element in elements] == [shape.path.to_svg_path() for shape in (star, dot, line)]
    star_element, dot_element, line_element = (element.attrib for element in elements)
    assert (star_element["fill"], star_element["stroke"], star_element["stroke-width"]) == ("#ffcc33", "#1a1a1a", "4")
    assert (dot_element["fill"], "stroke" in dot_element) == ("#e2412e", False)
    assert (line_element["fill"], line_element["stroke"], line_element["stroke-width"]) == ("none", "#e2412e", "2")


SERIES = [0.18, 0.35, 0.52, 0.41, 0.77, 0.93, 0.64, 0.49, 0.71, 0.28, 0.15, 0.33]


def test_series_bars_draws_twelve_bars_with_the_tallest_in_the_second_ink_on_top(run):
    bars = run("series_bars")["shapes"]
    assert len(bars) == 12
    assert [bar.attributes["index"] for bar in bars] == list(range(12))

    # the tallest (index 5 of the series) is last in paint order and the only one in #e2412e
    assert [bar.style.fill for bar in bars] == ["#15161aff"] * 11 + ["#e2412eff"]
    assert all(bar.style.stroke is None for bar in bars)

    # each bar stands on the line y = 900, 56 wide, 76 apart from x = 90, 700 high at a value of 1; the order each
    # bar got in the first stack survives `only` and `without`
    painted_in_series_order = [*range(5), *range(6, 12), 5]
    for bar, i in zip(bars, painted_in_series_order, strict=True):
        bounds = bar.path.bounds
        assert (bounds.x, bounds.width) == (90 + 76 * i, 56)
        assert bounds.height == pytest.approx(700 * SERIES[i])
        assert bounds.y + bounds.height == pytest.approx(900)
        assert bar.attributes["order"] == pytest.approx(i / 11)


def test_badge_frames_writes_one_second_at_24_frames_a_second_numbered_from_zero(run):
    written = run("badge_frames")["written"]
    assert written == [f"badge-{i:04d}.png" for i in range(24)]


def test_poster_overlaps_two_multiply_inks_on_paper_and_keeps_its_stack_of_shapes(run):
    found = run("poster")
    shapes, rgba = found["shapes"], found["rgba"]
    # paper, twelve stars, two inks, the zigzag, eight hairlines
    assert len(shapes) == 1 + 12 + 2 + 1 + 8
    assert [shape.style.blend for shape in shapes[13:15]] == ["multiply", "multiply"]
    assert len(rgba) == 800 * 600 * 4

    def pixel(x: int, y: int) -> tuple[int, ...]:
        start = (y * 800 + x) * 4
        return tuple(rgba[start : start + 4])

    # the corner is bare paper, opaque
    assert pixel(2, 2) == (246, 241, 228, 255)
    # where the inks overlap, each channel is the product of both inks and the paper under them, over 255 twice
    pink, blue, paper = (255, 79, 154), (46, 100, 226), (246, 241, 228)
    expected = tuple(a * b * c / 255**2 for a, b, c in zip(pink, blue, paper, strict=True))
    assert pixel(400, 300) == pytest.approx((*expected, 255), abs=1.5)
    # and the pink alone is only its own product with the paper
    assert pixel(200, 300)[:3] == pytest.approx(tuple(a * c / 255 for a, c in zip(pink, paper, strict=True)), abs=1.5)
    # the file the picture would be is well formed XML with one path per shape that has geometry
    root = ET.fromstring(found["svg"])  # noqa: S314  our own output
    assert len(root.findall("{http://www.w3.org/2000/svg}path")) == len(shapes)


def kinetic_layers(drawing: btvec.Drawing, **given: float | list[float]):
    """A frame of the kinetic type piece by layer: the paper, the blue plane, 7 pink letters, 7 hairline letters."""
    paper, plane, *letters = drawing.shapes(**given)
    assert len(letters) == 14
    return paper, plane, letters[:7], letters[7:]


def widths(shapes) -> list[float]:
    return [shape.path.bounds.width for shape in shapes]


def deep_inside(shape) -> tuple[float, float]:
    """A point of the letter that stays inside it when moved 4 pixels any way (the plates are 3 out of register)."""
    box = shape.path.bounds
    for y in range(int(box.y), int(box.y + box.height), 2):
        for x in range(int(box.x), int(box.x + box.width), 2):
            if all(shape.path.contains((x + dx, y + dy)) for dx in (-4, 0, 4) for dy in (-4, 0, 4)):
                return (x, y)
    raise AssertionError("no point deep inside the letter")


def assert_same_bounds(found, expected, tolerance=1e-6):
    assert len(found) == len(expected)
    for got, want in zip(found, expected, strict=True):
        for name in ("x", "y", "width", "height"):
            assert getattr(got.path.bounds, name) == pytest.approx(getattr(want.path.bounds, name), abs=tolerance)


def test_kinetic_type_sets_both_headlines_a_shape_a_letter_inside_the_frame_filling_the_measure(run):
    found = run("kinetic_type")
    for headline, chars in (("first", "NOSLEEP"), ("second", "GODREAM")):
        calm = found[headline].shapes()
        assert "".join(shape.attributes["char"] for shape in calm) == chars
        assert [shape.attributes["line"] for shape in calm] == [0, 0, 1, 1, 1, 1, 1]
        for shape in calm:
            bounds = shape.path.bounds
            assert bounds.x >= 0
            assert bounds.x + bounds.width <= 1080
            assert bounds.y >= 0
            assert bounds.y + bounds.height <= 1350
        # each line is fitted to the 980 pixel measure by the width axis: its letters fill it, centred on the frame
        for line in (0, 1):
            edges = [shape.path.bounds for shape in calm if shape.attributes["line"] == line]
            left, right = min(edge.x for edge in edges), max(edge.x + edge.width for edge in edges)
            assert 50 <= left < 100, (headline, line, left)
            assert 980 < right <= 1030, (headline, line, right)
            assert (left + right) / 2 == pytest.approx(540, abs=30)

    # `pulse` has an entry per character of "NO\nSLEEP": 8, the line break included. A kick on the N (entry 0) makes
    # the N wider, and the letters after it follow its advance; the second line is untouched.
    first = found["first"]
    calm, kicked = first.shapes(), first.shapes(pulse=[1, 0, 0, 0, 0, 0, 0, 0])
    assert kicked[0].path.bounds.width > calm[0].path.bounds.width
    assert kicked[1].path.bounds.width == pytest.approx(calm[1].path.bounds.width)
    assert [shape.path for shape in kicked[2:]] == [shape.path for shape in calm[2:]]


def test_kinetic_type_starts_as_hairlines_alone_on_the_paper_with_the_plane_below_the_frame(run):
    paper, plane, pink, hairlines = kinetic_layers(run("kinetic_type")["at_time"](1.0))
    assert paper.path.bounds == btvec.Rect(0, 0, 1080, 1350)
    assert plane.path.bounds.y == 1350
    assert [shape.style.opacity for shape in pink] == [0.0] * 7
    assert [shape.style.opacity for shape in hairlines] == [1.0] * 7
    assert all(shape.style.stroke is not None and shape.style.fill is None for shape in hairlines)


def test_kinetic_type_plane_is_half_risen_at_3_s_and_full_at_4_5_s_with_the_letters_knocked_out(run):
    at_time = run("kinetic_type")["at_time"]
    _, half, pink, _ = kinetic_layers(at_time(3.0))
    assert half.path.bounds.y == 675
    assert half.path.contains((5, 1340))  # blue below the plane's edge...
    assert not half.path.contains((5, 100))  # ...and none above it
    s = pink[2]  # the S of SLEEP, which the plane has passed
    assert s.path.bounds.y > 675
    assert not half.path.contains(deep_inside(s))  # the plane has a hole where the letter prints
    _, full, _, _ = kinetic_layers(at_time(4.5))
    assert full.path.bounds == btvec.Rect(0, 0, 1080, 1350)


def test_kinetic_type_prints_pink_from_4_s_and_a_kick_thickens_the_letters_it_has_reached(run):
    found = run("kinetic_type")
    at_time, calm = found["at_time"], found["first"].shapes()
    # at 5 s the print is on and the hairlines are gone
    _, _, pink, hairlines = kinetic_layers(at_time(5.0))
    assert [shape.style.opacity for shape in pink] == [1.0] * 7
    assert [shape.style.opacity for shape in hairlines] == [0.0] * 7
    # 0.4 s after a beat the kick has left the N and the O (no scatter: the hit is over) and is still in the second
    # line, 0.045 s later for each letter: the S has hardly felt it, then L, E, E, P are each heavier than the last
    _, _, pink, _ = kinetic_layers(at_time(4.4))
    grown = [now - before for now, before in zip(widths(pink), widths(calm), strict=True)]
    assert grown[0] == pytest.approx(0, abs=0.5)
    assert grown[1] == pytest.approx(0, abs=0.5)
    assert grown[2] == pytest.approx(0, abs=0.5)
    assert 1 < grown[3] < grown[4] < grown[5] < grown[6]


# t -> (bend, phase and turn at that second, worked by hand from the timeline; whether a kick is on)
BEATS: dict[float, tuple[dict[str, float], bool]] = {
    4.0: ({"bend": 0, "phase": 0, "turn": 0}, True),  # the drop, on the beat
    4.5: ({"bend": 0, "phase": 0.125, "turn": 0}, True),  # the next beat
    7.0: ({"bend": 1, "phase": 0.75, "turn": 0}, True),
    7.5: ({"bend": 1, "phase": 0.875, "turn": 0}, True),  # the last beat before the turn
    8.0: ({"bend": 0.5, "phase": 0, "turn": 0}, False),  # a beat, and the turn begins: no kick
    8.5: ({"bend": 0, "phase": 0.125, "turn": 7 / 27}, False),
}


@pytest.mark.parametrize("t", BEATS)
def test_kinetic_type_kicks_on_every_beat_from_the_drop_at_4_s_until_the_turn_at_8_s(run, t):
    found = run("kinetic_type")
    given, kicking = BEATS[t]
    _, _, hit, _ = kinetic_layers(found["at_time"](t))
    _, _, still, _ = kinetic_layers(found["piece"], rise=1, ink=1, weight=300, burst=0, pulse=[0] * 8, **given)
    if kicking:  # the N, first in the stagger, is at its fattest
        assert hit[0].path.bounds.width > still[0].path.bounds.width + 100
    else:
        assert_same_bounds(hit, still)


def test_kinetic_type_turns_into_the_second_headline_one_letter_after_the_other(run):
    found = run("kinetic_type")
    at_time, first, second = found["at_time"], found["first"], found["second"]
    start, end = first.resolve().shapes(weight=100), second.resolve().shapes()
    # before the turn (3.5 s, the hairlines at weight 100) it is the first headline
    assert_same_bounds(kinetic_layers(at_time(3.5))[3], start)
    # 8.8 s: the first letters have arrived, the last ones have not left, those between are on the way
    _, _, pink, _ = kinetic_layers(at_time(8.8))
    assert_same_bounds(pink[:2], end[:2], tolerance=0.01)
    assert_same_bounds(pink[6:], first.resolve().shapes()[6:], tolerance=0.01)
    for now, before, after in zip(pink[2:6], first.resolve().shapes()[2:6], end[2:6], strict=True):
        assert abs(now.path.bounds.width - before.path.bounds.width) > 1
        assert abs(now.path.bounds.width - after.path.bounds.width) > 1
    # from 9.5 s (weight 300 still) it is the second headline, each letter the bounds of its own
    _, _, pink, hairlines = kinetic_layers(at_time(9.5))
    assert_same_bounds(pink, end)
    assert_same_bounds(hairlines, end)
    # and the verb gives the same at its end
    assert_same_bounds(first.resolve().morph(second.resolve(), 1).shapes(), end)


def test_kinetic_type_bend_arches_the_line_up_and_moves_the_letters(run):
    at_time = run("kinetic_type")["at_time"]
    # 6.4 s is a whole second after 5.4 s: the same place between beats, before the bend begins (5.5 s)
    _, _, bent, _ = kinetic_layers(at_time(6.4))
    _, _, straight, _ = kinetic_layers(at_time(5.4))
    assert all(
        abs(a.path.bounds.y - b.path.bounds.y) > 1 or abs(a.path.bounds.x - b.path.bounds.x) > 1
        for a, b in zip(bent, straight, strict=True)
    )
    # the arch lifts the line: the highest letter top is well above the straight line's
    assert min(shape.path.bounds.y for shape in bent) < min(shape.path.bounds.y for shape in straight) - 50


def test_kinetic_type_thins_to_hairlines_at_the_floor_weight_with_no_cut_at_the_end(run):
    found = run("kinetic_type")
    at_time, second = found["at_time"], found["second"]
    _, _, pink, hairlines = kinetic_layers(at_time(12.0))
    assert [shape.style.opacity for shape in pink] == [0.0] * 7
    assert [shape.style.opacity for shape in hairlines] == [1.0] * 7
    assert_same_bounds(hairlines, second.resolve().shapes(weight=100))  # the floor of `weight`
    # no jump at the end: 0.01 s before it the print has gone and the hairlines are on
    _, _, pink, near = kinetic_layers(at_time(11.99))
    assert pink[0].style.opacity < 0.01
    assert near[0].style.opacity > 0.99
    assert_same_bounds(near, hairlines, tolerance=0.01)


def test_kinetic_type_keeps_the_print_until_11_5_s_and_fades_it_over_the_last_half_second(run):
    at_time = run("kinetic_type")["at_time"]
    for t, printed in ((11.4, 1.0), (11.75, 0.5)):
        _, _, pink, hairlines = kinetic_layers(at_time(t))
        assert [shape.style.opacity for shape in pink] == [pytest.approx(printed)] * 7, t
        assert [shape.style.opacity for shape in hairlines] == [pytest.approx(1 - printed)] * 7, t


def test_kinetic_type_frames_are_deterministic_and_the_whole_timeline_draws(run):
    at_time = run("kinetic_type")["at_time"]
    assert at_time(6.4).to_svg(1080, 1350) == at_time(6.4).to_svg(1080, 1350)
    for half_seconds in range(25):
        svg = at_time(half_seconds / 2).to_svg(1080, 1350)
        assert svg.startswith("<svg")


def test_bundled_type_sets_a_line_in_each_bundled_font_with_no_file_read(run):
    found = run("bundled_type")
    shapes = found["drawing"].shapes()
    # the paper, then one shape for each letter of the two lines (the spaces make none)
    chars = [shape.attributes["char"] for shape in shapes[1:]]
    assert "".join(chars) == "Soft&wonky" + "BricolageGrotesque"
    # both lines stand inside the 900 pixel frame, with a margin
    assert max(shape.path.bounds.x + shape.path.bounds.width for shape in shapes[1:]) < 850


def test_text_on_path_stands_its_letters_on_the_circle_and_cycle_turns_them_half_way_round(run):
    drawing = run("text_on_path")["drawing"]
    paper, orbit, *letters = drawing.shapes(cycle=0.0)
    assert [shape.attributes["char"] for shape in letters] == list("ROUNDANDROUNDANDROUNDAND")
    assert paper.path.bounds == btvec.Rect(0, 0, 1080, 1080)
    assert orbit.path.bounds == btvec.Rect(180, 180, 720, 720)

    def middle(shape: btvec.Shape) -> tuple[float, float]:
        bounds = shape.path.bounds
        return (bounds.x + bounds.width / 2, bounds.y + bounds.height / 2)

    for shape in letters:
        x, y = middle(shape)
        # the circle is 360 round (540, 540); the letters stand outside it, up to 90 pixels tall, and inside the frame
        assert 360 < ((x - 540) ** 2 + (y - 540) ** 2) ** 0.5 < 450
        bounds = shape.path.bounds
        assert bounds.x >= 0
        assert bounds.y >= 0
        assert bounds.x + bounds.width <= 1080
        assert bounds.y + bounds.height <= 1080
    # the text begins at the top of the circle; half a cycle on, it begins at the bottom
    first_x, first_y = middle(letters[0])
    assert abs(first_x - 540) < 100
    assert first_y < 540 - 360
    (_, _, *turned) = drawing.shapes(cycle=0.5)
    turned_x, turned_y = middle(turned[0])
    assert abs(turned_x - 540) < 100
    assert turned_y > 540 + 360


def test_draw_on_draws_no_ink_at_progress_0_the_letters_one_after_another_and_the_whole_word_at_1(run):
    found = run("draw_on")
    drawing, font = found["drawing"], found["font"]

    def inks(progress: float) -> list[float]:
        return [shape.path.length for shape in drawing.shapes(progress=progress)]

    def ink_in_pixels(progress: float) -> bool:
        return any(drawing.to_rgba(1080, 600, progress=progress)[3::4])

    assert inks(0) == [0.0] * 5
    assert not ink_in_pixels(0)
    # letters start in word order: while the second is drawing, the later ones have nothing yet
    midway = inks(0.3)
    assert midway[0] > midway[1] > 0
    assert midway[2:] == [0.0] * 3
    # a letter never loses ink as the progress grows
    for earlier, later in zip(inks(0.3), inks(0.6), strict=True):
        assert later >= earlier
    # at 1 each letter is whole: the outline of the letter itself, as the unsliced text has it
    whole = btvec.text("WRITE", font=font, size=240, axes={"wght": 300}, anchor=(540, 380), align="center")
    assert inks(1) == pytest.approx([shape.path.length for shape in whole.shapes()])
    assert ink_in_pixels(1)


def test_marching_dashes_move_one_pattern_length_on_over_a_phase_of_1_and_a_loop_is_seamless(run):
    drawing = run("marching_dashes")["drawing"]

    def starts(march: float) -> list[float]:
        (shape,) = drawing.shapes(march=march)
        return [piece.point_at(0).x for piece in shape.path.split()]

    # the line runs from x = 140 for 800 pixels: dashes 28 long every 44, the first at the start
    assert starts(0)[:3] == pytest.approx([140, 184, 228])
    assert starts(0)[-1] == pytest.approx(140 + 44 * 18)
    # a quarter of the 44-pixel cycle on: every dash 11 further along, a dash cut by the start included
    assert starts(0.25)[:2] == pytest.approx([140 + 11, 184 + 11])
    # a whole cycle is the same picture
    assert drawing.shapes(march=1)[0].path == drawing.shapes(march=0)[0].path


def test_beads_on_a_path_puts_twelve_petals_on_the_ring_and_a_whole_cycle_is_the_same_picture(run):
    drawing = run("beads_on_a_path")["drawing"]

    def petals(march: float) -> list[btvec.Shape]:
        return [shape for shape in drawing.shapes(march=march) if "copy" in shape.attributes]

    first = petals(0)
    assert [shape.attributes["copy"] for shape in first] == list(range(12))
    # every petal's middle is on the ring of radius 360 about (540, 540)
    for shape in first:
        box = shape.path.bounds
        x, y = box.x + box.width / 2 - 540, box.y + box.height / 2 - 540
        assert (x * x + y * y) ** 0.5 == pytest.approx(360, abs=1)
    # the first petal is at the top, and a quarter cycle moves each one three stops on, so petal 9 is at the top
    assert first[0].path.bounds.x + first[0].path.bounds.width / 2 == pytest.approx(540, abs=1)
    moved = petals(0.25)[9].path.bounds
    assert moved.y + moved.height / 2 == pytest.approx(180, abs=1)
    assert [shape.path for shape in petals(1)] == [shape.path for shape in first]
    # by_order sizes the petals: the last copy is larger than the first
    sizes = [shape.path.bounds.width * shape.path.bounds.height for shape in first]
    assert sizes[11] > sizes[0]


def test_glyph_scatter_stands_as_set_at_0_and_scatters_the_same_way_every_time_above_0(run):
    found = run("glyph_scatter")
    drawing, font = found["drawing"], found["font"]
    set_word = btvec.text("MIX!", font=font, size=260, axes={"wght": 800}, anchor=(540, 420), align="center").shapes()

    # the exclamation mark is two parts (its stem and dot); at 0 nothing has moved, so every part stays where the
    # letter has it and the parts of a letter together fill its box
    calm = drawing.shapes(scatter=0)
    assert [(shape.attributes["char"], shape.attributes["part"]) for shape in calm] == [
        ("M", 0),
        ("I", 0),
        ("X", 0),
        ("!", 0),
        ("!", 1),
    ]
    assert [shape.path.to_svg_path() for shape in calm[:3]] == [shape.path.to_svg_path() for shape in set_word[:3]]
    mark = set_word[3].path.bounds
    stem, dot = calm[3].path.bounds, calm[4].path.bounds
    assert (min(stem.x, dot.x), min(stem.y, dot.y)) == pytest.approx((mark.x, mark.y))
    assert max(stem.y + stem.height, dot.y + dot.height) == pytest.approx(mark.y + mark.height)

    # above 0 every part has moved, and the same value draws the same bytes, in a program run again as well
    moved = drawing.shapes(scatter=0.5)
    assert all(a.path != b.path for a, b in zip(moved, calm, strict=True))
    assert drawing.to_svg(1080, 640, scatter=0.5) == drawing.to_svg(1080, 640, scatter=0.5)
    assert run("glyph_scatter")["drawing"].to_svg(1080, 640, scatter=0.5) == drawing.to_svg(1080, 640, scatter=0.5)
    assert drawing.to_svg(1080, 640, scatter=1) != drawing.to_svg(1080, 640, scatter=0.5)


def test_cutout_cuts_the_letters_out_of_the_plate_and_keeps_the_lens_two_discs_share(run):
    paper, sheet, plate, crescents, lens = run("cutout")["shapes"]
    assert (paper.path.bounds, sheet.path.bounds) == (btvec.Rect(0, 0, 1000, 820), btvec.Rect(40, 40, 920, 460))
    # the plate is the rounded rectangle with one hole for each of the letters C, U and T
    assert plate.path.bounds == btvec.Rect(70, 70, 860, 400)
    assert plate.path.to_svg_path().count("M") == 4
    assert plate.path.contains((100, 100))
    assert not plate.path.contains((707, 300))  # the stem of the T is cut out
    assert plate.path.contains((500, 300))  # the counter of the U is not
    # the discs have radius 110 and their centres are 2 * (120 - 40 * 0.6) = 192 apart: the lens is 2 * (110 - 96)
    # wide and 2 * sqrt(110^2 - 96^2) high, about their middle
    box = lens.path.bounds
    assert (box.x, box.width) == pytest.approx((486, 28), abs=0.05)
    assert (box.y + box.height / 2, box.height) == pytest.approx(
        (640, 2 * (110**2 - 96**2) ** 0.5), abs=0.15
    )  # the tips lie on flattened arcs
    assert lens.path.contains((500, 640))
    assert not lens.path.contains((420, 640))
    # both crescents are one shape: they cover the discs outside the lens
    assert crescents.path.bounds.width == pytest.approx(412)  # 2 * (96 + 110)
    assert crescents.path.contains((420, 640))
    assert not crescents.path.contains((500, 640))


def test_cut_paper_layers_nest_each_one_a_step_wider_than_the_last_inside_a_ring_of_cut_line(run):
    paper, *layers, lettering, cut_line = run("cut_paper_layers")["shapes"]
    assert len(layers) == 4
    # the layers run from the outermost, 4 steps of 14 px wide, to the innermost, 1 step: each grows the lettering's box
    # by its distance on every side (a round join reaches exactly that far)
    box = lettering.path.bounds
    for layer, grown in zip(layers, [56, 42, 28, 14], strict=True):
        assert layer.path.bounds.x == pytest.approx(box.x - grown, abs=0.1)
        assert layer.path.bounds.y == pytest.approx(box.y - grown, abs=0.1)
        assert layer.path.bounds.width == pytest.approx(box.width + 2 * grown, abs=0.2)
        assert layer.path.bounds.height == pytest.approx(box.height + 2 * grown, abs=0.2)
    # the cut line is a ring 3 px wide round the layers 5 steps out: it paints its own edge, not the middle of the word
    assert cut_line.path.bounds.x == pytest.approx(box.x - 70 - 1.5, abs=0.1)
    assert cut_line.path.contains((cut_line.path.bounds.x + 1, box.y + box.height / 2))
    assert not cut_line.path.contains((box.x + box.width / 2, box.y + box.height / 2))
    assert paper.path.bounds == btvec.Rect(0, 0, 1000, 420)


def test_ring_is_a_band_round_the_circle_that_widens_with_the_half_width(run):
    found = run("ring")
    assert found["on_band"] is True
    ring = found["ring"]
    live = found["live"]
    assert isinstance(ring, btvec.Path)
    assert isinstance(live, btvec.Drawing)
    assert not ring.contains((200, 200))  # the centre is in the hole
    # the circle has radius 120 about (200, 200): the band reaches half_width either side of it
    for half_width in (4, 14, 40):
        (band,) = live.with_values(half_width=half_width).shapes()
        assert band.path.bounds.width == pytest.approx(240 + 2 * half_width, abs=0.1)
        assert band.path.contains((200, 200 - 120))
        assert not band.path.contains((200, 200))
        assert not band.path.contains((200, 200 - 120 - half_width - 1))


def test_halftone_draws_a_row_of_discs_of_rising_tone_and_a_big_disc_with_its_stroke(run):
    found = run("halftone")
    assert found["tones"] == pytest.approx([0.1, 0.2333333333, 0.3666666667, 0.5, 0.6333333333, 0.7666666667, 0.9])
    root = ET.fromstring(found["svg"])  # noqa: S314  our own output
    svg = "{http://www.w3.org/2000/svg}"
    # a clip path and a group of dots for each of the eight discs, the paper and the big disc's stroke are plain paths
    assert len(root.findall(f"{svg}clipPath")) == 8
    groups = [group for group in root.iter(f"{svg}g") if "clip-path" in group.attrib]
    assert len(groups) == 8
    # a disc of radius 38 on a lattice of 6 holds pi * 38^2 / 6^2 = about 126 dots, more with the ones on its edge
    dots = [path for group in groups[:7] if (path := group.find(f"{svg}path")) is not None]
    counts = [path.attrib["d"].count("M") for path in dots]
    assert len(dots) == 7
    assert all(110 < count < 200 for count in counts), counts
    big = root.findall(f"{svg}path")[-1]
    assert (big.attrib["fill"], big.attrib["stroke"], big.attrib["stroke-width"]) == ("none", "#c4372b", "3")


def test_gradient_type_fills_the_letters_with_one_gradient_across_the_frame_and_the_svg_names_each_gradient(run):
    found = run("gradient_type")
    drawing = found["drawing"]
    assert isinstance(drawing, btvec.Drawing)
    shapes = drawing.shapes()
    letters = [shape for shape in shapes if "glyph" in shape.attributes]
    assert len(letters) == len("Gradient")
    # every letter has the one gradient: it lies in the frame, not in the letter
    fills = {repr(letter.style.fill) for letter in letters}
    assert len(fills) == 1
    fade = letters[0].style.fill
    assert isinstance(fade, btvec.Gradient)
    assert (fade.start, fade.end) == (btvec.Point(60, 0), btvec.Point(540, 0))
    root = ET.fromstring(found["svg"])  # noqa: S314  our own output
    svg = "{http://www.w3.org/2000/svg}"
    assert len(root.findall(f"{svg}radialGradient")) == 2
    # one linear gradient for each letter, since each is a path of its own
    assert len(root.findall(f"{svg}linearGradient")) == len(letters)
    with pytest.raises(btvec.ParameterError, match="gradient"):
        drawing.to_mesh(900, 420)


def test_cut_type_clips_the_stripes_to_one_window_and_fades_every_letter_by_one_mask(run):
    found = run("cut_type")
    drawing = found["drawing"]
    assert isinstance(drawing, btvec.Drawing)
    shapes = drawing.shapes()
    stripes = [shape for shape in shapes if shape.style.clip is not None]
    letters = [shape for shape in shapes if shape.style.mask is not None]
    assert (len(stripes), len(letters)) == (19, len("Fade out") - 1)  # 900 / 48 stripes; the space is no shape
    # the window is the circle of radius 190 about (450, 210), the one region of every stripe
    assert stripes[0].style.clip is not None
    assert stripes[0].style.clip.bounds == btvec.Rect(260, 20, 380, 380)
    assert all(stripe.style.clip == stripes[0].style.clip for stripe in stripes)
    root = ET.fromstring(found["svg"])  # noqa: S314  our own output
    svg = "{http://www.w3.org/2000/svg}"
    # one clip path and one mask, however many shapes use them
    assert (len(root.findall(f"{svg}clipPath")), len(root.findall(f"{svg}mask"))) == (1, 1)
    with pytest.raises(btvec.ParameterError, match=r"clip|mask"):
        drawing.to_mesh(900, 420)


def test_svg_logo_picks_the_ring_by_its_id_and_the_mark_by_its_group(run):
    found = run("svg_logo")
    (ring,) = found["shapes"]
    assert ring.attributes["id"] == "ring"
    assert ring.attributes["group"] == "logo"
    assert ring.style.fill is None
    assert ring.style.stroke == "#1a1a1aff"
    assert ring.style.stroke_width == 24
    assert ring.path.bounds.width == pytest.approx(300)
    assert [shape.attributes["id"] for shape in found["mark"].shapes()] == ["spark", "dot"]
    assert [shape.attributes["id"] for shape in found["accent"].shapes()] == ["dot"]
    assert found["logo"].size == (400, 400)


def test_ransom_note_sets_every_letter_in_its_own_font_wrapped_and_fitted_to_the_measure(run):
    found = run("ransom_note")
    note = found["note"].shapes()
    # one shape per letter, each its own run, the spaces make none
    assert "".join(shape.attributes["char"] for shape in note) == "SEETHECATNOMORE"
    assert [shape.attributes["run"] for shape in note] == [at for at, c in enumerate(found["HEADLINE"]) if c != " "]
    # every line is the measure wide: the wrap is at whole words, so a word is never split between lines
    lines = sorted({shape.attributes["line"] for shape in note})
    assert len(lines) > 1
    for line in lines:
        shapes = [shape for shape in note if shape.attributes["line"] == line]
        left = min(shape.path.bounds.x for shape in shapes)
        right = max(shape.path.bounds.x + shape.path.bounds.width for shape in shapes)
        # the advances fill the measure exactly; the ink is the measure less the first and last side bearings
        assert 0.85 * found["MEASURE"] < right - left <= found["MEASURE"]
        assert abs((left + right) / 2 - 540) < 40
    words: dict[object, set[object]] = {}
    for shape in note:
        words.setdefault(shape.attributes["word"], set()).add(shape.attributes["line"])
    assert all(len(on) == 1 for on in words.values())
    # the bent line keeps its letters and stays inside the frame
    bent = found["bent"].shapes()
    assert len(bent) == 7
    assert all(shape.path.bounds.x >= 0 and shape.path.bounds.x + shape.path.bounds.width <= 1080 for shape in bent)


def test_brush_taper_draws_on_from_nothing_and_each_tip_is_a_point(run):
    drawing = run("brush_taper")["drawing"]

    def ink(progress: float) -> int:
        return sum(drawing.to_rgba(1080, 600, progress=progress)[3::4])

    assert ink(0) == 0
    assert 0 < ink(0.5) < ink(1)
    # the stroke is 36 wide at its fullest and a point at its first end, (120, 380): only the faint shadow is there
    pixels = drawing.to_rgba(1080, 600, progress=1)
    assert pixels[(380 * 1080 + 120) * 4 + 3] < 10


def test_bent_type_is_the_plain_line_at_progress_0_and_arches_ripples_and_twists_as_it_grows(run):
    found = run("bent_type")
    drawing, calm_svg = found["drawing"], found["frames"]["calm"]
    line = drawing.shapes()[1:]  # the first shape is the paper
    assert [shape.attributes["char"] for shape in line] == list("NOSLEEP")  # no shape for the space
    bounds = [shape.path.bounds for shape in line]
    # no bend: the letters stand on one baseline (the bottoms of N and O agree to a pixel; O overshoots a little)
    assert max(b.y + b.height for b in bounds) - min(b.y + b.height for b in bounds) < 12
    # bent: the S in the middle rises (the arch lifts it by 150 at most, the twist turns it back some)
    bent = [shape.path.bounds for shape in drawing.shapes(progress=1, phase=0.25)[1:]]
    assert bent[2].y < bounds[2].y - 30
    # and the line is no longer level: the bottoms of the N and the last P now differ by 40 pixels or more
    assert abs((bent[0].y + bent[0].height) - (bent[6].y + bent[6].height)) > 40
    assert found["frames"]["bent"] != calm_svg
    # a loop of `phase` 0 to 1 repeats exactly; a quarter of a cycle on, the ripple has moved
    at = lambda phase: drawing.to_svg(1080, 560, progress=1, phase=phase)  # noqa: E731
    assert at(1) == at(0)
    assert at(0.25) != at(0)
    # progress 0 does not look at the phase
    assert drawing.to_svg(1080, 560, progress=0, phase=0.4) == calm_svg
    # every frame keeps the whole line inside the canvas
    for p in (0.5, 1):
        for shape in drawing.shapes(progress=p, phase=0.25)[1:]:
            b = shape.path.bounds
            assert b.x >= 0
            assert b.x + b.width <= 1080
            assert b.y >= 0
            assert b.y + b.height <= 560


def test_wobble_outline_moves_with_phase_and_a_cycle_comes_back_to_where_it_began(run):
    found = run("wobble_outline")
    drawing = found["drawing"]
    shapes = lambda phase: [s.path.to_svg_path() for s in drawing.shapes(wght=400, phase=phase)]  # noqa: E731
    assert len(shapes(0)) == 5
    assert shapes(1) == shapes(0)
    assert shapes(0.3) != shapes(0)
    assert len(found["frames"]) == 3


def test_the_node_example_of_bent_type_carries_the_document_of_the_python_drawing(run):
    drawing = run("bent_type")["drawing"]
    document = EXAMPLES.parent / "crates" / "wasm" / "examples" / "bent_type.json"
    assert document.read_text(encoding="utf-8") == drawing.to_json()


def box(path: btvec.Path) -> tuple[float, float, float, float]:
    b = path.bounds
    return (b.x, b.y, b.width, b.height)


def test_glyph_morph_turns_a_letter_into_a_circle_and_cat_into_horse_on_one_progress(run):
    found = run("glyph_morph")
    drawing, word = found["drawing"], found["word"]
    at = lambda progress: drawing.shapes(progress=progress)  # noqa: E731
    # the first shape is the paper, then the letter, then one shape for each glyph of the longer word
    assert [len(at(p)) for p in (0, 0.5, 1)] == [7, 7, 7]
    # the letter is the R with its counter open (about (265, 245)), then the circle of 150 about (270, 285)
    assert not at(0)[1].path.contains((265, 245))
    circle = at(1)[1].path.bounds
    assert (circle.x, circle.y, circle.width, circle.height) == pytest.approx((120, 135, 300, 300), abs=0.1)
    assert at(1)[1].path.contains((265, 245))
    assert at(0)[1].path.bounds.width < at(0.5)[1].path.bounds.width < circle.width  # half way it is neither
    # the word is CAT, grows no glyph at 0 (the two extra ones are points) and is HORSE at 1, glyph for glyph
    cat, horse = word("CAT").shapes(), word("HORSE").shapes()
    start, end = at(0)[2:], at(1)[2:]
    for shape, wanted in zip(start[:3], cat, strict=True):
        assert box(shape.path) == pytest.approx(box(wanted.path), abs=0.01)
    assert all(shape.path.bounds.width == 0 for shape in start[3:])
    for shape, wanted in zip(end, horse, strict=True):
        assert box(shape.path) == pytest.approx(box(wanted.path), abs=0.01)
    # the letters go one after the other: half way the first is done and the last has not started
    middle = at(0.5)[2:]
    assert box(middle[0].path) == pytest.approx(box(horse[0].path), abs=0.01)
    assert middle[4].path.bounds.width == 0
    # the way back is progress going down: the same frame
    assert drawing.to_svg(1080, 560, progress=0.5) == found["frames"]["half"]
    assert len({found["frames"][name] for name in found["frames"]}) == 3
    # every frame keeps everything inside the canvas
    for p in (0, 0.25, 0.5, 0.75, 1):
        for shape in at(p):
            b = shape.path.bounds
            assert (b.x >= 0, b.y >= 0, b.x + b.width <= 1080, b.y + b.height <= 560) == (True,) * 4


def test_the_node_example_of_glyph_morph_carries_the_document_of_the_python_drawing(run):
    drawing = run("glyph_morph")["drawing"]
    document = EXAMPLES.parent / "crates" / "wasm" / "examples" / "glyph_morph.json"
    assert document.read_text(encoding="utf-8") == drawing.to_json()


def test_the_node_example_of_kinetic_type_carries_the_document_of_the_python_drawing(run):
    piece = run("kinetic_type")["piece"]
    document = EXAMPLES.parent / "crates" / "wasm" / "examples" / "kinetic_type.json"
    assert document.read_text(encoding="utf-8") == piece.to_json()


def test_flow_hatch_fills_each_letter_with_lines_that_stay_inside_it_and_crowd_at_its_edge(run):
    found = run("flow_hatch")
    drawing, word, tone = found["drawing"], found["word"], found["tone"]
    outline = reduce(lambda whole, part: whole.union(part), (shape.path for shape in word.shapes()))
    paper, *letters = drawing.shapes()
    assert paper.style.fill == "#f3eee4ff"
    assert len(letters) == 4
    assert {letter.style.stroke for letter in letters} == {"#16151aff"}
    points = []
    for letter in letters:
        numbers = letter.path.to_svg_path().replace("M", " ").replace("L", " ").split()
        points += [(float(x), float(y)) for x, y in zip(numbers[::2], numbers[1::2], strict=True)]
    assert len(points) > 1000
    # every vertex of every line lies in a letter (a hair of slack for the flattened edge)
    inside = outline.offset(0.5)
    assert all(inside.contains(point) for point in points)
    # the spacing closes up where the tone is 0: a third of the line vertices lie in the outer third of a stroke
    assert sum(tone.at(*point) < 0.3 for point in points) > len(points) / 3


def test_the_node_example_of_flow_hatch_carries_the_document_of_the_python_drawing(run):
    drawing = run("flow_hatch")["drawing"]
    document = EXAMPLES.parent / "crates" / "wasm" / "examples" / "flow_hatch.json"
    assert document.read_text(encoding="utf-8") == drawing.to_json()


def test_line_families_draws_each_hill_with_one_family_inside_its_circle(run):
    found = run("line_families")
    rows, rings, dots = found["drawing"].shapes()
    assert (rows.style.stroke, rings.style.fill, dots.style.fill) == ("#16151aff", "#16151aff", "#c8341cff")
    for shape, hill in zip((rows, rings, dots), found["hills"], strict=True):
        inner, outer = shape.path.bounds, hill.bounds
        assert inner.x >= outer.x - 2
        assert inner.x + inner.width <= outer.x + outer.width + 2
        assert inner.y >= outer.y - 2
        assert inner.y + inner.height <= outer.y + outer.height + 2
    # the plot has a layer for each of the two pens: ink and red
    assert found["frames"]["plot"].count('inkscape:groupmode="layer"') == 2


def test_the_node_example_of_line_families_carries_the_document_of_the_python_drawing(run):
    drawing = run("line_families")["drawing"]
    document = EXAMPLES.parent / "crates" / "wasm" / "examples" / "line_families.json"
    assert document.read_text(encoding="utf-8") == drawing.to_json()


def test_trace_stacks_three_papers_and_traces_the_dither_exactly(run):
    found = run("trace")
    *papers, red, ink = found["drawing"].shapes()
    assert [paper.attributes["level"] for paper in papers] == [0.2, 0.45, 0.7]
    assert [paper.style.fill for paper in papers] == ["#e9dcc3ff", "#c9a678ff", "#8a5a3bff"]
    # each paper lies inside the one below it: the stack is nested
    for lower, upper in pairwise(papers):
        assert upper.path.subtract(lower.path).to_svg_path() == ""
    # the dither is two colours that together are the strip's whole box
    assert (red.attributes["label"], ink.attributes["label"]) == (1, 0)
    strip = red.path.union(ink.path).bounds
    assert (strip.x, strip.y, strip.width, strip.height) == (60, 1000, 840, 126)


def test_the_node_example_of_trace_carries_the_document_of_the_python_drawing(run):
    drawing = run("trace")["drawing"]
    document = EXAMPLES.parent / "crates" / "wasm" / "examples" / "trace.json"
    assert document.read_text(encoding="utf-8") == drawing.to_json()


def test_trace_sequence_leaves_the_disc_alone_where_each_frame_alone_shows_the_dropouts(run):
    found = run("trace_sequence")
    outlines = lambda drawing: drawing.to_svg(640, 640).count("M")  # noqa: E731
    alone, steady = [list(map(outlines, found[name])) for name in ("alone", "drawing")]
    assert alone == [5, 4, 3, 7, 7]  # the disc and a hole for each block of dropout that frame has
    assert steady == [1, 1, 1, 3, 1]  # the disc, and where two frames' dropouts overlap, the one hole they share
