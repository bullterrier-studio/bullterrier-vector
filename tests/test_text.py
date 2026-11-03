"""`Font` and `text` through the public package. The oracle is fontTools, which reads the same font files with code that
shares nothing with the library: the outlines, the advances (`hmtx`, or `getGlyphSet(location=...)` along an axis), the
axes (`fvar`) and the feature tags (`GSUB`, `GPOS`) it reads are the expected values. Two-axis, pinned-axis and
odd-tag fonts are built here with fontTools, so no binary fixture hides what a font holds. `fraunces-subset.ttf` is
2000 units to the em: at `size=2000` a coordinate in the drawing is a coordinate in the font file, y pointing down."""

import hashlib
import math
import re
from collections.abc import Mapping
from io import BytesIO
from pathlib import Path
from types import MappingProxyType
from typing import Any

import pytest
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.basePen import BasePen
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTFont
from fontTools.ttLib.tables.TupleVariation import TupleVariation

import bullterrier_vector as btvec

FONTS = Path(__file__).resolve().parents[1] / "crates" / "core" / "tests" / "it" / "fonts"
FRAUNCES_BYTES = (FONTS / "fraunces-subset.ttf").read_bytes()
SYNTHETIC_BYTES = (FONTS / "synthetic.ttf").read_bytes()
FRAUNCES = btvec.Font(FRAUNCES_BYTES, name="fraunces")
REFERENCE = TTFont(FONTS / "fraunces-subset.ttf")
UPEM = 2000


# --- the oracle ----------------------------------------------------------------------------------------------------


class Commands(BasePen):
    """The glyph as the `M L Q C Z` commands of `Path.to_svg_path`, moved to `(x, y)` and scaled by `scale`, y down."""

    def __init__(self, glyph_set: Any, scale: float, x: float, y: float) -> None:
        super().__init__(glyph_set)
        self.commands: list[tuple[Any, ...]] = []
        self.scale, self.x, self.y = scale, x, y

    def at(self, point: tuple[float, float]) -> tuple[float, float]:
        return (self.x + point[0] * self.scale, self.y - point[1] * self.scale)

    def _moveTo(self, pt: tuple[float, float]) -> None:
        self.commands.append(("M", *self.at(pt)))

    def _lineTo(self, pt: tuple[float, float]) -> None:
        self.commands.append(("L", *self.at(pt)))

    def _qCurveToOne(self, pt1: tuple[float, float], pt2: tuple[float, float]) -> None:
        self.commands.append(("Q", *self.at(pt1), *self.at(pt2)))

    def _curveToOne(self, pt1: tuple[float, float], pt2: tuple[float, float], pt3: tuple[float, float]) -> None:
        self.commands.append(("C", *self.at(pt1), *self.at(pt2), *self.at(pt3)))

    def _closePath(self) -> None:
        self.commands.append(("Z",))


def expected_outline(
    character: str, *, size: float = 100, x: float = 0, y: float = 0, location: Mapping[str, float] | None = None
) -> list[tuple[Any, ...]]:
    """What fontTools draws for `character` in fraunces-subset.ttf, at `location` along the axes."""
    glyph_set = REFERENCE.getGlyphSet(location=location) if location else REFERENCE.getGlyphSet()
    pen = Commands(glyph_set, size / UPEM, x, y)
    glyph_set[REFERENCE.getBestCmap()[ord(character)]].draw(pen)
    return pen.commands


def outline_of(shape: btvec.Shape) -> list[tuple[Any, ...]]:
    """The path data of `shape` read back into commands: a letter and its numbers."""
    parts: list[list[Any]] = []
    for token in re.findall(r"[MLQCZ]|-?[0-9.e+-]+", shape.path.to_svg_path()):
        if token.isalpha():
            parts.append([token])
        else:
            parts[-1].append(float(token))
    return [tuple(part) for part in parts]


def commands_of(drawing: btvec.Drawing, **values: Any) -> list[tuple[Any, ...]]:
    """The commands of every shape of `drawing` in a row."""
    return [command for shape in drawing.shapes(**values) for command in outline_of(shape)]


def assert_same_outline(found: list[tuple[Any, ...]], expected: list[tuple[Any, ...]], tolerance: float) -> None:
    """The same commands, every number within `tolerance` pixels."""
    assert [command[0] for command in found] == [command[0] for command in expected]
    for command, wanted in zip(found, expected, strict=True):
        assert command[1:] == pytest.approx(wanted[1:], abs=tolerance), (command, wanted)


def origin_x(shape: btvec.Shape) -> float:
    """The `x` of the shape's `origin` attribute."""
    origin = shape.attributes["origin"]
    assert isinstance(origin, btvec.Point)
    return origin.x


def characters(drawing: btvec.Drawing) -> list[str]:
    """The `char` attribute of every shape."""
    found = [shape.attributes["char"] for shape in drawing.shapes()]
    assert all(isinstance(char, str) for char in found)
    return [char for char in found if isinstance(char, str)]


def advance(character: str, location: Mapping[str, float] | None = None) -> float:
    """The advance width fontTools reads for `character`, in font units, at `location`."""
    glyph_set = REFERENCE.getGlyphSet(location=location) if location else REFERENCE.getGlyphSet()
    return float(glyph_set[REFERENCE.getBestCmap()[ord(character)]].width)


# --- Font ----------------------------------------------------------------------------------------------------------


