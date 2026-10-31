"""Check that no tracked Python or Rust file outgrows the size budget.

An agent reads a file in chunks, and so does a person scrolling: a file under the budget stays navigable, one over it
should be split by topic. The budget is 1000 physical lines for source files (`.py` and `.rs`) and 1500 for test files
(anything under `tests/` or `crates/*/tests/`, and any file named `test_*.py`). Only files `git ls-files` lists are
counted, so build output and other untracked files never are; no tracked `.py` or `.rs` file is generated or vendored,
so there is no skip list.

A file that is already over budget is listed in pyproject.toml, mapping its repository-relative path to its line count:

    [tool.file-size.exempt]
    "src/bullterrier_vector/big.py" = 1234

A listed file may shrink but never grow. A listed file that is within budget again, or gone, fails too, so the table
cleans itself: delete the entry. Never raise a budget or an entry; split the file.
Run by hand with `python tools/check_file_size.py`; the test suite runs it too.

Interface:
    find_problems(root) -> what is wrong in the git repository at `root`, one message per file (empty when all is well)
    main(argv)          -> exit code: 0 clean, 1 problems found (listed on stderr)
"""

import argparse
import subprocess
import sys
import tomllib
from fnmatch import fnmatchcase
from pathlib import Path, PurePosixPath

SOURCE_BUDGET = 1000
TEST_BUDGET = 1500
CHECKED_SUFFIXES = (".py", ".rs")
EXEMPT_TABLE = "[tool.file-size.exempt]"


def find_problems(root: Path) -> list[str]:
    """Return one message for each tracked file of the repository at `root` that is over its budget (or over its listed
    size) and for each stale entry of `[tool.file-size.exempt]`; the messages name the file and say what to do."""
    root = Path(root)
    exempt = _exempt_sizes(root)
    sizes = {
        path: _line_count(root / path)
        for path in _tracked_files(root)
        if path.endswith(CHECKED_SUFFIXES) and (root / path).is_file()  # tracked but deleted: nothing to count
    }
    problems = []
    for path, size in sorted(sizes.items()):
        budget = _budget(path)
        if path in exempt:
            if size > exempt[path]:
                problems.append(
                    f"{path}: {size} lines, over its {exempt[path]} listed in {EXEMPT_TABLE}; a listed file may "
                    f"shrink but never grow: split it by topic (never raise the entry)"
                )
            elif size <= budget:
                problems.append(
                    f"{path}: {size} lines, within its budget of {budget}, so it is no longer exempt: delete the "
                    f"entry from {EXEMPT_TABLE}"
                )
        elif size > budget:
            problems.append(
                f"{path}: {size} lines, over the budget of {budget}: split it by topic (never raise the budget)"
            )
    problems += [
        f"{path}: listed in {EXEMPT_TABLE} but there is no such file: delete the entry"
        for path in sorted(exempt)
        if path not in sizes
    ]
    return problems


def _budget(path: str) -> int:
    return TEST_BUDGET if _is_test_file(PurePosixPath(path)) else SOURCE_BUDGET


def _is_test_file(path: PurePosixPath) -> bool:
    return path.parts[0] == "tests" or fnmatchcase(str(path), "crates/*/tests/*") or fnmatchcase(path.name, "test_*.py")


def _tracked_files(root: Path) -> list[str]:
    listing = subprocess.run(  # noqa: S603  fixed argv; `root` is a folder the caller named
        ["git", "-C", str(root), "ls-files", "-z"],  # noqa: S607  git is found on PATH
        capture_output=True,
        encoding="utf-8",
        check=True,
    )
    return [name for name in listing.stdout.split("\0") if name]


def _line_count(path: Path) -> int:
    """The physical lines of the file: its newlines, plus a last line that has none."""
    data = path.read_bytes()
    return data.count(b"\n") + (1 if data and not data.endswith(b"\n") else 0)


def _exempt_sizes(root: Path) -> dict[str, int]:
    pyproject = root / "pyproject.toml"
    if not pyproject.is_file():
        return {}
    table = tomllib.loads(pyproject.read_text(encoding="utf-8")).get("tool", {}).get("file-size", {})
    return {str(path): int(size) for path, size in table.get("exempt", {}).items()}


def main(argv: list[str] | None = None) -> int:
    """Check the repository at `argv[0]` (default: the repository this script lives in)."""
    parser = argparse.ArgumentParser(description="Check that no tracked Python or Rust file outgrows its size budget.")
    parser.add_argument("root", nargs="?", type=Path, default=Path(__file__).resolve().parent.parent)
    problems = find_problems(parser.parse_args(argv).root)
    if problems:
        listing = "\n".join(f"  {problem}" for problem in problems)
        print(
            f"file-size: files must stay under {SOURCE_BUDGET} lines (tests {TEST_BUDGET}):\n{listing}", file=sys.stderr
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
