"""`tools/bench.py` prints a table of the median time of `to_rgba` and `to_mesh` for the reference drawing, natively
and, when Node and the built WebAssembly package are there, in WebAssembly; it says in one line on stderr why a column
is missing (and exits 1 for that only when asked), and it exits 1 with one line for a request it cannot run. No test
here looks at a time: only at the shape of the table, the words, the exit code and what the reference drawing is.

Each case runs a copy of the tool in a scratch layout in a subprocess with an explicit environment, as a person runs it.
The WebAssembly package of the cases that need one is a stub of the package's public calls (the real one is built by
`poe wasm-test`): the subject here is the tool's use of Node, not the package.
"""

import json
import re
import runpy
import shutil
import subprocess
import sys
from collections.abc import Callable
from pathlib import Path
from typing import Any, NamedTuple

import pytest

ROOT = Path(__file__).resolve().parents[2]
TINY = ("--shapes", "5", "--size", "80x60", "--repeats", "2", "--warmups", "1")
STUB_PACKAGE = """\
class Mesh { free() {} }
class Drawing {
  static fromJson(text) {
    if (typeof text !== "string") throw new TypeError("text");
    return new Drawing();
  }
  toRgba(width, height) { return new Uint8Array(width * height * 4); }
  toMesh() { return new Mesh(); }
  toSvg() { return ""; }
}
module.exports = { Drawing, Mesh };
"""
ROW = re.compile(r"^\s*(to_rgba|to_mesh)((?:\s+\d+\.\d)+)(?:\s+\d+\.\d\dx)?$")


class Run(NamedTuple):
    code: int
    out: str
    err: str


@pytest.fixture
def layout(tmp_path: Path) -> Path:
    """A scratch copy of the tool and its Node half, laid out as in the repository (without `pkg/`)."""
    (tmp_path / "tools").mkdir()
    for name in ("bench.py", "bench.mjs"):
        shutil.copy(ROOT / "tools" / name, tmp_path / "tools" / name)
    (tmp_path / "examples" / "fonts").mkdir(parents=True)  # the font of --text
    shutil.copy(ROOT / "examples" / "fonts" / "anybody-subset.ttf", tmp_path / "examples" / "fonts")
    return tmp_path


@pytest.fixture
def stub_package(layout: Path) -> Path:
    """The layout with a stub of the WebAssembly package in `pkg/`."""
    (layout / "pkg").mkdir()
    (layout / "pkg" / "bullterrier_vector_wasm.js").write_text(STUB_PACKAGE, encoding="utf-8")
    return layout


@pytest.fixture
def bench(layout: Path, uv_run_env: dict[str, str]) -> Callable[..., Run]:
    """`bench(*args, env=None)`: run the copy of the tool with `args` in `env` (default: the `uv run` environment)."""

    def run(*args: str, env: dict[str, str] | None = None) -> Run:
        done = subprocess.run(
            [sys.executable, str(layout / "tools" / "bench.py"), *args],
            cwd=layout,
            env=env or uv_run_env,
            capture_output=True,
            encoding="utf-8",
            check=False,
        )
        return Run(done.returncode, done.stdout, done.stderr)

    return run


def rows(out: str) -> dict[str, list[str]]:
    """The numbers of each output row of a table, by output name."""
    found = {}
    for line in out.splitlines()[2:]:
        matched = ROW.match(line)
        assert matched, f"not a row of the table: {line!r}"
        found[matched[1]] = matched[2].split()
    return found


def test_the_native_table_has_a_row_for_each_output(bench):
    result = bench(*TINY)
    assert result.code == 0, result.err
    header, columns, *_ = result.out.splitlines()
    assert header == "reference drawing: a star and 5 shapes at 80x60, median of 2 calls in milliseconds"
    assert columns.split()[:2] == ["output", "native"]
    assert set(rows(result.out)) == {"to_rgba", "to_mesh"}


