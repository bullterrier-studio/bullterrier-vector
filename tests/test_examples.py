"""The example programs in examples/ run as far as they can draw and draw what their comments say. Each runs with
`runpy` in an empty folder, which it must leave empty: the picture stays in a variable. Expected values are worked out
by hand: the star's from the geometry of a five-pointed star, the bars' from the series printed in the program."""

import runpy
import xml.etree.ElementTree as ET
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


def test_kinetic_type_draws_a_shape_a_letter_inside_the_frame_and_a_kick_thickens_only_the_letter_it_hits(run):
    found = run("kinetic_type")
    letters = found["letters"]
    calm = letters.shapes()
    assert [shape.attributes["char"] for shape in calm] == list("NOSLEEP")
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
        assert 50 <= left < 100, (line, left)
        assert 980 < right <= 1030, (line, right)
        assert (left + right) / 2 == pytest.approx(540, abs=30)

    # `pulse` has an entry per character of "NO\nSLEEP": 8, the line break included. A kick on the N (entry 0) makes
    # the N wider, and the letters after it follow its advance; the second line is untouched.
    kicked = letters.shapes(pulse=[1, 0, 0, 0, 0, 0, 0, 0])
    assert kicked[0].path.bounds.width > calm[0].path.bounds.width
    assert kicked[1].path.bounds.width == pytest.approx(calm[1].path.bounds.width)
    assert [shape.path for shape in kicked[2:]] == [shape.path for shape in calm[2:]]


def test_kinetic_type_at_a_time_rises_the_plane_and_swaps_the_hairlines_for_the_print(run):
    at_time = run("kinetic_type")["at_time"]
    before, after = at_time(2.0).shapes(), at_time(4.0).shapes()
    paper, plane, *rest = before
    assert len(before) == len(after) == 1 + 1 + 7 + 7  # paper, plane, the letters printed, the letters as hairlines
    assert paper.path.bounds == btvec.Rect(0, 0, 1080, 1350)
    # half risen at 2 s (1 s in a rise of 2 s), all the way at 4 s
    assert plane.path.bounds.y == 1350 / 2
    assert after[1].path.bounds.y == 0
    assert [shape.style.opacity for shape in rest] == [0.0] * 7 + [1.0] * 7
    assert [shape.style.opacity for shape in after[2:]] == [1.0] * 7 + [0.0] * 7
    # the kick starts on the beat at 4 s with the first letter, the next ones a little behind it
    assert after[2].path.bounds.width > rest[0].path.bounds.width


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
