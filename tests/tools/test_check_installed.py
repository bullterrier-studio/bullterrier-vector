"""tools/check_installed.py: the smoke check of the build workflow names the file of the release artifact that the
installed package came from, and fails when it came from anywhere else, when it is another version, or when it does
not draw the documented square or reject two sides.

An installation is a `site` folder holding the package and its dist-info, as an installer leaves it; the artifact is a
folder of wheels and an sdist. Both are written here around a tiny stand-in package (no build), and the tool runs as
the workflow runs it, in a process of its own, with the site first on its import path.
"""

import json
import os
import subprocess
import sys
import zipfile
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
SQUARE = (
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="-10 -10 20 20"><polygon points="0,-10 10,0 0,10 -10,0"/></svg>'
)
EXTENSION = "bullterrier_vector/_native/_core.abi3.so"
LINUX_WHEEL = "bullterrier_vector-1.0-cp312-abi3-manylinux_2_28_x86_64.whl"
SDIST = "bullterrier_vector-1.0.tar.gz"


def package_source(square: str = SQUARE, rejects_two_sides: bool = True) -> str:
    """A stand-in `bullterrier_vector`: `polygon_svg` returns `square`, and raises `ParameterError` below 3 sides."""
    return f"""
class ParameterError(ValueError):
    pass


def polygon_svg(sides, radius):
    if sides < 3 and {rejects_two_sides}:
        raise ParameterError("sides: got " + str(sides))
    return {square!r}
"""


@pytest.fixture
def artifact(tmp_path) -> Path:
    """The artifact: a Linux and a Windows wheel, each with its own build of the extension, and the sdist."""
    folder = tmp_path / "dist"
    folder.mkdir()
    for name, extension, content in [
        (LINUX_WHEEL, EXTENSION, b"built for linux"),
        (
            "bullterrier_vector-1.0-cp312-abi3-win_amd64.whl",
            "bullterrier_vector/_native/_core.pyd",
            b"built for windows",
        ),
    ]:
        with zipfile.ZipFile(folder / name, "w") as wheel:
            wheel.writestr("bullterrier_vector/__init__.py", "")
            wheel.writestr(extension, content)
    (folder / SDIST).write_bytes(b"the sdist")
    return folder


def install(
    tmp_path: Path,
    *,
    extension: bytes = b"built for linux",
    version: str = "1.0",
    direct_url: Path | None = None,
    source: str | None = None,
) -> Path:
    """Write bullterrier-vector as an installer leaves it in `tmp_path/site`: the package (`source`, a stand-in by
    default), its extension, and the dist-info listing the extension; with `direct_url`, installed from that file
    (PEP 610's direct_url.json). Returns the site folder."""
    site = tmp_path / "site"
    (site / EXTENSION).parent.mkdir(parents=True)
    (site / "bullterrier_vector" / "__init__.py").write_text(source or package_source(), encoding="utf-8")
    (site / EXTENSION).write_bytes(extension)
    info = site / f"bullterrier_vector-{version}.dist-info"
    info.mkdir()
    info.joinpath("METADATA").write_text(
        f"Metadata-Version: 2.4\nName: bullterrier-vector\nVersion: {version}\n", encoding="utf-8"
    )
    info.joinpath("RECORD").write_text(f"{EXTENSION},,\n{info.name}/RECORD,,\n", encoding="utf-8")
    if direct_url is not None:
        url = {"url": direct_url.resolve().as_uri(), "archive_info": {}}
        info.joinpath("direct_url.json").write_text(json.dumps(url), encoding="utf-8")
    return site


def check_installed(artifact: Path, kind: str, *import_path: Path) -> subprocess.CompletedProcess:
    """Run the tool in a process of its own, with `import_path` first on its import path (as an installation is)."""
    # -S: without this environment's own site folder, whose editable install of the checkout would make a regular
    # package of what a test makes a namespace package; SYSTEMROOT is what Windows needs to start Python at all
    command = [sys.executable, "-S", str(ROOT / "tools" / "check_installed.py"), str(artifact), "--from", kind]
    env = {"PATH": "", "PYTHONPATH": os.pathsep.join(map(str, import_path))}
    if "SYSTEMROOT" in os.environ:
        env["SYSTEMROOT"] = os.environ["SYSTEMROOT"]
    return subprocess.run(command, env=env, capture_output=True, encoding="utf-8", check=False)


def test_a_wheel_install_names_the_artifacts_wheel_whose_extension_it_has(tmp_path, artifact):
    site = install(tmp_path)
    done = check_installed(artifact, "wheel", site)
    assert done.returncode == 0, done.stderr
    assert LINUX_WHEEL in done.stdout


def test_an_sdist_install_names_the_artifacts_sdist(tmp_path, artifact):
    site = install(tmp_path, extension=b"built here", direct_url=artifact / SDIST)
    done = check_installed(artifact, "sdist", site)
    assert done.returncode == 0, done.stderr
    assert SDIST in done.stdout


