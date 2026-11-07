"""Per-item numbers (`by_index`, `by_order`, `random`, `stagger`) through the public package: each shape of a list gets
its own value, the calls refuse what docs/api.md says they refuse, and a per-item number survives `to_json` and
`from_json`. Expected values are written by hand from the documented formulas; `random` is checked against the
published outputs of SplitMix64 (seed 0)."""

import json
from collections.abc import Sequence
from pathlib import Path
from typing import Any

import pytest

import bullterrier_vector as btvec

FONT = Path(__file__).resolve().parents[1] / "crates" / "core" / "tests" / "it" / "fonts" / "fraunces-subset.ttf"


def row(count: int) -> btvec.Drawing:
    """`count` squares stacked, each at x = 0; the stack writes their `order`: 0, 1/(count-1), ..., 1."""
    return btvec.Drawing(*[btvec.Path.rect(0, 0, 10, 10) for _ in range(count)])


def lefts(drawing: btvec.Drawing, **values: Any) -> list[float]:
    """The left edge of every shape `drawing` draws."""
    return [shape.path.bounds.x for shape in drawing.shapes(**values)]


def moved_by(count: int, number: Any, **values: Any) -> list[float]:
    """Where each of `count` squares lands when moved right by `number`."""
    return lefts(row(count).translate(number, 0), **values)


# --- by_index and by_order ----------------------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("count", "first", "last", "expected"),
    [
        pytest.param(5, 10, 50, [10, 20, 30, 40, 50], id="five-shapes"),
        pytest.param(3, 8, -8, [8, 0, -8], id="falling"),
        pytest.param(2, 1, 2, [1, 2], id="two-shapes-are-first-and-last"),
        pytest.param(4, 0, 3, [0, 1, 2, 3], id="four-shapes"),
        pytest.param(1, 3, 9, [3], id="one-shape-gives-first"),
    ],
)
@pytest.mark.parametrize("by", [btvec.by_index, btvec.by_order])
def test_a_linear_per_item_number_runs_from_first_on_the_first_shape_to_last_on_the_last(
    by: Any, count: int, first: float, last: float, expected: list[float]
) -> None:
    assert moved_by(count, by(first, last)) == expected


def test_by_index_counts_the_current_list_and_by_order_keeps_the_order_a_generator_wrote() -> None:
    # the first of three dropped: the two left are indexes 0 and 1, and keep orders 1/2 and 1
    kept = row(3).without(order=0)
    assert lefts(kept.translate(btvec.by_index(0, 10), 0)) == [0, 10]
    assert lefts(kept.translate(btvec.by_order(0, 10), 0)) == [5, 10]


def test_a_lone_path_has_order_zero() -> None:
    assert lefts(btvec.Drawing(btvec.Path.rect(0, 0, 10, 10)).translate(btvec.by_order(7, 9), 0)) == [7]


# --- stagger ------------------------------------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("spread", "progress", "expected"),
    [
        # five shapes at order 0, 1/4, 1/2, 3/4, 1 run while progress goes from order * spread for 1 - spread
        pytest.param(0.5, 0.0, [0, 0, 0, 0, 0], id="nothing-has-started"),
        pytest.param(0.5, 0.25, [0.5, 0.25, 0, 0, 0], id="the-first-is-half-way"),
        pytest.param(0.5, 0.5, [1, 0.75, 0.5, 0.25, 0], id="half-way"),
        pytest.param(0.5, 0.75, [1, 1, 1, 0.75, 0.5], id="the-last-is-half-way"),
        pytest.param(0.5, 1.0, [1, 1, 1, 1, 1], id="all-done"),
        pytest.param(0.0, 0.3, [0.3, 0.3, 0.3, 0.3, 0.3], id="spread-0-runs-all-together"),
        pytest.param(0.75, 0.5, [1, 1, 0.5, 0, 0], id="spread-0.75"),
        pytest.param(0.999, 0.0, [0, 0, 0, 0, 0], id="spread-0.999-at-the-start"),
        pytest.param(0.999, 0.999, [1, 1, 1, 1, 0], id="spread-0.999-the-last-has-not-started"),
        pytest.param(0.999, 1.0, [1, 1, 1, 1, 1], id="spread-0.999-at-the-end"),
        pytest.param(0.5, -1.0, [0, 0, 0, 0, 0], id="before-the-range-holds-0"),
        pytest.param(0.5, 2.0, [1, 1, 1, 1, 1], id="beyond-the-range-holds-1"),
    ],
)
def test_stagger_gives_each_shape_its_own_progress_as_the_documented_formula_says(
    spread: float, progress: float, expected: list[float]
) -> None:
    drawing = row(5).translate(btvec.stagger(btvec.Param("progress", default=0, min=-1, max=2), spread=spread), 0)
    found = lefts(drawing, progress=progress)
    assert found == pytest.approx(expected, abs=1e-9)


