"""Whether the installed bullterrier-vector is the release artifact's, and works: the build workflow's smoke install
runs it with the fresh environment's own Python, so a pass proves the files about to be published work, not a release
from PyPI, not a build made on the fly and not the checkout.

    python check_installed.py ARTIFACT --from wheel|sdist

- Which file: `wheel`: the installed extension is, byte for byte, the one in exactly one wheel of ARTIFACT, which is
  named. `sdist`: the package was built from ARTIFACT's sdist: its direct_url.json (PEP 610) names that file.
- The installed version is the version of that file's name.
- `import bullterrier_vector` resolves to a file of the installation, not of the checkout.
- `polygon_svg(4, 10.0)` is the documented square, and `polygon_svg(2, 1.0)` raises `ParameterError`.

Exit code 0 and the file's name on stdout; 1 and the reason on stderr otherwise. Standard library only: the
environment under test holds the package and nothing else.

Interface:
    verify(installed, artifact, kind) -> the artifact's file; raises InstallError saying why not
    main(argv)                        -> exit code
"""

import argparse
import hashlib
import importlib
import json
import sys
import zipfile
from importlib.metadata import Distribution, distribution
from pathlib import Path, PurePosixPath
from urllib.parse import urlparse
from urllib.request import url2pathname

PACKAGE = "bullterrier_vector"
EXTENSION = PurePosixPath(PACKAGE, "_native")  # where the extension `_core.<platform suffix>` lives
SQUARE = (  # polygon_svg(4, 10.0): the square of the Python test suite (tests/test_polygon.py)
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="-10 -10 20 20"><polygon points="0,-10 10,0 0,10 -10,0"/></svg>'
)


class InstallError(Exception):
    """The installed package did not come from the artifact, or does not work."""


def _is_extension(name: str) -> bool:
    path = PurePosixPath(name)
    return path.parent == EXTENSION and path.name.startswith("_core.") and path.suffix != ".pyi"


def _installed_file(installed: Distribution, artifact: Path, kind: str) -> Path:
    """The file of `artifact` that `installed` came from (a wheel, or the sdist, as `kind` says)."""
    if kind == "sdist":
        direct_url = installed.read_text("direct_url.json")
        if direct_url is None:
            raise InstallError("no direct_url.json: it was not installed from a file given by its path")
        source = Path(url2pathname(urlparse(json.loads(direct_url)["url"]).path))
        [sdist] = artifact.glob("*.tar.gz")
        if not (source.is_file() and source.samefile(sdist)):
            raise InstallError(f"it was built from {source}, not the artifact's sdist {sdist}")
        return sdist
    built = [file for file in installed.files or [] if _is_extension(file.as_posix())]
    if len(built) != 1:
        raise InstallError(f"{len(built)} builds of the extension are recorded as installed, not one")
    digest = hashlib.sha256(Path(str(installed.locate_file(built[0]))).read_bytes()).digest()
    matches = []
    for wheel in sorted(artifact.glob("*.whl")):
        with zipfile.ZipFile(wheel) as archive:
            names = [name for name in archive.namelist() if _is_extension(name)]
            if any(hashlib.sha256(archive.read(name)).digest() == digest for name in names):
                matches.append(wheel)
    if len(matches) != 1:
        raise InstallError(f"its extension {built[0]} is in {len(matches)} wheels of the artifact, not one: no wheel")
    return matches[0]


def _version_of(file: Path) -> str:
    """The version in a wheel's or an sdist's file name (`name-VERSION-tags.whl`, `name-VERSION.tar.gz`)."""
    return file.name.removesuffix(".tar.gz").split("-")[1]


def verify(installed: Distribution, artifact: Path, kind: str) -> Path:
    """The file of `artifact` that `installed` came from, once the imported package has been checked to be the
    installation's and to draw the documented square; else InstallError saying why not."""
    file = _installed_file(installed, artifact, kind)
    if installed.version != _version_of(file):
        raise InstallError(f"it is version {installed.version}, {file.name} is version {_version_of(file)}")
    package = importlib.import_module(PACKAGE)
    site = Path(str(installed.locate_file(""))).resolve()
    if not Path(package.__file__).resolve().is_relative_to(site):
        raise InstallError(f"{PACKAGE} is imported from {package.__file__}, outside the installation in {site}")
    if package.polygon_svg(4, 10.0) != SQUARE:
        raise InstallError(f"polygon_svg(4, 10.0) is not the documented square: {package.polygon_svg(4, 10.0)}")
    try:
        package.polygon_svg(2, 1.0)
    except package.ParameterError:
        return file
    raise InstallError("polygon_svg(2, 1.0) did not raise ParameterError")


def main(argv: list[str] | None = None) -> int:
    """Name the artifact's file this environment's bullterrier-vector came from; 1 with the reason if none."""
    parser = argparse.ArgumentParser(description="Name the artifact's file the installed bullterrier-vector came from.")
    parser.add_argument("artifact", type=Path, help="the folder of the release's distributions")
    parser.add_argument("--from", dest="kind", choices=["wheel", "sdist"], required=True, help="how it was installed")
    arguments = parser.parse_args(argv)
    try:
        source = verify(distribution("bullterrier-vector"), arguments.artifact, arguments.kind)
    except InstallError as error:
        print(
            f"check-installed: bullterrier-vector is not installed from {arguments.artifact}: {error}", file=sys.stderr
        )
        return 1
    print(f"check-installed: bullterrier-vector is installed from {source.name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
