"""tools/pre_push.py decides whether a push must pass the whole local gate first: pushes to main, and anything that
is not a branch (a release tag), do; any other branch is left to CI, so feature-branch pushes stay quick, and so is a
push that changes only documentation."""

import importlib.util
import shutil
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


@pytest.mark.parametrize("ref", ["refs/heads/main", "refs/tags/v1.2.3", "refs/tags/v1.2.3rc1", "", "main"])
def test_main_tags_and_anything_unclear_must_pass_the_gate(ref):
    assert pre_push.must_run(ref)


@pytest.mark.parametrize("ref", ["refs/heads/feature/glyphs", "refs/heads/fix/windows-links", "refs/heads/main2"])
def test_other_branches_are_left_to_ci(ref):
    assert not pre_push.must_run(ref)


def test_a_feature_branch_push_skips_the_gate_and_succeeds(monkeypatch, capsys):
    """With uv hidden a gate that did run would show as a failure here, never as the whole gate running."""
    monkeypatch.setenv("PRE_COMMIT_REMOTE_BRANCH", "refs/heads/feature/x")
    monkeypatch.setattr(shutil, "which", lambda _name: None)
    assert pre_push.main() == 0
    assert "uv is not on the PATH" not in capsys.readouterr().out


@pytest.mark.parametrize(
    ("path", "is_documentation"),
    [
        pytest.param("README.md", True, id="the README"),
        pytest.param("CONTRIBUTING.md", True, id="a top-level page"),
        pytest.param("docs/roadmap.md", True, id="a page in docs"),
        pytest.param("docs/images/logo.svg", True, id="a picture in docs/images"),
        pytest.param("docs/images/banner/wide.jpg", True, id="a nested picture"),
        pytest.param("docs/images/a.png", True, id="a png"),
        pytest.param("docs/images/B.SVG", True, id="a picture with a capitalised extension"),
        pytest.param("docs/images/photo.webp", True, id="a webp"),
        pytest.param("docs/images/make.py", False, id="a script in docs/images"),
        pytest.param("docs/images/banner/make.sh", False, id="a script in a subfolder of docs/images"),
        pytest.param("docs/images/README", False, id="a file without a picture extension"),
        pytest.param("docs/notes/ideas.md", False, id="a page in a subfolder of docs"),
        pytest.param("docs/make_banner.py", False, id="a script in docs"),
        pytest.param("docs/data.json", False, id="data in docs"),
        pytest.param("crates/core/README.md", False, id="a page inside a crate, which cargo packages"),
        pytest.param("src/bullterrier_vector/notes.md", False, id="a markdown file inside src"),
        pytest.param("src/bullterrier_vector/errors.py", False, id="a source file"),
        pytest.param("crates/core/src/lib.rs", False, id="a Rust file"),
        pytest.param("tests/tools/test_pre_push.py", False, id="a test"),
        pytest.param("Cargo.lock", False, id="the Rust lock file"),
        pytest.param("uv.lock", False, id="the Python lock file"),
        pytest.param(".github/workflows/ci.yml", False, id="a workflow"),
    ],
)
def test_only_pages_and_pictures_count_as_documentation(path, is_documentation):
    assert pre_push.is_documentation(path) is is_documentation


@pytest.mark.parametrize(
    ("changed", "gate_runs"),
    [
        pytest.param(
            {"README.md": "x", "docs/images/logo.svg": "<svg/>"}, False, id="a page and a picture skip the gate"
        ),
        pytest.param({"README.md": "x", "src/code.py": "x = 1"}, True, id="a page with one code file runs the gate"),
        pytest.param({"src/code.py": "x = 1"}, True, id="a code file alone runs the gate"),
        pytest.param({"docs/images/make.py": "x = 1"}, True, id="a script in docs/images runs the gate"),
        pytest.param({"docs/images/a.png": "x"}, False, id="a picture in docs/images skips the gate"),
    ],
)
def test_a_push_of_only_documentation_skips_the_gate(git_repo, monkeypatch, capsys, changed, gate_runs):
    """The gate is `uv run poe check`; with uv hidden it shows as an attempt that fails, so the test never runs it."""
    before = git_repo.commit("chore: start", {"start.txt": "start"})
    after = git_repo.commit("docs: change", changed)
    monkeypatch.chdir(git_repo.path)
    monkeypatch.setenv("PRE_COMMIT_REMOTE_BRANCH", "refs/heads/main")
    monkeypatch.setenv("PRE_COMMIT_FROM_REF", before)
    monkeypatch.setenv("PRE_COMMIT_TO_REF", after)
    monkeypatch.setattr(shutil, "which", lambda _name: None)
    assert pre_push.main() == (1 if gate_runs else 0)
    assert ("uv is not on the PATH" in capsys.readouterr().out) is gate_runs


