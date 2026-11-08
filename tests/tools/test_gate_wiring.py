"""Every gate with a proof that it can fail is part of `poe check` and `poe full`; the proofs themselves run in `full`
and, when what they read changed, in `check`; `poe fast` runs the Python tests without them: a gate that is proven but
not wired in fails nothing.

An architecture test of the poe tasks in pyproject.toml. The tasks are expanded the way poe runs them (a `ref` runs
the task it names), so a gate counts however deeply a tier reaches it. A known-bad pyproject.toml, one gate taken out
of a tier, the proofs run in the wrong place or a tier recording under another's name, is rejected naming what is
wrong."""

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
    "rust-fonts",
    "rust-wasm",
    "rust-test",
    "rust-test-layout",
    "wasm-test",
    "stubtest",
    "dead-code",
    "lint-workflows",
    "build-check",
    "file-size",
]
PROVEN_COMMANDS = ["ruff format --check .", "ruff check .", "deptry .", "mypy"]
GATES = [*PROVEN_TASKS, *PROVEN_COMMANDS]
NOT_IN_FAST = ["wasm-test"]  # a release build and Node: `check` and `full` only
SKIP_PROOFS = "-m 'not gate'"  # what keeps the slow proofs out of `fast`, `check` and the `test` task
ONLY_PROOFS = "-m gate"  # what the `proofs` task selects
TRIGGERED_PROOFS = "python tools/pre_push.py proofs"  # the step of `check` that runs them when they are due


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


def reached(tasks: dict[str, Any], name: str) -> set[str]:
    """The names of the tasks and the commands that tier `name` runs, however deep."""
    found = steps(tasks, name)
    return {task for path, _ in found for task in path} | {command for _, command in found}


def pytest_steps(tasks: dict[str, Any], name: str) -> list[str]:
    return [command for _, command in steps(tasks, name) if command.startswith("pytest ")]


def problems(pyproject: dict[str, Any]) -> list[str]:
    """What is wrong with how `fast`, `check` and `full` in `pyproject` are wired (empty when nothing is)."""
    tasks = pyproject["tool"]["poe"]["tasks"]
    found = []
    for tier in ("check", "full"):
        tier_reaches = reached(tasks, tier)
        found += [f"{tier} does not run {gate}" for gate in GATES if gate not in tier_reaches]
        commands = [command for _, command in steps(tasks, tier)]
        # a tier recording its pass under another's name would let a check pass satisfy a tag push
        for kind in ("start", "pass"):
            prefix = f"python tools/pre_push.py {kind} "
            found += [
                f"{tier}: `{c}` does not name {tier}" for c in commands if c.startswith(prefix) and c != prefix + tier
            ]
            if prefix + tier not in commands:
                found.append(f"{tier} lacks the `{prefix + tier}` step")
    fast_reaches = reached(tasks, "fast")
    found += [f"fast runs {gate}, which is for check and full only" for gate in NOT_IN_FAST if gate in fast_reaches]
    if "proofs" in fast_reaches:
        found.append("fast runs the proofs, which are for full")
    if TRIGGERED_PROOFS not in [command for _, command in steps(tasks, "check")]:
        found.append(f"check does not run the proofs when they are due (`{TRIGGERED_PROOFS}`)")
    if "proofs" in reached(tasks, "check"):
        found.append("check runs the proofs always; only full does")
    if "proofs" not in reached(tasks, "full"):
        found.append("full does not run the proofs")
    for tier in ("fast", "check", "full"):
        tests = pytest_steps(tasks, tier)
        if not tests:
            found.append(f"{tier} runs no pytest step")
        found += [
            f"{tier}: `{command}` names no marker, so it runs the proofs where they are not wanted"
            for command in tests
            if SKIP_PROOFS not in command and ONLY_PROOFS not in command
        ]
    # every test is run by full: the others and the proofs
    full_tests = pytest_steps(tasks, "full")
    if not any(SKIP_PROOFS in command for command in full_tests):
        found.append(f"full: no pytest step with {SKIP_PROOFS}")
    if not any(ONLY_PROOFS in command.replace(SKIP_PROOFS, "") for command in full_tests):
        found.append(f"full: no pytest step with {ONLY_PROOFS}")
    return found


