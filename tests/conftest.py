"""What tests in every folder share. Helpers used by one folder live in that folder's conftest.py."""

import os

import pytest


@pytest.fixture(autouse=True)
def no_enclosing_git(monkeypatch: pytest.MonkeyPatch) -> None:
    """Run every test as if no git were running it. A git hook (the gate that runs before a push), a linked worktree,
    `rebase -x` or `bisect` hands the repository it serves to its children in `GIT_*` variables, `GIT_DIR` among them,
    which would send the git of a test, in this process or in a subprocess, to that repository instead of the
    throwaway one: its `git init`, `git config` and `git commit` would land in the real repository."""
    for name in [name for name in os.environ if name.startswith("GIT_")]:
        monkeypatch.delenv(name)


@pytest.fixture(autouse=True)
def no_enclosing_hook(monkeypatch: pytest.MonkeyPatch) -> None:
    """Run every test as if no pre-commit hook were running it. The pre-push hook runs the gate, and so pytest, with
    `PRE_COMMIT_REMOTE_NAME`, `PRE_COMMIT_REMOTE_BRANCH`, `PRE_COMMIT_FROM_REF` and the like in its environment, which
    `tools/pre_push.py` reads: a test that does not set its own values would get the hook's, and pass or fail by what
    is being pushed. A test that needs one sets it."""
    for name in [name for name in os.environ if name.startswith("PRE_COMMIT_")]:
        monkeypatch.delenv(name)
