"""`[tool.gate-proofs] triggers` lists what the gate proofs read, copy, edit or anchor on: `poe check` runs the proofs
when a changed path matches one. A trigger that matches nothing (a path gone, renamed or mistyped) would silently stop
guarding what the proofs read, so every listed trigger must match a file of the checkout."""

import fnmatch
import tomllib
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
WILDCARDS = "*?["


def unmatched(triggers: list[str], root: Path) -> list[str]:
    """The triggers that match no file under `root`, using the same glob rule as tools/pre_push.py (`*` crosses `/`)."""
    missing = []
    for trigger in triggers:
        folder = Path(trigger).parent if any(mark in Path(trigger).name for mark in WILDCARDS) else Path(trigger)
        base = root / folder
        candidates = [base] if base.is_file() else (path for path in base.rglob("*") if path.is_file())
        if not any(fnmatch.fnmatchcase(path.relative_to(root).as_posix(), trigger) for path in candidates):
            missing.append(trigger)
    return missing


def test_every_listed_trigger_matches_a_file():
    triggers = tomllib.loads((ROOT / "pyproject.toml").read_text(encoding="utf-8"))["tool"]["gate-proofs"]["triggers"]
    assert triggers, "no trigger at all"
    assert unmatched(triggers, ROOT) == []


@pytest.mark.parametrize(
    ("triggers", "missing"),
    [
        pytest.param(["tools/*", "pyproject.toml", "docs/*.md"], [], id="folder glob, file and nested glob match"),
        pytest.param(["pyproject.toml", "gone.toml"], ["gone.toml"], id="a file that does not exist"),
        pytest.param(["tools/*", "toolz/*"], ["toolz/*"], id="a glob over a folder that does not exist"),
        pytest.param(["docs/*.nothing"], ["docs/*.nothing"], id="a glob that matches no file in an existing folder"),
    ],
)
def test_the_check_names_a_trigger_that_matches_nothing(tmp_path, triggers, missing):
    (tmp_path / "tools").mkdir()
    (tmp_path / "tools" / "a.py").write_text("a = 1\n", encoding="utf-8")
    (tmp_path / "docs").mkdir()
    (tmp_path / "docs" / "page.md").write_text("page\n", encoding="utf-8")
    (tmp_path / "pyproject.toml").write_text("", encoding="utf-8")
    assert unmatched(triggers, tmp_path) == missing
