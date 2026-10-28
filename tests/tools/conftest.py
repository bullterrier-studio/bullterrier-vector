"""What the tests of the tooling share: a scratch copy of the project to write one fault into (`project_copy`) and the
environment `uv run` gives a command (`uv_run_env`, `copy_env`)."""

import os
import shutil
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


@pytest.fixture
def uv_run_env() -> dict[str, str]:
    """The environment a command gets under `uv run`: this one, with its executables first on PATH."""
    return {**os.environ, "PATH": os.pathsep.join([str(Path(sys.executable).parent), os.environ.get("PATH", "")])}


@pytest.fixture
def copy_env(uv_run_env) -> dict[str, str]:
    """The environment for running a poe task in a copy of the project: its `uv run` uses this environment as it is
    (`UV_PROJECT_ENVIRONMENT`, `UV_NO_SYNC`) instead of making one for the copy and building the copy's package."""
    return {**uv_run_env, "UV_PROJECT_ENVIRONMENT": str(Path(sys.executable).parents[1]), "UV_NO_SYNC": "1"}