def test_stagger_spread_is_half_by_default_and_takes_an_expression() -> None:
    progress = btvec.Param("progress", default=0.5, min=0, max=1)
    spread = btvec.Param("spread", default=0.0, min=0, max=0.99)
    assert lefts(row(5).translate(btvec.stagger(progress), 0)) == pytest.approx([1, 0.75, 0.5, 0.25, 0], abs=1e-9)
    assert lefts(row(5).translate(btvec.stagger(progress * 2, spread=spread), 0)) == [1] * 5
    assert lefts(row(5).translate(btvec.stagger(progress, spread=spread), 0), spread=0.75) == pytest.approx(
        [1, 1, 0.5, 0, 0], abs=1e-9
    )


# --- random -------------------------------------------------------------------------------------------------------

# the first outputs of SplitMix64 seeded 0 (Vigna's reference implementation), whose top 53 bits make the numbers
SPLITMIX64_SEED_0 = (0xE220A8397B1DCDAF, 0x6E789E6AA1B965F4, 0x06C45D188009454F)


def test_random_is_the_published_generator_started_at_the_seed_and_read_at_the_shapes_index() -> None:
    expected = [(output >> 11) / 2**53 for output in SPLITMIX64_SEED_0]
    assert moved_by(3, btvec.random(0, 1)) == expected
    assert moved_by(3, btvec.random(0, 1, seed=0)) == expected


def test_random_scales_to_low_and_high() -> None:
    unit = moved_by(4, btvec.random(0, 1, seed=3))
    assert moved_by(4, btvec.random(-10, 30, seed=3)) == pytest.approx([-10 + 40 * value for value in unit], abs=1e-12)
    assert moved_by(3, btvec.random(5, 5)) == [5, 5, 5]


@pytest.mark.parametrize("seed", [0, 1, 7, 123_456_789, -1, -42, 2**53 - 1, 1 - 2**53])
def test_random_stays_within_low_and_high_and_repeats_for_the_same_seed(seed: int) -> None:
    found = moved_by(16, btvec.random(-3, 8, seed=seed))
    assert all(-3 <= value <= 8 for value in found)
    assert found == moved_by(16, btvec.random(-3, 8, seed=seed))
    other = seed + 1 if seed < 2**53 - 1 else seed - 1
    assert found != moved_by(16, btvec.random(-3, 8, seed=other))
    assert len(set(found)) == 16, "the value depends on the index"


def test_random_follows_the_shapes_current_index_not_the_shape() -> None:
    every = lefts(row(3).translate(btvec.random(0, 100, seed=9), 0))
    after_dropping_the_first = lefts(row(3).without(index=0).translate(btvec.random(0, 100, seed=9), 0))
    assert after_dropping_the_first == every[:2]
    assert after_dropping_the_first != every[1:]


def test_random_takes_every_operand_from_expressions_and_combines_with_the_operators() -> None:
    high = btvec.Param("high", default=10, min=0, max=100)
    seed = btvec.Param("seed", default=4, min=0, max=1000)
    drawing = row(5).translate(btvec.random(0, high, seed=seed) * 2, 0)
    assert lefts(drawing) == [2 * value for value in moved_by(5, btvec.random(0, 10, seed=4))]
    assert lefts(drawing, high=20, seed=5) == [2 * value for value in moved_by(5, btvec.random(0, 20, seed=5))]


