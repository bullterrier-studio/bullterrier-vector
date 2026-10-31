"""The file-size gate can fail: `tools/check_file_size.py` rejects a source file over 1000 lines and a test file over
1500, holds a listed over-budget file at its listed size, and rejects a listed entry that has gone stale.

An architecture test of the tooling: each case builds a throwaway git repository (`git_repo`) holding the files and the
`[tool.file-size.exempt]` table of its pyproject.toml, and runs the tool's `main` on it. Cheap, but marked `gate` like
the proofs of the other gates.
"""

import importlib.util
from pathlib import Path

import pytest

pytestmark = pytest.mark.gate

ROOT = Path(__file__).resolve().parents[2]


def _load_checker():
    spec = importlib.util.spec_from_file_location("check_file_size", ROOT / "tools" / "check_file_size.py")
    assert spec is not None
    assert spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


checker = _load_checker()


def lines(count: int) -> str:
    return "x = 1\n" * count


def pyproject(exempt: dict[str, int] | None = None) -> str:
    entries = "".join(f'"{path}" = {size}\n' for path, size in (exempt or {}).items())
    return f"[tool.file-size.exempt]\n{entries}"


def run(git_repo, files: dict[str, str], exempt: dict[str, int] | None = None, capsys=None) -> tuple[int, str]:
    """Commit `files` and the exempt table, run the gate on the repository: (exit code, what it printed on stderr)."""
    git_repo.commit("add files", {**files, "pyproject.toml": pyproject(exempt)})
    capsys.readouterr()
    code = checker.main([str(git_repo.path)])
    return code, capsys.readouterr().err


def test_files_within_budget_pass(git_repo, capsys):
    files = {"src/pkg/mod.py": lines(10), "tests/test_mod.py": lines(10), "crates/core/src/lib.rs": lines(10)}
    assert run(git_repo, files, capsys=capsys) == (0, "")


def test_a_source_file_over_1000_lines_fails_naming_it_its_size_and_the_budget(git_repo, capsys):
    code, message = run(git_repo, {"src/pkg/big.py": lines(1001), "src/pkg/ok.py": lines(5)}, capsys=capsys)
    assert code == 1
    assert "src/pkg/big.py" in message
    assert "1001" in message
    assert "1000" in message
    assert "split" in message
    assert "ok.py" not in message


def test_a_rust_source_file_over_1000_lines_fails(git_repo, capsys):
    code, message = run(git_repo, {"crates/core/src/big.rs": lines(1001)}, capsys=capsys)
    assert code == 1
    assert "crates/core/src/big.rs" in message


def test_a_source_file_of_exactly_1000_lines_passes(git_repo, capsys):
    assert run(git_repo, {"src/pkg/edge.py": lines(1000)}, capsys=capsys) == (0, "")


def test_a_file_without_a_final_newline_counts_its_last_line(git_repo, capsys):
    code, message = run(git_repo, {"src/pkg/edge.py": lines(1000) + "x = 1"}, capsys=capsys)
    assert code == 1
    assert "1001" in message


@pytest.mark.parametrize(
    "path",
    ["tests/test_big.py", "tests/tools/helper.py", "crates/core/tests/big.rs", "src/pkg/test_inside_src.py"],
)
def test_a_test_file_has_a_budget_of_1500_lines(git_repo, capsys, path):
    assert run(git_repo, {path: lines(1001)}, capsys=capsys) == (0, "")
    assert run(git_repo, {path: lines(1500)}, capsys=capsys) == (0, "")
    code, message = run(git_repo, {path: lines(1501)}, capsys=capsys)
    assert code == 1
    assert path in message
    assert "1501" in message
    assert "1500" in message


def test_files_that_are_not_python_or_rust_are_not_counted(git_repo, capsys):
    assert run(git_repo, {"docs/api.md": lines(5000), "uv.lock": lines(5000)}, capsys=capsys) == (0, "")


def test_an_untracked_file_is_not_counted(git_repo, capsys):
    git_repo.commit("start", {"src/pkg/ok.py": lines(1)})
    (git_repo.path / "src/pkg/scratch.py").write_text(lines(2000), encoding="utf-8")
    assert checker.main([str(git_repo.path)]) == 0
    capsys.readouterr()


class TestExemptFiles:
    """A file listed in `[tool.file-size.exempt]` may shrink but never grow."""

    PATH = "src/pkg/legacy.py"

    def test_at_its_listed_size_passes(self, git_repo, capsys):
        assert run(git_repo, {self.PATH: lines(1200)}, {self.PATH: 1200}, capsys) == (0, "")

    def test_one_line_over_its_listed_size_fails_naming_the_file_and_both_sizes(self, git_repo, capsys):
        code, message = run(git_repo, {self.PATH: lines(1201)}, {self.PATH: 1200}, capsys)
        assert code == 1
        assert self.PATH in message
        assert "1201" in message
        assert "1200" in message
        assert "split" in message

    def test_one_line_under_its_listed_size_passes_while_still_over_budget(self, git_repo, capsys):
        assert run(git_repo, {self.PATH: lines(1199)}, {self.PATH: 1200}, capsys) == (0, "")

    def test_a_test_file_is_listed_against_its_own_budget(self, git_repo, capsys):
        path = "tests/test_legacy.py"
        assert run(git_repo, {path: lines(1600)}, {path: 1600}, capsys) == (0, "")

    def test_an_unlisted_file_beside_a_listed_one_still_fails(self, git_repo, capsys):
        files = {self.PATH: lines(1200), "src/pkg/other.py": lines(1001)}
        code, message = run(git_repo, files, {self.PATH: 1200}, capsys)
        assert code == 1
        assert "src/pkg/other.py" in message
        assert "legacy.py" not in message


class TestStaleEntries:
    """A listed file that is within budget or gone makes the gate fail, so the table cleans itself."""

    PATH = "src/pkg/legacy.py"

    def test_a_file_now_within_budget_fails_telling_to_delete_the_entry(self, git_repo, capsys):
        code, message = run(git_repo, {self.PATH: lines(1000)}, {self.PATH: 1200}, capsys)
        assert code == 1
        assert self.PATH in message
        assert "1000" in message
        assert "delete the entry" in message

    def test_a_file_that_no_longer_exists_fails_telling_to_delete_the_entry(self, git_repo, capsys):
        code, message = run(git_repo, {"src/pkg/ok.py": lines(1)}, {self.PATH: 1200}, capsys)
        assert code == 1
        assert self.PATH in message
        assert "delete the entry" in message

    def test_a_file_deleted_from_the_working_tree_but_still_tracked_counts_as_gone(self, git_repo, capsys):
        git_repo.commit("add", {self.PATH: lines(1200), "pyproject.toml": pyproject({self.PATH: 1200})})
        (git_repo.path / self.PATH).unlink()
        assert checker.main([str(git_repo.path)]) == 1
        assert "delete the entry" in capsys.readouterr().err

    def test_a_test_file_between_the_two_budgets_is_stale(self, git_repo, capsys):
        path = "tests/test_legacy.py"
        code, message = run(git_repo, {path: lines(1200)}, {path: 1600}, capsys)
        assert code == 1
        assert "delete the entry" in message


def test_without_a_pyproject_the_table_is_empty(git_repo, capsys):
    git_repo.commit("add", {"src/pkg/big.py": lines(1001)})
    assert checker.main([str(git_repo.path)]) == 1
    assert "src/pkg/big.py" in capsys.readouterr().err


def test_this_repository_is_within_budget(capsys):
    assert checker.main([str(ROOT)]) == 0
    assert capsys.readouterr().err == ""
