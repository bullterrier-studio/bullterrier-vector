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
FONT_FILES = ROOT / "crates" / "core" / "tests" / "it" / "fonts"
# The fonts of the text drawings by the name a document calls them (`Font(..., name=)`): the bytes are what a loader
# gives `inputs`, in the oracle file and to the native extension alike.
FONTS = {
    "fraunces": (FONT_FILES / "fraunces-subset.ttf").read_bytes(),
    "synthetic": (FONT_FILES / "synthetic.ttf").read_bytes(),
}
FRAUNCES = btvec.Font(FONTS["fraunces"], name="fraunces")
SYNTHETIC = btvec.Font(FONTS["synthetic"], name="synthetic")
# The fonts that come with the library are not stored here: a document names them by digest and loads without bytes
BRICOLAGE = btvec.Font.bundled("bricolage_grotesque")
BUNDLED_FRAUNCES = btvec.Font.bundled("fraunces")


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

    # a frame with a hole in it: the hole runs the other way
    holed = btvec.Path.from_svg_path("M10 10H390V150H10Z M150 40V120H250V40Z")

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
        "text_wrapped": sized(
            btvec.text("Hi To Hif", font=FRAUNCES, size=44, width=110, leading=48, anchor=(12, 44)).fill(INK)
        ),
        "text_fit_size": sized(at_90("Hif", width=130, fit="size").fill(RED)),
        "text_fit_axis": sized(at_90("Hif", width=140, fit="wght").fill(BLUE)),
        "text_inside_circle": sized(
            btvec.text(
                "Hif To Hif Hi AV", font=FRAUNCES, size=22, align="center", inside=btvec.Path.circle((200, 80), 76)
            ).fill(INK)
        ),
        "text_inside_hole": sized(
            btvec.text("Hif To Hif Hi AV To", font=FRAUNCES, size=20, leading=26, inside=holed).fill(BLUE)
        ),
        "text_composites_64_deep": sized(btvec.text("AAA", font=SYNTHETIC, size=100, anchor=(10, 110)).fill(RED)),
        "text_heavy_glyph_and_moved_square": sized(
            btvec.text("BCB", font=SYNTHETIC, size=100, anchor=(10, 110)).fill(BLUE)
        ),
        "text_bundled_bricolage": sized(
            at_90("Hamburg", font=BRICOLAGE, axes={"wght": 500, "wdth": 80, "opsz": 40}).fill(INK)
        ),
        "text_bundled_fraunces": sized(
            at_90("Hamburg fi", font=BUNDLED_FRAUNCES, axes={"wght": 600, "SOFT": 100, "WONK": 0, "opsz": 72}).fill(RED)
        ),
    }
    draws: dict[str, list[Draw]] = {
        "text_param_axis": [(400, 160, {}), (400, 160, {"wght": 700}), (200, 80, {"wght": [300, 700, 300, 700]})],
    }
    return [(name, drawing, draws.get(name, [(400, 160, {}), (200, 80, {})])) for name, drawing in drawings.items()]


def bends() -> list[Entry]:
    """`along`: a text bent round a closed circle (a Param turns it, a list moves each of its ten glyphs) and an outline
    bent along an open path, past both of its ends. A bend computes with `sqrt` only, so the SVG text and the mesh are
    exact."""
    frame = (400, 400)
    turn = btvec.Param("turn", default=0, min=-1, max=1, meaning="how far round the circle")
    circle = btvec.Path.circle((200, 200), 120)
    type_ = btvec.text("Hif To Hif AV", font=FRAUNCES, size=44, anchor=(0, 0)).fill(INK)
    open_path = btvec.Path.from_svg_path("M20 300 C 120 40, 280 40, 380 300")
    bar = btvec.Path.rect(-60, -8, 180, 16).fill(RED)
    return [
        (
            "along_text_circle",
            btvec.Drawing(type_.along(circle, start=turn), size=frame),
            [
                (400, 400, {}),
                (200, 200, {"turn": 0.3}),
                (400, 400, {"turn": [0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9]}),
            ],
        ),
        (
            "along_open_path",
            btvec.Drawing(bar.along(open_path, start=0.9, near=(0, 0)), size=frame),
            [(400, 400, {}), (200, 200, {})],
        ),
    ]


