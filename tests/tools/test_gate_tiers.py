"""`poe check` runs the gate proofs only when what they read changed since they last passed; `poe full` always does.
The decision is `tools/pre_push.py`'s: it reads the changes from git against a baseline recorded in the git folder when
the proofs last passed, and the trigger list of `[tool.gate-proofs]` in pyproject.toml. The tests use real throwaway
repositories; anything git cannot say (no record, a shallow clone, a lost commit, no repository) runs the proofs."""

import importlib.util
import tomllib
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]


def _load():
    spec = importlib.util.spec_from_file_location("pre_push", ROOT / "tools" / "pre_push.py")
    assert spec is not None
    assert spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


pre_push = _load()
TRIGGERS = ["tools/", "pyproject.toml", "crates/wasm/src/lib.rs"]
LISTED = tomllib.loads((ROOT / "pyproject.toml").read_text(encoding="utf-8"))["tool"]["gate-proofs"]["triggers"]


@pytest.mark.parametrize(
    ("path", "triggers_the_proofs"),
    [
        pytest.param("tools/pre_push.py", True, id="a tool the gates run"),
        pytest.param("tests/tools/test_rust_gates.py", True, id="a proof"),
        pytest.param("tests/wasm/oracle.py", True, id="the oracle a proof edits"),
        pytest.param("pyproject.toml", True, id="the configuration every proof copies"),
        pytest.param("Cargo.lock", True, id="the lock the proofs build from"),
        pytest.param("clippy.toml", True, id="the lint configuration"),
        pytest.param(".github/workflows/test.yml", True, id="a workflow, which a proof lints"),
        pytest.param("crates/core/src/lib.rs", False, id="a file a proof only appends to: no text of it is matched"),
        pytest.param("crates/core/src/geometry.rs", False, id="another file a proof only appends to"),
        pytest.param("crates/wasm/src/lib.rs", True, id="the glue a proof breaks by text"),
        pytest.param("crates/core/src/output/screen.rs", False, id="an unrelated library file"),
        pytest.param("crates/core/tests/it/path.rs", False, id="a Rust test"),
        pytest.param("src/bullterrier_vector/frame_export.py", False, id="an unrelated Python module"),
        pytest.param("README.md", False, id="a page"),
        pytest.param("docs/design.md", False, id="a design page"),
    ],
)
def test_the_listed_triggers_decide_which_changed_paths_call_for_the_proofs(path, triggers_the_proofs):
    assert pre_push.triggers_proofs(path, LISTED) is triggers_the_proofs


@pytest.mark.parametrize(
    ("path", "triggers_the_proofs"),
    [
        pytest.param("tools/x.py", True, id="inside a listed folder"),
        pytest.param("tools", False, id="the folder's name without the slash is not inside it"),
        pytest.param("toolsmith/x.py", False, id="a folder that starts with the same letters"),
        pytest.param("pyproject.toml", True, id="a listed file"),
        pytest.param("sub/pyproject.toml", False, id="a file of that name elsewhere"),
        pytest.param("crates/wasm/src/lib.rs", True, id="a listed file in a folder"),
        pytest.param("crates/wasm/src/other.rs", False, id="its neighbour"),
    ],
)
def test_a_trigger_names_a_file_or_a_folder_ending_in_a_slash(path, triggers_the_proofs):
    assert pre_push.triggers_proofs(path, TRIGGERS) is triggers_the_proofs


def test_every_listed_trigger_exists():
    """A trigger for a path that is gone, renamed or mistyped would silently stop guarding what the proofs read."""
    assert LISTED, "no trigger at all"
    missing = [t for t in LISTED if not (ROOT / t).is_dir() and not (ROOT / t).is_file()]
    assert missing == []
    folders_without_slash = [t for t in LISTED if (ROOT / t).is_dir() and not t.endswith("/")]
    assert folders_without_slash == []


def level_with_a_pass(repo, monkeypatch):
    """A repository whose proofs passed on its only commit."""
    repo.commit("chore: start", {"tools/a.py": "a = 1", "src/lib.py": "x = 1", "README.md": "r"})
    monkeypatch.chdir(repo.path)
    pre_push.record_proofs_pass()


def test_without_a_record_of_a_pass_the_proofs_are_due(git_repo, monkeypatch):
    git_repo.commit("chore: start", {"tools/a.py": "a = 1"})
    monkeypatch.chdir(git_repo.path)
    assert "no record" in pre_push.proofs_due(TRIGGERS)


@pytest.mark.parametrize(
    ("change", "due"),
    [
        pytest.param({"src/lib.py": "x = 2"}, False, id="a committed change to an unlisted file"),
        pytest.param({"README.md": "page"}, False, id="a committed page"),
        pytest.param({"tools/a.py": "a = 2"}, True, id="a committed change to a file in a listed folder"),
        pytest.param({"tools/new.py": "b = 1"}, True, id="a new file in a listed folder"),
        pytest.param({"pyproject.toml": "[x]"}, True, id="a listed file"),
        pytest.param({"src/lib.py": "x = 2", "tools/a.py": "a = 3"}, True, id="one listed file among others"),
    ],
)
def test_the_proofs_are_due_when_a_commit_since_the_pass_touches_a_trigger(git_repo, monkeypatch, change, due):
    level_with_a_pass(git_repo, monkeypatch)
    assert pre_push.proofs_due(TRIGGERS) is None
    git_repo.commit("feat: change", change)
    assert (pre_push.proofs_due(TRIGGERS) is not None) is due


