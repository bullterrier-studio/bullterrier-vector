"""What the native extension answers, written as JSON for the Node tests of the WebAssembly package.

`poe wasm-test` runs this beside the build: the Node tests (`smoke.test.mjs`) call the WebAssembly package with the same
documents, sizes and values and expect exactly these results. The two are different binaries built from one core, so a
difference is a defect in the glue, the target or the build. Usage: `python tests/wasm/oracle.py <output file>`.
"""

import hashlib
import json
import runpy
import subprocess
import sys
from collections.abc import Callable
from pathlib import Path
from typing import Any

import bullterrier_vector as btvec

ROOT = Path(__file__).resolve().parents[2]
BARS = (ROOT / "docs" / "examples" / "json" / "series_bars.json").read_text(encoding="utf-8")


# The corpus: drawings the native extension and the WebAssembly package are compared on, built through the public API.
# Each entry is a name, a drawing and the draws to compare: (width, height, values). They cover what the outputs do
# differently: every join and cap, miter limits, opacity, multiply, alpha colours, holes and self-overlap, nested
# transforms, Params (a number and a list), no design frame, an empty drawing, a shape with an empty path, outputs of
# one pixel and of odd shapes. They are small so that forty of them stay quick.

Draw = tuple[int, int, dict[str, Any]]
Entry = tuple[str, btvec.Drawing, list[Draw]]

GOLD = "#ffcc33"
INK = "#1a1a1a"
RED = "#e2412e"
BLUE = "#336699"
FRAME = (100, 100)
CORNER = [(15, 85), (50, 15), (85, 85)]  # a sharp corner for the joins and the miter limit
SHARP = [(10, 20), (40, 20), (22, 50), (54, 26)]  # two turns, of 60 and 23 degrees: miter ratios 2 and 5


def star_drawing() -> btvec.Drawing:
    """The `drawing` examples/star.py leaves when run: the star has one definition on the Python side."""
    drawing = runpy.run_path(str(ROOT / "examples" / "star.py"))["drawing"]
    assert isinstance(drawing, btvec.Drawing)
    return drawing


def framed(*items: btvec.Path | btvec.Drawing) -> btvec.Drawing:
    """The items in the 100 by 100 design frame."""
    return btvec.Drawing(*items, size=FRAME)


def lifted() -> btvec.Drawing:
    """Two circles, both moved down by the Param `lift`: a number for both or a list with a number each."""
    lift = btvec.Param("lift", default=5, min=0, max=100, unit="px", meaning="how far the circles sink")
    return btvec.Drawing(
        btvec.Path.circle((30, 30), 10).fill(BLUE),
        btvec.Path.circle((70, 30), 10).fill("#993366"),
        size=(100, 80),
    ).translate(0, lift)


def joins_and_caps() -> list[Entry]:
    """Every join and every cap on a corner, a miter limit that cuts and one that does not."""
    corner = btvec.Path.polyline(CORNER)
    line = btvec.Path.line((20, 50), (80, 50))
    sharp = btvec.Path.polyline(SHARP)
    drawings = {
        **{f"join_{join}": corner.stroke(INK, width=12, join=join) for join in ("miter", "round", "bevel")},
        **{f"cap_{cap}": line.stroke(INK, width=16, cap=cap) for cap in ("butt", "round", "square")},
        **{f"miter_limit_{limit}": sharp.stroke(INK, width=8, join="miter", miter_limit=limit) for limit in (1, 3, 10)},
    }
    return [(name, framed(drawing), [(100, 100, {})]) for name, drawing in drawings.items()]


