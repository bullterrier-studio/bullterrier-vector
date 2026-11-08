"""Time the outputs on one reference drawing, natively and in WebAssembly, and print a table.

    uv run poe bench [--shapes N] [--size WxH] [--repeats N] [--warmups N] [--require-node]
                     [--text] [--booleans] [--halftone]

The reference drawing is a star and `--shapes` (default 200) filled, stroked, translucent and multiply shapes from a
seeded generator, at 1920x1080 by default. `to_rgba` and `to_mesh` are each called `--warmups` times (default 3) and
then `--repeats` times (default 20); the table shows the median of the timed calls in milliseconds. The native column is
the Python extension (`time.perf_counter`). The WebAssembly column is the package built by `uv run poe wasm-test`, run
in Node (`performance.now`) on the same drawing, loaded from its JSON. Without Node, or without the package in `pkg/`,
the WebAssembly column is left out and one line on stderr says why; `--require-node` makes that exit 1. The numbers are
for a person to read: nothing asserts on time. Results are recorded by hand in docs/benchmarks.md.

`--text` adds the cost of type, natively: a headline of two lines in a variable font (examples/fonts) whose weight is a
Param, timed as `shapes` (shaping and outlines), as `to_rgba` and as `to_mesh` at a new weight on every call, so none is
answered from the cache; as `shapes` with a weight for each letter, for a page of capitals, for the headline fitted to a
width by the width axis and for the page wrapped to a width; and once, a text of 100,000 letters each at its own weight.

`--halftone` adds the cost of a halftone, natively and in WebAssembly: a disc of about 3,000 dots, timed as `to_svg`,
`to_rgba` and `to_mesh` (the first two fill every dot once; the prototypes cut a disc per dot with a boolean, about
500 ms).

`--booleans` adds the cost of the boolean verbs, natively: the `shapes` of a line of capitals with and without `resolve`
and cut out of a plate with `subtract` (a new weight on every call, so the cache answers none), and, on a `Path`, the
`resolve` of a 100 by 100 grid of overlapping squares and the `union` of two such grids.

Interface:
    reference_drawing(shapes=200, seed=1, size=(1920, 1080)) -> Drawing  the drawing that is timed
    median_ms(call, warmups, repeats, clock=time.perf_counter) -> float  the median time of `call` in milliseconds
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
from itertools import count
from pathlib import Path

import bullterrier_vector as btvec

ROOT = Path(__file__).resolve().parent.parent
PACKAGE = ROOT / "pkg" / "bullterrier_vector_wasm.js"
NODE_SCRIPT = Path(__file__).resolve().with_name("bench.mjs")
OUTPUTS = ("to_rgba", "to_mesh")
HALFTONE_OUTPUTS = ("to_svg", *OUTPUTS)
HALFTONE_DISC = 310  # the radius of the disc of the halftone, in design pixels
HALFTONE_PITCH = 10  # the distance between its dots: pi * 310^2 / 10^2 is about 3,000 dots
HALFTONE_SIDE = 800  # the side of its design frame and of its output
FONT = ROOT / "examples" / "fonts" / "anybody-subset.ttf"
HEADLINE = "NO\nSLEEP"
HEADLINE_SIZE = (1080, 1350)
LINE = "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG"
LINE_SIZE = 100  # pixels to the em
GRID = 100  # squares along each side of the grid of overlapping squares
SQUARE, PITCH = 12, 10  # a square's side and the distance between squares: neighbours overlap by 2
GRID_SHIFT = 5  # how far the second grid of the union is moved
PAGE = "\n".join(["THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG 0123456789 " * 2] * 9)  # about a page of capitals
PAGE_LETTERS = len(PAGE)
PAGE_WIDTH = 1000  # pixels: the width the page is wrapped to
DISTINCT_LETTERS = 100_000  # the most letters a text holds (MAX_GLYPHS), each at its own weight
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


def median_ms(
    call: Callable[[], object], warmups: int, repeats: int, clock: Callable[[], float] = time.perf_counter
) -> float:
    """The median time of `repeats` calls of `call`, in milliseconds as `clock` (seconds) reads them, after `warmups`
    calls that are not timed."""
    for _ in range(warmups):
        call()
    times = []
    for _ in range(repeats):
        start = clock()
        call()
        times.append((clock() - start) * 1000)
    return statistics.median(times)


def native_times(
    drawing: btvec.Drawing, size: tuple[int, int], warmups: int, repeats: int, outputs: tuple[str, ...] = OUTPUTS
) -> dict[str, float]:
    """The median milliseconds of each of `outputs` through the Python extension."""
    width, height = size

    def timed(call: Callable[[int, int], object]) -> float:
        return median_ms(lambda: call(width, height), warmups, repeats)

    return {name: timed(getattr(drawing, name)) for name in outputs}


def halftone_drawing() -> btvec.Drawing:
    """A disc of about 3,000 dots on a design frame of `HALFTONE_SIDE` square."""
    middle = HALFTONE_SIDE / 2
    disc = btvec.Path.circle((middle, middle), HALFTONE_DISC)
    return btvec.Drawing(disc.halftone("#1a1a1a", tone=0.5, pitch=HALFTONE_PITCH, angle=45), size=(HALFTONE_SIDE,) * 2)


def node_times(
    document: str, size: tuple[int, int], warmups: int, repeats: int, outputs: tuple[str, ...] = OUTPUTS
) -> dict[str, float]:
    """The median milliseconds of each output in the WebAssembly package, run in Node on the drawing `document`.

    Raises `BenchError` saying why when Node or the package is missing or the run fails."""
    node = shutil.which("node")
    if node is None:
        raise BenchError("node not found on PATH")
    if not PACKAGE.is_file():
        raise BenchError("the WebAssembly package is not built (run `uv run poe wasm-test`)")
    request = json.dumps(
        {"document": document, "size": size, "warmups": warmups, "repeats": repeats, "outputs": list(outputs)}
    )
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
    header: str, native: dict[str, float], node: dict[str, float] | None, outputs: tuple[str, ...] = OUTPUTS
) -> str:
    """The rows to print: the line `header` of what was timed, a header, and a row per output."""
    columns = ["output", "native"] + (["WebAssembly", "ratio"] if node else [])
    rows = [columns]
    for output in outputs:
        row = [output, f"{native[output]:.1f}"]
        if node:
            row += [f"{node[output]:.1f}", f"{node[output] / native[output]:.2f}x"]
        rows.append(row)
    widths = [max(len(row[at]) for row in rows) for at in range(len(columns))]
    lines = [header] + ["  ".join(cell.rjust(width) for cell, width in zip(row, widths, strict=True)) for row in rows]
    return "\n".join(lines)


def text_times(warmups: int, repeats: int) -> dict[str, float]:
    """The median milliseconds of the headline in `FONT` when its weight moves, through the Python extension, and (last)
    the milliseconds of one `shapes` call over `DISTINCT_LETTERS` letters each at its own weight."""
    font = btvec.Font(FONT.read_bytes())
    weight = font.axes["wght"]
    span = weight.max - weight.min
    headline = btvec.text(HEADLINE, font=font, size=800, axes={"wdth": 50, "wght": weight}, leading=640)
    steps = count()

    def moved() -> float:
        """A weight the earlier calls did not use, within the axis."""
        return weight.min + (next(steps) * 0.37) % span

    def each_letter(letters: int) -> list[float]:
        """A weight for each of `letters` letters, none equal to another."""
        return [weight.min + (next(steps) * 0.37) % span for _ in range(letters)]

    page = btvec.text(PAGE, font=font, size=40, axes={"wght": weight})
    fitted = btvec.text(
        HEADLINE, font=font, size=800, axes={"wght": weight}, leading=640, width=HEADLINE_SIZE[0] - 100, fit="wdth"
    )
    wrapped = btvec.text(PAGE, font=font, size=40, axes={"wght": weight}, width=PAGE_WIDTH)
    width, height = HEADLINE_SIZE
    times = {
        "shapes, the weight moves": median_ms(lambda: headline.shapes(wght=moved()), warmups, repeats),
        f"to_rgba {width}x{height}, the weight moves": median_ms(
            lambda: headline.to_rgba(width, height, wght=moved()), warmups, repeats
        ),
        f"to_mesh {width}x{height}, the weight moves": median_ms(
            lambda: headline.to_mesh(width, height, wght=moved()), warmups, repeats
        ),
        "shapes, a weight for each letter": median_ms(
            lambda: headline.shapes(wght=each_letter(len(HEADLINE))), warmups, repeats
        ),
        f"shapes, {PAGE_LETTERS} letters, the weight moves": median_ms(
            lambda: page.shapes(wght=moved()), warmups, repeats
        ),
        "shapes, the headline fitted to a width by wdth, the weight moves": median_ms(
            lambda: fitted.shapes(wght=moved()), warmups, repeats
        ),
        f"shapes, {PAGE_LETTERS} letters wrapped to a width, the weight moves": median_ms(
            lambda: wrapped.shapes(wght=moved()), warmups, repeats
        ),
    }
    long = btvec.text("H" * DISTINCT_LETTERS, font=font, size=12, axes={"wght": weight})
    values = [weight.min + span * i / DISTINCT_LETTERS for i in range(DISTINCT_LETTERS)]
    start = time.perf_counter()
    long.shapes(wght=values)
    times[f"shapes, {DISTINCT_LETTERS} letters each at its own weight (once)"] = (time.perf_counter() - start) * 1000
    return times


def named_table(header: str, times: dict[str, float]) -> str:
    """A line of `header` over a row per timing: what was timed and its milliseconds."""
    width = max(len(name) for name in times)
    return "\n".join([header, *(f"{name.ljust(width)}  {ms:.3f}" for name, ms in times.items())])


def text_table(times: dict[str, float], repeats: int) -> str:
    """The rows of `text_times`: what was timed and its milliseconds."""
    return named_table(f"text: {HEADLINE!r} in a variable font, median of {repeats} calls in milliseconds", times)


def grid_path(shift: float = 0) -> btvec.Path:
    """`GRID` by `GRID` squares of side `SQUARE` every `PITCH`, moved by `shift` on both axes: one path of sub-paths."""
    squares = (
        f"M{x * PITCH + shift} {y * PITCH + shift} h{SQUARE} v{SQUARE} h-{SQUARE} Z"
        for y in range(GRID)
        for x in range(GRID)
    )
    return btvec.Path.from_svg_path(" ".join(squares))


def boolean_times(warmups: int, repeats: int) -> dict[str, float]:
    """The median milliseconds of the boolean verbs through the Python extension: a line of type with a new weight on
    every call, and two grids of overlapping squares."""
    font = btvec.Font(FONT.read_bytes())
    weight = font.axes["wght"]
    span = weight.max - weight.min
    steps = count()

    def moved() -> float:
        """A weight the earlier calls did not use, within the axis."""
        return weight.min + (next(steps) * 0.37) % span

    line = btvec.text(LINE, font=font, size=LINE_SIZE, axes={"wght": weight})
    width = line.shapes(wght=weight.min)[-1].path.bounds
    plate = btvec.Path.rect(-20, -LINE_SIZE, width.x + width.width + 40, LINE_SIZE * 1.4)
    grid, shifted = grid_path(), grid_path(GRID_SHIFT)
    letters = len(line.shapes())
    return {
        f"shapes, a line of {letters} letters, the weight moves": median_ms(
            lambda: line.shapes(wght=moved()), warmups, repeats
        ),
        "shapes, the same line resolved": median_ms(lambda: line.resolve().shapes(wght=moved()), warmups, repeats),
        "shapes, the same line cut out of a plate with subtract": median_ms(
            lambda: btvec.Drawing(plate).subtract(line).shapes(wght=moved()), warmups, repeats
        ),
        f"Path.resolve, {GRID}x{GRID} overlapping squares": median_ms(grid.resolve, warmups, repeats),
        f"Path.union, two {GRID}x{GRID} grids of overlapping squares": median_ms(
            lambda: grid.union(shifted), warmups, repeats
        ),
    }


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
    parser.add_argument("--text", action="store_true", help="also time text when its weight moves (native)")
    parser.add_argument("--booleans", action="store_true", help="also time resolve, subtract and union (native)")
    parser.add_argument("--halftone", action="store_true", help="also time a disc of about 3,000 halftone dots")
    arguments = parser.parse_args(argv)
    try:
        if min(arguments.shapes, arguments.warmups) < 0 or arguments.repeats < 1:
            raise BenchError("--shapes and --warmups must be 0 or more, --repeats 1 or more")
        size = parse_size(arguments.size)
        drawing = reference_drawing(arguments.shapes, size=size)
        native = native_times(drawing, size, arguments.warmups, arguments.repeats)
        typed = text_times(arguments.warmups, arguments.repeats) if arguments.text else None
        cut = boolean_times(arguments.warmups, arguments.repeats) if arguments.booleans else None
        dots = halftone_drawing() if arguments.halftone else None
        dotted = (
            native_times(dots, (HALFTONE_SIDE,) * 2, arguments.warmups, arguments.repeats, HALFTONE_OUTPUTS)
            if dots
            else None
        )
    except (BenchError, btvec.errors.BullterrierVectorError) as error:
        print(f"bench: {error}", file=sys.stderr)
        return 1
    node = None
    problem = None
    try:
        node = node_times(drawing.to_json(), size, arguments.warmups, arguments.repeats)
    except BenchError as error:
        problem = f"bench: no WebAssembly column: {error}"
    header = (
        f"reference drawing: a star and {arguments.shapes} shapes at {size[0]}x{size[1]}, "
        f"median of {arguments.repeats} calls in milliseconds"
    )
    print(table(header, native, node))
    if typed:
        print(text_table(typed, arguments.repeats))
    if cut:
        print(named_table(f"booleans: median of {arguments.repeats} calls in milliseconds", cut))
    if dots and dotted:
        side = (HALFTONE_SIDE,) * 2
        header = (
            f"halftone: a disc of about 3,000 dots at {side[0]}x{side[1]}, "
            f"median of {arguments.repeats} calls in milliseconds"
        )
        try:
            on_node = node_times(dots.to_json(), side, arguments.warmups, arguments.repeats, HALFTONE_OUTPUTS)
        except BenchError:
            on_node = None
        print(table(header, dotted, on_node, HALFTONE_OUTPUTS))
    if problem:
        print(problem, file=sys.stderr)
    return 1 if problem and arguments.require_node else 0


if __name__ == "__main__":
    sys.exit(main())