def _elsewhere(tmp_path: Path) -> Path:
    other = tmp_path / "elsewhere" / SDIST
    other.parent.mkdir()
    other.write_bytes(b"the sdist")
    return other


# (kind, how the install differs from a good one) -> what the refusal names
NOT_FROM_THE_ARTIFACT = {
    "a wheel whose extension no wheel of the artifact has (from PyPI, or built here)": (
        "wheel",
        {"extension": b"built somewhere else"},
        "no wheel",
    ),
    "an sdist install from no file (an index)": ("sdist", {}, "direct_url.json"),
    "an sdist install from a file outside the artifact": (
        "sdist",
        {"direct_url": _elsewhere},
        "not the artifact's sdist",
    ),
    "a version other than the artifact's (wheel)": ("wheel", {"version": "2.0"}, "version 2.0"),
    "a version other than the artifact's (sdist)": (
        "sdist",
        {"version": "2.0", "direct_url": lambda tmp_path: tmp_path / "dist" / SDIST},
        "version 2.0",
    ),
}


@pytest.mark.parametrize(
    ("kind", "install_as", "named"), NOT_FROM_THE_ARTIFACT.values(), ids=NOT_FROM_THE_ARTIFACT.keys()
)
def test_an_install_not_from_the_artifact_is_refused_saying_why(tmp_path, artifact, kind, install_as, named):
    options: dict = {key: (value(tmp_path) if callable(value) else value) for key, value in install_as.items()}
    done = check_installed(artifact, kind, install(tmp_path, **options))
    assert done.returncode == 1
    assert named in done.stderr, done.stderr


def test_a_package_with_no_file_is_refused_saying_so(tmp_path, artifact):
    """A folder without `__init__.py` imports as a namespace package, whose `__file__` is None."""
    site = install(tmp_path)
    (site / "bullterrier_vector" / "__init__.py").unlink()
    done = check_installed(artifact, "wheel", site)
    assert done.returncode == 1
    assert "has no __file__" in done.stderr
    assert "Traceback" not in done.stderr


@pytest.mark.parametrize(
    ("sdists", "named"),
    [
        pytest.param([], "holds 0 sdists", id="no sdist"),
        pytest.param(["bullterrier_vector-1.1.tar.gz"], "holds 2 sdists", id="two sdists"),
    ],
)
def test_an_artifact_without_exactly_one_sdist_is_refused_saying_so(tmp_path, artifact, sdists, named):
    site = install(tmp_path, extension=b"built here", direct_url=artifact / SDIST)
    if not sdists:
        (artifact / SDIST).unlink()
    for name in sdists:
        (artifact / name).write_bytes(b"another sdist")
    done = check_installed(artifact, "sdist", site)
    assert done.returncode == 1
    assert named in done.stderr
    assert "Traceback" not in done.stderr


def test_a_package_imported_from_the_checkout_instead_of_the_installation_is_refused(tmp_path, artifact):
    site = install(tmp_path)
    checkout = tmp_path / "checkout"
    (checkout / "bullterrier_vector").mkdir(parents=True)
    (checkout / "bullterrier_vector" / "__init__.py").write_text(package_source(), encoding="utf-8")
    done = check_installed(artifact, "wheel", checkout, site)  # the checkout comes first, as the working directory does
    assert done.returncode == 1
    assert "imported from" in done.stderr
    assert str(checkout) in done.stderr


WRONG_SQUARE = SQUARE.replace("20 20", "20 21")
DOES_NOT_SMOKE = {  # the package's source -> what the refusal names
    "polygon_svg(4, 10.0) returning another document": (package_source(square=WRONG_SQUARE), "polygon_svg(4, 10.0)"),
    "polygon_svg(2, 1.0) not raising ParameterError": (package_source(rejects_two_sides=False), "ParameterError"),
}


@pytest.mark.parametrize(("source", "named"), DOES_NOT_SMOKE.values(), ids=DOES_NOT_SMOKE.keys())
def test_a_package_that_does_not_draw_the_documented_square_or_reject_two_sides_is_refused(
    tmp_path, artifact, source, named
):
    done = check_installed(artifact, "wheel", install(tmp_path, source=source))
    assert done.returncode == 1
    assert named in done.stderr, done.stderr


def test_the_command_fails_visibly_for_this_environments_editable_install(artifact):
    # the suite runs on an editable install of the checkout: from no file of any artifact
    done = subprocess.run(
        [sys.executable, str(ROOT / "tools" / "check_installed.py"), str(artifact), "--from", "wheel"],
        capture_output=True,
        encoding="utf-8",
        check=False,
    )
    assert done.returncode == 1
    assert "not installed from" in done.stderr
