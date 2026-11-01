"""What the native extension answers, written as JSON for the Node tests of the WebAssembly package.

`poe wasm-test` runs this beside the build: the Node tests (`smoke.test.mjs`) call the WebAssembly package with the same
documents, sizes and values and expect exactly these results. The two are different binaries built from one core, so a
difference is a defect in the glue, the target or the build. Usage: `python tests/wasm/oracle.py <output file>`.
"""

import base64
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
FONT_FILES = ROOT / "crates" / "core" / "tests" / "fonts"
# The fonts of the text drawings by the name a document calls them (`Font(..., name=)`): the bytes are what a loader
# gives `inputs`, in the oracle file and to the native extension alike.
FONTS = {
    "fraunces": (FONT_FILES / "fraunces-subset.ttf").read_bytes(),
    "synthetic": (FONT_FILES / "synthetic.ttf").read_bytes(),
}
FRAUNCES = btvec.Font(FONTS["fraunces"], name="fraunces")
SYNTHETIC = btvec.Font(FONTS["synthetic"], name="synthetic")


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


def text_drawings() -> list[Entry]:
    """Text: kerning, a ligature, a composite glyph, a variable axis (a number, a Param, a value a character), tracking,
    lines, a stroked outline; and the synthetic font's glyph of 64 nested composites, one of 3999 segments and a
    moved square. The outlines of a font are computed by us (libm only), so the SVG text and the mesh are exact."""
    frame = (400, 160)
    weight = btvec.Param("wght", default=400, min=300, max=700, meaning="the weight")

    def sized(*items: btvec.Drawing) -> btvec.Drawing:
        return btvec.Drawing(*items, size=frame)

    def at_90(content: str, font: btvec.Font = FRAUNCES, **options: Any) -> btvec.Drawing:
        return btvec.text(content, font=font, size=90, anchor=(12, 110), **options)

    drawings = {
        "text_kerned_ligature_composite": sized(at_90("AV fi\u00e9").fill(INK)),
        "text_lightest": sized(at_90("Hif", axes={"wght": 300}).fill(INK)),
        "text_heaviest": sized(at_90("Hif", axes={"wght": 700}).fill(INK)),
        "text_between": sized(at_90("H\u00e9", axes={"wght": 537.5}).fill(RED)),
        "text_liga_off": sized(at_90("fifi", features={"liga": 0, "kern": 0}).fill(INK)),
        "text_param_axis": sized(at_90("HiHi", axes={"wght": weight}).fill(BLUE)),
        "text_lines_tracked": sized(
            btvec.text(
                "AV\nTo", font=FRAUNCES, size=60, tracking=80, leading=62, align="center", anchor=(200, 60)
            ).fill(GOLD)
        ),
        "text_stroked": sized(at_90("Ho").fill(GOLD).stroke(INK, width=3)),
        "text_composites_64_deep": sized(btvec.text("AAA", font=SYNTHETIC, size=100, anchor=(10, 110)).fill(RED)),
        "text_heavy_glyph_and_moved_square": sized(
            btvec.text("BCB", font=SYNTHETIC, size=100, anchor=(10, 110)).fill(BLUE)
        ),
    }
    draws: dict[str, list[Draw]] = {
        "text_param_axis": [(400, 160, {}), (400, 160, {"wght": 700}), (200, 80, {"wght": [300, 700, 300, 700]})],
    }
    return [(name, drawing, draws.get(name, [(400, 160, {}), (200, 80, {})])) for name, drawing in drawings.items()]


CORPUS_PARTS: list[Callable[[], list[Entry]]] = [
    joins_and_caps,
    shapes,
    painting,
    transforms_and_params,
    text_drawings,
]


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
    return refusal(call, text, {}, width, height, values)


def refused_loading(text: str, fonts: dict[str, str]) -> dict[str, Any]:
    """`fromJson` of `text` given these inputs (names to keys of `FONTS`), which the core refuses."""
    return refusal("fromJson", text, fonts, 10, 10, {})


def refusal(
    call: str, text: str, fonts: dict[str, str], width: float, height: float, values: dict[str, float]
) -> dict[str, Any]:
    """What the native extension raises for `call` on `text` loaded with `fonts` (the inputs a loader supplies)."""
    try:
        drawing = btvec.Drawing.from_json(text, inputs={name: FONTS[key] for name, key in fonts.items()})
        if call != "fromJson":
            getattr(drawing, SNAKE[call])(width, height, **values)
    except btvec.BullterrierVectorError as error:
        return {
            "call": call,
            "json": text,
            "fonts": fonts,
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


def inputs_of(text: str) -> dict[str, str]:
    """The inputs a document declares, each as the key of `FONTS` its font is stored under (the same as its name)."""
    return {name: name for name in json.loads(text).get("inputs", {})}


def compared_corpus(pixels: Path) -> list[dict[str, Any]]:
    """The corpus: each drawing as the document `to_json` writes, its inputs, and the native answers for its draws."""
    pixels.mkdir(parents=True, exist_ok=True)
    return [
        {
            "name": name,
            "json": drawing.to_json(),
            "inputs": inputs_of(drawing.to_json()),
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


def main(output: Path) -> None:
    """Write the answers to `output`, and the native pixels of the corpus to a folder beside it."""
    star_json = star_drawing().to_json()
    lifted_json = lifted().to_json()
    text_json = btvec.text("H", font=FRAUNCES).to_json()
    answers = {
        "fonts": {key: base64.b64encode(data).decode("ascii") for key, data in FONTS.items()},
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
        "corpus": compared_corpus(output.parent / "oracle-pixels"),
        "star": star_everywhere(output.parent / "oracle-pixels"),
        "refusals": [
            refused("fromJson", "not a document"),
            refused("fromJson", '{"version": 1, "drawings": {"a": [{"nope": 1}]}}'),
            refused("toSvg", star_json, 0, 10),
            refused("toSvg", star_json, 10, 1080.5),
            refused("toSvg", lifted_json, 100, 100, nope=1),
            refused("toSvg", lifted_json, 100, 100, lift=500),
            refused("fromJson", text_json),  # the font is declared and not supplied
            refused_loading(text_json, {"fraunces": "synthetic"}),  # other bytes than the declared font
            refused_loading(star_json, {"fraunces": "fraunces"}),  # an input nothing declares
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