def shapes() -> list[Entry]:
    """Shapes: polygons, circles, rounded rectangles, arcs from path data, curves."""
    return [
        ("polygon_triangle", framed(btvec.Path.polygon((50, 50), 40, 3).fill(RED)), [(100, 100, {})]),
        (
            "polygon_hexagon",
            framed(btvec.Path.polygon((50, 50), 40, 6).fill(BLUE).stroke(INK, width=3)),
            [(100, 100, {})],
        ),
        ("circle", framed(btvec.Path.circle((50, 50), 40).fill(GOLD)), [(100, 100, {}), (60, 30, {})]),
        ("circle_stroked", framed(btvec.Path.circle((50, 50), 30).stroke(INK, width=10)), [(100, 100, {})]),
        ("rounded_rect", framed(btvec.Path.rect(15, 20, 70, 60, radius=12).fill(RED)), [(100, 100, {})]),
        (
            "rounded_rect_stroked",
            framed(btvec.Path.rect(15, 20, 70, 60, radius=12).fill(GOLD).stroke(INK, width=4)),
            [(100, 100, {})],
        ),
        (
            "arc_from_path_data",
            framed(btvec.Path.from_svg_path("M 10 60 A 40 30 0 0 1 90 60 L 50 90 Z").fill(BLUE).stroke(INK, width=3)),
            [(100, 100, {})],
        ),
        (
            "curves",
            framed(
                btvec.Path()
                .move_to((10, 80))
                .curve_to((10, 10), (90, 10), (90, 80))
                .quad_to((50, 40), (10, 80))
                .close()
                .fill("#66aa44")
            ),
            [(100, 100, {})],
        ),
        (
            "star_five",
            framed(btvec.Path.star((50, 50), 45, 18, points=5).fill(GOLD).stroke(INK, width=2)),
            [(100, 100, {})],
        ),
        ("star_many_points", framed(btvec.Path.star((50, 50), 45, 30, points=24).fill(RED)), [(100, 100, {})]),
        (
            "open_polyline_stroked",
            framed(btvec.Path.polyline([(10, 80), (40, 20), (70, 70), (90, 10)]).stroke(INK, width=5)),
            [(100, 100, {})],
        ),
        ("thin_hairline", framed(btvec.Path.line((10, 10), (90, 70)).stroke(INK, width=0.5)), [(100, 100, {})]),
    ]


def painting() -> list[Entry]:
    """Opacity, multiply, alpha colours, holes, self-overlap."""
    disc = btvec.Path.circle((40, 50), 30)
    other = btvec.Path.circle((62, 50), 30)
    ring = btvec.Path.circle((50, 50), 40).fill(GOLD)
    hole = btvec.Path.from_svg_path("M 10 10 L 90 10 L 90 90 L 10 90 Z M 30 30 L 30 70 L 70 70 L 70 30 Z")
    bowtie = btvec.Path.polyline([(10, 10), (90, 90), (90, 10), (10, 90)], closed=True)
    return [
        ("opacity_half", framed(disc.fill(RED).stroke(INK, width=8).opacity(0.5)), [(100, 100, {})]),
        ("opacity_two_shapes", framed(disc.fill(RED).opacity(0.5), other.fill(BLUE).opacity(0.5)), [(100, 100, {})]),
        ("multiply", framed(disc.fill("#ffaa00"), other.fill("#00aaff").blend("multiply")), [(100, 100, {})]),
        (
            "alpha_colours",
            framed(disc.fill("#ff000080"), other.fill("#0000ff80").stroke("#00000080", width=6)),
            [(100, 100, {})],
        ),
        ("alpha_zero_fill", framed(disc.fill("#ff000000").stroke(INK, width=2)), [(100, 100, {})]),
        ("hole", framed(hole.fill(BLUE)), [(100, 100, {})]),
        ("hole_stroked", framed(hole.fill(BLUE).stroke(INK, width=6)), [(100, 100, {})]),
        ("self_overlap", framed(bowtie.fill(RED).stroke(INK, width=4)), [(100, 100, {})]),
        ("stroke_over_fill_translucent", framed(ring.stroke("#00000080", width=20)), [(100, 100, {})]),
        (
            "stacked_layers",
            framed(disc.fill(GOLD), other.fill(RED).opacity(0.7), disc.stroke(INK, width=3)),
            [(100, 100, {})],
        ),
    ]


