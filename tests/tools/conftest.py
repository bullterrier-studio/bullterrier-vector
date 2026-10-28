"""What the tests of the tooling share: a scratch copy of the project to write one fault into (`project_copy`), a
throwaway git repository (`git_repo`) and the environment `uv run` gives a command (`uv_run_env`, `copy_env`)."""

import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]


@pytest.fixture
def project_copy(tmp_path):
    """`project_copy(*names)` copies the project's src/ (without bytecode caches) and the files `names` at its root
    into tmp_path, and returns tmp_path: a scratch project to write one fault into, the real one untouched."""

    def copy(*names: str) -> Path:
        shutil.copytree(ROOT / "src", tmp_path / "src", ignore=shutil.ignore_patterns("__pycache__", "*.pyc"))
        for name in names:
            shutil.copy(ROOT / name, tmp_path)
        return tmp_path

    return copy


class GitRepo:
    """A throwaway repository on `main`, isolated from the developer's git configuration (no global or system config,
    so none of their signing, hooks or default branch leaks in). Its environment is this process's, from which the
    autouse `no_enclosing_git` fixture (tests/conftest.py) has removed every `GIT_*` variable."""

    def __init__(self, path: Path) -> None:
        self.path = path / "work"
        self.path.mkdir()
        empty_config = path / "gitconfig"
        empty_config.write_text("", encoding="utf-8")
        self.env = {**os.environ, "GIT_CONFIG_GLOBAL": str(empty_config), "GIT_CONFIG_NOSYSTEM": "1"}
        self.git("init", "-q", "-b", "main")
        self.git("config", "user.name", "Test Author")
        self.git("config", "user.email", "author@example.com")

    def git(self, *args: str) -> str:
        """Run git in the repository and return its output (stripped); fail the test if git fails."""
        result = subprocess.run(
            ["git", *args], cwd=self.path, env=self.env, capture_output=True, encoding="utf-8", check=False
        )
        assert result.returncode == 0, f"git {' '.join(args)}: {result.stderr}"
        return result.stdout.strip()

    def commit(self, message: str, files: dict[str, str]) -> str:
        """Write `files` (name to text), commit everything with `message` and return the commit's hash."""
        for name, text in files.items():
            (self.path / name).parent.mkdir(parents=True, exist_ok=True)
            (self.path / name).write_text(text, encoding="utf-8")
        self.git("add", "--all")
        self.git("commit", "-q", "-m", message)
        return self.git("rev-parse", "HEAD")


@pytest.fixture
def git_repo(tmp_path) -> GitRepo:
    return GitRepo(tmp_path)


@pytest.fixture
def uv_run_env() -> dict[str, str]:
    """The environment a command gets under `uv run`: this one, with its executables first on PATH."""
    return {**os.environ, "PATH": os.pathsep.join([str(Path(sys.executable).parent), os.environ.get("PATH", "")])}


@pytest.fixture
def copy_env(uv_run_env) -> dict[str, str]:
    """The environment for running a poe task in a copy of the project: its `uv run` uses this environment as it is
    (`UV_PROJECT_ENVIRONMENT`, `UV_NO_SYNC`) instead of making one for the copy and building the copy's package."""
    return {**uv_run_env, "UV_PROJECT_ENVIRONMENT": str(Path(sys.executable).parents[1]), "UV_NO_SYNC": "1"}
