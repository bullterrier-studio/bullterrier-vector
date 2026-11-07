"""`Drawing.slice` through the public package: each shape keeps the piece of its outline between two positions, and the
positions may differ per shape, so a text draws on letter by letter. Expected values are written by hand (a line of
known length, the documented stagger window)."""

import itertools
import json
from pathlib import Path
from typing import Any

import pytest

import bullterrier_vector as btvec

FONT = Path(__file__).resolve().parents[1] / "crates" / "core" / "tests" / "it" / "fonts" / "fraunces-subset.ttf"


def line(y: float = 0) -> btvec.Path:
    """A horizontal line 80 long from x = 10."""
    return btvec.Path().move_to((10, y)).line_to((90, y))


def word() -> btvec.Drawing:
    return btvec.text("HIFIT", font=btvec.Font(FONT.read_bytes()))


@pytest.mark.parametrize(
    ("start", "end", "left", "right"),
    [
        pytest.param(0, 0.5, 10, 50, id="first-half"),
        pytest.param(0.25, 0.75, 30, 70, id="middle-half"),
        pytest.param(0.5, 1, 50, 90, id="second-half"),
        pytest.param(0, 1, 10, 90, id="whole"),
        pytest.param(0.125, 0.125, 20, 20, id="a-point"),
    ],
)
def test_slice_keeps_the_piece_of_a_line_between_the_positions(
    start: float, end: float, left: float, right: float
) -> None:
    (shape,) = btvec.Drawing(line()).slice(start, end).shapes()
    assert shape.path.bounds.x == pytest.approx(left)
    assert shape.path.bounds.x + shape.path.bounds.width == pytest.approx(right)


def test_slice_of_the_whole_is_the_path_and_equals_the_path_method() -> None:
    (shape,) = btvec.Drawing(line()).slice(0, 1).shapes()
    assert shape.path.to_svg_path() == line().to_svg_path()
    square = btvec.Path.rect(0, 0, 10, 10)
    near = (10, 10)
    (from_corner,) = btvec.Drawing(square).slice(0.1, 0.6, near=near).shapes()
    assert from_corner.path == square.slice(0.1, 0.6, near=near)
    assert from_corner.path != square.slice(0.1, 0.6)


def test_slice_gives_each_shape_its_own_end_from_a_per_item_number() -> None:
    lines = btvec.Drawing(line(0), line(10), line(20))
    assert [s.path.bounds.width for s in lines.slice(0, btvec.by_index(0, 1)).shapes()] == pytest.approx([0, 40, 80])
    assert [s.path.bounds.width for s in lines.slice(btvec.by_index(0, 1), 1).shapes()] == pytest.approx([80, 40, 0])


def test_a_text_draws_on_letter_by_letter() -> None:
    progress = btvec.Param("progress", default=0, min=0, max=1)
    whole = word().shapes()
    draw_on = word().slice(0, btvec.stagger(progress, spread=0.6))
    for at in (0, 0.2, 0.5, 0.75, 1):
        shapes = draw_on.shapes(progress=at)
        # the letter at order p runs while the progress goes from p * 0.6 to p * 0.6 + 0.4
        wanted = [min(max((at - p * 0.6) / 0.4, 0), 1) for p in (0, 0.25, 0.5, 0.75, 1)]
        found = [s.path.length / w.path.length for s, w in zip(shapes, whole, strict=True)]
        assert found == pytest.approx(wanted, abs=1e-3), at
        assert all(before >= after - 1e-3 for before, after in itertools.pairwise(found)), found


def test_a_shape_with_nothing_in_it_stays() -> None:
    shapes = btvec.Drawing(btvec.Path(), line()).slice(0, 0.5).shapes()
    assert shapes[0].path.to_svg_path() == ""
    assert shapes[1].path.bounds.width == pytest.approx(40)


@pytest.mark.parametrize(
    ("start", "end", "named"),
    [
        pytest.param(1.1, 1, "start", id="start-above-1"),
        pytest.param(-0.1, 0.5, "start", id="start-below-0"),
        pytest.param(0, 1.1, "end", id="end-above-1"),
        pytest.param(0, -0.1, "end", id="end-below-0"),
        pytest.param(0.8, 0.2, "start", id="start-after-end"),
        pytest.param(float("nan"), 1, "start", id="start-nan"),
        pytest.param(0, float("nan"), "end", id="end-nan"),
    ],
)
def test_slice_refuses_what_it_cannot_cut_naming_the_argument(start: float, end: float, named: str) -> None:
    with pytest.raises(btvec.ParameterError, match=named):
        btvec.Drawing(line()).slice(start, end).shapes()


@pytest.mark.parametrize(("start", "end"), [(0, 1), (0, 0), (1, 1), (0.5, 0.5)])
def test_slice_accepts_the_ends_of_its_range(start: float, end: float) -> None:
    assert len(btvec.Drawing(line()).slice(start, end).shapes()) == 1


def test_slice_refuses_a_near_that_is_not_a_number() -> None:
    with pytest.raises(btvec.ParameterError, match="near"):
        btvec.Drawing(btvec.Path.rect(0, 0, 4, 4)).slice(0, 1, near=(float("inf"), 0)).shapes()


@pytest.mark.parametrize(
    ("make", "written"),
    [
        pytest.param(lambda d: d.slice(0.25, 0.75), {"slice": {"start": 0.25, "end": 0.75}}, id="numbers"),
        pytest.param(lambda d: d.slice(0, 0.5, near=(10, 10)), {"slice": {"end": 0.5, "near": [10, 10]}}, id="near"),
    ],
)
def test_slice_is_saved_and_loads_back_to_the_same_shapes(make: Any, written: Any) -> None:
    drawing = make(btvec.Drawing(btvec.Path.rect(0, 0, 10, 10)))
    text = drawing.to_json()
    (call,) = [c for c in json.loads(text)["root"] if "slice" in c]
    assert {k: v for k, v in call["slice"].items() if k in written["slice"]} == written["slice"]
    again = btvec.Drawing.from_json(text)
    assert again.to_json() == text
    assert again.shapes() == drawing.shapes()


def test_a_document_with_a_slice_past_its_range_is_refused() -> None:
    text = btvec.Drawing(line()).slice(0.25, 0.75).to_json()
    with pytest.raises(btvec.FormatError, match="end"):
        btvec.Drawing.from_json(text.replace("0.75", "1.5"))


def test_slice_draws_the_same_every_time() -> None:
    drawing = word().slice(0, btvec.stagger(0.5, spread=0.6))
    assert drawing.shapes() == drawing.shapes()
    assert drawing.to_svg(200, 100) == drawing.to_svg(200, 100)