def test_a_push_whose_range_is_unknown_runs_the_gate(monkeypatch, capsys):
    monkeypatch.setenv("PRE_COMMIT_REMOTE_BRANCH", "refs/heads/main")
    monkeypatch.delenv("PRE_COMMIT_FROM_REF", raising=False)
    monkeypatch.delenv("PRE_COMMIT_TO_REF", raising=False)
    monkeypatch.setattr(shutil, "which", lambda _name: None)
    assert pre_push.main() == 1
    assert "uv is not on the PATH" in capsys.readouterr().out


@pytest.mark.parametrize(
    ("from_ref", "to_ref"),
    [
        pytest.param("0" * 40, "HEAD", id="a pushed branch that is new on the remote: no earlier commit"),
        pytest.param("f" * 40, "HEAD", id="a commit this clone does not have"),
        pytest.param("HEAD", "", id="an empty end of the range"),
        pytest.param("", "HEAD", id="an empty start of the range"),
    ],
)
def test_a_range_git_cannot_read_runs_the_gate(git_repo, monkeypatch, capsys, from_ref, to_ref):
    git_repo.commit("docs: only a page", {"README.md": "x"})
    monkeypatch.chdir(git_repo.path)
    monkeypatch.setenv("PRE_COMMIT_REMOTE_BRANCH", "refs/heads/main")
    monkeypatch.setenv("PRE_COMMIT_FROM_REF", from_ref)
    monkeypatch.setenv("PRE_COMMIT_TO_REF", to_ref)
    monkeypatch.setattr(shutil, "which", lambda _name: None)
    assert pre_push.main() == 1
    assert "uv is not on the PATH" in capsys.readouterr().out


def test_a_push_that_changes_nothing_runs_the_gate(git_repo, monkeypatch, capsys):
    """An empty list of changed files is no proof that only documentation changed."""
    head = git_repo.commit("docs: a page", {"README.md": "x"})
    monkeypatch.chdir(git_repo.path)
    monkeypatch.setenv("PRE_COMMIT_REMOTE_BRANCH", "refs/heads/main")
    monkeypatch.setenv("PRE_COMMIT_FROM_REF", head)
    monkeypatch.setenv("PRE_COMMIT_TO_REF", head)
    monkeypatch.setattr(shutil, "which", lambda _name: None)
    assert pre_push.main() == 1
    assert "uv is not on the PATH" in capsys.readouterr().out


def test_a_documentation_only_push_of_a_tag_skips_the_gate_too(git_repo, monkeypatch):
    before = git_repo.commit("chore: start", {"start.txt": "start"})
    after = git_repo.commit("docs: change", {"docs/roadmap.md": "x"})
    monkeypatch.chdir(git_repo.path)
    monkeypatch.setenv("PRE_COMMIT_REMOTE_BRANCH", "refs/tags/v0.1.0")
    monkeypatch.setenv("PRE_COMMIT_FROM_REF", before)
    monkeypatch.setenv("PRE_COMMIT_TO_REF", after)
    monkeypatch.setattr(shutil, "which", lambda _name: None)
    assert pre_push.main() == 0


def test_a_rename_of_code_into_docs_runs_the_gate(git_repo, monkeypatch, capsys):
    """With rename detection `git diff --name-only` lists only the new name, `docs/lib.md`: that would skip the gate."""
    before = git_repo.commit("chore: start", {"src/lib.rs": "pub fn draw() -> u32 {\n    1\n}\n" * 20})
    (git_repo.path / "docs").mkdir()
    git_repo.git("mv", "src/lib.rs", "docs/lib.md")
    git_repo.git("commit", "-q", "-m", "docs: move")
    after = git_repo.git("rev-parse", "HEAD")
    assert git_repo.git("diff", "--name-status", "-M", f"{before}..{after}").startswith("R100")  # git sees a rename
    monkeypatch.chdir(git_repo.path)
    monkeypatch.setenv("PRE_COMMIT_REMOTE_BRANCH", "refs/heads/main")
    monkeypatch.setenv("PRE_COMMIT_FROM_REF", before)
    monkeypatch.setenv("PRE_COMMIT_TO_REF", after)
    monkeypatch.setattr(shutil, "which", lambda _name: None)
    assert pre_push.main() == 1
    assert "uv is not on the PATH" in capsys.readouterr().out
