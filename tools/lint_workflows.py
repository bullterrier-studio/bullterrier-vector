"""Lint the GitHub workflows offline: `poe lint-workflows` runs it from the project root.

- actionlint: syntax, expressions, job wiring (and shellcheck on the `run:` scripts when it is installed).
- zizmor, `--offline` so the gate needs no network and no token: unpinned actions, template injection, persisted
  credentials, over-broad permissions.

A repository with no `.github` folder has nothing to lint and passes. Any `.github` folder is linted by both tools,
which fail when it holds no workflow or one they cannot read: only the absence of the folder is tolerated.

Both tools always run, so one report names every finding. Exit code 0 when both pass (or there is no `.github`);
otherwise the first failing tool's exit code, each report printed by its tool.
"""

import shutil
import subprocess
import sys
from pathlib import Path

GITHUB = Path(".github")


def main() -> int:
    """Run both linters on `.github` in the current directory: the exit code of the first that fails, else 0."""
    if not GITHUB.is_symlink() and not GITHUB.exists():
        print("lint-workflows: no .github folder, nothing to lint")
        return 0
    codes = []
    for command in (["actionlint"], ["zizmor", "--offline", str(GITHUB)]):
        # dev tools installed beside this interpreter, so the environment `uv run` made is the one searched first
        program = shutil.which(command[0], path=str(Path(sys.executable).parent)) or command[0]
        codes.append(subprocess.run([program, *command[1:]], check=False).returncode)  # noqa: S603  fixed tools, a fixed path
    return next((code for code in codes if code != 0), 0)


if __name__ == "__main__":
    sys.exit(main())
