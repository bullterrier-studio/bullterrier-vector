"""What the tests of the tooling share: a scratch copy of the project to write one fault into (`project_copy`), a
throwaway git repository (`git_repo`) and the environments the proofs run commands in (`explicit_env`, `uv_run_env`,
`copy_env`, `cargo_env`): built from an allow-list, so a variable of the developer's shell or of a CI runner
(`RUSTFLAGS`, `GIT_DIR`, `CARGO_BUILD_TARGET`, `UV_*`) can neither fail a proof nor make one pass."""

import os
import shutil
import subprocess
import sys
from collections.abc import Callable
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
# What a command of a proof inherits from the process: where programs and the toolchains are, and the temporary and
# home folders. The Windows names are what Python, git and cargo need there to start at all (SYSTEMROOT, COMSPEC,
# PATHEXT) and to find a home, a cache and a temporary folder (USERPROFILE, APPDATA, LOCALAPPDATA, TEMP, TMP); each is
# passed on when set. Anything else is set by the proof itself.
INHERITED = (
    "PATH",
    "HOME",
    "CARGO_HOME",
    "RUSTUP_HOME",
    "TMPDIR",
    "SYSTEMROOT",
    "USERPROFILE",
    "TEMP",
    "TMP",
    "APPDATA",
    "LOCALAPPDATA",
    "COMSPEC",
    "PATHEXT",
)


@pytest.fixture(scope="module")
def explicit_env() -> Callable[..., dict[str, str]]:
    """`explicit_env(**extra)`: the `INHERITED` variables that are set in this process now, plus `extra`. Nothing else
    of this process's environment (a `RUSTFLAGS`, a `GIT_DIR`) reaches a command of a proof."""

    def build(**extra: str) -> dict[str, str]:
        return {**{name: os.environ[name] for name in INHERITED if name in os.environ}, **extra}

    return build


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
    so none of their signing, hooks or default branch leaks in). Its environment is the explicit one it is given."""

    def __init__(self, path: Path, env: dict[str, str]) -> None:
        self.path = path / "work"
        self.path.mkdir()
        empty_config = path / "gitconfig"
        empty_config.write_text("", encoding="utf-8")
        self.env = {**env, "GIT_CONFIG_GLOBAL": str(empty_config), "GIT_CONFIG_NOSYSTEM": "1"}
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

    def add_remote(self, name: str = "origin") -> None:
        """Add a new empty bare repository, next to the working one, as the remote `name`."""
        bare = self.path.parent / f"{name}.git"
        self.git("init", "-q", "--bare", str(bare))
        self.git("remote", "add", name, str(bare))

    def commit(self, message: str, files: dict[str, str]) -> str:
        """Write `files` (name to text), commit everything with `message` and return the commit's hash."""
        for name, text in files.items():
            (self.path / name).parent.mkdir(parents=True, exist_ok=True)
            (self.path / name).write_text(text, encoding="utf-8")
        self.git("add", "--all")
        self.git("commit", "-q", "-m", message)
        return self.git("rev-parse", "HEAD")


@pytest.fixture
def git_repo(tmp_path, explicit_env) -> GitRepo:
    return GitRepo(tmp_path, explicit_env())


@pytest.fixture
def uv_run_env(explicit_env) -> dict[str, str]:
    """The environment a command gets under `uv run`: the explicit one, with its executables first on PATH."""
    return explicit_env(PATH=os.pathsep.join([str(Path(sys.executable).parent), os.environ.get("PATH", "")]))


@pytest.fixture
def copy_env(uv_run_env) -> dict[str, str]:
    """The environment for running a poe task in a copy of the project: its `uv run` uses this environment as it is
    (`UV_PROJECT_ENVIRONMENT`, `UV_NO_SYNC`) instead of making one for the copy and building the copy's package."""
    return {**uv_run_env, "UV_PROJECT_ENVIRONMENT": str(Path(sys.executable).parents[1]), "UV_NO_SYNC": "1"}


@pytest.fixture(scope="module")
def cargo_env(explicit_env, tmp_path_factory) -> dict[str, str]:
    """`uv run`'s environment (PyO3 finds this Python), offline, uncoloured, with one build folder for every copy."""
    return explicit_env(
        PATH=os.pathsep.join([str(Path(sys.executable).parent), os.environ.get("PATH", "")]),
        CARGO_TARGET_DIR=str(tmp_path_factory.mktemp("target")),
        CARGO_NET_OFFLINE="true",
        CARGO_TERM_COLOR="never",
        NO_COLOR="1",
    )
