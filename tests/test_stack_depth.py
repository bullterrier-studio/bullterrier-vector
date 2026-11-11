"""The deepest structures the library accepts, and the deepest anything can build, must be drawn, printed, compared,
saved, loaded and dropped on the smallest thread stack the library supports (docs/design-decisions.md, "Bounded
depth"). A Python thread gets 512 KB on macOS, the main thread 8 MB; a crash here is a segfault, so each scenario
runs in a subprocess and the test reports the step it died in.

The expected result is only "finished": what each output looks like is tested where the output is."""

import subprocess
import sys
from pathlib import Path

import pytest

# the font whose glyph `A` is a chain of 64 composites, the deepest nesting the reader draws (tests/it/fonts)
DEEPEST_COMPOSITES = (
    Path(__file__).resolve().parents[1] / "crates" / "core" / "tests" / "it" / "fonts" / "synthetic.ttf"
)

SMALLEST_STACK_KB = 256  # the smallest thread stack supported; design-decisions.md says why this number

# The program each subprocess runs: a scenario name and a stack size in KB. It prints each step before it starts and
# `done` at the end, so a crash leaves the step it died in as the last line.
PROBE = '''
import os
import sys
import threading
import traceback

import bullterrier_vector as btvec

scenario, stack_kb, font_file = sys.argv[1], int(sys.argv[2]), sys.argv[3]
other = btvec.Path.circle((5, 5), 1).fill("#445566")


def step(name):
    print(name, flush=True)


def nested(leaf_expression):
    """128 drawings nested (a drawing holding two items per level, so the nesting is real), over a leaf that reads x."""
    drawing = btvec.Path.circle((5, 5), 2).fill("#112233").translate(leaf_expression, 0)
    for _ in range(127):
        drawing = btvec.Drawing(drawing, other)
    return drawing


def nested_through_other(leaf_expression):
    """128 drawings nested through the `other` of a boolean step (union, subtract, intersect, exclude in turn)."""
    drawing = btvec.Path.circle((5, 5), 2).fill("#112233").translate(leaf_expression, 0)
    for level in range(127):
        step = ("union", "subtract", "intersect", "exclude")[level % 4]
        drawing = getattr(btvec.Path.circle((5, 5), 4).fill("#445566"), step)(drawing)
    return drawing


def nested_through(step_name, levels):
    """`levels` clips or masks nested through the `region` or the `source` of the next. A mask's source may be clipped
    but not masked, so masks alternate with a clip of the drawing below them (two drawings a level)."""
    drawing = btvec.Path.circle((5, 5), 2).fill("#112233").translate(btvec.Param("x", default=1, min=0, max=10), 0)
    for _ in range(levels):
        ink = btvec.Path.circle((5, 5), 4).fill("#445566")
        if step_name == "mask":
            drawing = ink.mask(btvec.Path.circle((5, 5), 3).fill("#ffffff").clip(drawing))
        else:
            drawing = ink.clip(drawing)
    return drawing


def deepest_cut_document(step_name):
    """The deepest nesting through `step_name` that `to_json` accepts, saved, loaded, drawn and dropped."""
    for levels in range(127, 0, -1):
        drawing = nested_through(step_name, levels // 2 if step_name == "mask" else levels)
        try:
            text = drawing.to_json()
        except btvec.BullterrierVectorError:
            continue
        step("from_json")
        loaded = btvec.Drawing.from_json(text)
        step("draw the loaded document")
        loaded.to_svg(10, 10)
        loaded.to_rgba(10, 10)
        step("drop")
        return
    raise AssertionError("no nesting is saved")


def every_output_refusing(drawing):
    """Every output of a drawing whose mesh is refused (a clip and a mask have no range in it): the rest must draw."""
    for name in ("shapes", "to_svg", "to_rgba", "to_png"):
        step(name)
        method = getattr(drawing, name)
        method() if name == "shapes" else method(10, 10)
    step("to_mesh")
    try:
        drawing.to_mesh(10, 10)
    except btvec.ParameterError:
        pass
    step("parameters")
    drawing.parameters()
    step("repr")
    repr(drawing)
    step("to_json")
    try:
        drawing.to_json()
    except btvec.BullterrierVectorError:
        pass  # a nesting past what the format holds is refused; the refusal must not overflow either


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


def every_output(drawing, params=True):
    for name in ("shapes", "to_svg", "to_rgba", "to_png", "to_mesh"):
        step(name)
        method = getattr(drawing, name)
        method() if name == "shapes" else method(10, 10)
    if params:
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
    elif scenario == "nested_through_other":
        drawing = nested_through_other(x)
        every_output(drawing)
        step("drop")
        del drawing
    elif scenario in ("nested_through_clip", "nested_through_mask"):
        name = scenario.removeprefix("nested_through_")
        drawing = nested_through(name, 63 if name == "mask" else 127)
        every_output_refusing(drawing)
        step("drop")
        del drawing
    elif scenario in ("cut_document_clip", "cut_document_mask"):
        deepest_cut_document(scenario.removeprefix("cut_document_"))
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
    elif scenario == "nested_with_the_deepest_per_item_expression":
        deepest = btvec.by_index(x, 1)
        for level in range(126):
            deepest = (btvec.by_order(0, deepest), btvec.random(0, deepest), btvec.stagger(deepest))[level % 3]
        drawing = nested(deepest)
        every_output(drawing)
        step("drop")
        del drawing
    elif scenario == "per_item_expression_a_loop_builds":
        a, b = btvec.by_index(x, 1), btvec.by_index(x, 1)
        for level in range(100_000):
            a, b = btvec.random(0, a, seed=level), btvec.random(0, b, seed=level)
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
            raise SystemExit("a per-item expression 100,000 levels deep was drawn")
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
    elif scenario == "svg_documents":
        def svg(groups):
            return (
                '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 9 9">' + "<g>" * groups
                + '<rect width="1" height="1"/>' + "</g>" * groups + "</svg>"
            )
        step("deepest svg")
        drawing = btvec.Drawing.from_svg(svg(126))  # the root and the rect are two of the 128 levels
        drawing.shapes()
        drawing.to_svg(9, 9)
        btvec.Drawing.from_json(drawing.to_json())
        for groups in (127, 100_000):
            step(f"svg nested {groups} groups")
            try:
                btvec.Drawing.from_svg(svg(groups))
            except btvec.ParameterError:
                pass
            else:
                raise SystemExit("a too deep svg was read")
        step("drop")
        del drawing
    elif scenario == "text_in_the_deepest_composite_glyph":
        step("font")
        font_bytes = open(font_file, "rb").read()
        font = btvec.Font(font_bytes, name="deep")
        step("text")
        drawing = btvec.text("AAA", font=font, size=100)
        every_output(drawing, params=False)
        step("to_json and from_json")
        loaded = btvec.Drawing.from_json(drawing.to_json(), inputs={"deep": font_bytes})
        every_output(loaded, params=False)
        step("drop")
        del drawing, loaded, font
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
        [sys.executable, "-c", PROBE, scenario, str(stack_kb), str(DEEPEST_COMPOSITES)],
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
        pytest.param("nested_through_other", id="128-drawings-nested-through-the-other-of-booleans"),
        pytest.param("nested_through_clip", id="128-drawings-nested-through-the-region-of-clips"),
        pytest.param("nested_through_mask", id="128-drawings-nested-through-the-source-of-masks"),
        pytest.param("cut_document_clip", id="deepest-document-of-clips-to_json-accepts"),
        pytest.param("cut_document_mask", id="deepest-document-of-masks-to_json-accepts"),
        pytest.param("nested_with_the_deepest_expression", id="128-drawings-nested-over-a-128-level-expression"),
        pytest.param("expression_a_loop_builds", id="expression-100000-levels-built-by-a-loop"),
        pytest.param(
            "nested_with_the_deepest_per_item_expression",
            id="128-drawings-nested-over-a-128-level-per-item-expression",
        ),
        pytest.param("per_item_expression_a_loop_builds", id="per-item-expression-100000-levels-built-by-a-loop"),
        pytest.param("deepest_document", id="deepest-document-to_json-accepts"),
        pytest.param("hostile_documents", id="documents-nested-100000-levels"),
        pytest.param("svg_documents", id="svg-128-levels-and-100000-levels"),
        pytest.param("text_in_the_deepest_composite_glyph", id="text-of-a-glyph-of-64-nested-composites"),
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
