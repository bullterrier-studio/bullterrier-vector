"""tools/check_dists.py: a folder of built distributions passes only when twine accepts every file and each wheel holds
the package's code (its stub and `py.typed` included), one build of its extension for its platform, and nothing else,
and every sdist holds the Rust sources a build of the extension needs.

The distributions are a tiny package's, written here file by file as a build backend would (no build: a wheel is a zip
with its metadata and RECORD), in a project whose pyproject.toml configures check-wheel-contents and maturin as ours
does. Each bad case changes one thing of the good set.
"""

import base64
import hashlib
import io
import subprocess
import sys
import tarfile
import zipfile
from collections.abc import Mapping
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
# one xdist worker: every case runs twine and check-wheel-contents
pytestmark = pytest.mark.xdist_group("check-dists")

PYPROJECT = """[tool.check-wheel-contents]
package = "src/demo"
package_omit = ["__pycache__", "*.pyc", "test_*.py"]

[tool.maturin]
module-name = "demo._native._core"
"""
CARGO = '[workspace]\nmembers = ["crates/core", "crates/python"]\n'
RUST = {  # what an sdist holds besides the package (and its metadata) so the extension can be built from it
    "Cargo.toml": CARGO,
    "Cargo.lock": "# lock\n",
    "rust-toolchain.toml": '[toolchain]\nchannel = "1.99.1"\n',
    "crates/core/Cargo.toml": "[package]\n",
    "crates/python/Cargo.toml": "[package]\n",
}
TREE = {  # the package as the checkout has it
    "demo/__init__.py": '"""Demo."""\n',
    "demo/shapes.py": '"""Shapes."""\n',
    "demo/py.typed": "",
    "demo/_native/__init__.py": '"""Native."""\n',
    "demo/_native/_core.pyi": "def apply() -> None: ...\n",
}
METADATA = "Metadata-Version: 2.1\nName: demo\nVersion: 1.0\nSummary: Demo.\nDescription-Content-Type: {kind}\n\n{text}"
GOOD_DESCRIPTION = ("text/markdown", "# Demo\n\nRenders.\n")


def wheel(folder: Path, platform: str, files: Mapping[str, bytes | str]) -> Path:
    """A wheel of `demo` for `platform` holding `files`, with its dist-info and a RECORD of their hashes."""
    info = "demo-1.0.dist-info"
    entries = {
        **files,
        f"{info}/METADATA": METADATA.format(kind=GOOD_DESCRIPTION[0], text=GOOD_DESCRIPTION[1]),
        f"{info}/WHEEL": f"Wheel-Version: 1.0\nGenerator: test\nRoot-Is-Purelib: false\nTag: cp312-abi3-{platform}\n",
    }
    record = []
    for name, content in entries.items():
        data = content.encode() if isinstance(content, str) else content
        digest = base64.urlsafe_b64encode(hashlib.sha256(data).digest()).rstrip(b"=").decode()
        record.append(f"{name},sha256={digest},{len(data)}")
    path = folder / f"demo-1.0-cp312-abi3-{platform}.whl"
    with zipfile.ZipFile(path, "w") as archive:
        for name, content in entries.items():
            archive.writestr(name, content)
        archive.writestr(f"{info}/RECORD", "\n".join([*record, f"{info}/RECORD,,"]) + "\n")
    return path


def sdist(folder: Path, description: tuple[str, str] = GOOD_DESCRIPTION, rust: Mapping[str, str] = RUST) -> Path:
    """An sdist of `demo` holding its metadata, pyproject.toml and the `rust` files."""
    path = folder / "demo-1.0.tar.gz"
    data = METADATA.format(kind=description[0], text=description[1])
    files = {"PKG-INFO": data, "pyproject.toml": PYPROJECT, **rust}
    with tarfile.open(path, "w:gz") as archive:
        for name, text in files.items():
            member = tarfile.TarInfo(f"demo-1.0/{name}")
            member.size = len(text.encode())
            archive.addfile(member, io.BytesIO(text.encode()))
    return path


@pytest.fixture
def project(tmp_path) -> Path:
    """The demo project, with an extension built in its checkout (as an editable install leaves one), and an empty
    `dist` folder."""
    (tmp_path / "pyproject.toml").write_text(PYPROJECT, encoding="utf-8")
    (tmp_path / "Cargo.toml").write_text(CARGO, encoding="utf-8")
    for name, text in TREE.items():
        (tmp_path / "src" / name).parent.mkdir(parents=True, exist_ok=True)
        (tmp_path / "src" / name).write_text(text, encoding="utf-8")
    (tmp_path / "src" / "demo" / "_native" / "_core.abi3.so").write_bytes(b"built here")
    (tmp_path / "dist").mkdir()
    return tmp_path


LINUX = ("manylinux_2_28_x86_64", "demo/_native/_core.abi3.so")
WINDOWS = ("win_amd64", "demo/_native/_core.pyd")


