"""Every gate with a proof that it can fail is part of `poe check`, and `poe fast` runs the Python tests without the
slow proofs: a gate that is proven but not wired in fails nothing.

An architecture test of the poe tasks in pyproject.toml. The tasks are expanded the way poe runs them (a `ref` runs
the task it names), so a gate counts however deeply `check` reaches it. A known-bad pyproject.toml, one gate taken out
of `check` or the marker misplaced, is rejected naming what is wrong."""

import copy
import tomllib
from pathlib import Path
from typing import Any

import pytest

ROOT = Path(__file__).resolve().parents[2]
# The gates whose failing is proven (tests/tools/, marker `gate`): a task of the same name, or a command written
# out in `check`. The one list of what must be wired; a new proof adds its gate here.
PROVEN_TASKS = [
    "rust-fmt",
    "rust-lint",
    "rust-doc",
    "rust-shear",
    "rust-deny",
    "rust-wasm",
    "rust-test",
    "stubtest",
    "dead-code",
    "lint-workflows",
    "build-check",
]
PROVEN_COMMANDS = ["ruff format --check .", "ruff check .", "deptry .", "mypy"]
GATES = [*PROVEN_TASKS, *PROVEN_COMMANDS]
SKIP_PROOFS = "-m 'not gate'"  # what keeps the slow proofs out of `fast`


def steps(tasks: dict[str, Any], name: str, path: tuple[str, ...] = ()) -> list[tuple[tuple[str, ...], str]]:
    """The commands task `name` runs, each with the names of the tasks that lead to it (the outermost first)."""
    task = tasks[name]
    here = (*path, name)
    if "cmd" in task:
        return [(here, task["cmd"])]
    found = []
    for item in task["sequence"]:
        match item:
            case str():
                found.append((here, item))
            case {"ref": referenced}:
                found += steps(tasks, referenced, here)
            case {"cmd": command}:
                found.append((here, command))
            case _:
                pytest.fail(f"{name}: a sequence item this test cannot expand: {item!r}")
    return found


def pytest_steps(tasks: dict[str, Any], name: str) -> list[str]:
    return [command for _, command in steps(tasks, name) if command.startswith("pytest ")]


def problems(pyproject: dict[str, Any]) -> list[str]:
    """What is wrong with how `check` and `fast` in `pyproject` are wired (empty when nothing is)."""
    tasks = pyproject["tool"]["poe"]["tasks"]
    check = steps(tasks, "check")
    reached = {name for path, _ in check for name in path} | {command for _, command in check}
    found = [f"check does not run {gate}" for gate in GATES if gate not in reached]
    fast_tests, check_tests = pytest_steps(tasks, "fast"), pytest_steps(tasks, "check")
    found += [f"fast: `{command}` lacks {SKIP_PROOFS}" for command in fast_tests if SKIP_PROOFS not in command]
    found += [f"check: `{command}` has {SKIP_PROOFS}" for command in check_tests if SKIP_PROOFS in command]
    found += [
        f"{name} runs no pytest step" for name, tests in (("fast", fast_tests), ("check", check_tests)) if not tests
    ]
    return found


def pyproject() -> dict[str, Any]:
    return tomllib.loads((ROOT / "pyproject.toml").read_text(encoding="utf-8"))


def test_the_gates_as_wired_pass():
    assert problems(pyproject()) == []


def without(gate: str) -> dict[str, Any]:
    """pyproject.toml with `gate` taken out of the sequence of `check`."""
    copied = copy.deepcopy(pyproject())
    sequence = copied["tool"]["poe"]["tasks"]["check"]["sequence"]
    remaining = [item for item in sequence if item not in (gate, {"ref": gate})]
    assert len(remaining) == len(sequence) - 1, f"{gate} is not a step of check"
    copied["tool"]["poe"]["tasks"]["check"]["sequence"] = remaining
    return copied


@pytest.mark.parametrize("gate", GATES)
def test_a_gate_taken_out_of_check_is_rejected_naming_it(gate):
    assert problems(without(gate)) == [f"check does not run {gate}"]


def test_fast_without_the_marker_is_rejected():
    copied = pyproject()
    fast = copied["tool"]["poe"]["tasks"]["fast"]
    fast["sequence"] = [
        {**item, "cmd": item["cmd"].replace(" -m 'not gate'", "")} if isinstance(item, dict) and "cmd" in item else item
        for item in fast["sequence"]
    ]
    assert problems(copied) == ["fast: `pytest -q -n auto` lacks -m 'not gate'"]


def test_check_with_the_marker_is_rejected():
    copied = pyproject()
    copied["tool"]["poe"]["tasks"]["test"]["cmd"] += " -m 'not gate'"
    assert problems(copied) == ["check: `pytest -q -n auto -m 'not gate'` has -m 'not gate'"]