@pytest.mark.parametrize(
    ("write", "due"),
    [
        pytest.param(("tools/a.py", "a = 2"), True, id="an uncommitted edit of a listed file"),
        pytest.param(("tools/untracked.py", "u = 1"), True, id="an untracked file in a listed folder"),
        pytest.param(("src/lib.py", "x = 3"), False, id="an uncommitted edit of an unlisted file"),
        pytest.param(("notes.txt", "n"), False, id="an untracked unlisted file"),
    ],
)
def test_changes_not_yet_committed_count_as_changes(git_repo, monkeypatch, write, due):
    level_with_a_pass(git_repo, monkeypatch)
    name, text = write
    (git_repo.path / name).write_text(text, encoding="utf-8")
    assert (pre_push.proofs_due(TRIGGERS) is not None) is due


def test_a_deleted_trigger_file_calls_for_the_proofs(git_repo, monkeypatch):
    level_with_a_pass(git_repo, monkeypatch)
    git_repo.git("rm", "-q", "tools/a.py")
    assert pre_push.proofs_due(TRIGGERS) is not None


def test_a_change_made_before_the_pass_is_not_a_change_since_it(git_repo, monkeypatch):
    git_repo.commit("chore: start", {"tools/a.py": "a = 1"})
    git_repo.commit("feat: tool", {"tools/a.py": "a = 2"})
    monkeypatch.chdir(git_repo.path)
    pre_push.record_proofs_pass()
    git_repo.commit("feat: library", {"src/lib.py": "x = 1"})
    assert pre_push.proofs_due(TRIGGERS) is None


def test_a_pass_on_a_dirty_tree_is_not_remembered(git_repo, monkeypatch):
    """The proofs saw files that the commit does not hold, so no commit stands for what they checked."""
    git_repo.commit("chore: start", {"tools/a.py": "a = 1"})
    monkeypatch.chdir(git_repo.path)
    (git_repo.path / "tools" / "a.py").write_text("a = 2", encoding="utf-8")
    pre_push.record_proofs_pass()
    assert pre_push.proofs_due(TRIGGERS) is not None


def test_a_baseline_commit_git_no_longer_has_calls_for_the_proofs(git_repo, monkeypatch):
    level_with_a_pass(git_repo, monkeypatch)
    (git_repo.path / ".git" / "gate-proofs-passed").write_text("f" * 40, encoding="utf-8")
    assert "cannot" in pre_push.proofs_due(TRIGGERS)


def test_a_shallow_clone_calls_for_the_proofs(git_repo, monkeypatch, tmp_path, explicit_env):
    """Its history stops short, so a diff against the recorded commit may be missing what happened before it."""
    level_with_a_pass(git_repo, monkeypatch)
    git_repo.commit("feat: more", {"src/lib.py": "x = 2"})
    clone = tmp_path / "shallow"
    git_repo.git("clone", "-q", "--depth", "1", f"file://{git_repo.path}", str(clone))
    monkeypatch.chdir(clone)
    baseline = clone / ".git" / "gate-proofs-passed"
    baseline.write_text(git_repo.git("rev-parse", "HEAD"), encoding="utf-8")  # a commit the clone has
    assert "shallow" in pre_push.proofs_due(TRIGGERS)


def test_outside_a_repository_the_proofs_are_due(tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    assert pre_push.proofs_due(TRIGGERS) is not None


# The step `poe check` runs ------------------------------------------------------------------------------------------

PROOFS_TASK = '[tool.poe.tasks]\nproofs = "python -c \\"print(\'the proofs ran\')\\""\n'
FAILING_PROOFS_TASK = '[tool.poe.tasks]\nproofs = "python -c \\"raise SystemExit(3)\\""\n'


def test_check_skips_the_proofs_when_nothing_they_read_changed_and_says_so(git_repo, monkeypatch, capsys):
    git_repo.commit("chore: start", {"pyproject.toml": PROOFS_TASK, "tools/a.py": "a = 1", "src/lib.py": "x = 1"})
    monkeypatch.chdir(git_repo.path)
    pre_push.record_proofs_pass()
    git_repo.commit("feat: library", {"src/lib.py": "x = 2"})
    assert pre_push.run_proofs_if_due(["tools/"]) == 0
    out = capsys.readouterr().out
    assert "the proofs ran" not in out
    assert "skipped" in out


def test_check_runs_the_proofs_when_due_and_remembers_the_pass(git_repo, monkeypatch, capfd):
    git_repo.commit("chore: start", {"pyproject.toml": PROOFS_TASK, "tools/a.py": "a = 1"})
    monkeypatch.chdir(git_repo.path)
    assert pre_push.run_proofs_if_due(["tools/"]) == 0
    assert "the proofs ran" in capfd.readouterr().out
    assert pre_push.proofs_due(["tools/"]) is None, "the pass is remembered"


def test_check_fails_with_the_proofs_status_and_remembers_nothing(git_repo, monkeypatch):
    git_repo.commit("chore: start", {"pyproject.toml": FAILING_PROOFS_TASK, "tools/a.py": "a = 1"})
    monkeypatch.chdir(git_repo.path)
    assert pre_push.run_proofs_if_due(["tools/"]) == 3
    assert pre_push.proofs_due(["tools/"]) is not None