def test_a_font_lists_its_axes_as_params_with_the_ranges_and_names_of_its_fvar_table() -> None:
    wanted = {
        axis.axisTag: (axis.minValue, axis.defaultValue, axis.maxValue, REFERENCE["name"].getDebugName(axis.axisNameID))
        for axis in REFERENCE["fvar"].axes
    }
    found = {tag: (axis.min, axis.default, axis.max, axis.meaning) for tag, axis in FRAUNCES.axes.items()}
    assert found == wanted
    assert all(isinstance(axis, btvec.Param) and axis.name == tag for tag, axis in FRAUNCES.axes.items())


def test_a_font_lists_the_feature_tags_of_its_gsub_and_gpos_tables_sorted() -> None:
    wanted = sorted(
        {record.FeatureTag for table in ("GSUB", "GPOS") for record in REFERENCE[table].table.FeatureList.FeatureRecord}
    )
    assert FRAUNCES.features == wanted
    assert btvec.Font(SYNTHETIC_BYTES).features == ["kern"]


def test_a_static_font_has_no_axes_and_the_axes_it_gives_are_a_new_dict_each_time() -> None:
    assert btvec.Font(SYNTHETIC_BYTES).axes == {}
    FRAUNCES.axes.clear()
    assert list(FRAUNCES.axes) == ["wght"]


def test_a_font_has_the_name_it_was_given_or_none() -> None:
    assert FRAUNCES.name == "fraunces"
    assert btvec.Font(FRAUNCES_BYTES).name is None
    assert btvec.Font(bytearray(FRAUNCES_BYTES)).axes.keys() == FRAUNCES.axes.keys()


@pytest.mark.parametrize(
    ("data", "kind", "named"),
    [
        pytest.param(b"", btvec.ParameterError, "data", id="no bytes"),
        pytest.param(b"not a font", btvec.ParameterError, "not a font", id="bytes that are no font"),
        pytest.param(FRAUNCES_BYTES[:200], btvec.ParameterError, "data", id="a font cut short"),
        pytest.param(bytes(32 * 1024 * 1024 + 1), btvec.ParameterError, "MAX_FONT_BYTES", id="past the size limit"),
        pytest.param("fonts/anybody.ttf", TypeError, "got a str", id="a path instead of bytes"),
        pytest.param(None, TypeError, "got a NoneType", id="none"),
    ],
)
def test_a_font_that_cannot_be_read_is_refused_naming_data(data: Any, kind: type[Exception], named: str) -> None:
    with pytest.raises(kind, match=named):
        btvec.Font(data)


@pytest.mark.parametrize("name", ["1st", "with space", "", "ü"])
def test_a_font_name_that_is_not_a_name_is_refused_naming_it(name: str) -> None:
    with pytest.raises(btvec.ParameterError, match=rf'name.*got "{name}"'):
        btvec.Font(FRAUNCES_BYTES, name=name)


# --- the outlines and the advances, against fontTools ---------------------------------------------------------------


@pytest.mark.parametrize(
    ("character", "location"),
    [
        pytest.param("H", None, id="H"),
        pytest.param("é", None, id="e-acute, a composite"),
        pytest.param("f", None, id="f, curves"),
        pytest.param("H", {"wght": 300}, id="H at the lightest"),
        pytest.param("H", {"wght": 700}, id="H at the heaviest"),
        pytest.param("é", {"wght": 700}, id="e-acute at the heaviest"),
        pytest.param("é", {"wght": 600}, id="e-acute between the default and the heaviest"),
    ],
)
def test_a_glyph_is_the_outline_fontTools_draws_to_within_a_font_unit(
    character: str, location: Mapping[str, float] | None
) -> None:
    drawn = btvec.text(character, font=FRAUNCES, size=100, anchor=(10, 200), axes=location)
    # a unit of the font is 100 / 2000 pixels: the library rounds a variation delta to a unit where fontTools does not
    assert_same_outline(commands_of(drawn), expected_outline(character, x=10, y=200, location=location), 0.05)


def test_the_font_default_location_is_the_location_with_no_axes_given() -> None:
    assert commands_of(btvec.text("H", font=FRAUNCES, axes={"wght": 400})) == commands_of(
        btvec.text("H", font=FRAUNCES)
    )


def test_a_glyph_after_another_starts_at_the_advance_of_the_first() -> None:
    first, second = btvec.text("HH", font=FRAUNCES, size=100).shapes()
    assert second.path.bounds.x - first.path.bounds.x == pytest.approx(advance("H") * 100 / UPEM)


def test_a_pair_is_kerned_by_the_font() -> None:
    # HarfBuzz (uharfbuzz) shapes "AV" with A advancing 1334, where the hmtx table says 1424: GPOS pair kerning of -90
    kerned, plain = (
        origin_x(btvec.text("AV", font=FRAUNCES, size=UPEM, features=features).shapes()[1])
        for features in ({}, {"kern": 0})
    )
    assert advance("A") == 1424
    assert (plain - kerned) == pytest.approx(90)


def test_a_ligature_is_one_glyph_until_the_feature_is_switched_off() -> None:
    def chars(**options: Any) -> list[str]:
        return characters(btvec.text("fi", font=FRAUNCES, **options))

    assert chars() == ["f"]
    assert chars(features={"liga": 1}) == ["f"]
    assert chars(features={"liga": 0}) == ["f", "i"]


# --- size, tracking, leading, align, anchor, attributes ------------------------------------------------------------


def left_edges(**options: Any) -> list[float]:
    """The left edge of every glyph of "HH\\nH" in fraunces-subset.ttf at `options`."""
    return [shape.path.bounds.x for shape in btvec.text("HH\nH", font=FRAUNCES, **options).shapes()]


