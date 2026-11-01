"""Time the outputs on one reference drawing, natively and in WebAssembly, and print a table.

    uv run poe bench [--shapes N] [--size WxH] [--repeats N] [--warmups N] [--require-node]

The reference drawing is a star and `--shapes` (default 200) filled, stroked, translucent and multiply shapes from a
seeded generator, at 1920x1080 by default. `to_rgba` and `to_mesh` are each called `--warmups` times (default 3) and
then `--repeats` times (default 20); the table shows the median of the timed calls in milliseconds. The native column is
the Python extension (`time.perf_counter`). The WebAssembly column is the package built by `uv run poe wasm-test`, run
in Node (`performance.now`) on the same drawing, loaded from its JSON. Without Node, or without the package in `pkg/`,
the WebAssembly column is left out and one line on stderr says why; `--require-node` makes that exit 1. The numbers are
for a person to read: nothing asserts on time. Results are recorded by hand in docs/benchmarks.md.

Interface:
    reference_drawing(shapes=200, seed=1) -> Drawing  the drawing that is timed
    main(argv) -> exit code: 0 the table was printed, 1 a bad request or, with `--require-node`, no WebAssembly column
"""

import argparse
import json
import random
import re
import shutil
import statistics
import subprocess
import sys
import time
from collections.abc import Callable
from pathlib import Path

import bullterrier_vector as btvec

ROOT = Path(__file__).resolve().parent.parent
PACKAGE = ROOT / "pkg" / "bullterrier_vector_wasm.js"
NODE_SCRIPT = Path(__file__).resolve().with_name("bench.mjs")
OUTPUTS = ("to_rgba", "to_mesh")
# constant: the reference scene's own numbers, fixed so that results recorded at different times compare
SEED = 1
SHAPES = 200
WARMUPS = 3
REPEATS = 20
DEFAULT_SIZE = (1920, 1080)
SIZE_RANGE = (20, 160)  # the radius of a shape, in design pixels
STROKE_WIDTHS = (1, 12)
TRANSLUCENT_SHARE = 0.4  # shapes with an alpha colour
OPACITY_SHARE = 0.3  # shapes with an opacity below 1
MULTIPLY_SHARE = 0.2  # shapes drawn with multiply


class BenchError(Exception):
    """A request the tool cannot carry out; the message says what, in one line."""


def colour(rng: random.Random, *, translucent: bool) -> str:
    """A random `#rrggbb`, or `#rrggbbaa` with an alpha between 25% and 90%."""
    base = f"#{rng.randrange(0x1000000):06x}"
    return base + f"{rng.randrange(64, 230):02x}" if translucent else base


def outline(rng: random.Random, width: int, height: int) -> btvec.Path:
    """A random closed shape of one of five kinds, placed anywhere in the frame."""
    centre = (rng.uniform(0, width), rng.uniform(0, height))
    radius = rng.uniform(*SIZE_RANGE)
    match rng.choice(("circle", "polygon", "star", "rounded", "curved")):
        case "circle":
            return btvec.Path.circle(centre, radius)
        case "polygon":
            return btvec.Path.polygon(centre, radius, rng.randrange(3, 9))
        case "star":
            return btvec.Path.star(centre, radius, radius * rng.uniform(0.3, 0.7), points=rng.randrange(4, 12))
        case "rounded":
            return btvec.Path.rect(
                centre[0] - radius, centre[1] - radius / 2, 2 * radius, radius, radius=rng.uniform(1, radius / 2)
            )
        case _:
            x, y = centre
            return (
                btvec.Path()
                .move_to((x - radius, y))
                .curve_to((x - radius, y - radius), (x + radius, y - radius), (x + radius, y))
                .curve_to((x + radius, y + radius), (x - radius, y + radius), (x - radius, y))
                .close()
            )


def reference_drawing(shapes: int = SHAPES, seed: int = SEED, size: tuple[int, int] = DEFAULT_SIZE) -> btvec.Drawing:
    """A star and `shapes` random filled, stroked, translucent and multiply shapes, the same for the same arguments."""
    rng = random.Random(seed)  # noqa: S311  a seeded scene for timing, not a secret
    star = btvec.Path.star((size[0] / 2, size[1] / 2), 400, 160, points=5)
    items = [star.fill("#ffcc33").stroke("#1a1a1a", width=6)]
    for _ in range(shapes):
        path = outline(rng, *size)
        translucent = rng.random() < TRANSLUCENT_SHARE
        match rng.choice(("fill", "stroke", "both")):
            case "fill":
                item = path.fill(colour(rng, translucent=translucent))
            case "stroke":
                item = path.stroke(colour(rng, translucent=translucent), width=rng.uniform(*STROKE_WIDTHS))
            case _:
                item = path.fill(colour(rng, translucent=translucent)).stroke(
                    colour(rng, translucent=False), width=rng.uniform(*STROKE_WIDTHS)
                )
        if rng.random() < OPACITY_SHARE:
            item = item.opacity(rng.uniform(0.3, 0.9))
        if rng.random() < MULTIPLY_SHARE:
            item = item.blend("multiply")
        items.append(item)
    return btvec.Drawing(*items, size=(size[0], size[1]))