def test_text_adds_the_rows_of_a_headline_whose_weight_moves_and_of_a_text_of_a_weight_a_letter(bench):
    result = bench(*TINY, "--text")
    assert result.code == 0, result.err
    header, *timed = result.out.splitlines()[4:]
    assert header == "text: 'NO\\nSLEEP' in a variable font, median of 2 calls in milliseconds"
    assert [line.rsplit(None, 1)[0] for line in timed] == [
        "shapes, the weight moves",
        "to_rgba 1080x1350, the weight moves",
        "to_mesh 1080x1350, the weight moves",
        "shapes, a weight for each letter",
        "shapes, 998 letters, the weight moves",
        "shapes, the headline fitted to a width by wdth, the weight moves",
        "shapes, 998 letters wrapped to a width, the weight moves",
        "shapes, 100000 letters each at its own weight (once)",
    ]
    assert all(float(line.rsplit(None, 1)[1]) >= 0 for line in timed)  # a number; nothing asserts on time


def test_without_text_no_text_row_is_printed(bench):
    assert "text:" not in bench(*TINY).out


def test_booleans_adds_the_rows_of_a_line_of_type_and_two_grids_of_squares(bench):
    result = bench(*TINY, "--booleans")
    assert result.code == 0, result.err
    header, *timed = result.out.splitlines()[4:]
    assert header == "booleans: median of 2 calls in milliseconds"
    assert [line.rsplit(None, 1)[0] for line in timed] == [
        "shapes, a line of 35 letters, the weight moves",
        "shapes, the same line resolved",
        "shapes, the same line cut out of a plate with subtract",
        "Path.resolve, 100x100 overlapping squares",
        "Path.union, two 100x100 grids of overlapping squares",
    ]
    assert all(float(line.rsplit(None, 1)[1]) >= 0 for line in timed)  # a number; nothing asserts on time
    assert "booleans:" not in bench(*TINY).out


def test_halftone_adds_a_table_of_a_disc_of_dots_with_the_svg_row(bench):
    result = bench(*TINY, "--halftone")
    assert result.code == 0, result.err
    header, columns, *timed = result.out.splitlines()[4:]
    assert header == "halftone: a disc of about 3,000 dots at 800x800, median of 2 calls in milliseconds"
    assert columns.split()[:2] == ["output", "native"]
    assert [line.split()[0] for line in timed] == ["to_svg", "to_rgba", "to_mesh"]
    assert all(float(line.split()[1]) >= 0 for line in timed)  # a number; nothing asserts on time
    assert "halftone:" not in bench(*TINY).out


def test_a_missing_package_is_one_line_and_the_native_column_stays(bench):
    result = bench(*TINY)
    assert result.code == 0
    assert [len(row) for row in rows(result.out).values()] == [1, 1]
    assert result.err.splitlines() == [
        "bench: no WebAssembly column: the WebAssembly package is not built (run `uv run poe wasm-test`)"
    ]


def test_a_missing_package_fails_only_when_node_is_required(bench):
    result = bench(*TINY, "--require-node")
    assert result.code == 1
    assert "not built" in result.err
    assert len(result.err.splitlines()) == 1
    assert set(rows(result.out)) == {"to_rgba", "to_mesh"}, "the native table is still printed"


def test_a_missing_node_is_one_line(stub_package, bench, explicit_env):
    without_node = explicit_env(PATH=str(Path(sys.executable).parent))
    result = bench(*TINY, env=without_node)
    assert result.code == 0
    assert result.err.splitlines() == ["bench: no WebAssembly column: node not found on PATH"]
    assert bench(*TINY, "--require-node", env=without_node).code == 1


def test_with_node_and_the_package_the_table_has_a_webassembly_column(stub_package, bench):
    result = bench(*TINY, "--require-node")
    assert result.code == 0, result.err
    assert result.err == ""
    assert result.out.splitlines()[1].split() == ["output", "native", "WebAssembly", "ratio"]
    assert {name: len(numbers) for name, numbers in rows(result.out).items()} == {"to_rgba": 2, "to_mesh": 2}


