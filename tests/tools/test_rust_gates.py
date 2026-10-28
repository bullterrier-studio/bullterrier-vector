"""The Rust gates can fail: each Rust task of `poe check` rejects a known-bad change, naming the rule, and passes the
workspace as it is.

An architecture test of the tooling: its subject is the configuration (the poe tasks in pyproject.toml, the lints in
Cargo.toml, rustfmt.toml, deny.toml), so it runs those tasks, the way `uv run poe check` does, in a copy of the Rust
workspace into which one offence at a time is written. The copies share one build folder, so the dependencies are
compiled once, and nothing needs the network.
"""

import os
import shutil
import subprocess
import sys
from collections.abc import Callable, Iterator
from dataclasses import dataclass
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
# one xdist worker runs this module, so its copy of the workspace and its build folder are made once
pytestmark = [pytest.mark.gate, pytest.mark.xdist_group("rust-gates")]
WORKSPACE_FILES = [
    "pyproject.toml",
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "rustfmt.toml",
    "clippy.toml",
    "deny.toml",
]
CORE = "crates/core/src/lib.rs"
CORE_MANIFEST = "crates/core/Cargo.toml"
CARGO_TASKS = ["rust-fmt", "rust-lint", "rust-doc", "rust-wasm", "rust-shear", "rust-deny", "rust-test"]


@dataclass(frozen=True)
class Verdict:
    returncode: int
    report: str  # what the tool printed, both streams


@pytest.fixture(scope="module")
def workspace(tmp_path_factory) -> Path:
    root = tmp_path_factory.mktemp("workspace")
    for name in WORKSPACE_FILES:
        shutil.copy(ROOT / name, root)
    shutil.copytree(ROOT / "crates", root / "crates")
    return root


@pytest.fixture(scope="module")
def cargo_env(tmp_path_factory) -> dict[str, str]:
    """`uv run`'s environment (PyO3 finds this Python), offline, uncoloured, with one build folder for every copy."""
    return {
        **os.environ,
        "PATH": os.pathsep.join([str(Path(sys.executable).parent), os.environ.get("PATH", "")]),
        "CARGO_TARGET_DIR": str(tmp_path_factory.mktemp("target")),
        "CARGO_NET_OFFLINE": "true",
        "CARGO_TERM_COLOR": "never",
        "NO_COLOR": "1",
    }


def poe(workspace: Path, task: str, env: dict[str, str]) -> Verdict:
    # the simple executor: the commands run in this environment, as under `uv run`, without making one for the copy
    command = [sys.executable, "-m", "poethepoet", "--executor", "simple", task]
    result = subprocess.run(command, cwd=workspace, env=env, capture_output=True, encoding="utf-8", check=False)
    return Verdict(result.returncode, result.stdout + result.stderr)


@pytest.fixture
def edit(workspace, cargo_env) -> Iterator[Callable[[str, Callable[[str], str]], None]]:
    """`edit(relative, change)` rewrites one file of the copy with `change(text)` for one test; afterwards the copy is
    as it was, Cargo.lock included. A manifest edit updates the lock offline, as `cargo add` would."""
    touched = {"Cargo.lock": (workspace / "Cargo.lock").read_text(encoding="utf-8")}

    def change(relative: str, rewrite: Callable[[str], str]) -> None:
        path = workspace / relative
        touched.setdefault(relative, path.read_text(encoding="utf-8"))
        path.write_text(rewrite(path.read_text(encoding="utf-8")), encoding="utf-8")
        if relative.endswith("Cargo.toml"):
            subprocess.run(
                ["cargo", "metadata", "--offline", "--format-version", "1"],
                cwd=workspace,
                env=cargo_env,
                capture_output=True,
                check=True,
            )

    yield change
    for relative, text in touched.items():
        (workspace / relative).write_text(text, encoding="utf-8")


def appended(source: str) -> Callable[[str], str]:
    return lambda text: text + source


def replaced(old: str, new: str) -> Callable[[str], str]:
    def rewrite(text: str) -> str:
        assert old in text
        return text.replace(old, new)

    return rewrite


@pytest.mark.parametrize("task", CARGO_TASKS)
def test_the_workspace_as_it_is_passes_each_rust_gate(workspace, cargo_env, task):
    verdict = poe(workspace, task, cargo_env)
    assert verdict.returncode == 0, verdict.report