def test_the_default_options_are_a_hundred_pixels_to_the_em_from_the_origin_aligned_left() -> None:
    assert commands_of(btvec.text("HH", font=FRAUNCES)) == commands_of(
        btvec.text("HH", font=FRAUNCES, size=100, tracking=0, leading=120, align="left", anchor=(0, 0))
    )


def test_size_is_pixels_to_the_em() -> None:
    assert_same_outline(commands_of(btvec.text("H", font=FRAUNCES, size=50)), expected_outline("H", size=50), 1e-9)


def test_tracking_is_thousandths_of_an_em_between_glyphs_not_after_the_last() -> None:
    plain, tracked = left_edges(), left_edges(tracking=100)  # 100 / 1000 of 100 pixels
    assert tracked[1] - plain[1] == pytest.approx(10)
    assert tracked[0] == plain[0]
    # the first glyph of the next line does not move, and the negative tracking tightens
    assert tracked[2] == plain[2]
    assert left_edges(tracking=-100)[1] - plain[1] == pytest.approx(-10)


def test_leading_is_pixels_between_baselines_and_one_and_a_fifth_em_by_default() -> None:
    def lines(**options: Any) -> list[float]:
        return [shape.path.bounds.y for shape in btvec.text("H\nH", font=FRAUNCES, **options).shapes()]

    default, tight = lines(size=100), lines(size=100, leading=50)
    assert default[1] - default[0] == pytest.approx(120)
    assert tight[1] - tight[0] == pytest.approx(50)


@pytest.mark.parametrize(
    ("align", "first_line_shift", "second_line_shift"),
    [
        pytest.param("left", 0, 0, id="left"),
        pytest.param("center", -1656 * 100 / UPEM, -1656 * 100 / UPEM / 2, id="center"),
        pytest.param("right", -2 * 1656 * 100 / UPEM, -1656 * 100 / UPEM, id="right"),
    ],
)
def test_each_line_is_aligned_by_its_own_width_against_the_anchor(
    align: str, first_line_shift: float, second_line_shift: float
) -> None:
    # "HH" is two advances of 1656 wide and "H" one
    plain, aligned = left_edges(), left_edges(align=align)
    assert [aligned[0] - plain[0], aligned[2] - plain[2]] == pytest.approx([first_line_shift, second_line_shift])


def test_the_anchor_moves_the_first_baseline_and_a_point_is_as_good_as_a_tuple() -> None:
    at_origin = commands_of(btvec.text("H", font=FRAUNCES))
    moved = commands_of(btvec.text("H", font=FRAUNCES, anchor=btvec.Point(30, 40)))
    assert_same_outline(moved, [(c[0], *[v + (30, 40)[i % 2] for i, v in enumerate(c[1:])]) for c in at_origin], 1e-9)
    assert moved == commands_of(btvec.text("H", font=FRAUNCES, anchor=(30, 40)))


def test_shapes_carry_the_glyph_char_word_line_and_order_of_the_text() -> None:
    shapes = btvec.text("Hi To\nH", font=FRAUNCES).shapes()  # the space draws nothing
    rows = [(s.attributes["glyph"], s.attributes["char"], s.attributes["word"], s.attributes["line"]) for s in shapes]
    assert rows == [(0, "H", 0, 0), (1, "i", 0, 0), (2, "T", 1, 0), (3, "o", 1, 0), (4, "H", 2, 1)]
    assert [s.attributes["order"] for s in shapes] == pytest.approx([0, 0.25, 0.5, 0.75, 1])
    assert [s.attributes["index"] for s in shapes] == [0, 1, 2, 3, 4]


def test_a_text_with_nothing_to_draw_is_an_empty_drawing() -> None:
    assert btvec.text("", font=FRAUNCES).shapes() == []
    assert btvec.text(" \n ", font=FRAUNCES).shapes() == []


# --- axes as Params and per character -------------------------------------------------------------------------------


def test_an_axis_given_a_param_is_drawn_at_the_value_the_draw_gives_it() -> None:
    drawing = btvec.text("é", font=FRAUNCES, axes={"wght": FRAUNCES.axes["wght"]})
    assert [param.name for param in drawing.parameters()] == ["wght"]
    assert commands_of(drawing) == commands_of(btvec.text("é", font=FRAUNCES))
    assert commands_of(drawing, wght=700) == commands_of(btvec.text("é", font=FRAUNCES, axes={"wght": 700}))
    assert commands_of(drawing, wght=700) != commands_of(drawing, wght=300)


def test_an_expression_on_an_axis_is_worked_out_when_drawn_and_checked_against_the_range() -> None:
    boost = btvec.Param("boost", default=0, min=0, max=500)
    drawing = btvec.text("H", font=FRAUNCES, axes={"wght": 300 + boost})
    assert commands_of(drawing, boost=400) == commands_of(btvec.text("H", font=FRAUNCES, axes={"wght": 700}))
    with pytest.raises(btvec.ParameterError, match=r"wght.*300 to 700.*800"):
        drawing.shapes(boost=500)