# --- what a call refuses ------------------------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("number", "word"),
    [
        pytest.param(btvec.random(5, 1), "low", id="low-above-high"),
        pytest.param(btvec.random(0, 1, seed=1.5), "seed", id="seed-with-a-fraction"),
        pytest.param(btvec.random(0, 1, seed=2**53), "seed", id="seed-beyond-2^53-1"),
        pytest.param(btvec.stagger(0.5, spread=1), "spread", id="spread-1"),
        pytest.param(btvec.stagger(0.5, spread=-0.1), "spread", id="spread-below-0"),
        pytest.param(1 + btvec.by_index(0, btvec.random(2, 1)), "low", id="inside-a-sum"),
        pytest.param(-btvec.stagger(0.5, spread=2), "spread", id="under-a-negation"),
    ],
)
def test_a_step_refuses_the_operands_a_per_item_number_cannot_take_naming_the_argument(
    number: btvec.Expr, word: str
) -> None:
    with pytest.raises(btvec.ParameterError, match=rf"translate\)\.dx: .*{word}"):
        row(2).translate(number, 0)


@pytest.mark.parametrize(
    "number",
    [
        pytest.param(btvec.random(1, 1), id="low-equal-to-high"),
        pytest.param(btvec.random(0, 1, seed=2**53 - 1), id="seed-2^53-1"),
        pytest.param(btvec.random(0, 1, seed=1 - 2**53), id="seed-minus-2^53-plus-1"),
        pytest.param(btvec.stagger(0.5, spread=0), id="spread-0"),
        pytest.param(btvec.stagger(0.5, spread=0.999), id="spread-0.999"),
    ],
)
def test_a_step_accepts_the_operands_at_the_edge_of_what_is_accepted(number: btvec.Expr) -> None:
    assert len(row(2).translate(number, 0).shapes()) == 2


@pytest.mark.parametrize(
    ("make", "values", "word"),
    [
        pytest.param(lambda p: btvec.random(p, 2), {"p": 3}, "low", id="low-above-high"),
        pytest.param(lambda p: btvec.random(0, 2, seed=p), {"p": 2.5}, "seed", id="fractional-seed"),
        pytest.param(lambda p: btvec.stagger(0.5, spread=p), {"p": 1}, "spread", id="spread-1"),
        pytest.param(lambda p: btvec.stagger(0.5, spread=p), {"p": 1.5}, "spread", id="spread-1.5"),
    ],
)
def test_operands_known_only_when_drawn_are_refused_when_drawn(make: Any, values: dict[str, float], word: str) -> None:
    drawing = row(2).translate(make(btvec.Param("p", default=0.5, min=0, max=10)), 0)
    with pytest.raises(btvec.ParameterError, match=word):
        drawing.shapes(**values)


@pytest.mark.parametrize("argument", ["size", "tracking", "leading", "width"])
def test_an_argument_read_once_refuses_a_per_item_number_anywhere_in_it(argument: str) -> None:
    font = btvec.Font(FONT.read_bytes())
    options: dict[str, Any] = {"size": 100, argument: 1 + btvec.by_index(10, 20) * 2}
    with pytest.raises(btvec.ParameterError, match=rf"{argument}: .*per-item"):
        btvec.text("H", font=font, **options)


def test_font_axes_take_a_per_item_number_for_each_character() -> None:
    font = btvec.Font(FONT.read_bytes())

    def widths(weight: Any) -> list[float]:
        return [
            shape.path.bounds.width for shape in btvec.text("HH", font=font, size=2000, axes={"wght": weight}).shapes()
        ]

    light, bold, mixed = widths(300), widths(700), widths(btvec.by_index(300, 700))
    assert light[0] != pytest.approx(bold[0], abs=1)
    assert mixed == [light[0], bold[1]]


# --- printing and saving ------------------------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("number", "printed"),
    [
        pytest.param(btvec.by_index(0, 1), "by_index(first=0, last=1)", id="by_index"),
        pytest.param(btvec.by_order(2, 5), "by_order(first=2, last=5)", id="by_order"),
        pytest.param(btvec.random(-10, 10, seed=7), "random(low=-10, high=10, seed=7)", id="random"),
        pytest.param(
            btvec.stagger(btvec.Param("p", default=0, min=0, max=1)), "stagger(progress=p, spread=0.5)", id="stagger"
        ),
        pytest.param(btvec.random(-10, 10, seed=7) * 2, "random(low=-10, high=10, seed=7) * 2", id="combined"),
    ],
)
def test_a_per_item_number_prints_as_a_call_with_its_operands_named(number: btvec.Expr, printed: str) -> None:
    assert str(number) == printed