def per_item() -> list[Entry]:
    """Per-item numbers and the list steps on text: `random`, `stagger`, `by_index` and `by_order` in transforms, a
    transform about the default origin, `split`, `merge` (all, and by word) and `slice` (alone and staggered). The
    seeded hash and the slicing compute with integers and `sqrt` only, so the SVG text and the mesh are exact."""
    frame = (400, 160)
    amount = btvec.Param("amount", default=1, min=0, max=1, unit="fraction", meaning="how far the letters have gone")
    progress = btvec.Param("progress", default=0.5, min=0, max=1, unit="fraction", meaning="how much is drawn")

    def at_90(content: str) -> btvec.Drawing:
        return btvec.text(content, font=FRAUNCES, size=90, anchor=(12, 110))

    def sized(*items: btvec.Drawing) -> btvec.Drawing:
        return btvec.Drawing(*items, size=frame)

    def draws(**values: float) -> list[Draw]:
        return [(400, 160, {}), (200, 80, values)]

    drawings = {
        "item_random": sized(
            at_90("Hif To")
            .rotate(btvec.random(-20, 20, seed=4) * amount)
            .translate(btvec.random(-15, 15, seed=5) * amount, btvec.random(-10, 10, seed=6) * amount)
            .fill(INK)
        ),
        "item_stagger": sized(at_90("Hif To").translate(0, btvec.stagger(amount, spread=0.6) * -30).fill(RED)),
        "item_by_index_by_order": sized(
            at_90("Hif To").rotate(btvec.by_index(-15, 15)).translate(btvec.by_order(0, 30), 0).fill(BLUE)
        ),
        "item_default_origin": sized(at_90("Hif To").rotate(12).scale(0.8).fill(GOLD)),
        "item_split": sized(at_90("Hi fi").split().translate(btvec.random(-12, 12, seed=7) * amount, 0).fill(INK)),
        "item_merge_all": sized(at_90("Hif To").merge().fill(RED).stroke(INK, width=2)),
        "item_merge_by_word": sized(at_90("Hif To AV").merge(by="word").rotate(btvec.by_index(-6, 6)).fill(BLUE)),
        "item_slice": sized(at_90("Hif").slice(0.2, 0.7).stroke(INK, width=3)),
        "item_slice_stagger": sized(at_90("Hif To").slice(0, btvec.stagger(progress, spread=0.6)).stroke(INK, width=3)),
    }
    # the second draw sets the Params the drawing reads (a call naming one it does not read is refused)
    values = {
        **{name: {"amount": 0.4} for name in ["item_random", "item_stagger", "item_split"]},
        "item_slice_stagger": {"progress": 0.8},
    }
    return [(name, drawing, draws(**values.get(name, {}))) for name, drawing in drawings.items()]


