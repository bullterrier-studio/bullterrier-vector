"""tools/check_dist_contents.py: vector's sdist holds the Rust sources a build of the extension needs and the bundled
fonts' files, and sdists and wheels carry the licence files `[project] license-files` names and each font's `OFL.txt`.

The distributions are written here file by file (a wheel is a zip, an sdist a tarball), in a throwaway project whose
root is the tool's working directory. Each bad case changes one thing of the good set."""

import io
import subprocess
import sys
import tarfile
import zipfile
from collections.abc import Mapping
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
FONT = "crates/core/fonts/demo"
PYPROJECT = '[project]\nlicense-files = ["LICENSE"]\n'
CARGO = '[workspace]\nmembers = ["crates/core", "crates/python"]\n'
SDIST_FILES = {
    "Cargo.toml": CARGO,
    "Cargo.lock": "# lock\n",
    "rust-toolchain.toml": "[toolchain]\n",
    "crates/core/Cargo.toml": "[package]\n",
    "crates/python/Cargo.toml": "[package]\n",
    f"{FONT}/demo.woff2": "font\n",
    f"{FONT}/OFL.txt": "OFL\n",
    "LICENSE": "MIT\n",
}
WHEEL_FILES = {
    "demo/__init__.py": "",
    "demo-1.0.dist-info/licenses/LICENSE": "MIT\n",
    f"demo-1.0.dist-info/licenses/{FONT}/OFL.txt": "OFL\n",
}


def write_sdist(folder: Path, files: Mapping[str, str]) -> None:
    with tarfile.open(folder / "demo-1.0.tar.gz", "w:gz") as archive:
        for name, text in files.items():
            member = tarfile.TarInfo(f"demo-1.0/{name}")
            member.size = len(text.encode())
            archive.addfile(member, io.BytesIO(text.encode()))


def write_wheel(folder: Path, files: Mapping[str, str]) -> None:
    with zipfile.ZipFile(folder / "demo-1.0-cp312-abi3-manylinux_2_28_x86_64.whl", "w") as archive:
        for name, text in files.items():
            archive.writestr(name, text)


def run(tmp_path: Path, sdist: Mapping[str, str] | None, wheel: Mapping[str, str] | None):
    """The tool, run in a project whose checkout has the demo font, on a folder holding the given distributions."""
    (tmp_path / "pyproject.toml").write_text(PYPROJECT, encoding="utf-8")
    (tmp_path / "Cargo.toml").write_text(CARGO, encoding="utf-8")
    (tmp_path / FONT).mkdir(parents=True)
    (tmp_path / FONT / "demo.woff2").write_text("font\n", encoding="utf-8")
    (tmp_path / FONT / "OFL.txt").write_text("OFL\n", encoding="utf-8")
    folder = tmp_path / "dist"
    folder.mkdir()
    if sdist is not None:
        write_sdist(folder, sdist)
    if wheel is not None:
        write_wheel(folder, wheel)
    tool = ROOT / "tools" / "check_dist_contents.py"
    return subprocess.run(
        [sys.executable, str(tool), "dist"], cwd=tmp_path, capture_output=True, encoding="utf-8", check=False
    )


def without(files: Mapping[str, str], *names: str) -> dict[str, str]:
    return {name: text for name, text in files.items() if name not in names}


def test_complete_distributions_pass(tmp_path):
    done = run(tmp_path, SDIST_FILES, WHEEL_FILES)
    assert done.returncode == 0, done.stderr
    assert "every distribution holds its sources and licences" in done.stdout


@pytest.mark.parametrize(
    ("sdist", "wheel", "named"),
    [
        pytest.param(without(SDIST_FILES, "Cargo.lock"), WHEEL_FILES, "Cargo.lock", id="sdist without the lock"),
        pytest.param(
            without(SDIST_FILES, "crates/python/Cargo.toml"),
            WHEEL_FILES,
            "crates/python/Cargo.toml",
            id="sdist without a member's manifest",
        ),
        pytest.param(
            without(SDIST_FILES, f"{FONT}/demo.woff2"),
            WHEEL_FILES,
            f"{FONT}/demo.woff2",
            id="sdist without a font file",
        ),
        pytest.param(
            without(SDIST_FILES, "LICENSE"), WHEEL_FILES, "licence file LICENSE", id="sdist without the licence"
        ),
        pytest.param(
            without(SDIST_FILES, f"{FONT}/OFL.txt"),
            WHEEL_FILES,
            f"licence file {FONT}/OFL.txt",
            id="sdist without a font licence",
        ),
        pytest.param(
            SDIST_FILES,
            without(WHEEL_FILES, "demo-1.0.dist-info/licenses/LICENSE"),
            "licence file LICENSE",
            id="wheel without the licence",
        ),
        pytest.param(
            SDIST_FILES,
            without(WHEEL_FILES, f"demo-1.0.dist-info/licenses/{FONT}/OFL.txt"),
            f"licence file {FONT}/OFL.txt",
            id="wheel without a font licence",
        ),
    ],
)
def test_a_missing_source_or_licence_fails_naming_it(tmp_path, sdist, wheel, named):
    done = run(tmp_path, sdist, wheel)
    assert done.returncode == 1, done.stdout + done.stderr
    assert named in done.stderr
