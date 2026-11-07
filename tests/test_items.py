"""`Drawing.split` and `Drawing.merge` through the public package: a shape becomes its parts, and shapes with an equal
attribute become one. Expected values are written by hand or read back from the path data drawn (a sub-path is an `M`;
bounds are the union of the members' bounds)."""

import json
from pathlib import Path
from typing import Any

import pytest

import bullterrier_vector as btvec

FONT = Path(__file__).resolve().parents[1] / "crates" / "core" / "tests" / "it" / "fonts" / "fraunces-subset.ttf"
TWO_SQUARES = "M0 0 L4 0 L4 4 Z M10 2 L16 2 L16 8 L10 8 Z"


def words() -> btvec.Drawing:
    """The glyphs 0 to 4 `H`, `i`, `T`, `e`, `l`, in the words "Hi" and "Tel"."""
    return btvec.text("Hi Tel", font=btvec.Font(FONT.read_bytes())).fill("#2a5bff")


def subpaths(shape: btvec.Shape) -> int:
    """How many sub-paths the outline of `shape` has."""
    return shape.path.to_svg_path().count("M")


def test_split_gives_the_stem_and_the_dot_of_an_i_as_two_parts_that_keep_what_the_glyph_had() -> None:
    whole = words().only(char="i").shapes()[0]
    parts = words().only(char="i").split().shapes()
    assert (subpaths(whole), len(parts)) == (2, 2)
    for number, part in enumerate(parts):
        assert part.attributes["part"] == number
        assert part.attributes["index"] == number
        assert subpaths(part) == 1
        assert part.style == whole.style
        for kept in ("glyph", "char", "word", "line", "order"):
            assert part.attributes[kept] == whole.attributes[kept], kept
        bounds = part.path.bounds
        assert part.attributes["origin"] == btvec.Point(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2)
    assert parts[0].attributes["origin"] != whole.attributes["origin"]


def test_split_keeps_an_outline_with_its_hole_as_one_part() -> None:
    (part,) = btvec.text("o", font=btvec.Font(FONT.read_bytes())).split().shapes()
    assert (subpaths(part), part.attributes["part"]) == (2, 0)


def test_split_of_a_path_adds_part_and_the_middle_of_each_part_as_origin() -> None:
    shapes = btvec.Drawing(btvec.Path.from_svg_path(TWO_SQUARES)).split().shapes()
    assert [(s.attributes["part"], s.attributes["origin"]) for s in shapes] == [
        (0, btvec.Point(2, 2)),
        (1, btvec.Point(13, 5)),
    ]


def test_split_keeps_a_shape_with_nothing_in_it_and_renumbers_the_list() -> None:
    shapes = btvec.Drawing(btvec.Path(), btvec.Path.rect(0, 0, 4, 4)).split().shapes()
    assert [(s.attributes["index"], s.attributes["part"], subpaths(s)) for s in shapes] == [(0, 0, 0), (1, 0, 1)]


@pytest.mark.parametrize(
    ("by", "groups"),
    [
        pytest.param(None, [[0, 1, 2, 3, 4]], id="all"),
        pytest.param("line", [[0, 1, 2, 3, 4]], id="one-line"),
        pytest.param("word", [[0, 1], [2, 3, 4]], id="words"),
        pytest.param("glyph", [[0], [1], [2], [3], [4]], id="glyphs-are-alone"),
        pytest.param("index", [[0], [1], [2], [3], [4]], id="indexes-are-alone"),
    ],
)
def test_merge_makes_a_shape_of_each_group_with_the_attributes_of_its_first_member(
    by: str | None, groups: list[list[int]]
) -> None:
    before = words().shapes()
    after = words().merge(by=by).shapes()
    assert [subpaths(s) for s in after] == [sum(subpaths(before[m]) for m in members) for members in groups]
    for shape, members in zip(after, groups, strict=True):
        first = before[members[0]]
        assert {k: v for k, v in shape.attributes.items() if k != "index"} == {
            k: v for k, v in first.attributes.items() if k != "index"
        }
        assert shape.style == first.style
    assert [s.attributes["index"] for s in after] == list(range(len(groups)))


def test_merge_covers_what_its_members_covered_and_sits_where_the_first_member_was() -> None:
    before = btvec.text("iHi", font=btvec.Font(FONT.read_bytes())).shapes()
    after = btvec.text("iHi", font=btvec.Font(FONT.read_bytes())).merge(by="char").shapes()
    assert [s.attributes["char"] for s in after] == ["i", "H"]
    left = min(before[0].path.bounds.x, before[2].path.bounds.x)
    right = max(
        before[0].path.bounds.x + before[0].path.bounds.width, before[2].path.bounds.x + before[2].path.bounds.width
    )
    assert after[0].path.bounds.x == pytest.approx(left)
    assert after[0].path.bounds.width == pytest.approx(right - left)