# One offence each: (task, file, change, what the report must name: the rule, and where it was broken)
OFFENCES = {
    "unformatted code": ("rust-fmt", CORE, appended("pub  fn  spaced( )->u8{1}\n"), ["Diff in", CORE]),
    "an unwrap": (
        "rust-lint",
        CORE,
        appended("\n/// Probe.\n#[must_use]\npub fn probe(text: &str) -> u8 {\n    text.parse().unwrap()\n}\n"),
        ["unwrap_used", CORE],
    ),
    "an expect": (
        "rust-lint",
        CORE,
        appended(
            '\n/// Probe.\n#[must_use]\npub fn probe(text: &str) -> u8 {\n    text.parse().expect("a number")\n}\n'
        ),
        ["expect_used", CORE],
    ),
    "a panic": (
        "rust-lint",
        CORE,
        appended('\n/// Probe.\npub fn probe() {\n    panic!("no");\n}\n'),
        ["clippy::panic", CORE],
    ),
    "printing": (
        "rust-lint",
        CORE,
        appended('\n/// Probe.\npub fn probe() {\n    println!("hello");\n}\n'),
        ["print_stdout", CORE],
    ),
    "dead code": ("rust-lint", CORE, appended("\nfn never_called() {}\n"), ["dead-code", "never_called", CORE]),
    "a public function without documentation": (
        "rust-lint",
        CORE,
        appended("\npub fn undocumented() {}\n"),
        ["missing-docs", CORE],
    ),
    "pub where no user of the crate reaches it": (
        "rust-lint",
        "crates/core/src/polygon.rs",
        appended("\n/// Probe.\npub fn reachable_only_inside() {}\n"),
        ["unreachable-pub", "crates/core/src/polygon.rs"],
    ),
    "unsafe code in the core": (
        "rust-lint",
        CORE,
        appended(
            "\n/// Probe.\n#[must_use]\npub fn probe(value: &u8) -> u8 {\n    unsafe { *std::ptr::from_ref(value) }\n}"
        ),
        ["unsafe_code", CORE],
    ),
    "a pedantic lint": (
        "rust-lint",
        CORE,
        appended("\n/// Probe.\n#[must_use]\npub fn probe(value: u32) -> u8 {\n    value as u8\n}\n"),
        ["cast_possible_truncation", CORE],
    ),
    "a broken link in the documentation": (
        "rust-doc",
        CORE,
        appended("\n/// Like [`Nowhere`].\npub fn probe() {}\n"),
        ["broken-intra-doc-links", "Nowhere"],
    ),
    "platform trigonometry": (
        "rust-lint",
        CORE,
        appended("\n/// Probe.\n#[must_use]\npub fn probe(x: f64) -> f64 {\n    x.sin()\n}\n"),
        ["disallowed_methods", "crate::math", CORE],
    ),
    "reading a file in the core": (
        "rust-lint",
        CORE,
        appended('\n/// Probe.\npub fn probe() {\n    drop(std::fs::read("x"));\n}\n'),
        ["disallowed_methods", "callers pass bytes", CORE],
    ),
    "the clock": (
        "rust-lint",
        CORE,
        appended("\n/// Probe.\npub fn probe() {\n    let _ = std::time::Instant::now();\n}\n"),
        ["disallowed_methods", "no clock", CORE],
    ),
    "hyperbolic maths": (
        "rust-lint",
        CORE,
        appended("\n/// Probe.\n#[must_use]\npub fn probe(x: f64) -> f64 {\n    x.asinh()\n}\n"),
        ["disallowed_methods", "crate::math", CORE],
    ),
    "reading the environment": (
        "rust-lint",
        CORE,
        appended('\n/// Probe.\npub fn probe() {\n    drop(std::env::var_os("X"));\n}\n'),
        ["disallowed_methods", "callers pass bytes", CORE],
    ),
    "the clock through the Unix epoch": (
        "rust-lint",
        CORE,
        appended("\n/// Probe.\npub fn probe() {\n    let _ = std::time::UNIX_EPOCH.elapsed();\n}\n"),
        ["disallowed_methods", "no clock", CORE],
    ),
    "opening a file with OpenOptions": (
        "rust-lint",
        CORE,
        appended("\n/// Probe.\npub fn probe() {\n    let _ = std::fs::OpenOptions::new();\n}\n"),
        ["disallowed_types", "callers pass bytes", CORE],
    ),
    "a file handle in a signature": (
        "rust-lint",
        CORE,
        appended("\n/// Probe.\npub fn probe(_file: &std::fs::File) {}\n"),
        ["disallowed_types", "callers pass bytes", CORE],
    ),
    "a SystemTime in a signature": (
        "rust-lint",
        CORE,
        appended("\n/// Probe.\npub fn probe(_time: std::time::SystemTime) {}\n"),
        ["disallowed_types", "no clock", CORE],
    ),
    "a hash-ordered map": (
        "rust-lint",
        CORE,
        appended(
            "\n/// Probe.\n#[must_use]\npub fn probe() -> std::collections::HashMap<u8, u8> {\n"
            "    Default::default()\n}\n"
        ),
        ["disallowed_types", CORE],
    ),
    "code that does not build for WebAssembly": (
        "rust-wasm",
        CORE,
        appended(
            "\n/// Probe.\n#[must_use]\npub fn probe(text: &std::ffi::OsStr) -> &[u8] {\n"
            "    use std::os::unix::ffi::OsStrExt;\n    text.as_bytes()\n}\n"
        ),
        ["cannot find `unix` in `os`", CORE],
    ),
    "a dependency no code uses": (
        "rust-shear",
        CORE_MANIFEST,
        replaced("[dependencies]\n", '[dependencies]\nlibc = "0.2"\n'),
        ["unused dependency `libc`", CORE_MANIFEST],
    ),
    "PyO3 in the pure-Rust core": (
        "rust-deny",
        CORE_MANIFEST,
        replaced("[dependencies]\n", "[dependencies]\npyo3.workspace = true\n"),
        ["banned", "pyo3", "parent 'bullterrier-vector = "],  # the core, by name
    ),
    "a licence outside the allow-list": (
        "rust-deny",
        CORE_MANIFEST,
        replaced("license.workspace = true", 'license = "GPL-3.0-only"'),
        ["rejected", "GPL-3.0-only"],
    ),
    "a failing Rust test": (
        "rust-test",
        CORE,
        appended(
            "\n#[cfg(test)]\nmod probe {\n    #[test]\n    fn fails() {\n        assert_eq!(1 + 1, 3);\n    }\n}\n"
        ),
        ["test result: FAILED", "probe::fails"],
    ),
}


