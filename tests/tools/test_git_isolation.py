"""A test that uses git never reaches the repository that runs it. A git hook (the pre-push gate runs pytest), a linked
worktree, `rebase -x` and `bisect` pass the repository they serve in `GIT_DIR`, `GIT_INDEX_FILE` and `GIT_WORK_TREE`;
without the autouse `no_enclosing_git` fixture the `git init` and `git commit` of a `git_repo` would land in it.

The proof runs a probe test in its own pytest process, with those variables pointing at a throwaway "enclosing"
repository made in tmp_path (never this repository), and checks that repository afterwards. A control run without the
fixture shows the probe does reach the enclosing repository when nothing defends it.
"""

import os
import shutil
import subprocess
import sys
from pathlib import Path
from typing import NamedTuple

ROOT = Path(__file__).resolve().parents[2]

PROBE = """def test_probe(git_repo):
    git_repo.commit("chore: inside the probe", {"probe.txt": "probe"})
"""
# the conftest of tests/ without its autouse fixture: what the probe would run under if the fixture were missing
NO_FIXTURE = '"""The control: nothing defends the test from an enclosing git."""\n'


def git(repo: Path, *args: str) -> str:
    """Run git in `repo` (a folder of tmp_path) and return its output. The environment is built here, not taken on
    trust from the autouse fixture this module proves: none of the surrounding process's `GIT_*` variables, and no
    global or system configuration (an empty file beside `repo`), so neither a hook's `GIT_DIR` nor a developer's
    `commit.gpgsign` or `core.hooksPath` can reach the setup or the checks of these tests."""
    empty_config = repo.parent / "gitconfig"
    empty_config.touch()
    env = {name: value for name, value in os.environ.items() if not name.startswith("GIT_")}
    env |= {"GIT_CONFIG_GLOBAL": str(empty_config), "GIT_CONFIG_NOSYSTEM": "1"}
    result = subprocess.run(
        ["git", "-c", "user.name=T", "-c", "user.email=t@example.com", *args],
        cwd=repo,
        env=env,
        capture_output=True,
        encoding="utf-8",
        check=False,
    )
    assert result.returncode == 0, f"git {' '.join(args)}: {result.stderr}"
    return result.stdout


class Run(NamedTuple):
    enclosing: Path
    head: str  # the enclosing repository's HEAD before the probe ran
    status: str  # and its `git status --porcelain`
    probe: subprocess.CompletedProcess[str]


def run_probe(tmp_path: Path, tests_conftest: str) -> Run:
    """Run the probe under `tests_conftest` while `GIT_*` points at an enclosing repository."""
    enclosing = tmp_path / "enclosing"
    enclosing.mkdir()
    git(enclosing, "init", "-q", "-b", "main")
    (enclosing / "real.txt").write_text("real", encoding="utf-8")
    git(enclosing, "add", "--all")
    git(enclosing, "commit", "-q", "-m", "chore: the real work")
    (enclosing / "dirty.txt").write_text("uncommitted", encoding="utf-8")  # the status must not change either
    head, status = git(enclosing, "rev-parse", "HEAD"), git(enclosing, "status", "--porcelain")

    probe = tmp_path / "probe"
    (probe / "tools").mkdir(parents=True)
    (probe / "conftest.py").write_text(tests_conftest, encoding="utf-8")
    shutil.copy(ROOT / "tests" / "tools" / "conftest.py", probe / "tools" / "conftest.py")
    (probe / "tools" / "test_probe.py").write_text(PROBE, encoding="utf-8")
    # nothing from this process but the PATH: only what a hook would hand over
    env = {
        "PATH": os.environ["PATH"],
        "GIT_DIR": str(enclosing / ".git"),
        "GIT_WORK_TREE": str(enclosing),
        "GIT_INDEX_FILE": str(enclosing / ".git" / "index"),
        "GIT_AUTHOR_NAME": "Enclosing",
        "GIT_AUTHOR_EMAIL": "enclosing@example.com",
        "GIT_COMMITTER_NAME": "Enclosing",
        "GIT_COMMITTER_EMAIL": "enclosing@example.com",
    }
    pytest_args = ["-q", "-n", "0", "-p", "no:randomly", "-p", "no:cacheprovider", "--import-mode=importlib"]
    result = subprocess.run(
        [sys.executable, "-m", "pytest", *pytest_args, "--rootdir", str(probe), str(probe)],
        cwd=probe,
        env=env,
        capture_output=True,
        encoding="utf-8",
        check=False,
    )
    return Run(enclosing, head, status, result)


def test_a_git_repo_never_reaches_the_repository_that_enclosing_git_variables_name(tmp_path):
    run = run_probe(tmp_path, (ROOT / "tests" / "conftest.py").read_text(encoding="utf-8"))
    assert run.probe.returncode == 0, run.probe.stdout + run.probe.stderr  # the probe committed in its own repository
    assert git(run.enclosing, "rev-parse", "HEAD") == run.head
    assert git(run.enclosing, "status", "--porcelain") == run.status


def test_without_the_fixture_the_probe_does_reach_the_enclosing_repository(tmp_path):
    """The control: this is what the fixture prevents, so the test above can fail."""
    run = run_probe(tmp_path, NO_FIXTURE)
    assert git(run.enclosing, "rev-parse", "HEAD") != run.head
