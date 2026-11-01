"""Check that no font crate is built with its `std` feature.

The font crates (`read-fonts`, `harfrust`) call the platform's `hypot` and other maths when their `std` feature is on,
and the platform's maths differs between targets; with `libm` and no `std` they use the same code everywhere, which is
what makes a glyph outline the same bytes natively and in WebAssembly (docs/design-decisions.md, "Fonts"). Cargo
unifies features across the whole workspace, so one crate asking for `std` (or for default features) turns it on for
all: `cargo tree -e features` shows what is on, and this fails when `std` is, naming the crate.

Run by hand with `python tools/check_font_features.py`; `poe rust-fonts` runs it, in `poe check`.

Interface:
    guarded(root)            -> (crate, version) of each guarded crate `root`/Cargo.toml pins
    std_enabled(crate, tree) -> whether `cargo tree -e features` output `tree` has `crate` with `std` on
    main(argv)               -> exit code: 0 no guarded crate has `std`, 1 one has (listed on stderr)
"""

import argparse
import re
import subprocess
import sys
import tomllib
from pathlib import Path

GUARDED = ("read-fonts", "harfrust", "skrifa")
ROOT = Path(__file__).resolve().parent.parent


def guarded(root: Path) -> list[tuple[str, str]]:
    """Each guarded crate pinned in `[workspace.dependencies]` of `root`/Cargo.toml, with its exact version."""
    manifest = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
    dependencies = manifest["workspace"]["dependencies"]
    found = []
    for name in GUARDED:
        if name in dependencies:
            spec = dependencies[name]
            version = spec if isinstance(spec, str) else spec["version"]
            found.append((name, version.lstrip("=")))
    return found


def std_enabled(crate: str, tree: str) -> bool:
    """Whether `tree` (the inverted feature tree of `crate`) shows the `std` feature of `crate` switched on."""
    return re.search(rf'^{re.escape(crate)} feature "std"', tree, re.MULTILINE) is not None


def main(argv: list[str] | None = None) -> int:
    """Check the workspace at `argv[0]` (default: the repository this script lives in)."""
    parser = argparse.ArgumentParser(description="Check that no font crate is built with its std feature.")
    parser.add_argument("root", nargs="?", type=Path, default=ROOT)
    root = parser.parse_args(argv).root
    problems = []
    for crate, version in guarded(root):
        command = ["cargo", "tree", "--workspace", "--locked", "-e", "features", "-i", f"{crate}@{version}"]
        tree = subprocess.run(  # noqa: S603  fixed argv; `crate` and `version` come from the workspace manifest
            [*command, "--prefix", "none"],
            cwd=root,
            capture_output=True,
            encoding="utf-8",
            check=True,
        ).stdout
        if std_enabled(crate, tree):
            problems.append(
                f"{crate} {version} is built with its `std` feature: "
                'use `default-features = false, features = ["libm"]`'
            )
    if problems:
        listing = "\n".join(f"  {problem}" for problem in problems)
        print(
            f"rust-fonts: the font crates must not use std (their maths would be the platform's):\n{listing}",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
