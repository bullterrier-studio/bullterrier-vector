"""Vector's own gates are part of `poe check` and `poe full`: they hang on `project-gates`, the extension point the
template runs last in both, and the slow ones stay out of `fast`. The template's own steps are wired by
test_project_wiring.py.

A gate with a proof that it can fail (tests/tools/, marker `gate`) that no tier runs would pass unnoticed whatever the
code does, so this test fails when `check` or `full` lacks one, or `fast` runs one that is for the gate alone. It reads
pyproject.toml; the known-bad rows edit a copy of it in memory."""

import copy
import tomllib
from pathlib import Path
from typing import Any

import pytest

ROOT = Path(__file__).resolve().parents[2]
PROJECT_GATES = ["file-size", "rust-fonts", "wasm-build", "rust-test-stack", "wasm-test"]
FULL_ONLY = ["dist-contents"]  # reads the distributions `build-check` builds: `full` only
NOT_IN_FAST = ["wasm-build", "rust-test-stack", "wasm-test"]  # builds and Node: `check` and `full` only


def pyproject() -> dict[str, Any]:
    return tomllib.loads((ROOT / "pyproject.toml").read_text(encoding="utf-8"))


def reached(tasks: dict[str, Any], name: str) -> set[str]:
    """The names of the tasks that task `name` runs, however deep."""
    task = tasks[name]
    found = {name}
    for item in task.get("sequence", []):
        if isinstance(item, dict) and "ref" in item:
            found |= reached(tasks, item["ref"])
    return found


def problems(pyproject: dict[str, Any]) -> list[str]:
    """What is wrong with how vector's gates hang on the tiers (empty when nothing is)."""
    tasks = pyproject["tool"]["poe"]["tasks"]
    found = []
    for tier in ("check", "full"):
        tier_reaches = reached(tasks, tier)
        found += [f"{tier} does not run {gate}" for gate in PROJECT_GATES if gate not in tier_reaches]
    found += [f"full does not run {gate}" for gate in FULL_ONLY if gate not in reached(tasks, "full")]
    found += [f"check runs {gate}, which is for full only" for gate in FULL_ONLY if gate in reached(tasks, "check")]
    fast_reaches = reached(tasks, "fast")
    found += [f"fast runs {gate}, which is for check and full only" for gate in NOT_IN_FAST if gate in fast_reaches]
    return found


def edited(change) -> dict[str, Any]:
    copied = copy.deepcopy(pyproject())
    change(copied["tool"]["poe"]["tasks"])
    return copied


def test_the_gates_as_wired_pass():
    assert problems(pyproject()) == []


@pytest.mark.parametrize("gate", PROJECT_GATES)
def test_a_gate_taken_out_of_project_gates_is_rejected_for_both_tiers(gate):
    def drop(tasks):
        sequence = tasks["project-gates"]["sequence"]
        sequence[:] = [item for item in sequence if item != {"ref": gate}]

    assert problems(edited(drop)) == [f"check does not run {gate}", f"full does not run {gate}"]


def test_project_gates_left_out_of_the_shared_steps_is_rejected_for_both_tiers():
    def drop(tasks):
        tasks["gates"]["sequence"][:] = [i for i in tasks["gates"]["sequence"] if i != {"ref": "project-gates"}]

    expected = [f"{tier} does not run {gate}" for tier in ("check", "full") for gate in PROJECT_GATES]
    assert problems(edited(drop)) == expected


@pytest.mark.parametrize("gate", NOT_IN_FAST)
def test_fast_running_a_check_only_gate_is_rejected(gate):
    copied = edited(lambda tasks: tasks["fast"]["sequence"].append({"ref": gate}))
    assert problems(copied) == [f"fast runs {gate}, which is for check and full only"]


def test_a_full_only_step_taken_out_of_project_full_is_rejected():
    def drop(tasks):
        tasks["project-full"]["sequence"][:] = []

    assert problems(edited(drop)) == ["full does not run dist-contents"]


def test_check_running_a_full_only_step_is_rejected():
    copied = edited(lambda tasks: tasks["check"]["sequence"].append({"ref": "dist-contents"}))
    assert problems(copied) == ["check runs dist-contents, which is for full only"]