def test_a_list_for_an_axis_gives_each_character_its_own_value_and_the_advances_follow() -> None:
    light, heavy = {"wght": 300}, {"wght": 700}
    drawn = btvec.text("HH", font=FRAUNCES, size=UPEM, axes={"wght": FRAUNCES.axes["wght"]})
    first, second = drawn.shapes(wght=[300, 700])  # at size=UPEM a font unit is a pixel
    assert_same_outline(outline_of(first), expected_outline("H", size=UPEM, location=light), 1)
    # the second letter is the heavy one, starting where the light one's advance ends
    assert_same_outline(outline_of(second), expected_outline("H", size=UPEM, x=advance("H", light), location=heavy), 1)
    assert origin_x(second) == pytest.approx(advance("H", light) + advance("H", heavy) / 2, abs=1)


def test_a_list_with_the_wrong_number_of_entries_is_refused_naming_the_param_and_both_counts() -> None:
    drawn = btvec.text("HH", font=FRAUNCES, axes={"wght": FRAUNCES.axes["wght"]})
    with pytest.raises(btvec.ParameterError, match=r"wght.*3.*2"):
        drawn.shapes(wght=[300, 400, 700])


def test_a_heavier_weight_makes_a_wider_word_by_the_advances_of_the_font() -> None:
    def third_origin(weight: float) -> float:
        return origin_x(btvec.text("HHH", font=FRAUNCES, axes={"wght": weight}).shapes()[2])

    assert third_origin(300) < third_origin(400) < third_origin(700)
    for weight in (300, 400, 700):  # the third glyph's middle: two advances and half of a third
        assert third_origin(weight) == pytest.approx(2.5 * advance("H", {"wght": weight}) * 100 / UPEM, abs=0.05)


# --- fonts built here: two axes, a pinned axis, a tag that is no name -------------------------------------------------

SQUARE_AT_ORIGIN = [(0, 0), (0, 100), (100, 100), (100, 0)]  # a 100 unit square in a 1000 unit em


def built_font(axes: list[tuple[str, float, float, float, str]], deltas: dict[str, list[tuple[int, int]]]) -> bytes:
    """A font whose only glyph, `A`, is the square above; `deltas[tag]` moves its four points (and four phantom points
    that the glyph table also holds) by that much at the top of the axis `tag`."""
    builder = FontBuilder(1000, isTTF=True)
    builder.setupGlyphOrder([".notdef", "A"])
    builder.setupCharacterMap({0x41: "A"})
    pen = TTGlyphPen(None)
    pen.moveTo(SQUARE_AT_ORIGIN[0])
    for point in SQUARE_AT_ORIGIN[1:]:
        pen.lineTo(point)
    pen.closePath()
    builder.setupGlyf({".notdef": TTGlyphPen(None).glyph(), "A": pen.glyph()})
    builder.setupHorizontalMetrics({".notdef": (500, 0), "A": (500, 0)})
    builder.setupHorizontalHeader(ascent=800, descent=-200)
    builder.setupNameTable({"familyName": "Probe", "styleName": "Regular"})
    builder.setupOS2()
    builder.setupPost()
    builder.setupFvar(axes, [])
    if deltas:
        builder.setupGvar(
            {
                "A": [
                    TupleVariation({tag: (0, 1, 1)}, [*moves, (0, 0), (0, 0), (0, 0), (0, 0)])
                    for tag, moves in deltas.items()
                ]
            }
        )
    out = BytesIO()
    builder.save(out)
    return out.getvalue()


# `wght` is first in the font and `wdth` second, the reverse of alphabetical order: at the top of `wght` the right edge
# (points 2 and 3) moves 50 units, at the top of `wdth` the top edge (points 1 and 2) moves 30 units
TWO_AXES = built_font(
    [("wght", 100, 400, 900, "Weight"), ("wdth", 50, 100, 200, "Width")],
    {"wght": [(0, 0), (0, 0), (50, 0), (50, 0)], "wdth": [(0, 0), (0, 30), (0, 30), (0, 0)]},
)


def square_size(font: btvec.Font, **axes: float) -> tuple[float, float]:
    """The width and height in pixels of the square `A` at 100 pixels to the em (10 pixels for 100 units)."""
    bounds = btvec.text("A", font=font, axes=axes).shapes()[0].path.bounds
    return (bounds.width, bounds.height)


@pytest.mark.parametrize(
    ("axes", "size"),
    [
        pytest.param({}, (10, 10), id="the defaults"),
        pytest.param({"wght": 900}, (15, 10), id="the top of wght moves the right edge by 50 units"),
        pytest.param({"wdth": 200}, (10, 13), id="the top of wdth moves the top edge by 30 units"),
        pytest.param({"wght": 650}, (12.5, 10), id="halfway up wght, half of the move"),
        pytest.param({"wdth": 200, "wght": 900}, (15, 13), id="both"),
        pytest.param({"wght": 100}, (10, 10), id="the bottom of wght: no delta defined"),
    ],
)
def test_each_axis_of_a_two_axis_font_moves_the_edge_the_font_ties_to_it(
    axes: dict[str, float], size: tuple[float, float]
) -> None:
    assert square_size(btvec.Font(TWO_AXES), **axes) == pytest.approx(size, abs=0.01)


def test_the_axes_of_a_font_come_in_the_order_of_its_fvar_table_not_alphabetical_order() -> None:
    assert list(btvec.Font(TWO_AXES).axes) == ["wght", "wdth"]


PINNED = built_font([("wght", 400, 400, 400, "Weight"), ("wdth", 50, 100, 200, "Width")], {})
ODD_TAG = built_font([("a b ", 0, 50, 100, "Odd"), ("wdth", 50, 100, 200, "Width")], {})