def booleans() -> list[Entry]:
    """`union`, `subtract`, `intersect`, `exclude` and `resolve` on shapes: overlapping squares and circles, a ring, a
    shape with a hole, an intersection that is empty, a Param moving `other`, two tolerances and glyph outlines resolved
    and combined. The engine works on an integer grid and the flattening computes with `libm` only, so the SVG text and
    the mesh are exact."""
    square = btvec.Path.rect(20, 20, 40, 40)
    shifted = btvec.Path.rect(40, 40, 40, 40)
    disc = btvec.Path.circle((50, 50), 35)
    hole = btvec.Path.circle((50, 50), 15)
    bowtie = btvec.Path.from_svg_path("M15 15 L85 85 L85 15 L15 85 Z")
    offset = btvec.Param("offset", default=0, min=-50, max=50, unit="px", meaning="how far the cutter is moved")
    frame = (400, 160)

    def sized(*items: btvec.Drawing) -> btvec.Drawing:
        return btvec.Drawing(*items, size=frame)

    def at_90(content: str, font: btvec.Font = FRAUNCES, **options: Any) -> btvec.Drawing:
        return btvec.text(content, font=font, size=90, anchor=(12, 110), **options)

    drawings = {
        "bool_union": framed(square.fill(RED)).union(shifted),
        "bool_subtract": framed(square.fill(BLUE)).subtract(shifted),
        "bool_intersect": framed(square.fill(GOLD)).intersect(shifted),
        "bool_exclude": framed(square.fill(RED).stroke(INK, width=2)).exclude(shifted),
        "bool_ring": framed(disc.fill(INK)).subtract(hole),
        "bool_circles_exclude_many": framed(disc.fill(BLUE), square.fill(RED)).exclude(
            btvec.Drawing(hole, hole.translate(30, 0), shifted)
        ),
        "bool_intersect_empty": framed(square.fill(RED), disc.fill(BLUE).translate(200, 0)).intersect(
            hole.translate(0, 50)
        ),
        "bool_param_other": framed(disc.fill(GOLD)).subtract(btvec.Drawing(hole).translate(offset, 0)),
        "bool_tolerance_coarse": framed(disc.fill(RED)).union(hole.translate(20, 0), tolerance=2),
        "bool_tolerance_fine": framed(disc.fill(RED)).union(hole.translate(20, 0), tolerance=0.001),
        "bool_resolve_bowtie": framed(bowtie.fill(BLUE).stroke(INK, width=3)).resolve(),
        "bool_resolve_hole_union": framed(
            btvec.Path.from_svg_path("M5 5 L95 5 L95 95 L5 95 Z M30 30 L30 70 L70 70 L70 30 Z").fill(RED)
        ).union(hole),
        "bool_text_resolved": sized(at_90("Hif").fill(INK).resolve()),
        "bool_text_cut_by_circle": sized(at_90("Hamburg", font=BRICOLAGE, axes={"wght": 700}).fill(RED)).subtract(
            btvec.Path.circle((120, 60), 40)
        ),
        "bool_text_clipped": sized(at_90("Hif To").fill(BLUE)).intersect(btvec.Path.circle((200, 80), 70)),
        "bool_text_cut_by_text": sized(at_90("HH").fill(INK)).subtract(at_90("i").translate(4, 0)),
    }
    draws: dict[str, list[Draw]] = {
        "bool_param_other": [(100, 100, {}), (200, 200, {"offset": 25})],
    }
    return [
        (
            name,
            drawing,
            draws.get(
                name,
                [(400, 160, {}), (200, 80, {})] if name.startswith("bool_text") else [(100, 100, {}), (50, 50, {})],
            ),
        )
        for name, drawing in drawings.items()
    ]


def offsets() -> list[Entry]:
    """`offset` and `outline` on shapes: each join and cap, a sharp corner at two miter limits, an inset, a ring that
    loses width, a distance that collapses a shape, a coarse tolerance, a per-item list, a Param and text grown and
    shrunk. The engine works on an integer grid and the arcs come from `libm` only, so the SVG text and the mesh are
    exact."""
    square = btvec.Path.rect(30, 30, 40, 40)
    ring = btvec.Path.circle((50, 50), 35).subtract(btvec.Path.circle((50, 50), 15))
    sharp = btvec.Path.from_svg_path("M15 80 L50 15 L85 80 Z")
    line = btvec.Path.from_svg_path("M20 70 L50 25 L80 70")
    grow = btvec.Param("grow", default=0, min=-20, max=20, unit="px", meaning="how far the outline is moved")
    frame = (400, 160)

    def sized(*items: btvec.Drawing) -> btvec.Drawing:
        return btvec.Drawing(*items, size=frame)

    def at_90(content: str, font: btvec.Font = FRAUNCES, **options: Any) -> btvec.Drawing:
        return btvec.text(content, font=font, size=90, anchor=(12, 110), **options)

    drawings = {
        "offset_round": framed(square.fill(RED)).offset(8),
        "offset_miter": framed(sharp.fill(BLUE)).offset(8, join="miter"),
        "offset_miter_limit_low": framed(sharp.fill(BLUE)).offset(8, join="miter", miter_limit=1.2),
        "offset_bevel": framed(sharp.fill(GOLD)).offset(8, join="bevel"),
        "offset_inset": framed(sharp.fill(RED).stroke(INK, width=2)).offset(-6, join="miter"),
        "offset_inset_collapses": framed(square.fill(RED), ring.fill(BLUE)).offset(-25),
        "offset_ring_shrinks": framed(ring.fill(INK)).offset(-6),
        "offset_tolerance_coarse": framed(btvec.Path.circle((50, 50), 30).fill(GOLD)).offset(10, tolerance=1),
        "offset_per_item": framed(btvec.Drawing(square, square.scale(0.5), ring.scale(0.4)).fill(BLUE)).offset(
            btvec.by_index(-4, 8), join="miter"
        ),
        "offset_param": framed(sharp.fill(RED)).offset(grow),
        "outline_round": framed(line.stroke(INK, width=2)).outline(14),
        "outline_butt_miter": framed(line.fill(RED)).outline(14, join="miter", cap="butt"),
        "outline_square_bevel": framed(line.fill(BLUE)).outline(14, join="bevel", cap="square"),
        "offset_text_grown": sized(at_90("Hif").fill(INK).offset(3)),
        "offset_text_shrunk": sized(at_90("Hif To", font=BRICOLAGE, axes={"wght": 800}).fill(RED).offset(-2)),
        "outline_text": sized(at_90("Hif", font=BRICOLAGE).fill(BLUE).outline(2, join="miter")),
    }
    draws: dict[str, list[Draw]] = {"offset_param": [(100, 100, {}), (200, 200, {"grow": 7})]}
    return [
        (
            name,
            drawing,
            draws.get(name, [(400, 160, {}), (200, 80, {})] if "text" in name else [(100, 100, {}), (50, 50, {})]),
        )
        for name, drawing in drawings.items()
    ]


