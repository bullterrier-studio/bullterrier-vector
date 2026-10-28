"""The workflow gate can fail: `poe lint-workflows` rejects a GitHub workflow that actionlint finds broken and one that
zizmor finds unsafe, naming the rule and the file, passes a safe one, and passes when the repository has no `.github`
folder at all (and only then).

An architecture test of the tooling: its subject is the `lint-workflows` task in pyproject.toml and the script it runs
(tools/lint_workflows.py), so it runs that task, the way `uv run poe check` does, in a throwaway git repository holding
a copy of the task and, in `.github/workflows/`, one probe workflow at a time. The probes are never written into this
repository.
"""

import shutil
import subprocess
import sys
import textwrap
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
# one xdist worker runs this module, so the two tools' start-up costs are not paid by several workers at once
pytestmark = [pytest.mark.gate, pytest.mark.xdist_group("workflow-gate")]

CHECKOUT = "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1"
GOOD = f"""\
name: Good
on: [push]
permissions:
  contents: read
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: {CHECKOUT}
        with:
          persist-credentials: false
      - run: echo hello
"""


@dataclass(frozen=True)
class Verdict:
    returncode: int
    report: str  # what the tools printed, both streams


@pytest.fixture
def lint(git_repo, copy_env) -> Callable[..., Verdict]:
    """`lint(**workflows)` writes each keyword (file stem to YAML text) into the repository's `.github/workflows/` and
    runs `poe lint-workflows`; with no keyword the repository has no `.github` folder. With `subdirectory`, it runs
    the script itself (what the task runs) from that folder of the repository instead, since poe starts in the root."""
    shutil.copy(ROOT / "pyproject.toml", git_repo.path)
    (git_repo.path / "tools").mkdir()
    shutil.copy(ROOT / "tools" / "lint_workflows.py", git_repo.path / "tools")

    def run(*, subdirectory: str | None = None, **workflows: str) -> Verdict:
        folder = git_repo.path / ".github" / "workflows"
        for stem, text in workflows.items():
            folder.mkdir(parents=True, exist_ok=True)
            (folder / f"{stem}.yml").write_text(textwrap.dedent(text), encoding="utf-8")
        if subdirectory is None:
            command, cwd = [sys.executable, "-m", "poethepoet", "lint-workflows"], git_repo.path
        else:
            cwd = git_repo.path / subdirectory
            cwd.mkdir(parents=True, exist_ok=True)
            command = [sys.executable, str(git_repo.path / "tools" / "lint_workflows.py")]
        result = subprocess.run(command, cwd=cwd, env=copy_env, capture_output=True, encoding="utf-8", check=False)
        return Verdict(result.returncode, result.stdout + result.stderr)

    return run


def test_a_safe_workflow_passes(lint):
    verdict = lint(good=GOOD)
    assert verdict.returncode == 0, verdict.report


def test_an_action_not_pinned_to_a_commit_fails(lint):
    verdict = lint(unpinned=GOOD.replace(CHECKOUT, "actions/checkout@v7"))
    assert verdict.returncode != 0
    assert "unpinned-uses" in verdict.report
    assert "unpinned.yml" in verdict.report


def test_a_pull_request_target_trigger_fails(lint):
    # only the trigger is rejected (zizmor's `dangerous-triggers`: it runs with a write token and secrets on a pull
    # request's behalf, whatever the job does); a checkout of the pull request's head is not what this row proves
    probe = """\
        name: Target
        on: pull_request_target
        permissions:
          contents: read
        jobs:
          build:
            runs-on: ubuntu-latest
            steps:
              - run: echo hello
    """
    verdict = lint(target=probe)
    assert verdict.returncode != 0
    assert "dangerous-triggers" in verdict.report
    assert "target.yml" in verdict.report


def test_a_run_step_that_interpolates_the_pull_request_title_fails(lint):
    probe = GOOD.replace("echo hello", 'echo "${{ github.event.pull_request.title }}"')
    verdict = lint(injection=probe)
    assert verdict.returncode != 0
    assert "template-injection" in verdict.report
    assert "injection.yml" in verdict.report


def test_a_job_with_write_all_permissions_fails(lint):
    probe = GOOD.replace("    runs-on: ubuntu-latest\n", "    runs-on: ubuntu-latest\n    permissions: write-all\n")
    verdict = lint(broad=probe)
    assert verdict.returncode != 0
    assert "excessive-permissions" in verdict.report
    assert "broad.yml" in verdict.report


def test_a_workflow_with_a_syntax_error_fails(lint):
    verdict = lint(broken=GOOD.replace("jobs:", "jobs\n"))
    assert verdict.returncode != 0
    assert "syntax-check" in verdict.report  # actionlint's rule
    assert "broken.yml" in verdict.report


def test_a_workflow_is_linted_when_the_script_starts_in_a_subdirectory(lint):
    # the project is where the script lives, not where it was started: a run from docs/ must not find "no .github"
    verdict = lint(subdirectory="docs", unpinned=GOOD.replace(CHECKOUT, "actions/checkout@v7"))
    assert verdict.returncode != 0
    assert "unpinned-uses" in verdict.report
    assert "unpinned.yml" in verdict.report


def test_the_repositorys_own_workflows_pass(uv_run_env, tmp_path):
    # known-good: the real tree, linted in place (read-only). A missing `.github` passes the gate, so the files are
    # asserted to exist; the script is started from elsewhere, as it finds the project from its own location.
    for name in ("ci", "lint", "test", "security", "pre-commit", "build"):
        assert (ROOT / ".github" / "workflows" / f"{name}.yml").is_file(), f"{name}.yml is missing"
    result = subprocess.run(
        [sys.executable, str(ROOT / "tools" / "lint_workflows.py")],
        cwd=tmp_path,
        env=uv_run_env,
        capture_output=True,
        encoding="utf-8",
        check=False,
    )
    assert result.returncode == 0, result.stdout + result.stderr


def test_a_repository_without_a_github_folder_passes(lint):
    verdict = lint()
    assert verdict.returncode == 0, verdict.report


def test_a_github_folder_without_workflows_fails(lint, git_repo):
    # only a missing `.github` is tolerated: an empty one is a mistake the tools report
    (git_repo.path / ".github" / "workflows").mkdir(parents=True)
    verdict = lint()
    assert verdict.returncode != 0
    assert "no YAML file was found" in verdict.report