def good_wheels(dist: Path) -> None:
    """A Linux and a Windows wheel, each with the package and its own platform's build of the extension."""
    for platform, extension in (LINUX, WINDOWS):
        wheel(dist, platform, {**TREE, extension: f"built for {platform}".encode()})


def check_dists(project: Path) -> subprocess.CompletedProcess:
    command = [sys.executable, str(ROOT / "tools" / "check_dists.py"), "dist"]
    return subprocess.run(command, cwd=project, capture_output=True, encoding="utf-8", check=False)


def test_an_sdist_and_a_wheel_per_platform_each_with_its_own_extension_pass(project):
    sdist(project / "dist")
    good_wheels(project / "dist")
    done = check_dists(project)
    assert done.returncode == 0, done.stderr
    assert "every distribution passes" in done.stdout


def _no_extension(dist: Path) -> None:
    wheel(dist, LINUX[0], TREE)


def _two_extensions(dist: Path) -> None:
    wheel(dist, LINUX[0], {**TREE, LINUX[1]: b"one", "demo/_native/_core.cpython-312.so": b"two"})


def _a_stray_file(dist: Path) -> None:
    wheel(dist, LINUX[0], {**TREE, LINUX[1]: b"built", "demo/notes.txt": "left behind\n"})


def _a_test_file(dist: Path) -> None:
    wheel(dist, LINUX[0], {**TREE, LINUX[1]: b"built", "demo/test_shapes.py": "def test_x(): ...\n"})


def _code_left_out(dist: Path) -> None:
    files = {name: text for name, text in TREE.items() if name != "demo/shapes.py"}
    wheel(dist, LINUX[0], {**files, LINUX[1]: b"built"})


def _no_py_typed(dist: Path) -> None:
    files = {name: text for name, text in TREE.items() if name != "demo/py.typed"}
    wheel(dist, LINUX[0], {**files, LINUX[1]: b"built"})


def _no_stub(dist: Path) -> None:
    files = {name: text for name, text in TREE.items() if name != "demo/_native/_core.pyi"}
    wheel(dist, LINUX[0], {**files, LINUX[1]: b"built"})


def _sdist_without_the_rust_sources(dist: Path) -> None:
    (dist / "demo-1.0.tar.gz").unlink()
    sdist(dist, rust={})


def _sdist_without_a_workspace_member(dist: Path) -> None:
    (dist / "demo-1.0.tar.gz").unlink()
    sdist(dist, rust={name: text for name, text in RUST.items() if name != "crates/python/Cargo.toml"})


def _sdist_without_the_toolchain(dist: Path) -> None:
    (dist / "demo-1.0.tar.gz").unlink()
    sdist(dist, rust={name: text for name, text in RUST.items() if name != "rust-toolchain.toml"})


def _unrenderable_description(dist: Path) -> None:
    (dist / "demo-1.0.tar.gz").unlink()
    sdist(dist, ("text/x-rst", "A title\n=====\n\n.. not-a-directive::\n\n`unclosed\n"))


BAD = {  # fault -> what the report must name
    "a wheel without the extension": (_no_extension, ["holds 0 builds", "demo/_native/_core"]),
    "a wheel with two builds of it": (_two_extensions, ["holds 2 builds"]),
    "a file the package does not have (W102)": (_a_stray_file, ["W102", "demo/notes.txt"]),
    "a test file in the wheel (W102)": (_a_test_file, ["W102", "demo/test_shapes.py"]),
    "code the wheel left out (W101)": (_code_left_out, ["W101", "demo/shapes.py"]),
    "a wheel without py.typed (W101)": (_no_py_typed, ["W101", "demo/py.typed"]),
    "a wheel without the stub (W101)": (_no_stub, ["W101", "demo/_native/_core.pyi"]),
    "an sdist without crates/": (_sdist_without_the_rust_sources, ["demo-1.0.tar.gz", "crates/core/Cargo.toml"]),
    "an sdist without one workspace member": (_sdist_without_a_workspace_member, ["crates/python/Cargo.toml"]),
    "an sdist without the toolchain file": (_sdist_without_the_toolchain, ["rust-toolchain.toml"]),
    "a description PyPI cannot render (twine)": (_unrenderable_description, ["long_description"]),
}


@pytest.mark.parametrize(("fault", "named"), BAD.values(), ids=BAD.keys())
def test_each_fault_fails_the_check_naming_it(project, fault, named):
    sdist(project / "dist")
    good_wheels(project / "dist")
    fault(project / "dist")
    done = check_dists(project)
    assert done.returncode == 1
    for name in named:
        assert name in done.stderr, done.stderr


def test_an_empty_folder_fails(project):
    done = check_dists(project)
    assert done.returncode == 1
    assert "no sdist or wheel" in done.stderr
