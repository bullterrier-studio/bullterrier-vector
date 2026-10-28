"""The dependency gate can fail: deptry, run with the project's configuration, rejects library code that imports a
package the project does not declare, or one that is only a development or a transitive dependency, and a declared
dependency that no code imports, and passes the package as it is.

An architecture test of the tooling: its subject is the `[tool.deptry]` configuration in pyproject.toml, so it runs
deptry the way `uv run poe check` does, on a copy of the package into which the offence is written.
"""

import subprocess
import sys
from pathlib import Path

import pytest

pytestmark = [pytest.mark.gate, pytest.mark.xdist_group("dependency-gate")]
PROBE = Path("src") / "bullterrier_vector" / "probe.py"


def deptry(checkout: Path) -> tuple[int, str]:
    command = [sys.executable, "-m", "deptry", "--no-ansi", "."]
    result = subprocess.run(command, cwd=checkout, capture_output=True, encoding="utf-8", check=False)
    return result.returncode, result.stdout + result.stderr


def finding_of(report: str, rule: str) -> str:
    [finding] = [line for line in report.splitlines() if rule in line]  # exactly this offence, once
    return finding


def test_the_package_as_it_is_declares_what_it_imports(project_copy):
    returncode, report = deptry(project_copy("pyproject.toml"))
    assert returncode == 0, report


@pytest.mark.parametrize(
    ("module", "rule"),
    [
        ("pytest", "DEP004"),  # a development dependency: an installed library would not have it
        ("pluggy", "DEP003"),  # installed, but only because pytest needs it: not declared by the project
        ("surely_not_installed_anywhere", "DEP001"),  # declared nowhere
    ],
)
def test_library_code_importing_an_undeclared_package_fails_naming_it(project_copy, module, rule):
    checkout = project_copy("pyproject.toml")
    (checkout / PROBE).write_text(f'"""Probe."""\n\nimport {module}  # noqa: F401\n', encoding="utf-8")
    returncode, report = deptry(checkout)
    assert returncode != 0, report
    finding = finding_of(report, rule)
    assert "probe.py" in finding
    assert module in finding


def test_a_declared_dependency_no_code_imports_fails_naming_it(project_copy):
    checkout = project_copy("pyproject.toml")
    manifest = checkout / "pyproject.toml"
    text = manifest.read_text(encoding="utf-8")
    assert "dependencies = []" in text
    manifest.write_text(text.replace("dependencies = []", 'dependencies = ["pluggy"]'), encoding="utf-8")
    returncode, report = deptry(checkout)
    assert returncode != 0, report
    finding = finding_of(report, "DEP002")
    assert "pyproject.toml" in finding
    assert "pluggy" in finding
