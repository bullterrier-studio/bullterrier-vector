"""The deepest structures the library accepts, and the deepest anything can build, must be drawn, printed, compared,
saved, loaded and dropped on the smallest thread stack the library supports (docs/design-decisions.md, "Bounded
depth"). A Python thread gets 512 KB on macOS, the main thread 8 MB; a crash here is a segfault, so each scenario
runs in a subprocess and the test reports the step it died in.

The expected result is only "finished": what each output looks like is tested where the output is."""

import subprocess
import sys

import pytest

SMALLEST_STACK_KB = 256  # the smallest thread stack supported; design-decisions.md says why this number

# The program each subprocess runs: a scenario name and a stack size in KB. It prints each step before it starts and
# `done` at the end, so a crash leaves the step it died in as the last line.
PROBE = '''
import os
import sys
import threading
import traceback

import bullterrier_vector as btvec

scenario, stack_kb = sys.argv[1], int(sys.argv[2])
other = btvec.Path.circle((5, 5), 1).fill("#445566")


def step(name):
    print(name, flush=True)


def nested(leaf_expression):
    """128 drawings nested (a drawing holding two items per level, so the nesting is real), over a leaf that reads x."""
    drawing = btvec.Path.circle((5, 5), 2).fill("#112233").translate(leaf_expression, 0)
    for _ in range(127):
        drawing = btvec.Drawing(drawing, other)
    return drawing


def deepest_json_drawing():
    """The deepest nesting `to_json` accepts, as a document."""
    x = btvec.Param("x", default=1, min=0, max=10)
    for levels in range(128, 0, -1):
        try:
            drawing = btvec.Path.circle((5, 5), 2).fill("#112233").translate(x, 0)
            for _ in range(levels - 1):
                drawing = btvec.Drawing(drawing, other)
            return drawing.to_json()
        except btvec.BullterrierVectorError:
            continue


def every_output(drawing):
    for name in ("shapes", "to_svg", "to_rgba", "to_png", "to_mesh"):
        step(name)
        method = getattr(drawing, name)
        method() if name == "shapes" else method(10, 10)
    step("with_values")
    drawing.with_values(x=2)
    step("draw with values")
    drawing.to_rgba(10, 10, x=3)
    step("parameters")
    drawing.parameters()
    step("repr")
    repr(drawing)
    step("to_json")
    try:
        drawing.to_json()
    except btvec.BullterrierVectorError:
        pass  # a nesting past what the format holds is refused; the refusal must not overflow either


def run():
    x = btvec.Param("x", default=1, min=0, max=10)
    if scenario == "nested":
        drawing = nested(x)
        every_output(drawing)
        step("drop")
        del drawing
    elif scenario == "nested_with_the_deepest_expression":
        deepest = x
        for _ in range(127):
            deepest = deepest + 1
        drawing = nested(deepest)
        every_output(drawing)
        step("drop")
        del drawing
    elif scenario == "expression_a_loop_builds":
        a, b = x + 1, x + 1
        for _ in range(100_000):
            a = a + 1
            b = b + 1
        step("repr")
        repr(a)
        step("str")
        str(a)
        step("hash")
        hash(a)
        step("==")
        a == b
        step("draw is refused")
        try:
            btvec.Drawing(btvec.Path.circle((5, 5), 2).fill("#112233").translate(a, 0), size=(10, 10)).to_rgba(10, 10)
        except btvec.ParameterError:
            pass
        else:
            raise SystemExit("an expression 100,000 levels deep was drawn")
        step("drop")
        del a, b
    elif scenario == "deepest_document":
        step("build and save")
        text = deepest_json_drawing()
        step("from_json")
        drawing = btvec.Drawing.from_json(text)
        every_output(drawing)
        step("drop")
        del drawing
    elif scenario == "hostile_documents":
        for name, text in (("arrays", "[" * 200_000), ("objects", '{"a":' * 200_000), ("mixed", '{"a":[' * 100_000)):
            step(name)
            try:
                btvec.Drawing.from_json(text)
            except btvec.FormatError:
                pass
            else:
                raise SystemExit("a hostile document was read")
    elif scenario == "a_probe_that_dies":  # known-bad input for this test's own runner
        step("about to die")
        os._exit(7)
    step("done")


def run_or_exit():
    try:
        run()
    except BaseException:  # a thread's exception would not change the exit status; say it and die
        traceback.print_exc()
        os._exit(1)


threading.stack_size(stack_kb * 1024)
thread = threading.Thread(target=run_or_exit)
thread.start()
thread.join()
'''


def run_probe(scenario: str, stack_kb: int = SMALLEST_STACK_KB) -> subprocess.CompletedProcess[str]:
    """Run `scenario` in a fresh Python with a thread of `stack_kb` KB."""
    return subprocess.run(
        [sys.executable, "-c", PROBE, scenario, str(stack_kb)],
        capture_output=True,
        text=True,
        encoding="utf-8",
        timeout=300,
        check=False,
    )


def last_step(probe: subprocess.CompletedProcess[str]) -> str:
    """The last step a probe printed: where it died, or `done`."""
    lines = probe.stdout.splitlines()
    return lines[-1] if lines else "(nothing printed)"


@pytest.mark.parametrize(
    "scenario",
    [
        pytest.param("nested", id="128-drawings-nested"),
        pytest.param("nested_with_the_deepest_expression", id="128-drawings-nested-over-a-128-level-expression"),
        pytest.param("expression_a_loop_builds", id="expression-100000-levels-built-by-a-loop"),
        pytest.param("deepest_document", id="deepest-document-to_json-accepts"),
        pytest.param("hostile_documents", id="documents-nested-100000-levels"),
    ],
)
def test_the_deepest_structures_work_on_the_smallest_supported_thread_stack(scenario):
    probe = run_probe(scenario)
    assert (probe.returncode, last_step(probe)) == (0, "done"), (
        f"{scenario} died in step {last_step(probe)!r} with exit status {probe.returncode} "
        f"on a {SMALLEST_STACK_KB} KB stack:\n{probe.stderr[-2000:]}"
    )


def test_the_runner_reports_a_probe_that_dies_and_the_step_it_died_in():
    probe = run_probe("a_probe_that_dies")
    assert (probe.returncode, last_step(probe)) == (7, "about to die")