@pytest.mark.parametrize(
    ("data", "listed"),
    [
        pytest.param(PINNED, ["wdth"], id="an axis with nothing to vary"),
        pytest.param(ODD_TAG, ["wdth"], id="a tag that is no Param name"),
    ],
)
def test_an_axis_a_param_cannot_hold_is_not_listed_but_is_named_when_one_is_refused_and_set_by_its_tag(
    data: bytes, listed: list[str]
) -> None:
    font = btvec.Font(data)
    assert list(font.axes) == listed
    tag = "wght" if data == PINNED else "a b "
    value = 400 if data == PINNED else 50
    assert btvec.text("A", font=font, axes={tag: value}).shapes()  # settable by tag, inside its range
    with pytest.raises(btvec.ParameterError, match=re.escape(f"{tag} (")):  # the refusal lists it among the axes
        btvec.text("A", font=font, axes={"nope": 1})


def hebrew_font() -> bytes:
    """A font of two Hebrew letters, squares 400 and 100 units wide and 500 and 200 units to advance, with a `wght` axis
    that moves nothing: only the cut of a text into runs can differ between two weights."""
    builder = FontBuilder(1000, isTTF=True)
    builder.setupGlyphOrder([".notdef", "alef", "bet"])
    builder.setupCharacterMap({0x5D0: "alef", 0x5D1: "bet"})
    glyphs = {".notdef": TTGlyphPen(None).glyph()}
    for name, width in (("alef", 400), ("bet", 100)):
        pen = TTGlyphPen(None)
        pen.moveTo((0, 0))
        for point in ((0, 100), (width, 100), (width, 0)):
            pen.lineTo(point)
        pen.closePath()
        glyphs[name] = pen.glyph()
    builder.setupGlyf(glyphs)
    builder.setupHorizontalMetrics({".notdef": (500, 0), "alef": (500, 0), "bet": (200, 0)})
    builder.setupHorizontalHeader(ascent=800, descent=-200)
    builder.setupNameTable({"familyName": "Probe", "styleName": "Regular"})
    builder.setupOS2()
    builder.setupPost()
    builder.setupFvar([("wght", 100, 400, 900, "Weight")], [])
    out = BytesIO()
    builder.save(out)
    return out.getvalue()


def test_a_right_to_left_line_cut_into_runs_stands_as_it_does_uncut() -> None:
    font = btvec.Font(hebrew_font())
    line = "אבב"  # alef, bet, bet: read from the right, so alef stands rightmost
    uncut = btvec.text(line, font=font, size=1000).shapes()
    cut = btvec.text(line, font=font, size=1000, axes={"wght": font.axes["wght"]}).shapes(wght=[100, 100, 900])
    assert [(shape.path.bounds.x, shape.path.bounds.width) for shape in cut] == [
        (shape.path.bounds.x, shape.path.bounds.width) for shape in uncut
    ]
    assert [shape.path.bounds.width for shape in uncut] == [100, 100, 400]  # the independent oracle: hand reading


# --- layout: width wraps the lines, fit makes each exactly that wide ----------------------------------------------


def lines_of(drawing: btvec.Drawing, **values: Any) -> list[object]:
    """The `line` attribute of every shape."""
    return [shape.attributes["line"] for shape in drawing.shapes(**values)]


H, SPACE = advance("H"), advance(" ")


@pytest.mark.parametrize(
    ("content", "width", "lines"),
    [
        pytest.param("H H", 2 * H + SPACE, [0, 0], id="exactly the width fits"),
        pytest.param("H H", 2 * H + SPACE - 1, [0, 1], id="one unit less wraps"),
        pytest.param("H H H", 2 * H + SPACE, [0, 0, 1], id="greedy"),
        pytest.param("H   H", H, [0, 1], id="the spaces at a break are dropped"),
        pytest.param("HH H", 1, [0, 0, 1], id="a word wider than the width stands alone"),
        pytest.param("H\n\nH", 10 * H, [0, 2], id="an empty paragraph keeps its line"),
    ],
)
def test_width_wraps_a_line_at_whitespace_as_late_as_it_fits(content: str, width: float, lines: list[int]) -> None:
    assert lines_of(btvec.text(content, font=FRAUNCES, size=UPEM, width=width)) == lines


def test_a_wrapped_line_starts_at_the_anchor_and_stands_one_leading_below() -> None:
    shapes = btvec.text("H H", font=FRAUNCES, size=UPEM, width=H, anchor=(10, 20)).shapes()
    assert_same_outline(outline_of(shapes[0]), expected_outline("H", size=UPEM, x=10, y=20), 1e-9)
    assert_same_outline(outline_of(shapes[1]), expected_outline("H", size=UPEM, x=10, y=20 + 1.2 * UPEM), 1e-9)


def test_the_breaks_follow_the_width_param_and_not_the_size_param() -> None:
    size = btvec.Param("size", default=UPEM, min=500, max=4000)
    width = btvec.Param("width", default=2 * H + SPACE - 1, min=1, max=100000)
    drawing = btvec.text("H H", font=FRAUNCES, size=size, width=width)
    assert lines_of(drawing) == [0, 1]
    assert lines_of(drawing, size=1000) == [0, 1]  # narrower glyphs would fit; the lines are made at the default size
    assert lines_of(drawing, width=2 * H + SPACE) == [0, 0]


