"""Check what vector's distributions must hold beyond the template's `check_dists.py` (`poe dist-contents`).

For the sdist and wheels in a folder (`dist/check`, built by `build-check`), run from the project's root:

- Each sdist holds what builds the extension from a clean unpack: the Rust workspace's `Cargo.toml`, `Cargo.lock` and
  `rust-toolchain.toml`, the manifest of every workspace member and the bundled fonts' files.
- Each sdist and each wheel holds the licence files `[project] license-files` names and the `OFL.txt` of every bundled
  font (`crates/core/fonts/*/OFL.txt`, found in the checkout, so dropping one from `license-files` fails too); a wheel
  holds them under `*.dist-info/licenses/`.

Exit code 0 when every distribution passes, 1 with one line per problem otherwise.
"""

import argparse
import sys
import tarfile
import tomllib
import zipfile
from pathlib import Path, PurePosixPath

# the bundled fonts: the file each is compiled from and the licence text it carries (docs/design-decisions.md, "Fonts")
FONT_FILES = "crates/core/fonts/*/*.woff2"
FONT_LICENCES = "crates/core/fonts/*/OFL.txt"


def licence_files(pyproject: dict) -> list[str]:
    """The licence files a distribution must hold: those `license-files` names (patterns expanded in the checkout) and
    the licence of every bundled font."""
    wanted = {
        found.as_posix()
        for pattern in pyproject["project"]["license-files"]
        for found in (Path().glob(pattern) if "*" in pattern else [Path(pattern)])  # a plain name is wanted as written
    }
    return sorted(wanted | {path.as_posix() for path in Path().glob(FONT_LICENCES)})


def missing_from_sdist(sdist: Path, licences: list[str]) -> list[str]:
    """Problems of one sdist: what a build of the Rust extension needs (the checkout's workspace says what) or a licence
    file it lacks."""
    workspace = tomllib.loads(Path("Cargo.toml").read_text(encoding="utf-8"))["workspace"]
    fonts = [path.as_posix() for path in Path().glob(FONT_FILES)]  # compiled into the extension
    needed = [
        "Cargo.toml",
        "Cargo.lock",
        "rust-toolchain.toml",
        *fonts,
        *(f"{member}/Cargo.toml" for member in workspace["members"]),
    ]
    with tarfile.open(sdist) as archive:
        shipped = {PurePosixPath(*PurePosixPath(name).parts[1:]).as_posix() for name in archive.getnames()}
    missing = [name for name in needed if name not in shipped]
    problems = [f"{sdist.name}: lacks {', '.join(missing)}, which a build of the extension needs"] if missing else []
    unlicensed = [name for name in licences if name not in shipped]
    return [*problems, f"{sdist.name}: lacks the licence file {', '.join(unlicensed)}"] if unlicensed else problems


def missing_from_wheel(wheel: Path, licences: list[str]) -> list[str]:
    """Problems of one wheel: the licence files it lacks under `*.dist-info/licenses/`."""
    with zipfile.ZipFile(wheel) as archive:
        shipped = set(archive.namelist())
    absent = [
        name
        for name in licences
        if not any(PurePosixPath(entry).match(f"*.dist-info/licenses/{name}") for entry in shipped)
    ]
    return [f"{wheel.name}: lacks the licence file {', '.join(absent)} under *.dist-info/licenses/"] if absent else []


def check(folder: Path) -> list[str]:
    """Every problem of the distributions in `folder` (empty when it holds none to check)."""
    licences = licence_files(tomllib.loads(Path("pyproject.toml").read_text(encoding="utf-8")))
    problems = []
    for sdist in sorted(folder.glob("*.tar.gz")):
        problems += missing_from_sdist(sdist, licences)
    for wheel in sorted(folder.glob("*.whl")):
        problems += missing_from_wheel(wheel, licences)
    return problems


def main(argv: list[str] | None = None) -> int:
    """Run the check on the folder named on the command line; the exit code."""
    parser = argparse.ArgumentParser(description="Check vector's sdist sources and licence files.")
    parser.add_argument("folder", type=Path, help="the folder of built distributions, e.g. dist/check")
    problems = check(parser.parse_args(argv).folder)
    for problem in problems:
        print(f"dist-contents: {problem}", file=sys.stderr)
    if not problems:
        print("dist-contents: every distribution holds its sources and licences")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