def test_merging_the_parts_of_each_glyph_gives_the_glyphs_back() -> None:
    whole = words().shapes()
    again = words().split().merge(by="glyph").shapes()
    assert [subpaths(s) for s in again] == [subpaths(s) for s in whole]
    assert [s.attributes["glyph"] for s in again] == [s.attributes["glyph"] for s in whole]


@pytest.mark.parametrize(
    ("make", "named"),
    [
        pytest.param(lambda: words().merge(by="nope"), "nope", id="not an attribute"),
        pytest.param(lambda: words().merge(by=""), "by", id="empty name"),
        pytest.param(lambda: words().merge(by="Glyph"), "Glyph", id="names are case sensitive"),
        pytest.param(lambda: words().merge(by=1), "by", id="not a name"),  # type: ignore[arg-type]
    ],
)
def test_merge_refuses_a_name_that_is_no_attribute_naming_the_argument(make: Any, named: str) -> None:
    with pytest.raises((btvec.ParameterError, TypeError), match=named):
        make()


@pytest.mark.parametrize("by", ["part", "copy", "level"])
def test_merge_refuses_an_attribute_no_shape_carries_when_drawn(by: str) -> None:
    drawing = words().merge(by=by)
    with pytest.raises(btvec.ParameterError, match=rf"merge\).by.*{by}"):
        drawing.shapes()


def test_merge_accepts_an_attribute_every_shape_carries_and_shapes_that_lack_it_are_one_group() -> None:
    # every glyph but the i is one part: the parts 0 and 1 are the two groups
    assert [s.attributes["part"] for s in words().split().merge(by="part").shapes()] == [0, 1]
    square = btvec.Path.rect(0, 0, 4, 4)
    mixed = btvec.Drawing(square, square.translate(9, 0), btvec.text("i", font=btvec.Font(FONT.read_bytes())).split())
    assert [(s.attributes.get("part"), subpaths(s)) for s in mixed.merge(by="part").shapes()] == [
        (None, 2),
        (0, 1),
        (1, 1),
    ]


@pytest.mark.parametrize(
    ("make", "written"),
    [
        pytest.param(lambda d: d.split(), {"split": {}}, id="split"),
        pytest.param(lambda d: d.merge(), {"merge": {}}, id="merge"),
        pytest.param(lambda d: d.split().merge(by="part"), {"merge": {"by": "part"}}, id="merge-by"),
    ],
)
def test_split_and_merge_are_saved_and_load_back_to_the_same_shapes(make: Any, written: Any) -> None:
    drawing = make(btvec.Drawing(btvec.Path.from_svg_path(TWO_SQUARES)))
    text = drawing.to_json()
    assert written in json.loads(text)["root"]
    again = btvec.Drawing.from_json(text)
    assert again.to_json() == text
    assert again.shapes() == drawing.shapes()


def test_a_document_that_merges_by_a_name_that_is_no_attribute_is_refused() -> None:
    text = btvec.Drawing(btvec.Path.from_svg_path(TWO_SQUARES)).split().merge(by="part").to_json()
    with pytest.raises(btvec.FormatError, match="nope"):
        btvec.Drawing.from_json(text.replace('"by":"part"', '"by":"nope"'))


# --- resolve on text ----------------------------------------------------------------------------------------------


def parts_after_resolve(content: str) -> list[btvec.Shape]:
    """The parts of one glyph once its overlapping contours are made one outline."""
    return btvec.text(content, font=btvec.Font(FONT.read_bytes())).resolve().split().shapes()


@pytest.mark.parametrize(
    ("content", "parts", "contours"),
    [
        # the crossbar and the two stems of the H overlap: one outline
        pytest.param("H", 1, 1, id="H-overlapping-contours-are-one-part"),
        pytest.param("i", 2, 1, id="i-stem-and-dot-stay-two-parts"),
        pytest.param("o", 1, 2, id="o-keeps-its-hole"),
    ],
)
def test_resolve_before_split_finds_the_true_parts_of_a_glyph(content: str, parts: int, contours: int) -> None:
    found = parts_after_resolve(content)
    assert len(found) == parts
    assert subpaths(found[0]) == contours


def test_split_alone_takes_the_overlapping_contours_of_an_h_for_several_parts_and_merge_does_not_unite() -> None:
    font = btvec.Font(FONT.read_bytes())
    assert len(btvec.text("H", font=font).split().shapes()) > 1
    assert subpaths(btvec.text("H", font=font).split().merge().shapes()[0]) == 3


def test_resolve_keeps_the_attributes_of_each_glyph_and_per_item_numbers_still_vary_them() -> None:
    before = words().shapes()
    after = words().resolve().shapes()
    assert [s.attributes for s in after] == [s.attributes for s in before]
    moved = words().resolve().translate(btvec.by_index(0, 40), 0).shapes()
    assert [m.path.bounds.x - a.path.bounds.x for m, a in zip(moved, after, strict=True)] == pytest.approx(
        [0, 10, 20, 30, 40]
    )