# The drawings whose mesh the core refuses (a blurred shadow has no triangle form): the comparison is that WebAssembly
# refuses with the native error.
MESH_REFUSED = {"shadow_blurred", "shadow_blurred_group", "shadow_blur_param", "knockout_blurred_shadow_below"}


def shadows() -> list[Entry]:
    """Shadows: a hard copy (the shape moved by an offset between pixels, a stroke, a translucent tint), a blur, a blur
    under opacity and multiply, an offset per shape and a Param blur. The copy is the shape moved, so the SVG text and
    the mesh are exact; the blur is `vello_cpu`'s filter, drawn alike natively and in WebAssembly, and the mesh of a
    blur is refused."""
    disc = btvec.Path.circle((45, 50), 25)
    square = btvec.Path.rect(55, 25, 30, 30)
    blur = btvec.Param("blur", default=10, min=0, max=60, unit="px", meaning="the blur radius of the shadows")
    drawings = {
        "shadow_hard": framed(disc.fill(RED).shadow(dx=6.5, dy=9.25, blur=0, color="#00000066")),
        "shadow_hard_stroked": framed(
            square.fill(GOLD).stroke(INK, width=5).shadow(dx=-8, dy=10, blur=0, color="#1a1a1ab3"),
            disc.stroke(BLUE, width=6).shadow(dx=5, dy=5, blur=0),
        ),
        "shadow_blurred": framed(disc.fill(RED).shadow(dy=6, blur=14), square.fill(BLUE).shadow(dx=4, dy=8, blur=6)),
        "shadow_blurred_group": framed(
            disc.fill(GOLD),
            disc.translate(22, 10).fill("#ff4f9a").stroke(INK, width=3).shadow(dx=6, dy=6, blur=10).opacity(0.7),
            square.translate(-20, 30).fill("#4fa0ff").shadow(dx=6, dy=6, blur=10, color="#202080c0").blend("multiply"),
        ),
        "shadow_per_item": framed(
            btvec.Drawing(
                disc.scale(0.4, about=(20, 25)), disc.scale(0.4, about=(60, 25)), disc.scale(0.4, about=(80, 70))
            )
            .fill(RED)
            .shadow(dy=btvec.by_index(0, 16), blur=0, color="#000000")
        ),
        "shadow_blur_param": framed(disc.fill(BLUE).shadow(blur=blur)),
    }
    draws: dict[str, list[Draw]] = {
        "shadow_blur_param": [(100, 100, {"blur": 4}), (200, 200, {"blur": 30})],
        "shadow_hard": [(100, 100, {}), (200, 200, {})],
        "shadow_blurred": [(100, 100, {}), (200, 200, {})],
    }
    return [(name, drawing, draws.get(name, [(100, 100, {})])) for name, drawing in drawings.items()]


