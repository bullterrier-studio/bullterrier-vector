"""`characters` and `marks` through the public package. Expected values are worked out by hand from the rules in
docs/api.md, and from the shipped font read with fontTools (code that shares nothing with the library): the advance of
every printable ASCII glyph, the ink of `H`, the capital height."""

import json
from dataclasses import dataclass
from pathlib import Path

import pytest
from fontTools.pens.boundsPen import BoundsPen
from fontTools.ttLib import TTFont

import bullterrier_vector as btvec

FONT_FILE = (
    Path(__file__).resolve().parents[1] / "crates" / "core" / "fonts" / "jetbrains_mono" / "jetbrains_mono.woff2"
)


@dataclass
class ImageCharacters:
    """What bullterrier-image's `characters` returns, as far as vector reads it."""

    text: str
    colors: list[list[str]]


def test_the_bundled_monospaced_font_has_one_advance_for_every_printable_ascii_glyph() -> None:
    font = TTFont(FONT_FILE)
    cmap, hmtx = font.getBestCmap(), font["hmtx"]
    advances = {hmtx[cmap[code]][0] for code in range(0x21, 0x7F)}
    assert advances == {600}
    assert "jetbrains_mono" in btvec.Font.bundled_names()


def test_the_default_font_is_the_bundled_monospaced_one() -> None:
    default = btvec.characters("HH", cell=(6, 12)).shapes()
    named = btvec.characters("HH", cell=(6, 12), font=btvec.Font.bundled("jetbrains_mono")).shapes()
    assert [shape.path.to_svg_path() for shape in default] == [shape.path.to_svg_path() for shape in named]


def test_a_capital_h_is_centred_in_its_cell_by_the_font_numbers() -> None:
    font = TTFont(FONT_FILE)
    pen = BoundsPen(font.getGlyphSet())
    font.getGlyphSet()["H"].draw(pen)
    x_min, y_min, x_max, y_max = pen.bounds
    cap = font["OS/2"].sCapHeight
    size = 10.0
    (shape,) = btvec.characters("H", cell=(6, 12), size=size).shapes()
    bounds = shape.path.bounds
    unit = size / font["head"].unitsPerEm
    assert bounds.x == pytest.approx(3 + (x_min - 300) * unit)
    assert bounds.x + bounds.width == pytest.approx(3 + (x_max - 300) * unit)
    assert bounds.y == pytest.approx(6 - cap / 2 * unit)
    assert bounds.y + bounds.height == pytest.approx(6 - cap / 2 * unit + (y_max - y_min) * unit)


def test_rows_may_be_a_list_and_the_shapes_carry_char_row_and_column() -> None:
    from_text = btvec.characters("a b\n.#", cell=8).shapes()
    from_list = btvec.characters(["a b", ".#"], cell=8).shapes()
    assert [shape.path.to_svg_path() for shape in from_text] == [shape.path.to_svg_path() for shape in from_list]
    found = [(s.attributes["char"], s.attributes["row"], s.attributes["column"]) for s in from_text]
    assert found == [("a", 0, 0), ("b", 0, 2), (".", 1, 0), ("#", 1, 1)]


def test_image_output_plugs_in_and_each_colour_lands_on_its_cell() -> None:
    art = ImageCharacters("#.\n @", [["#112233", "#445566"], ["#778899", "#aabbcc"]])
    shapes = btvec.characters(art.text, cell=(6, 12), colors=art.colors).shapes()
    assert [shape.style.fill for shape in shapes] == ["#112233ff", "#445566ff", "#aabbccff"]


def test_a_colour_grid_that_does_not_match_the_rows_is_a_parameter_error() -> None:
    with pytest.raises(btvec.ParameterError, match="row 0 has 1 colours"):
        btvec.characters("ab", cell=8, colors=[["#000000"]])
    with pytest.raises(btvec.ParameterError, match="color"):
        btvec.characters("a", cell=8, colors=[["red"]])


@pytest.mark.parametrize("rows", ["a\tb", "\x07", "a\r\nb"])
def test_a_control_character_in_the_rows_is_a_parameter_error(rows: str) -> None:
    with pytest.raises(btvec.ParameterError, match="control character"):
        btvec.characters(rows, cell=8)


@pytest.mark.parametrize("cell", [0, -1, float("nan"), (6, 0), (float("inf"), 6)])
def test_a_cell_that_is_not_positive_and_finite_is_a_parameter_error(cell: object) -> None:
    with pytest.raises(btvec.ParameterError, match="cell"):
        btvec.characters("a", cell=cell)  # type: ignore[arg-type]


def test_wrong_kinds_of_arguments_are_type_errors() -> None:
    with pytest.raises(TypeError, match="rows"):
        btvec.characters(5, cell=8)  # type: ignore[arg-type]
    with pytest.raises(TypeError, match="cell"):
        btvec.characters("a", cell="big")  # type: ignore[arg-type]


def test_characters_save_and_load_as_the_same_document() -> None:
    drawing = btvec.characters("ab\ncd", cell=(7, 9), size=8, colors=[["#000000"] * 2] * 2)
    text = drawing.to_json()
    assert btvec.Drawing.from_json(text).to_json() == text
    assert json.loads(text)["root"][0]["characters"]["cell"] == [7, 9]


def test_a_box_and_a_point_with_labels_make_the_documented_lines() -> None:
    shapes = btvec.marks([(40, 60, 120, 80, "H")], [(200, 90)], bracket=0.25, cross=10, label_size=12).shapes()
    roles = [(s.attributes["role"], s.attributes["mark"]) for s in shapes]
    assert roles == [("bracket", 0)] * 4 + [("leader", 0), ("label", 0), ("cross", 1), ("cross", 1)]
    assert shapes[0].path.to_svg_path() == "M60 60 L40 60 L40 80"
    assert shapes[4].path.to_svg_path() == "M160 60 L182 38"
    assert shapes[6].path.to_svg_path() == "M190 90 L210 90"
    assert shapes[7].path.to_svg_path() == "M200 80 L200 100"


def test_marks_accept_param_numbers_and_styling_is_by_role() -> None:
    x = btvec.Param("x", default=5, min=0, max=100)
    drawing = btvec.marks(points=[(x, 20, "A")]).only(role="label")
    assert drawing.shapes(x=30)[0].attributes["role"] == "label"
    assert drawing.shapes(x=30)[0].path.bounds.x > 30


def test_nothing_to_mark_is_an_empty_drawing() -> None:
    assert btvec.marks().shapes() == []


@pytest.mark.parametrize(
    ("call", "named"),
    [
        (lambda: btvec.marks([(0, 0, 0, 5)]), "width"),
        (lambda: btvec.marks([(0, 0, 5, 5)], bracket=0.6), "bracket"),
        (lambda: btvec.marks(points=[(float("nan"), 0)]), "x"),
        (lambda: btvec.marks(points=[(0, 0, "a\tb")]), "label"),
        (lambda: btvec.marks(points=[(0, 0)] * 10_001), "MAX_MARKS"),
    ],
)
def test_marks_the_rules_forbid_are_parameter_errors(call: object, named: str) -> None:
    with pytest.raises(btvec.ParameterError, match=named):
        call()  # type: ignore[operator]


@pytest.mark.parametrize("item", [(1, 2, 3), (1, 2, 3, 4, "a", "b"), "abcd", 5])
def test_a_box_that_is_not_four_numbers_and_a_label_is_a_type_error(item: object) -> None:
    with pytest.raises(TypeError, match="boxes"):
        btvec.marks([item])  # type: ignore[list-item]