def test_fit_size_makes_each_line_the_size_at_which_it_is_the_width_wide() -> None:
    # "H" is H units wide at 2000 to the em: 2 H wide at 4000. "HH" is 2 H wide at 2000. Each line stands 1.2 of its own
    # size below the one before.
    shapes = btvec.text("H\nHH", font=FRAUNCES, size=UPEM, width=2 * H, fit="size").shapes()
    assert_same_outline(outline_of(shapes[0]), expected_outline("H", size=2 * UPEM), 1e-9)
    assert_same_outline(outline_of(shapes[1]), expected_outline("H", size=UPEM, y=1.2 * UPEM), 1e-9)
    assert_same_outline(outline_of(shapes[2]), expected_outline("H", size=UPEM, x=H, y=1.2 * UPEM), 1e-9)


@pytest.mark.parametrize("weight", [300, 400, 500, 600, 700])
def test_fit_on_an_axis_sets_the_axis_at_which_the_line_is_the_width_wide(weight: int) -> None:
    step = round(advance("H", {"wght": weight}))  # the shaper rounds an advance to a unit; fontTools does not
    shapes = btvec.text("HHH", font=FRAUNCES, size=UPEM, width=3 * step, fit="wght").shapes()
    assert origin_x(shapes[1]) - origin_x(shapes[0]) == pytest.approx(step)
    assert origin_x(shapes[2]) - origin_x(shapes[1]) == pytest.approx(step)


def test_a_width_the_axis_cannot_reach_is_refused_when_drawn_naming_the_line_and_the_nearest_width() -> None:
    narrowest = 3 * round(advance("H", {"wght": 300}))
    drawing = btvec.text("HHH", font=FRAUNCES, size=UPEM, width=narrowest - 1, fit="wght")
    with pytest.raises(btvec.ParameterError, match=rf"fit.*line 0.*{narrowest:g}.*300"):
        drawing.shapes()


def test_width_and_fit_are_saved_in_the_document_and_load_back_to_the_same_shapes() -> None:
    drawing = btvec.text("HHH", font=FRAUNCES, size=UPEM, width=3 * H, fit="wght")
    text = drawing.to_json()
    assert '"width":' in text
    assert '"fit":"wght"' in text
    loaded = btvec.Drawing.from_json(text, inputs={"fraunces": FRAUNCES_BYTES})
    assert loaded.to_json() == text
    assert loaded.shapes() == drawing.shapes()
    assert '"width"' not in btvec.text("H", font=FRAUNCES).to_json()


# --- bad arguments name the argument and the value ----------------------------------------------------------------


@pytest.mark.parametrize(
    ("options", "kind", "named"),
    [
        pytest.param({"align": "middle"}, btvec.ParameterError, r'align.*got "middle"', id="align"),
        pytest.param({"size": 0}, btvec.ParameterError, r"size.*above 0.*got 0", id="size of zero"),
        pytest.param({"size": -5}, btvec.ParameterError, r"size.*got -5", id="a negative size"),
        pytest.param({"size": float("nan")}, btvec.ParameterError, r"size.*NaN", id="size is not a number"),
        pytest.param({"size": "big"}, TypeError, r"'big'", id="size is a string"),
        pytest.param({"tracking": float("inf")}, btvec.ParameterError, r"tracking.*inf", id="infinite tracking"),
        pytest.param({"leading": "tall"}, TypeError, r"'tall'", id="leading is a string"),
        pytest.param({"anchor": (1,)}, TypeError, r"anchor.*\(1,\)", id="a one-number anchor"),
        pytest.param(
            {"anchor": btvec.Param("a", default=1, min=0, max=2)},
            TypeError,
            r"anchor.*plain numbers.*Param",
            id="a Param anchor",
        ),
        pytest.param(
            {"axes": {"wght": 9999}}, btvec.ParameterError, r"axes.*wght.*300 to 700.*9999", id="axis out of range"
        ),
        pytest.param({"axes": {"nope": 1}}, btvec.ParameterError, r"axes.*nope.*wght", id="an axis the font lacks"),
        pytest.param({"axes": {"w": 1}}, btvec.ParameterError, r'axes.*"w".*four', id="a tag of one character"),
        pytest.param({"axes": {"wght": "bold"}}, TypeError, r"'bold'", id="an axis value that is a string"),
        pytest.param(
            {"axes": {"wght": float("nan")}}, btvec.ParameterError, r"axes.*NaN", id="an axis value that is nan"
        ),
        pytest.param({"axes": {1: 400}}, TypeError, r"str", id="an axis tag that is a number"),
        pytest.param(
            {"features": {"zzzz": 1}},
            btvec.ParameterError,
            r"features.*zzzz.*kern, liga",
            id="a feature the font lacks",
        ),
        pytest.param(
            {"features": {"liga": -1}}, btvec.ParameterError, r"features.*liga.*-1", id="a negative feature value"
        ),
        pytest.param(
            {"features": {"liga": 2**32}}, btvec.ParameterError, r"features.*liga.*4294967296", id="a feature past u32"
        ),
        pytest.param({"features": {"liga": 1.5}}, TypeError, r"features.*liga.*1\.5", id="a fractional feature value"),
        pytest.param(
            {"features": {"liga": "on"}}, TypeError, r"features.*liga.*'on'", id="a feature value that is a string"
        ),
        pytest.param({"width": 0}, btvec.ParameterError, r"width.*above 0.*got 0", id="a width of zero"),
        pytest.param({"width": float("nan")}, btvec.ParameterError, r"width.*NaN", id="a width that is not a number"),
        pytest.param({"width": "wide"}, TypeError, r"'wide'", id="a width that is a string"),
        pytest.param({"fit": "size"}, btvec.ParameterError, r"fit.*width", id="a fit without a width"),
        pytest.param({"width": 100, "fit": "bold"}, btvec.ParameterError, r'fit.*"bold"', id="a fit that is no axis"),
        pytest.param(
            {"width": 100, "fit": "wght", "axes": {"wght": 400}},
            btvec.ParameterError,
            r"fit.*axes",
            id="a fit and axes",
        ),
        pytest.param({"width": 100, "fit": 5}, TypeError, r"fit", id="a fit that is a number"),
        pytest.param({"font": "Fraunces"}, TypeError, r"'Fraunces'|font", id="a font that is a name"),
    ],
)
def test_a_bad_argument_is_refused_naming_the_argument_and_the_value(
    options: dict[str, Any], kind: type[Exception], named: str
) -> None:
    arguments: dict[str, Any] = {"font": FRAUNCES, **options}
    with pytest.raises(kind, match=named):
        btvec.text("fi", **arguments)