def knockouts() -> list[Entry]:
    """Knockouts: a hole, a stroke only, a stroke and fill with a shape after, a hard shadow cut, a blurred shadow cut
    (its mesh refused as a blurred shadow's is), two knockouts with a shape between them, glyphs cut by a disc and a
    hole moved by a Param. The mask is SVG text, so it is exact; the cut of the mesh is the engine's on an integer
    grid, so the mesh buffers are exact; the pixels are `vello_cpu`'s, within the tolerance."""
    paper = btvec.Path.rect(5, 5, 90, 90)
    disc = btvec.Path.circle((50, 50), 22)
    square = btvec.Path.rect(30, 30, 40, 40)
    shift = btvec.Param("shift", default=0, min=-30, max=30, unit="px", meaning="how far the hole moves right")
    drawings = {
        "knockout_hole": framed(paper.fill(RED), disc.fill(INK).knockout()),
        "knockout_stroke_only": framed(
            paper.fill(BLUE), square.fill("none").stroke(INK, width=9, join="miter").knockout()
        ),
        "knockout_fill_stroke_then_shape": framed(
            paper.fill(GOLD).stroke(INK, width=4),
            disc.translate(-12, 0).fill(INK).stroke(INK, width=7).knockout(),
            disc.translate(14, 0).scale(0.6).fill("#ff4f9a"),
        ),
        "knockout_hard_shadow_below": framed(
            square.translate(-10, -10).fill(RED).shadow(dx=18, dy=22, blur=0, color="#000000"),
            disc.translate(18, 18).fill(INK).knockout(),
        ),
        "knockout_blurred_shadow_below": framed(
            square.translate(-10, -10).fill(RED).shadow(dx=10, dy=12, blur=12),
            disc.translate(18, 18).fill(INK).knockout(),
        ),
        "knockout_two_with_a_shape_between": framed(
            paper.fill(RED),
            btvec.Path.rect(15, 15, 30, 30).fill(INK).knockout(),
            btvec.Path.rect(0, 0, 100, 50).fill(BLUE),
            btvec.Path.rect(35, 35, 30, 30).fill(INK).knockout(),
        ),
        "knockout_glyphs": btvec.Drawing(
            btvec.text("Hif", font=FRAUNCES, size=90, anchor=(12, 110)).fill(INK),
            btvec.Path.circle((60, 70), 26).fill(RED).knockout(),
            size=(200, 140),
        ),
        "knockout_param": framed(
            paper.fill(GOLD), btvec.Drawing(disc).translate(shift, 0).scale(0.7).fill(INK).knockout()
        ),
    }
    draws: dict[str, list[Draw]] = {
        "knockout_param": [(100, 100, {}), (200, 200, {"shift": 18})],
        "knockout_glyphs": [(200, 140, {}), (100, 70, {})],
    }
    return [(name, drawing, draws.get(name, [(100, 100, {}), (50, 50, {})])) for name, drawing in drawings.items()]