@pytest.mark.parametrize(("task", "relative", "change", "named"), OFFENCES.values(), ids=OFFENCES.keys())
def test_each_offence_fails_its_gate_naming_the_rule(workspace, cargo_env, edit, task, relative, change, named):
    edit(relative, change)
    verdict = poe(workspace, task, cargo_env)
    assert verdict.returncode != 0, verdict.report
    for name in named:
        assert name in verdict.report, verdict.report


# --- stubtest: the stub against the built extension ---------------------------------------------------------------


def stubtest(checkout: Path, env: dict[str, str]) -> Verdict:
    # the gate's own task; the stub is read from the copy (mypy_path = "src" in its pyproject.toml)
    command = [sys.executable, "-m", "poethepoet", "stubtest"]
    result = subprocess.run(command, cwd=checkout, env=env, capture_output=True, encoding="utf-8", check=False)
    return Verdict(result.returncode, result.stdout + result.stderr)


STUB = "src/bullterrier_vector/_native/_core.pyi"


def test_the_stub_as_it_is_matches_the_extension(project_copy, copy_env):
    verdict = stubtest(project_copy("pyproject.toml"), copy_env)
    assert verdict.returncode == 0, verdict.report


INIT = "src/bullterrier_vector/__init__.py"
FUNCTION = "def polygon_svg(sides: int, radius: float) -> str: ...\n"


@pytest.mark.parametrize(
    ("edits", "named"),
    [
        pytest.param(
            {STUB: [("sides: int", "count: int")]},
            ["bullterrier_vector._native._core.polygon_svg"],
            id="a parameter renamed",
        ),
        # the function is also taken out of the package's imports and `__all__`, so that nothing but the stub
        # disagrees and stubtest itself reaches its comparison
        pytest.param(
            {
                STUB: [(FUNCTION, ""), ('__all__ = ["polygon_svg"]', "__all__ = []")],
                INIT: [
                    (
                        "from bullterrier_vector._native._core import polygon_svg",
                        "from bullterrier_vector._native import _core",
                    ),
                    ('"ParameterError", "polygon_svg"]', '"ParameterError"]'),
                ],
            },
            ["bullterrier_vector._native._core.polygon_svg", "not present in stub"],
            id="a function left out",
        ),
    ],
)
def test_a_stub_that_disagrees_with_the_extension_fails_naming_the_function(project_copy, copy_env, edits, named):
    checkout = project_copy("pyproject.toml")
    for relative, changes in edits.items():
        text = (checkout / relative).read_text(encoding="utf-8")
        for old, new in changes:
            assert old in text
            text = text.replace(old, new)
        (checkout / relative).write_text(text, encoding="utf-8")
    verdict = stubtest(checkout, copy_env)
    assert verdict.returncode != 0, verdict.report
    for name in named:
        assert name in verdict.report, verdict.report