def transforms_and_params() -> list[Entry]:
    """Nested transforms, Params with a number and with a list, a drawing with no frame, an empty one."""
    square = btvec.Path.rect(-20, -20, 40, 40)
    nested = (
        btvec.Drawing(square.fill(RED).stroke(INK, width=3), square.scale(0.5).fill(GOLD))
        .rotate(30, about=(0, 0))
        .translate(40, 30)
        .scale(1.5, about=(50, 50))
    )
    param = btvec.Param("size", default=20, min=1, max=60, unit="px", meaning="the radius")
    return [
        ("nested_transforms", framed(nested), [(100, 100, {})]),
        ("lifted_default", lifted(), [(100, 80, {})]),
        ("lifted_number", lifted(), [(100, 80, {"lift": 30}), (200, 160, {"lift": 30})]),
        ("lifted_list", lifted(), [(100, 80, {"lift": [0, 40]})]),
        (
            "param_scale",
            framed(btvec.Path.star((50, 50), 40, 16).fill(GOLD)).scale(param / 20, about=(50, 50)),
            [(100, 100, {"size": 40})],
        ),
        (
            "unsized",
            btvec.Drawing(btvec.Path.rect(5, 5, 50, 30).fill(RED), btvec.Path.circle((40, 30), 12).fill(BLUE)),
            [(80, 60, {})],
        ),
        ("empty", btvec.Drawing(size=FRAME), [(100, 100, {})]),
        ("empty_unsized", btvec.Drawing(), [(8, 8, {})]),
        ("empty_path", framed(btvec.Path().fill(RED), btvec.Path.circle((50, 50), 20).fill(BLUE)), [(100, 100, {})]),
        ("one_pixel", framed(btvec.Path.circle((50, 50), 40).fill(RED)), [(1, 1, {})]),
        ("wide", framed(btvec.Path.circle((50, 50), 40).fill(RED).stroke(INK, width=4)), [(300, 20, {})]),
        ("tall", framed(btvec.Path.star((50, 50), 45, 20).fill(BLUE)), [(20, 200, {})]),
        ("star", star_drawing(), [(400, 400, {}), (200, 200, {}), (800, 300, {})]),
    ]


CORPUS_PARTS: list[Callable[[], list[Entry]]] = [joins_and_caps, shapes, painting, transforms_and_params]


def corpus() -> list[Entry]:
    """Every drawing of the comparison, by unique name."""
    entries = [entry for part in CORPUS_PARTS for entry in part()]
    names = [name for name, _, _ in entries]
    assert len(names) == len(set(names)), "a corpus name is used twice"
    return entries


def parameter(param: btvec.Param) -> dict[str, Any]:
    """A Param as the plain object the package describes it with."""
    return {
        "name": param.name,
        "default": param.default,
        "min": param.min,
        "max": param.max,
        "unit": param.unit,
        "meaning": param.meaning,
    }


def drawn(text: str, width: int, height: int, **values: float | list[float]) -> dict[str, Any]:
    """One `toSvg` call and the text the native extension writes for it."""
    svg = btvec.Drawing.from_json(text).to_svg(width, height, **values)
    return {"width": width, "height": height, "values": values, "svg": svg}


def document(text: str, draws: list[dict[str, Any]]) -> dict[str, Any]:
    """A document, what the native extension reads from it, and its drawings."""
    drawing = btvec.Drawing.from_json(text)
    return {
        "json": text,
        "canonical": drawing.to_json(),
        "size": drawing.size,
        "parameters": [parameter(param) for param in drawing.parameters()],
        "draws": draws,
    }


SNAKE = {"toSvg": "to_svg", "toRgba": "to_rgba", "toPng": "to_png", "toMesh": "to_mesh"}


def refused(call: str, text: str, width: float = 10, height: float = 10, **values: float) -> dict[str, Any]:
    """A call the core refuses: the exception class and message the native extension raises for it."""
    try:
        drawing = btvec.Drawing.from_json(text)
        if call != "fromJson":
            getattr(drawing, SNAKE[call])(width, height, **values)
    except btvec.BullterrierVectorError as error:
        return {
            "call": call,
            "json": text,
            "width": width,
            "height": height,
            "values": values,
            "name": type(error).__name__,
            "message": str(error),
        }
    sys.exit(f"oracle: {call} did not raise for {text[:40]!r}")


def sha256(data: bytes) -> str:
    """The SHA-256 of `data` in hex: Node hashes the WebAssembly side with `node:crypto`, so the two share no code."""
    return hashlib.sha256(data).hexdigest()


def compared(drawing: btvec.Drawing, name: str, index: int, draw: Draw, pixels: Path) -> dict[str, Any]:
    """What the native extension makes of one draw of the corpus: hashes of the exact outputs, pixels in a file."""
    width, height, values = draw
    mesh = drawing.to_mesh(width, height, **values)
    file = f"{name}-{index}.rgba"
    (pixels / file).write_bytes(drawing.to_rgba(width, height, **values))
    return {
        "width": width,
        "height": height,
        "values": values,
        "svg_sha256": sha256(drawing.to_svg(width, height, **values).encode("utf-8")),
        "mesh": {
            "vertices_sha256": sha256(mesh.vertices.tobytes()),
            "indices_sha256": sha256(mesh.indices.tobytes()),
            "ranges_sha256": sha256(mesh.ranges.tobytes()),
            "vertices_length": len(mesh.vertices),
            "indices_length": len(mesh.indices),
            "ranges_length": len(mesh.ranges),
            "generation": mesh.generation,
        },
        "rgba_file": file,
    }