def median_ms(call: Callable[[], object], warmups: int, repeats: int) -> float:
    """The median wall time of `repeats` calls of `call`, in milliseconds, after `warmups` calls that are not timed."""
    for _ in range(warmups):
        call()
    times = []
    for _ in range(repeats):
        start = time.perf_counter()
        call()
        times.append((time.perf_counter() - start) * 1000)
    return statistics.median(times)


def native_times(drawing: btvec.Drawing, size: tuple[int, int], warmups: int, repeats: int) -> dict[str, float]:
    """The median milliseconds of each output through the Python extension."""
    width, height = size
    return {
        "to_rgba": median_ms(lambda: drawing.to_rgba(width, height), warmups, repeats),
        "to_mesh": median_ms(lambda: drawing.to_mesh(width, height), warmups, repeats),
    }


def node_times(document: str, size: tuple[int, int], warmups: int, repeats: int) -> dict[str, float]:
    """The median milliseconds of each output in the WebAssembly package, run in Node on the drawing `document`.

    Raises `BenchError` saying why when Node or the package is missing or the run fails."""
    node = shutil.which("node")
    if node is None:
        raise BenchError("node not found on PATH")
    if not PACKAGE.is_file():
        raise BenchError("the WebAssembly package is not built (run `uv run poe wasm-test`)")
    request = json.dumps({"document": document, "size": size, "warmups": warmups, "repeats": repeats})
    done = subprocess.run(  # noqa: S603  node from PATH, our own script, the request on stdin
        [node, str(NODE_SCRIPT), str(PACKAGE)],
        input=request,
        capture_output=True,
        encoding="utf-8",
        check=False,
    )
    if done.returncode != 0:
        lines = done.stderr.strip().splitlines()
        raise BenchError(f"node failed: {next((line for line in lines if 'Error' in line), 'no message')}")
    return {name: float(done_ms) for name, done_ms in json.loads(done.stdout).items()}


def table(
    shapes: int, size: tuple[int, int], repeats: int, native: dict[str, float], node: dict[str, float] | None
) -> str:
    """The rows to print: one line of what was timed, a header, and a row per output."""
    header = (
        f"reference drawing: a star and {shapes} shapes at {size[0]}x{size[1]}, "
        f"median of {repeats} calls in milliseconds"
    )
    columns = ["output", "native"] + (["WebAssembly", "ratio"] if node else [])
    rows = [columns]
    for output in OUTPUTS:
        row = [output, f"{native[output]:.1f}"]
        if node:
            row += [f"{node[output]:.1f}", f"{node[output] / native[output]:.2f}x"]
        rows.append(row)
    widths = [max(len(row[at]) for row in rows) for at in range(len(columns))]
    lines = [header] + ["  ".join(cell.rjust(width) for cell, width in zip(row, widths, strict=True)) for row in rows]
    return "\n".join(lines)


def parse_size(text: str) -> tuple[int, int]:
    """`(width, height)` of a size written `WxH`."""
    found = re.fullmatch(r"(\d+)x(\d+)", text)
    if found is None:
        raise BenchError(f"--size {text}: write it as WIDTHxHEIGHT in pixels, such as 1920x1080")
    return int(found[1]), int(found[2])


def main(argv: list[str] | None = None) -> int:
    """Run the benchmark named by `argv`; see the module docstring."""
    parser = argparse.ArgumentParser(prog="bench", description=__doc__.splitlines()[0])
    parser.add_argument("--shapes", type=int, default=SHAPES, help=f"random shapes besides the star (default {SHAPES})")
    parser.add_argument("--size", default="1920x1080", help="WIDTHxHEIGHT in pixels (default 1920x1080)")
    parser.add_argument("--repeats", type=int, default=REPEATS, help=f"timed calls (default {REPEATS})")
    parser.add_argument("--warmups", type=int, default=WARMUPS, help=f"calls before timing (default {WARMUPS})")
    parser.add_argument("--require-node", action="store_true", help="exit 1 when the WebAssembly column is missing")
    arguments = parser.parse_args(argv)
    try:
        if min(arguments.shapes, arguments.warmups) < 0 or arguments.repeats < 1:
            raise BenchError("--shapes and --warmups must be 0 or more, --repeats 1 or more")
        size = parse_size(arguments.size)
        drawing = reference_drawing(arguments.shapes, size=size)
        native = native_times(drawing, size, arguments.warmups, arguments.repeats)
    except (BenchError, btvec.errors.BullterrierVectorError) as error:
        print(f"bench: {error}", file=sys.stderr)
        return 1
    node = None
    problem = None
    try:
        node = node_times(drawing.to_json(), size, arguments.warmups, arguments.repeats)
    except BenchError as error:
        problem = f"bench: no WebAssembly column: {error}"
    print(table(arguments.shapes, size, arguments.repeats, native, node))
    if problem:
        print(problem, file=sys.stderr)
    return 1 if problem and arguments.require_node else 0


if __name__ == "__main__":
    sys.exit(main())