def pyproject() -> dict[str, Any]:
    return tomllib.loads((ROOT / "pyproject.toml").read_text(encoding="utf-8"))


def test_the_gates_as_wired_pass():
    assert problems(pyproject()) == []


def without(gate: str) -> dict[str, Any]:
    """pyproject.toml with `gate` taken out of the sequence that `check` and `full` share (`gates`)."""
    copied = copy.deepcopy(pyproject())
    sequence = copied["tool"]["poe"]["tasks"]["gates"]["sequence"]
    remaining = [item for item in sequence if item not in (gate, {"ref": gate})]
    assert len(remaining) == len(sequence) - 1, f"{gate} is not a step of gates"
    copied["tool"]["poe"]["tasks"]["gates"]["sequence"] = remaining
    return copied


@pytest.mark.parametrize("gate", GATES)
def test_a_gate_taken_out_of_the_shared_steps_is_rejected_for_both_tiers(gate):
    assert problems(without(gate)) == [f"check does not run {gate}", f"full does not run {gate}"]


@pytest.mark.parametrize("tier", ["check", "full"])
def test_a_tier_that_does_not_reach_the_shared_steps_is_rejected_naming_it(tier):
    def drop(tasks):
        tasks[tier]["sequence"].remove({"ref": "gates"})

    found = problems(edited(drop))
    assert f"{tier} does not run stubtest" in found
    assert [line for line in found if not line.startswith(tier)] == []


def edited(change) -> dict[str, Any]:
    copied = pyproject()
    change(copied["tool"]["poe"]["tasks"])
    return copied


def test_fast_running_a_check_only_gate_is_rejected():
    copied = edited(lambda tasks: tasks["fast"]["sequence"].append({"ref": "wasm-test"}))
    assert problems(copied) == ["fast runs wasm-test, which is for check and full only"]


def test_fast_running_the_proofs_is_rejected():
    copied = edited(lambda tasks: tasks["fast"]["sequence"].append({"ref": "proofs"}))
    assert "fast runs the proofs, which are for full" in problems(copied)


def test_a_test_task_without_the_marker_is_rejected_in_every_tier_that_runs_it():
    copied = edited(lambda tasks: tasks["test"].update(cmd=tasks["test"]["cmd"].replace(f" {SKIP_PROOFS}", "")))
    unmarked = "pytest -q -n auto"
    assert problems(copied) == [
        *(
            f"{tier}: `{unmarked}` names no marker, so it runs the proofs where they are not wanted"
            for tier in ("fast", "check", "full")
        ),
        "full: no pytest step with -m 'not gate'",
    ]


def test_check_running_the_proofs_always_is_rejected():
    copied = edited(lambda tasks: tasks["check"]["sequence"].append({"ref": "proofs"}))
    assert problems(copied) == ["check runs the proofs always; only full does"]


def test_check_without_the_triggered_proofs_step_is_rejected():
    copied = edited(lambda tasks: tasks["check"]["sequence"].remove(TRIGGERED_PROOFS))
    assert problems(copied) == [f"check does not run the proofs when they are due (`{TRIGGERED_PROOFS}`)"]


def test_full_without_the_proofs_is_rejected():
    copied = edited(lambda tasks: tasks["full"]["sequence"].remove({"ref": "proofs"}))
    assert problems(copied) == ["full does not run the proofs", "full: no pytest step with -m gate"]


def test_full_remembering_its_pass_as_check_is_rejected():
    """A check pass would then satisfy a tag push."""

    def rename(tasks):
        sequence = tasks["full"]["sequence"]
        sequence[sequence.index("python tools/pre_push.py pass full")] = "python tools/pre_push.py pass check"

    assert problems(edited(rename)) == [
        "full: `python tools/pre_push.py pass check` does not name full",
        "full lacks the `python tools/pre_push.py pass full` step",
    ]
