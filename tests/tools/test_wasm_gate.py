"""The WebAssembly gate can fail: `poe wasm-test` builds the package, runs it in Node against what the native extension
answers, passes the workspace as it is, and fails, naming the test, for a defect in the glue (the sides of the output
swapped, the values of the Params dropped, a wrong name on an error), for a build without the SIMD feature set and for
an expected text that differs from the package's by one character.

An architecture test of the tooling: its subject is the `wasm-test` task in pyproject.toml, so it runs that task, the
way `uv run poe check` does, in a copy of the workspace into which one offence at a time is written. The copies share
one build folder, so the core is compiled once.
"""

import shutil
import subprocess
import sys
from collections.abc import Callable, Iterator
from dataclasses import dataclass
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
# one xdist worker runs this module, so the copy and its build folder are made once
pytestmark = [pytest.mark.gate, pytest.mark.xdist_group("wasm-gate")]
WORKSPACE_FILES = [
    "pyproject.toml",
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "rustfmt.toml",
    "clippy.toml",
    "deny.toml",
]
GLUE = "crates/wasm/src/lib.rs"
ORACLE = "tests/wasm/oracle.py"


@dataclass(frozen=True)
class Verdict:
    returncode: int
    report: str  # what the tools printed, both streams


@pytest.fixture(scope="module")
def workspace(tmp_path_factory) -> Path:
    root = tmp_path_factory.mktemp("wasm-workspace")
    for name in WORKSPACE_FILES:
        shutil.copy(ROOT / name, root)
    shutil.copytree(ROOT / "crates", root / "crates")
    shutil.copytree(ROOT / "tests" / "wasm", root / "tests" / "wasm", ignore=shutil.ignore_patterns("__pycache__"))
    # the oracle reads an example document of the format
    shutil.copytree(ROOT / "docs" / "examples" / "json", root / "docs" / "examples" / "json")
    return root


@pytest.fixture
def wasm_env(cargo_env, copy_env) -> dict[str, str]:
    """`cargo_env` (offline, one build folder), and `copy_env`'s `uv run` that uses this environment as it is."""
    return {**cargo_env, "UV_PROJECT_ENVIRONMENT": copy_env["UV_PROJECT_ENVIRONMENT"], "UV_NO_SYNC": "1"}


def wasm_test(workspace: Path, env: dict[str, str]) -> Verdict:
    command = [sys.executable, "-m", "poethepoet", "--executor", "simple", "wasm-test"]
    result = subprocess.run(command, cwd=workspace, env=env, capture_output=True, encoding="utf-8", check=False)
    return Verdict(result.returncode, result.stdout + result.stderr)


@pytest.fixture
def edit(workspace) -> Iterator[Callable[[str, str, str], None]]:
    """`edit(relative, old, new)` replaces `old` by `new` in one file of the copy for one test; afterwards the copy is
    as it was."""
    touched: dict[str, str] = {}

    def change(relative: str, old: str, new: str) -> None:
        path = workspace / relative
        text = path.read_text(encoding="utf-8")
        assert old in text
        touched.setdefault(relative, text)
        path.write_text(text.replace(old, new), encoding="utf-8")

    yield change
    for relative, text in touched.items():
        (workspace / relative).write_text(text, encoding="utf-8")


def test_the_workspace_as_it_is_passes(workspace, wasm_env):
    verdict = wasm_test(workspace, wasm_env)
    assert verdict.returncode == 0, verdict.report
    assert "ℹ fail 0" in verdict.report


# One offence each: (file, the text to replace, its replacement, what the report must name: the failing tests)
OFFENCES = {
    "the sides of the output swapped": (
        GLUE,
        "vector::Size::px(width, height)",
        "vector::Size::px(height, width)",
        ["✖ series_bars: toSvg writes the text the native extension writes"],
    ),
    "the values of the Params dropped": (
        GLUE,
        "&read_values(values)?",
        "&vector::Values::new()",
        ["✖ lifted: toSvg writes the text the native extension writes"],
    ),
    "the wrong name on an error": (
        GLUE,
        '"FormatError"',
        '"FormatFailure"',
        ["✖ fromJson refuses with the native FormatError"],
    ),
    "a build without the SIMD feature set": (
        "pyproject.toml",
        '{ RUSTFLAGS = "-C target-feature=+simd128" }',
        '{ RUSTFLAGS = "" }',
        ["✖ the module is built for WebAssembly SIMD"],
    ),
    "an expected text that differs by one character": (
        ORACLE,
        '"svg": svg}',
        '"svg": svg + " "}',
        ["✖ star: toSvg writes the text the native extension writes"],
    ),
    "an expected SVG of a corpus drawing that differs by one character": (
        ORACLE,
        'drawing.to_svg(width, height, **values).encode("utf-8")',
        '(drawing.to_svg(width, height, **values) + " ").encode("utf-8")',
        ["✖ star: the svg text is the native one", "star 400x400: svg"],
    ),
    "the PNG replaced by the pixels": (
        GLUE,
        "self.0.to_png(vector::Size::px(width, height)",
        "self.0.to_rgba(vector::Size::px(width, height)",
        ["✖ star: the png holds the pixels of toRgba", "star 400x400: png"],
    ),
    "the pixels of the native side reversed": (
        ORACLE,
        "write_bytes(drawing.to_rgba(width, height, **values))",
        "write_bytes(drawing.to_rgba(width, height, **values)[::-1])",
        ["✖ star: the pixels look the native ones", "star 400x400: pixels", "channel values differ"],
    ),
    "the indices pointer of the mesh pointing at the vertices": (
        GLUE,
        "self.0.indices().as_ptr().addr()",
        "self.0.vertices().as_ptr().addr()",
        ["✖ star: the mesh buffers are the native ones", "star 400x400: mesh"],
    ),
    "a mesh length counted in bytes": (
        GLUE,
        "self.0.vertices().len()\n",
        "self.0.vertices().len() * 4\n",
        ["✖ star: the mesh buffers are the native ones", "star 400x400: mesh"],
    ),
    "an expected mesh generation that differs by one": (
        ORACLE,
        '"generation": mesh.generation,',
        '"generation": mesh.generation + 1,',
        ["✖ star: the mesh buffers are the native ones", "star 400x400: mesh"],
    ),
}


@pytest.mark.parametrize(("relative", "old", "new", "named"), list(OFFENCES.values()), ids=list(OFFENCES))
def test_a_defect_in_the_glue_or_a_different_expectation_fails_naming_the_test(
    workspace, wasm_env, edit, relative, old, new, named
):
    edit(relative, old, new)
    verdict = wasm_test(workspace, wasm_env)
    assert verdict.returncode != 0, verdict.report
    for name in named:
        assert name in verdict.report
