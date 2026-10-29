"""The build gate can fail: `poe build-check` builds the sdist and the wheel (maturin, with the Rust extension) and
rejects a distribution PyPI would refuse to render (twine), a wheel with a file that cannot be imported or with anything
but the library's code (check-wheel-contents), a wheel without its build of the extension, and an sdist that cannot
rebuild it (tools/check_dists.py runs all of them), and passes the project as it is.

An architecture test of the tooling: its subject is the `build-check` task in pyproject.toml, so it runs that task, the
way `uv run poe check` does, in a copy of the project into which one fault at a time is written.
"""

import shutil
import subprocess
import sys
import tarfile
import tomllib
import zipfile
from dataclasses import dataclass
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
# one xdist worker runs this module, so the Rust build folder its builds share is reused across its tests
pytestmark = [pytest.mark.gate, pytest.mark.xdist_group("build-gate")]
RUST_FILES = ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml")
PACKAGE = "src/bullterrier_vector"


@dataclass(frozen=True)
class Verdict:
    returncode: int
    report: str  # what the tools printed, both streams


@pytest.fixture(scope="module")
def rust_build_folder(tmp_path_factory) -> Path:
    return tmp_path_factory.mktemp("target")


@pytest.fixture
def project(project_copy, rust_build_folder) -> Path:
    copy = project_copy("pyproject.toml", "README.md", "LICENSE", ".gitignore", *RUST_FILES)
    (copy / "tools").mkdir()
    shutil.copy(ROOT / "tools" / "check_dists.py", copy / "tools")
    shutil.copytree(ROOT / "crates", copy / "crates")
    return copy


@pytest.fixture
def build_check(project, copy_env, rust_build_folder):
    """Run `poe build-check` in the copy: its `uv run` uses this environment, and the copies share one build of the
    Rust dependencies."""

    def run() -> Verdict:
        env = {**copy_env, "CARGO_TARGET_DIR": str(rust_build_folder)}
        command = [sys.executable, "-m", "poethepoet", "build-check"]
        result = subprocess.run(command, cwd=project, env=env, capture_output=True, encoding="utf-8", check=False)
        return Verdict(result.returncode, result.stdout + result.stderr)

    return run


def test_the_project_as_it_is_builds_clean_distributions(project, build_check):
    verdict = build_check()
    assert verdict.returncode == 0, verdict.report
    cargo = tomllib.loads((project / "Cargo.toml").read_text(encoding="utf-8"))
    version = cargo["workspace"]["package"]["version"]  # pyproject.toml's version is dynamic: maturin reads this one
    [sdist] = (project / "dist" / "check").glob("*.tar.gz")
    [wheel] = (project / "dist" / "check").glob("*.whl")  # sdist and wheel, nothing else
    assert len(list((project / "dist" / "check").iterdir())) == 2
    assert sdist.name == f"bullterrier_vector-{version}.tar.gz"
    name, wheel_version, python, abi, _platform = wheel.name.removesuffix(".whl").split("-")
    assert (name, wheel_version) == ("bullterrier_vector", version)
    # one wheel per platform for every Python from 3.12 (the stable ABI), with the extension in the package
    assert (python, abi) == ("cp312", "abi3")
    shipped = zipfile.ZipFile(wheel).namelist()
    assert "bullterrier_vector/py.typed" in shipped  # the package is typed
    assert "bullterrier_vector/_native/_core.pyi" in shipped  # its types, for the package's own type checking
    extensions = [entry for entry in shipped if entry.startswith("bullterrier_vector/_native/_core.")]
    assert len([entry for entry in extensions if entry.endswith((".so", ".pyd"))]) == 1, extensions
    # built from the sdist with rustup, the extension is compiled by the toolchain CI tests, as the wheels are
    with tarfile.open(sdist) as archive:
        names = archive.getnames()
    for needed in ("rust-toolchain.toml", "Cargo.lock", "crates/core/Cargo.toml", "crates/python/Cargo.toml"):
        assert f"bullterrier_vector-{version}/{needed}" in names


def test_a_long_description_pypi_cannot_render_fails_the_gate(project, build_check):
    (project / "README.md").unlink()
    (project / "README.rst").write_text("A title\n=====\n\n.. not-a-directive::\n\n`unclosed\n", encoding="utf-8")
    pyproject = project / "pyproject.toml"
    pyproject.write_text(
        pyproject.read_text(encoding="utf-8").replace('readme = "README.md"', 'readme = "README.rst"'), encoding="utf-8"
    )
    verdict = build_check()
    assert verdict.returncode != 0
    assert "long_description" in verdict.report  # twine: "`long_description` has syntax errors"


def test_a_wheel_that_ships_a_test_or_a_module_that_cannot_be_imported_fails_the_gate_naming_each_file(
    project, build_check
):
    # one build for the two faults (a build takes seconds); each shipped file must be named
    package = project / PACKAGE
    (package / "test_stray.py").write_text("def test_x(): ...\n", encoding="utf-8")
    (package / "not-importable.py").write_text('"""A module with a dash."""\n', encoding="utf-8")
    verdict = build_check()
    assert verdict.returncode != 0
    assert "W102" in verdict.report  # check-wheel-contents: wheel library contains files not in the package tree
    assert "W004" in verdict.report  # check-wheel-contents: module is not located at importable path
    for shipped in ("test_stray.py", "not-importable.py"):
        assert shipped in verdict.report


def test_a_wheel_without_the_extension_where_the_package_has_it_fails_the_gate(project, build_check):
    pyproject = project / "pyproject.toml"
    text = pyproject.read_text(encoding="utf-8")
    assert 'module-name = "bullterrier_vector._native._core"' in text
    pyproject.write_text(
        text.replace("bullterrier_vector._native._core", "bullterrier_vector.native._core"), encoding="utf-8"
    )
    (project / PACKAGE / "native").mkdir()
    (project / PACKAGE / "native" / "__init__.py").write_text('"""Native."""\n', encoding="utf-8")
    verdict = build_check()
    assert verdict.returncode != 0
    assert "W101" in verdict.report  # check-wheel-contents: wheel library is missing files in the package tree
    assert "bullterrier_vector/_native/_core." in verdict.report


def test_an_sdist_without_the_toolchain_file_fails_the_gate(project, build_check):
    pyproject = project / "pyproject.toml"
    text = pyproject.read_text(encoding="utf-8")
    inclusion = 'include = [{ path = "rust-toolchain.toml", format = "sdist" }]'
    assert inclusion in text
    pyproject.write_text(text.replace(inclusion, ""), encoding="utf-8")
    verdict = build_check()
    assert verdict.returncode != 0
    # the sdist's own report, naming the one file it lacks (a list would name more)
    assert ".tar.gz: lacks rust-toolchain.toml, which a build of the extension needs" in verdict.report