def test_per_item_numbers_equal_when_built_the_same_way() -> None:
    assert btvec.random(0, 1, seed=3) == btvec.random(0, 1, seed=3)
    assert btvec.random(0, 1, seed=3) != btvec.random(0, 1, seed=4)
    assert btvec.by_index(0, 1) != btvec.by_order(0, 1)
    assert isinstance(btvec.stagger(0.5), btvec.Expr)


@pytest.mark.parametrize(
    ("number", "written"),
    [
        pytest.param(btvec.by_index(0, 20), {"by_index": {"first": 0, "last": 20}}, id="by_index"),
        pytest.param(
            btvec.by_index(0, 0), {"by_index": {"first": 0, "last": 0}}, id="equal-to-the-default-is-still-written"
        ),
        pytest.param(btvec.by_order(5, 6), {"by_order": {"first": 5, "last": 6}}, id="by_order"),
        pytest.param(
            btvec.random(-10, 10, seed=7), {"random": {"low": -10, "high": 10, "seed": 7}}, id="random-with-its-seed"
        ),
        pytest.param(
            btvec.random(-10, 10), {"random": {"low": -10, "high": 10, "seed": 0}}, id="random-writes-the-default-seed"
        ),
        pytest.param(
            btvec.stagger(btvec.Param("p", default=0.6, min=0, max=1), spread=0.25),
            {"stagger": {"progress": {"param": "p"}, "spread": 0.25}},
            id="stagger",
        ),
    ],
)
def test_a_per_item_number_is_saved_whole_and_loads_back_to_the_same_shapes(number: btvec.Expr, written: Any) -> None:
    drawing = row(4).translate(number, 0)
    text = drawing.to_json()
    assert json.loads(text)["root"][1]["translate"]["dx"] == written
    loaded = btvec.Drawing.from_json(text)
    assert loaded.to_json() == text
    assert loaded.shapes() == drawing.shapes()


def test_a_document_leaves_out_a_seed_and_a_spread_for_their_defaults() -> None:
    def document(dx: Any) -> str:
        params = [{"name": "p", "default": 0.5, "min": 0, "max": 1, "unit": "", "meaning": ""}]
        root = [{"stack": {"items": [[{"path": "M0 0 L1 0"}]] * 4}}, {"translate": {"dx": dx, "dy": 0}}]
        return json.dumps({"version": 1, "params": params if "param" in json.dumps(dx) else None, "root": root})

    def moved(dx: Any) -> Sequence[float]:
        text = document(dx).replace('"params": null, ', "")
        return lefts(btvec.Drawing.from_json(text))

    assert moved({"random": {"low": 0, "high": 1}}) == moved({"random": {"low": 0, "high": 1, "seed": 0}})
    assert moved({"stagger": {"progress": {"param": "p"}}}) == pytest.approx([1, 2 / 3, 1 / 3, 0], abs=1e-9)


@pytest.mark.parametrize(
    ("dx", "word"),
    [
        pytest.param({"by_index": {"first": 0}}, "last", id="a-missing-field"),
        pytest.param({"by_index": {"first": 0, "last": 1, "step": 2}}, "not a field", id="an-unknown-field"),
        pytest.param({"by_order": [0, 1]}, "object", id="not-an-object"),
        pytest.param({"random": {"low": 3, "high": 1}}, "low", id="low-above-high"),
        pytest.param({"stagger": {"progress": 0.5, "spread": 1}}, "spread", id="spread-1"),
        pytest.param({"by_index": {"first": {"neg": 3}, "last": 1}}, "plain number", id="a-plain-operand-expression"),
    ],
)
def test_a_document_with_a_bad_per_item_number_is_refused(dx: Any, word: str) -> None:
    root = [{"stack": {"items": [[{"path": "M0 0 L1 0"}]] * 2}}, {"translate": {"dx": dx, "dy": 0}}]
    with pytest.raises(btvec.FormatError, match=word):
        btvec.Drawing.from_json(json.dumps({"version": 1, "root": root}))


def test_a_document_refuses_a_per_item_number_in_an_argument_read_once() -> None:
    text = json.dumps(
        {
            "version": 1,
            "inputs": {"a": {"kind": "font"}},
            "root": [{"text": {"content": "H", "font": "a", "size": {"by_order": {"first": 9, "last": 90}}}}],
        }
    )
    with pytest.raises(btvec.FormatError, match="per-item"):
        btvec.Drawing.from_json(text, inputs={"a": FONT.read_bytes()})