def test_a_failing_node_run_is_reported_in_one_line(stub_package, bench):
    (stub_package / "pkg" / "bullterrier_vector_wasm.js").write_text("throw new Error('broken package');\n", "utf-8")
    result = bench(*TINY, "--require-node")
    assert result.code == 1
    assert len(result.err.splitlines()) == 1
    assert "node failed" in result.err
    assert "broken package" in result.err


@pytest.mark.parametrize(
    ("args", "named"),
    [
        pytest.param(("--size", "big"), "--size", id="a size that is not WxH"),
        pytest.param(("--repeats", "0"), "--repeats", id="no timed calls"),
        pytest.param(("--shapes", "-1"), "--shapes", id="a negative number of shapes"),
        pytest.param(("--size", "0x10"), "size", id="a size the library refuses"),
    ],
)
def test_a_request_it_cannot_run_is_one_line_and_exit_1(bench, args, named):
    result = bench("--warmups", "0", "--repeats", "1", "--shapes", "1", *args)
    assert result.code == 1
    assert result.out == ""
    assert len(result.err.splitlines()) == 1
    assert named in result.err


@pytest.fixture(scope="module")
def reference() -> Callable[..., Any]:
    """The tool's `reference_drawing`, as its module docstring promises it."""
    return runpy.run_path(str(ROOT / "tools" / "bench.py"))["reference_drawing"]


def test_the_reference_drawing_is_the_same_for_the_same_arguments(reference):
    assert reference(30).to_json() == reference(30).to_json()
    assert reference(30).to_json() != reference(30, 2).to_json()


def test_the_reference_drawing_is_a_star_and_the_shapes_asked_for_at_the_size_asked(reference):
    drawing = reference(200)
    assert len(drawing.shapes()) == 201
    assert drawing.size == (1920, 1080)
    assert reference(7, size=(100, 50)).size == (100, 50)


def test_the_reference_drawing_mixes_fills_strokes_translucency_and_multiply(reference):
    styles = [shape.style for shape in reference(200).shapes()]
    assert any(style.fill and not style.stroke for style in styles)
    assert any(style.stroke and not style.fill for style in styles)
    assert any(style.fill and style.stroke for style in styles)
    assert any(style.fill and style.fill[7:] and style.fill[7:] != "ff" for style in styles), "an alpha colour"
    assert any(style.opacity < 1 for style in styles)
    assert any(style.blend == "multiply" for style in styles)
    assert json.loads(reference(200).to_json())["version"] == 1


@pytest.fixture(scope="module")
def median_ms() -> Callable[..., float]:
    """The tool's `median_ms`, as its module docstring promises it."""
    return runpy.run_path(str(ROOT / "tools" / "bench.py"))["median_ms"]


def ticking(*durations: float) -> Callable[[], float]:
    """A clock that reads `durations` apart, one pair of readings per timed call."""
    readings = iter([reading for duration in durations for reading in (0.0, duration)])
    return lambda: next(readings)


@pytest.mark.parametrize(
    ("durations", "expected"),
    [
        pytest.param((0.005, 0.001, 0.003), 3.0, id="the middle of an odd number, not the smallest or the first"),
        pytest.param((0.001, 0.002, 0.003, 0.010), 2.5, id="the mean of the middle two of an even number"),
        pytest.param((0.004,), 4.0, id="a single call"),
    ],
)
def test_the_median_is_the_middle_time_in_milliseconds(median_ms, durations, expected):
    assert median_ms(lambda: None, 0, len(durations), ticking(*durations)) == pytest.approx(expected)


def test_warmups_are_called_but_not_timed(median_ms):
    calls = []
    # a clock with room for the 2 timed calls only: a timed warmup would run it dry
    assert median_ms(lambda: calls.append(1), 3, 2, ticking(0.001, 0.003)) == pytest.approx(2.0)
    assert len(calls) == 5
