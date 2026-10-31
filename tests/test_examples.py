"""The example programs in examples/ run as far as `shapes()` and draw what their comments say. Each runs with `runpy`
in an empty folder, which it must leave empty: writing the picture is V3. Expected values are worked out by hand:
the star's from the geometry of a five-pointed star, the bars' from the series printed in the program."""

import runpy
from pathlib import Path

import pytest

import bullterrier_vector as btvec

EXAMPLES = Path(__file__).resolve().parents[1] / "examples"


@pytest.fixture
def run(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    """Run an example in an empty folder; gives the shapes it drew."""
    monkeypatch.chdir(tmp_path)

    def run_example(name: str) -> list[btvec.Shape]:
        found = runpy.run_path(str(EXAMPLES / f"{name}.py"), run_name="__main__")
        assert list(tmp_path.iterdir()) == []
        shapes = found["shapes"]
        assert isinstance(shapes, list)
        return shapes

    return run_example


def test_star_draws_the_star_a_dot_on_its_top_tip_and_a_line_from_its_far_side(run):
    star, dot, line = run("star")
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


SERIES = [0.18, 0.35, 0.52, 0.41, 0.77, 0.93, 0.64, 0.49, 0.71, 0.28, 0.15, 0.33]


def test_series_bars_draws_twelve_bars_with_the_tallest_in_the_second_ink_on_top(run):
    bars = run("series_bars")
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