def test_axes_features_and_inputs_may_be_any_mapping_as_the_stub_says() -> None:
    text = btvec.text("H", font=FRAUNCES, axes={"wght": 700}, features={"liga": 0})
    given = btvec.text("H", font=FRAUNCES, axes=MappingProxyType({"wght": 700}), features=MappingProxyType({"liga": 0}))
    assert given.to_json() == text.to_json()
    loaded = btvec.Drawing.from_json(text.to_json(), inputs=MappingProxyType({"fraunces": FRAUNCES_BYTES}))
    assert loaded.to_json() == text.to_json()


def test_a_content_that_is_not_text_or_holds_a_lone_surrogate_is_refused_naming_content() -> None:
    with pytest.raises(TypeError, match="content"):
        btvec.text(5, font=FRAUNCES)  # type: ignore[arg-type]  # the wrong type is the point
    with pytest.raises(btvec.ParameterError, match=r"content.*surrogate"):
        btvec.text("a\ud800b", font=FRAUNCES)


def test_a_text_of_more_than_a_hundred_thousand_characters_is_refused_naming_the_content() -> None:
    with pytest.raises(btvec.ParameterError, match=r"content.*100000|content.*100,000"):
        btvec.text("H" * 100_001, font=FRAUNCES)


def test_the_arguments_after_content_are_keywords() -> None:
    with pytest.raises(TypeError, match="positional"):
        btvec.text("H", FRAUNCES)  # type: ignore[call-arg]  # the font is a keyword argument


# --- JSON: the font is an input, named, with its digest ------------------------------------------------------------


def test_a_text_saves_its_font_as_a_named_input_with_the_sha256_of_its_bytes_and_loads_back_given_the_bytes() -> None:
    drawing = btvec.text("fi H", font=FRAUNCES, size=80, axes={"wght": 650}, features={"liga": 0}, align="center")
    text = drawing.to_json()
    assert f'"sha256":"{hashlib.sha256(FRAUNCES_BYTES).hexdigest()}"' in text
    assert '"fraunces"' in text
    assert FRAUNCES_BYTES[:64].hex() not in text  # the document holds no bytes
    loaded = btvec.Drawing.from_json(text, inputs={"fraunces": FRAUNCES_BYTES})
    assert loaded.to_json() == text
    assert loaded.to_svg(400, 300) == drawing.to_svg(400, 300)
    assert loaded.shapes() == drawing.shapes()


def test_a_font_without_a_name_is_called_font1_in_the_document() -> None:
    text = btvec.text("H", font=btvec.Font(FRAUNCES_BYTES)).to_json()
    assert btvec.Drawing.from_json(text, inputs={"font1": bytearray(FRAUNCES_BYTES)}).shapes()


@pytest.mark.parametrize(
    ("inputs", "kind", "named"),
    [
        pytest.param(None, btvec.ParameterError, r"inputs\.fraunces", id="no inputs"),
        pytest.param({}, btvec.ParameterError, r"inputs\.fraunces", id="an empty dict"),
        pytest.param(
            {"fraunces": SYNTHETIC_BYTES}, btvec.ParameterError, r"inputs\.fraunces.*SHA-256", id="other bytes"
        ),
        pytest.param(
            {"fraunces": FRAUNCES_BYTES, "extra": b"x"},
            btvec.ParameterError,
            r"inputs.*extra",
            id="an input nothing declares",
        ),
        pytest.param({"fraunces": FRAUNCES_BYTES[:100]}, btvec.ParameterError, r"inputs\.fraunces", id="a cut font"),
        pytest.param({"fraunces": "bytes"}, TypeError, r"inputs\[fraunces\].*got a str", id="text instead of bytes"),
        pytest.param({3: FRAUNCES_BYTES}, TypeError, r"str", id="a name that is a number"),
    ],
)
def test_a_document_loaded_without_its_font_or_with_the_wrong_inputs_is_refused_naming_the_input(
    inputs: Any, kind: type[Exception], named: str
) -> None:
    text = btvec.text("H", font=FRAUNCES).to_json()
    with pytest.raises(kind, match=named):
        btvec.Drawing.from_json(text, inputs=inputs)


def test_a_document_without_inputs_loads_with_or_without_the_argument() -> None:
    text = btvec.Drawing(btvec.Path.circle((5, 5), 2)).to_json()
    assert btvec.Drawing.from_json(text, inputs={}).to_json() == text


# --- inside -------------------------------------------------------------------------------------------------------


def pen_outlines(shapes: list[btvec.Shape], pens: list[tuple[float, float]]) -> None:
    """Every shape is the glyph `H` with its pen at the matching (x, baseline)."""
    assert len(shapes) == len(pens)
    for shape, (x, y) in zip(shapes, pens, strict=True):
        assert_same_outline(outline_of(shape), expected_outline("H", size=UPEM, x=x, y=y), 1e-9)


