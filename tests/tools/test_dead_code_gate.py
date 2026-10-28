"""The dead-code gate can fail: `poe dead-code` (vulture, with the project's configuration) rejects an unused function,
class, method and import, naming each, and passes the package as it is.

An architecture test of the tooling: its subject is the `dead-code` task and `[tool.vulture]` in pyproject.toml, so it
runs that task, the way `uv run poe check` does, in a copy of the project into which the dead code is written.
"""

import subprocess
import sys
from pathlib import Path

import pytest

pytestmark = [pytest.mark.gate, pytest.mark.xdist_group("dead-code-gate")]
PROBE = Path("src") / "bullterrier_vector" / "leftover.py"

# One kind of dead code each: (source, the name vulture must report)
DEAD_CODE = {
    "a function": (
        '"""Probe."""\n\n\ndef never_called_helper() -> int:\n    """Nothing calls this."""\n    return 3\n',
        "never_called_helper",
    ),
    "a class": ('"""Probe."""\n\n\nclass NeverUsedThing:\n    """Nothing makes one."""\n', "NeverUsedThing"),
    "a method": (
        '"""Probe."""\n\n\nclass Used:\n    """Made below."""\n\n    def never_called_method(self) -> int:\n'
        '        """Nothing calls this."""\n        return 3\n\n\nUSED = Used()\n',
        "never_called_method",
    ),
    "an import": ('"""Probe."""\n\nfrom pathlib import PurePath\n', "PurePath"),
}


def dead_code(checkout: Path, env: dict[str, str]) -> subprocess.CompletedProcess[str]:
    # the simple executor runs the task's command from this environment, as under `uv run`
    command = [sys.executable, "-m", "poethepoet", "--executor", "simple", "dead-code"]
    return subprocess.run(command, cwd=checkout, env=env, capture_output=True, encoding="utf-8", check=False)


def test_the_package_as_it_is_has_no_dead_code(project_copy, uv_run_env):
    result = dead_code(project_copy("pyproject.toml"), uv_run_env)
    assert result.returncode == 0, result.stdout + result.stderr
    assert "unused" not in result.stdout


@pytest.mark.parametrize(("source", "name"), DEAD_CODE.values(), ids=DEAD_CODE.keys())
def test_dead_code_fails_the_gate_by_name(project_copy, uv_run_env, source, name):
    checkout = project_copy("pyproject.toml")
    (checkout / PROBE).write_text(source, encoding="utf-8")
    result = dead_code(checkout, uv_run_env)
    assert result.returncode != 0, result.stdout + result.stderr
    [finding] = [line for line in result.stdout.splitlines() if f"'{name}'" in line]
    assert "leftover.py" in finding