def compared_corpus(pixels: Path) -> list[dict[str, Any]]:
    """The corpus: each drawing as the document `to_json` writes, and the native answers for its draws."""
    pixels.mkdir(parents=True, exist_ok=True)
    return [
        {
            "name": name,
            "json": drawing.to_json(),
            "draws": [compared(drawing, name, index, draw, pixels) for index, draw in enumerate(draws)],
        }
        for name, drawing, draws in corpus()
    ]


def rust_star(output: str) -> bytes:
    """What the Rust example (crates/core/examples/star.rs) writes to standard output for `output`: svg, png or rgba."""
    command = ["cargo", "run", "--quiet", "--locked", "-p", "bullterrier-vector", "--example", "star", "--", output]
    return subprocess.run(command, cwd=ROOT, capture_output=True, check=True).stdout


def star_everywhere(pixels: Path) -> dict[str, Any]:
    """Milestone M1: the star from Python and from the Rust example, as SVG text and as pixels in files."""
    star = star_drawing()
    (pixels / "star-python.rgba").write_bytes(star.to_rgba(400, 400))
    (pixels / "star-rust.rgba").write_bytes(rust_star("rgba"))
    return {
        "python_svg": star.to_svg(400, 400),
        "rust_svg": rust_star("svg").decode("utf-8"),
        "python_rgba_file": "star-python.rgba",
        "rust_rgba_file": "star-rust.rgba",
    }


def deepest_document() -> str:
    """The document of the deepest nesting `to_json` accepts: drawings nested over a leaf that reads a Param."""
    x = btvec.Param("x", default=1, min=0, max=10)
    other = btvec.Path.circle((5, 5), 1).fill("#445566")
    for levels in range(128, 0, -1):
        try:
            drawing = btvec.Path.circle((5, 5), 2).fill("#112233").translate(x, 0)
            for _ in range(levels - 1):
                drawing = btvec.Drawing(drawing, other)
            return drawing.to_json()
        except btvec.BullterrierVectorError:
            continue
    raise AssertionError("not even one drawing can be saved")


def main(output: Path) -> None:
    """Write the answers to `output`, and the native pixels of the corpus to a folder beside it."""
    star_json = star_drawing().to_json()
    lifted_json = lifted().to_json()
    answers = {
        "documents": {
            "series_bars": document(BARS, [drawn(BARS, 1080, 1080), drawn(BARS, 540, 270)]),
            "star": document(star_json, [drawn(star_json, 400, 400), drawn(star_json, 800, 300)]),
            "lifted": document(
                lifted_json,
                [
                    drawn(lifted_json, 100, 80),
                    drawn(lifted_json, 200, 200, lift=30),
                    drawn(lifted_json, 100, 80, lift=[0, 40]),
                ],
            ),
        },
        "deepest_document": deepest_document(),
        "corpus": compared_corpus(output.parent / "oracle-pixels"),
        "star": star_everywhere(output.parent / "oracle-pixels"),
        "refusals": [
            refused("fromJson", "not a document"),
            refused("fromJson", '{"version": 1, "drawings": {"a": [{"nope": 1}]}}'),
            refused("toSvg", star_json, 0, 10),
            refused("toSvg", star_json, 10, 1080.5),
            refused("toSvg", lifted_json, 100, 100, nope=1),
            refused("toSvg", lifted_json, 100, 100, lift=500),
            *[
                refused(call, json_text, width, height, **values)
                for call in ("toRgba", "toPng", "toMesh")
                for json_text, width, height, values in [
                    (star_json, 0, 10, {}),
                    (star_json, 10, 1080.5, {}),
                    (star_json, 65001 if call != "toMesh" else 100001, 10, {}),
                    (lifted_json, 100, 100, {"nope": 1}),
                    (lifted_json, 100, 100, {"lift": 500}),
                ]
            ],
        ],
    }
    output.write_text(json.dumps(answers), encoding="utf-8")


if __name__ == "__main__":
    main(Path(sys.argv[1]))