HOLED = btvec.Path.from_svg_path("M0 0H10000V20000H0Z M3000 3000V17000H6000V3000Z")  # the hole runs the other way
CAPITAL = 1400  # the cap height of fraunces-subset.ttf, a capital below the top of a shape is the first baseline


@pytest.mark.parametrize(
    ("shape", "content", "options", "pens"),
    [
        pytest.param(
            btvec.Path.rect(500, 300, 2 * H + SPACE, 20000),
            "H H H",
            {},
            [(500, 300 + CAPITAL), (500 + H + SPACE, 300 + CAPITAL), (500, 300 + CAPITAL + 1.2 * UPEM)],
            id="a rectangle is a width, a capital below its top",
        ),
        pytest.param(
            HOLED,
            "H H H H H H",
            {},
            [
                *[(k * (H + SPACE), CAPITAL) for k in range(4)],
                (6000, CAPITAL + 1.2 * UPEM),
                (6000 + H + SPACE, CAPITAL + 1.2 * UPEM),
            ],
            id="round a hole the widest stretch takes the line",
        ),
        pytest.param(
            btvec.Path.rect(1000, 0, 9000, 9000),
            "H",
            {"align": "center"},
            [(5500 - H / 2, CAPITAL)],
            id="centre against the middle of the stretch",
        ),
        pytest.param(
            btvec.Path.rect(1000, 0, 9000, 9000),
            "H",
            {"align": "right"},
            [(10000 - H, CAPITAL)],
            id="right against its end",
        ),
        pytest.param(
            btvec.Path.rect(0, 0, 10000, 20000),
            "H\n\nH",
            {"leading": 3000},
            [(0, CAPITAL), (0, CAPITAL + 6000)],
            id="an empty line takes a baseline, a leading below",
        ),
    ],
)
def test_inside_sets_each_line_in_the_stretches_of_the_shape(
    shape: btvec.Path, content: str, options: dict[str, Any], pens: list[tuple[float, float]]
) -> None:
    pen_outlines(btvec.text(content, font=FRAUNCES, size=UPEM, inside=shape, **options).shapes(), pens)


def test_a_text_the_shape_cannot_hold_is_refused_naming_the_word_and_the_line() -> None:
    drawing = btvec.text("H H H", font=FRAUNCES, size=UPEM, inside=btvec.Path.rect(0, 0, 2 * H + SPACE, 3000))
    with pytest.raises(btvec.ParameterError, match=r'inside.*does not fit.*"H".*line 1'):
        drawing.shapes()


@pytest.mark.parametrize(
    ("options", "named"),
    [
        pytest.param({"width": 100}, r"inside.*width", id="a width"),
        pytest.param({"fit": "size"}, r"inside.*fit", id="a fit"),
        pytest.param({"anchor": (1, 2)}, r"inside.*anchor", id="an anchor"),
        pytest.param({"leading": 0}, r"leading.*0.*above 0", id="a leading of zero"),
        pytest.param({"leading": -2.5}, r"leading.*-2.5", id="a negative leading"),
    ],
)
def test_inside_excludes_what_places_the_text_and_wants_a_leading_above_zero(
    options: dict[str, Any], named: str
) -> None:
    with pytest.raises(btvec.ParameterError, match=named):
        btvec.text("H", font=FRAUNCES, inside=btvec.Path.rect(0, 0, 9, 9), **options)


def test_inside_wants_a_path() -> None:
    with pytest.raises(TypeError, match="inside"):
        btvec.text("H", font=FRAUNCES, inside="M0 0H9V9H0Z")  # type: ignore[arg-type]  # the wrong type is the point


def test_the_shape_is_saved_as_path_data_and_loads_back_to_the_same_shapes() -> None:
    drawing = btvec.text("H H H", font=FRAUNCES, size=UPEM, inside=btvec.Path.rect(500, 300, 2 * H + SPACE, 20000))
    text = drawing.to_json()
    assert '"inside":"M500 300' in text
    loaded = btvec.Drawing.from_json(text, inputs={"fraunces": FRAUNCES_BYTES})
    assert loaded.to_json() == text
    assert loaded.shapes() == drawing.shapes()
    assert '"inside"' not in btvec.text("H", font=FRAUNCES).to_json()


# --- along --------------------------------------------------------------------------------------------------------


def test_text_carried_along_a_circle_has_its_glyphs_at_the_distances_their_pens_stood():
    radius, start = 300.0, 0.2
    ring = btvec.Path.circle((0, 0), radius)
    flat = btvec.text("HIHIHI", font=FRAUNCES, size=60)
    bent = flat.along(ring, start=start)
    for before, after in zip(flat.shapes(), bent.shapes(), strict=True):
        assert after.attributes == before.attributes
        # the middle of a glyph (an H or an I turns about its middle without the middle of its box moving) goes to
        # the angle the distance over the radius makes, and to a radius of 300 plus its height above the baseline
        x = before.path.bounds.x + before.path.bounds.width / 2
        y = before.path.bounds.y + before.path.bounds.height / 2
        angle = (start * 2 * math.pi * radius + x) / radius
        reach = radius - y
        middle = after.path.bounds
        centre = (middle.x + middle.width / 2, middle.y + middle.height / 2)
        assert centre == pytest.approx((reach * math.sin(angle), -reach * math.cos(angle)), abs=1.5)