def halftones() -> list[Entry]:
    """Halftones: a disc and a ring (a hole) on a turned lattice, a stroke over dots under opacity, a translucent
    colour on overlapping dots, a hard shadow of dots, a knockout of dots, a tone per shape and a Param tone, tone 0 and
    tone 1.
    The dots are computed in design pixels by the one core, so the SVG text and the mesh are exact; the pixels are the
    rasteriser's, as for any shape."""
    disc = btvec.Path.circle((50, 50), 38)
    ring = btvec.Path.from_svg_path("M10 10 L90 10 L90 90 L10 90 Z M30 30 L30 70 L70 70 L70 30 Z")
    tone = btvec.Param("tone", default=0.5, min=0, max=1, unit="", meaning="the share of each cell the dots leave bare")
    drawings = {
        "halftone_disc": framed(disc.halftone(INK, tone=0.5, pitch=6.5, angle=30)),
        "halftone_ring": framed(ring.halftone(RED, tone=0.35, pitch=7, angle=-12.5)),
        "halftone_stroked": framed(disc.halftone(BLUE, tone=0.4, pitch=8).stroke(INK, width=3).opacity(0.6)),
        "halftone_translucent_overlap": framed(disc.halftone("#00008080", tone=0.08, pitch=12, angle=0)),
        "halftone_shadow": framed(
            btvec.Path.circle((42, 44), 30)
            .halftone(RED, tone=0.5, pitch=8)
            .shadow(dx=9, dy=12, blur=0, color="#0000ff99")
        ),
        "halftone_knockout": framed(
            btvec.Path.rect(5, 5, 90, 90).fill(GOLD),
            btvec.Path.rect(5, 5, 90, 90).halftone(INK, tone=0.5, pitch=16).knockout(),
        ),
        "halftone_per_item": framed(
            btvec.Drawing(
                disc.scale(0.4, about=(20, 25)), disc.scale(0.4, about=(60, 25)), disc.scale(0.4, about=(80, 70))
            ).halftone(INK, tone=btvec.by_index(0.15, 0.85), pitch=btvec.by_index(4, 6))
        ),
        "halftone_param": framed(disc.halftone(BLUE, tone=tone, pitch=6)),
        "halftone_solid": framed(disc.halftone(RED, tone=0)),
        "halftone_bare": framed(disc.halftone(RED, tone=1).stroke(INK, width=2)),
    }
    draws: dict[str, list[Draw]] = {
        "halftone_param": [(100, 100, {"tone": 0.3}), (200, 200, {"tone": 0.8})],
        "halftone_disc": [(100, 100, {}), (200, 200, {})],
    }
    return [(name, drawing, draws.get(name, [(100, 100, {})])) for name, drawing in drawings.items()]


CORPUS_PARTS: list[Callable[[], list[Entry]]] = [
    joins_and_caps,
    shapes,
    painting,
    transforms_and_params,
    text_drawings,
    bends,
    per_item,
    booleans,
    offsets,
    shadows,
    knockouts,
    halftones,
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
    file = f"{name}-{index}.rgba"
    (pixels / file).write_bytes(drawing.to_rgba(width, height, **values))
    return {
        "width": width,
        "height": height,
        "values": values,
        "svg_sha256": sha256(drawing.to_svg(width, height, **values).encode("utf-8")),
        "mesh": refused_mesh(drawing, draw)
        if name in MESH_REFUSED
        else described(drawing.to_mesh(width, height, **values)),
        "rgba_file": file,
    }


def described(mesh: btvec.Mesh) -> dict[str, Any]:
    """A mesh as the hashes and lengths of its buffers."""
    return {
        "vertices_sha256": sha256(mesh.vertices.tobytes()),
        "indices_sha256": sha256(mesh.indices.tobytes()),
        "ranges_sha256": sha256(mesh.ranges.tobytes()),
        "vertices_length": len(mesh.vertices),
        "indices_length": len(mesh.indices),
        "ranges_length": len(mesh.ranges),
        "generation": mesh.generation,
    }


def refused_mesh(drawing: btvec.Drawing, draw: Draw) -> dict[str, Any]:
    """The refusal `to_mesh` raises for a drawing of `MESH_REFUSED`: the exception class and message."""
    width, height, values = draw
    try:
        drawing.to_mesh(width, height, **values)
    except btvec.BullterrierVectorError as error:
        return {"refusal": {"name": type(error).__name__, "message": str(error)}}
    sys.exit("oracle: to_mesh did not raise for a drawing of MESH_REFUSED")


def inputs_of(text: str) -> dict[str, str]:
    """The inputs a document declares that a loader supplies, each as the key of `FONTS` its font is stored under (the
    same as its name). A bundled font is declared too but loads with no bytes, so a declared input is supplied only when
    its digest is that of the stored font (the bundled `fraunces` shares its name with the stored subset)."""
    declared = json.loads(text).get("inputs", {})
    return {
        name: name
        for name, spec in declared.items()
        if name in FONTS and spec["sha256"] == hashlib.sha256(FONTS[name]).hexdigest()
    }


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
