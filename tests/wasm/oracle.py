"""What the native extension answers, written as JSON for the Node tests of the WebAssembly package.

`poe wasm-test` runs this beside the build: the Node tests (`smoke.test.mjs`) call the WebAssembly package with the same
documents, sizes and values and expect exactly these results. The two are different binaries built from one core, so a
difference is a defect in the glue, the target or the build. Usage: `python tests/wasm/oracle.py <output file>`.
"""

import base64
import hashlib
import json
import math
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
    # a map is an input too: 16 by 16 grayscale bytes, bright on the left, with a dark spot at the lower right
    "tone_map": bytes(
        max(0, min(255, 40 + 14 * column - (180 if (column - 11) ** 2 + (row - 11) ** 2 < 12 else 0)))
        for row in range(16)
        for column in range(16)
    ),
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
TAPER = [(0, 0), (0.15, 1), (0.85, 1), (1, 0)]
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
        ("blend_screen", framed(disc.fill("#ffaa00"), other.fill("#00aaff").blend("screen")), [(100, 100, {})]),
        (
            "blend_soft_light_opacity",
            framed(disc.fill("#ffaa00"), other.fill("#40a0ff").opacity(0.7).blend("soft-light")),
            [(100, 100, {})],
        ),
        (
            "blend_color_dodge",
            framed(disc.fill("#806040"), other.fill("#80a0c0").blend("color-dodge")),
            [(100, 100, {})],
        ),
        (
            "blend_difference_shadow",
            framed(
                disc.fill("#ffaa00"),
                other.fill("#00aaff").shadow(dx=6, dy=6, blur=0, color="#202080c0").blend("difference"),
            ),
            [(100, 100, {})],
        ),
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
        # runs: three fonts, sizes and axes in one line, wrapped, fitted, set inside a shape, an axis read from a Param
        "text_runs_mixed_wrapped": sized(
            btvec.text(
                [
                    btvec.run("AV fi", font=FRAUNCES, size=60, axes={"wght": 700}),
                    btvec.run("Hamb", font=BRICOLAGE, size=44, axes={"wght": 800, "wdth": 75}),
                    btvec.run(" CB", font=SYNTHETIC, size=50),
                ],
                width=300,
                leading=56,
                align="center",
                anchor=(200, 56),
            ).fill(INK)
        ),
        "text_runs_fit_size": sized(
            btvec.text(
                [btvec.run("Hi", font=FRAUNCES, size=40), btvec.run("Ham", font=BRICOLAGE, size=70)],
                width=380,
                fit="size",
                anchor=(10, 70),
            ).fill(RED)
        ),
        "text_runs_fit_axis": sized(
            btvec.text(
                [
                    btvec.run("Hif", font=FRAUNCES),
                    btvec.run("CB", font=SYNTHETIC, size=60),
                    btvec.run("Ho", font=BRICOLAGE),
                ],
                size=90,
                width=300,
                fit="wght",
                anchor=(10, 110),
            ).fill(BLUE)
        ),
        "text_runs_inside_circle": sized(
            btvec.text(
                [btvec.run("Hif To Hif", font=FRAUNCES, size=22), btvec.run(" Hi AV", font=BRICOLAGE, size=26)],
                align="center",
                inside=btvec.Path.circle((200, 80), 76),
            ).fill(INK)
        ),
        "text_runs_param_axis": sized(
            btvec.text(
                [
                    btvec.run("HiHi", font=FRAUNCES, axes={"wght": weight}),
                    btvec.run("fi", font=BRICOLAGE, tracking=40, axes={"wght": weight + 100}),
                ],
                size=80,
                anchor=(12, 110),
            ).fill(GOLD)
        ),
    }
    draws: dict[str, list[Draw]] = {
        "text_param_axis": [(400, 160, {}), (400, 160, {"wght": 700}), (200, 80, {"wght": [300, 700, 300, 700]})],
        "text_runs_param_axis": [(400, 160, {}), (400, 160, {"wght": 700}), (200, 80, {"wght": 450})],
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
    peak = btvec.Param("peak", default=1, min=0, max=1, unit="fraction", meaning="the width at the end of the stroke")
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
        "outline_taper_open": framed(btvec.Path.from_svg_path("M10 50 C 30 10 70 90 90 50").fill(INK)).outline(
            14, profile=[(0, 0), (0.15, 1), (0.85, 1), (1, 0)]
        ),
        "outline_taper_closed": framed(btvec.Path.circle((50, 50), 30).fill(RED)).outline(
            10, profile=[(0, 0.2), (0.5, 1), (1, 0.2)]
        ),
        "outline_taper_dashed": framed(btvec.Path.line((10, 50), (90, 50)).fill(BLUE))
        .dash([30, 10])
        .outline(12, profile=[(0, 0), (0.5, 1), (1, 0)]),
        "outline_taper_param": framed(btvec.Path.line((10, 50), (90, 50)).fill(INK)).outline(
            10, profile=[(0, 0), (1, peak)]
        ),
        "outline_text": sized(at_90("Hif", font=BRICOLAGE).fill(BLUE).outline(2, join="miter")),
        "stroke_taper_alone": framed(
            btvec.Path.from_svg_path("M10 50 C 30 10 70 90 90 50").stroke(INK, width=13, profile=TAPER)
        ),
        "stroke_taper_with_fill": framed(
            btvec.Path.circle((50, 50), 30).fill(GOLD).stroke(INK, width=12, profile=[(0, 0.2), (0.5, 1), (1, 0.2)])
        ),
        "stroke_taper_shadow_blend": framed(
            btvec.Path.line((10, 50), (90, 50))
            .fill(RED)
            .stroke("#2040c080", width=16, profile=[(0, 0), (0.5, 1), (1, 0)])
            .shadow(dx=5, dy=5, blur=0, color="#00000080")
            .blend("multiply")
        ),
        "stroke_taper_sliced": framed(
            btvec.Path.line((10, 50), (90, 50)).stroke(INK, width=16, profile=TAPER).slice(0, 0.6)
        ),
        "stroke_taper_param": framed(
            btvec.Path.line((10, 50), (90, 50)).stroke(BLUE, width=12, profile=[(0, 0), (1, peak)])
        ),
    }
    draws: dict[str, list[Draw]] = {
        "offset_param": [(100, 100, {}), (200, 200, {"grow": 7})],
        "outline_taper_param": [(100, 100, {}), (200, 200, {"peak": 0.4})],
        "stroke_taper_param": [(100, 100, {}), (200, 200, {"peak": 0.4})],
    }
    return [
        (
            name,
            drawing,
            draws.get(name, [(400, 160, {}), (200, 80, {})] if "text" in name else [(100, 100, {}), (50, 50, {})]),
        )
        for name, drawing in drawings.items()
    ]


# The drawings whose mesh the core refuses (a blurred shadow, a gradient and a blend beyond multiply have no triangle
# form): the comparison is that WebAssembly refuses with the native error.
MESH_REFUSED = {
    "blend_screen",
    "blend_soft_light_opacity",
    "blend_color_dodge",
    "blend_difference_shadow",
    "shadow_blurred",
    "shadow_blurred_group",
    "shadow_blur_param",
    "knockout_blurred_shadow_below",
    "gradient_linear_fill",
    "gradient_radial_stroke",
    "gradient_translucent_stops",
    "gradient_stroke_dots",
    "gradient_hard_shadow",
    "gradient_blurred_shadow",
    "gradient_framed",
    "clip_disc",
    "clip_shadows",
    "clip_two_shapes",
    "mask_flat",
    "mask_gradient",
    "mask_translucent_shadow",
    "clip_and_mask_framed",
}


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


def gradients() -> list[Entry]:
    """Gradient paints: a linear fill, a radial stroke, stops of different alpha (interpolated unpremultiplied), a
    stroke whose zero-length pieces overlap its line, a hard and a blurred shadow that follow the paint's alpha, a
    frame at two sizes, and a knockout that ignores its gradient (its mesh is made). The geometry goes through the one
    `View`, so the SVG text is exact; the pixels are the rasteriser's; a gradient has no mesh."""
    disc = btvec.Path.circle((50, 50), 38)
    fade = btvec.linear_gradient([(0, RED), (0.6, GOLD), (1, BLUE)], start=(10, 10), end=(90, 80))
    clear = btvec.linear_gradient([(0, "#e2412e00"), (1, "#336699ff")], start=(20, 0), end=(80, 0))
    glow = btvec.radial_gradient([(0, "#ffffff"), (1, "#00000000")], center=(50, 50), radius=40)
    ring = btvec.radial_gradient([(0, "#00aa44c0"), (1, "#aa0044c0")], center=(50, 50), radius=45)
    dotted = (
        btvec.Path.from_svg_path("M15 50 L85 50")
        .move_to((50, 50))
        .line_to((50, 50))
        .move_to((30, 22))
        .line_to((30, 22))
    )
    drawings = {
        "gradient_linear_fill": framed(btvec.Path.rect(10, 10, 80, 80).fill(fade)),
        "gradient_radial_stroke": framed(disc.fill(glow).stroke(ring, width=9)),
        "gradient_translucent_stops": framed(btvec.Path.rect(10, 10, 80, 80).fill(clear)),
        "gradient_stroke_dots": framed(dotted.stroke(ring, width=14, cap="round")),
        "gradient_hard_shadow": framed(
            btvec.Path.rect(20, 20, 50, 50).fill(clear).shadow(dx=10, dy=12, blur=0, color="#000000a0")
        ),
        "gradient_blurred_shadow": framed(
            btvec.Path.rect(20, 20, 50, 50).fill(clear).shadow(dx=8, dy=10, blur=14, color="#000000d0")
        ),
        "gradient_framed": framed(btvec.Path.rect(0, 0, 100, 100).fill(fade)),
        "gradient_knockout": framed(
            btvec.Path.rect(0, 0, 100, 100).fill(GOLD), disc.fill(fade).stroke(ring, width=5).knockout()
        ),
    }
    draws: dict[str, list[Draw]] = {"gradient_framed": [(100, 100, {}), (240, 160, {})]}
    return [(name, drawing, draws.get(name, [(100, 100, {}), (50, 50, {})])) for name, drawing in drawings.items()]


def cuts() -> list[Entry]:
    """Clips and masks: a clip with its edge a curve, one with a hard and a blurred shadow, two shapes of one step
    (one region), a flat and a gradient mask, a mask of translucent shapes with a shadow, both on a frame at two sizes,
    and a knockout that ignores both (its mesh is made). The region goes through the one `View`, so the SVG text is
    exact; the pixels are the rasteriser's; neither has a mesh."""
    disc = btvec.Path.circle((50, 50), 38)
    region = btvec.Path.circle((40, 55), 30)
    fade = btvec.linear_gradient([(0, "#000000"), (1, "#ffffff")], start=(10, 0), end=(90, 0))
    whole = btvec.Path.rect(0, 0, 100, 100)
    drawings = {
        "clip_disc": framed(disc.fill(BLUE).clip(region)),
        "clip_shadows": framed(
            btvec.Path.rect(15, 15, 40, 40).fill(RED).shadow(dx=12, dy=8, blur=0, color="#000000a0").clip(region),
            btvec.Path.rect(45, 45, 40, 40).fill(GOLD).shadow(dx=6, dy=6, blur=10, color="#000000c0").clip(region),
        ),
        "clip_two_shapes": framed(
            btvec.Drawing(disc.fill(GOLD), region.fill(RED)).clip(btvec.Path.rect(20, 20, 60, 60))
        ),
        "mask_flat": framed(disc.fill(RED).mask(whole.fill("#808080"))),
        "mask_gradient": framed(disc.fill(BLUE).stroke(INK, width=6).mask(whole.fill(fade))),
        "mask_translucent_shadow": framed(
            disc.fill(GOLD)
            .shadow(dx=8, dy=8, blur=0, color="#000000b0")
            .mask(btvec.Drawing(region.fill("#ffffffc0"), btvec.Path.circle((65, 50), 25).fill("#ffffff").opacity(0.6)))
        ),
        "clip_and_mask_framed": framed(
            whole.fill(RED).blend("multiply").opacity(0.8).clip(region).mask(whole.fill(fade))
        ),
        "cut_knockout": framed(whole.fill(GOLD), disc.fill(BLUE).knockout().clip(region).mask(whole.fill("#000000"))),
    }
    draws: dict[str, list[Draw]] = {"clip_and_mask_framed": [(100, 100, {}), (240, 160, {})]}
    return [(name, drawing, draws.get(name, [(100, 100, {}), (50, 50, {})])) for name, drawing in drawings.items()]


def resamples() -> list[Entry]:
    """`resample` on shapes: an open and a closed outline, a curve, a per-item spacing and a Param spacing."""
    line = btvec.Path.from_svg_path("M15 70 L50 25 L85 70")
    circle = btvec.Path.circle((50, 50), 30)
    gap = btvec.Param("gap", default=9, min=3, max=60, unit="px", meaning="the distance between corners")
    drawings = {
        "resample_open": framed(line.stroke(INK, width=2)).resample(12),
        "resample_closed": framed(btvec.Path.rect(20, 20, 60, 60).fill(RED)).resample(25),
        "resample_curve": framed(circle.fill(BLUE)).resample(20),
        "resample_per_item": framed(btvec.Drawing(circle, circle.scale(0.5)).fill(GOLD)).resample(
            btvec.by_index(30, 12)
        ),
        "resample_param": framed(line.stroke(INK, width=2)).resample(gap),
    }
    draws: dict[str, list[Draw]] = {"resample_param": [(100, 100, {}), (200, 200, {"gap": 40})]}
    return [(name, drawing, draws.get(name, [(100, 100, {}), (50, 50, {})])) for name, drawing in drawings.items()]


def waves() -> list[Entry]:
    """`wave` on shapes: a line, a closed glyph-like outline at an angle, text whose wave runs across its letters with a
    Param phase, and a per-item amplitude."""
    line = btvec.Path.from_svg_path("M10 50 L90 50")
    glyph = btvec.Path.from_svg_path("M20 80 L20 20 L55 20 Q80 20 80 40 Q80 60 55 60 L40 60 L40 80 Z")
    phase = btvec.Param("phase", default=0, min=0, max=2, unit="", meaning="how far the wave has travelled")
    drawings = {
        "wave_line": framed(line.stroke(INK, width=2)).wave(amplitude=8, wavelength=40),
        "wave_closed_glyph": framed(glyph.fill(RED)).wave(amplitude=5, wavelength=45, angle=30, phase=0.2),
        "wave_text_param": framed(btvec.text("Hif", font=FRAUNCES, size=60, anchor=(8, 70)).fill(BLUE)).wave(
            amplitude=4, wavelength=50, phase=phase
        ),
        "wave_per_item": framed(btvec.Drawing(glyph, glyph.translate(0, 10)).fill(GOLD)).wave(
            amplitude=btvec.by_index(2, 9), wavelength=30
        ),
    }
    draws: dict[str, list[Draw]] = {"wave_text_param": [(100, 100, {}), (200, 200, {"phase": 0.4})]}
    return [(name, drawing, draws.get(name, [(100, 100, {}), (50, 50, {})])) for name, drawing in drawings.items()]


def wobbles() -> list[Entry]:
    """`wobble` on shapes: a line, a closed glyph-like outline, text whose noise runs across its letters with a Param
    phase, and a few seeds."""
    line = btvec.Path.from_svg_path("M10 50 L90 50")
    glyph = btvec.Path.from_svg_path("M20 80 L20 20 L55 20 Q80 20 80 40 Q80 60 55 60 L40 60 L40 80 Z")
    phase = btvec.Param("phase", default=0, min=0, max=2, unit="", meaning="how far the noise has moved")
    drawings = {
        "wobble_line": framed(line.stroke(INK, width=2)).wobble(amplitude=6, wavelength=40),
        "wobble_closed_glyph": framed(glyph.fill(RED)).wobble(amplitude=4, wavelength=45, phase=0.2, seed=11),
        "wobble_text_param": framed(btvec.text("Hif", font=FRAUNCES, size=60, anchor=(8, 70)).fill(BLUE)).wobble(
            amplitude=3, wavelength=50, phase=phase, seed=-5
        ),
        "wobble_seeds": framed(btvec.Drawing(glyph, glyph.translate(0, 10)).fill(GOLD)).wobble(
            amplitude=3, wavelength=30, seed=btvec.by_index(1, 2)
        ),
    }
    draws: dict[str, list[Draw]] = {"wobble_text_param": [(100, 100, {}), (200, 200, {"phase": 0.4})]}
    return [(name, drawing, draws.get(name, [(100, 100, {}), (50, 50, {})])) for name, drawing in drawings.items()]


def radials() -> list[Entry]:
    """`twist` and `bulge` on shapes: a glyph-like outline twisted at two angles, a circle bulged out and pinched in,
    and Params driving the angle and the amount."""
    glyph = btvec.Path.from_svg_path("M20 80 L20 20 L55 20 Q80 20 80 40 Q80 60 55 60 L40 60 L40 80 Z")
    circle = btvec.Path.circle((50, 50), 30)
    progress = btvec.Param("progress", default=0, min=0, max=1, unit="", meaning="how far the twist has turned")
    amount = btvec.Param("amount", default=0, min=-0.9, max=1.4, unit="", meaning="how far the middle is pushed out")
    drawings = {
        "twist_glyph_90": framed(glyph.fill(RED)).twist(90),
        "twist_glyph_minus_200_radius": framed(glyph.fill(RED)).twist(-200, radius=45, about=(50, 50)),
        "bulge_circle_out": framed(circle.stroke(INK, width=2)).bulge(1.2, radius=45),
        "bulge_circle_pinched": framed(circle.stroke(INK, width=2)).bulge(-0.7, radius=45),
        "twist_param_angle": framed(glyph.fill(BLUE)).twist(progress * 180),
        "bulge_param_amount": framed(btvec.text("Hi", font=FRAUNCES, size=60, anchor=(8, 70)).fill(GOLD)).bulge(amount),
    }
    draws: dict[str, list[Draw]] = {
        "twist_param_angle": [(100, 100, {}), (200, 200, {"progress": 0.6})],
        "bulge_param_amount": [(100, 100, {}), (200, 200, {"amount": -0.5})],
    }
    return [(name, drawing, draws.get(name, [(100, 100, {}), (50, 50, {})])) for name, drawing in drawings.items()]


def envelopes() -> list[Entry]:
    """`envelope`, `arch` and `Path.arc`: a word between a flat-bottomed arc and between two arcs, down, a flag between
    two waves, a Param amount and bend, an arc drawn as a stroke, and two lines of text bent by one shared frame."""
    word = btvec.text("Hif", font=FRAUNCES, size=56, anchor=(8, 66)).fill(INK)
    up = btvec.Path.arc((50, 110), 90, 235, 305)  # bulges up the screen
    down = btvec.Path.arc((50, -30), 90, 55, 125)  # bulges down
    wave = btvec.Path.from_svg_path("M0 40 C30 5 70 75 100 40")
    amount = btvec.Param("amount", default=0, min=0, max=1, unit="", meaning="how far the word is bent")
    bend = btvec.Param("bend", default=0, min=-0.5, max=0.5, unit="", meaning="how far the middle is raised")
    drawings = {
        "envelope_top_arc_flat_bottom": framed(word).envelope(up),
        "envelope_band_same_arc": framed(word).envelope(up, up.translate(0, 30)),
        "envelope_down": framed(word.fill(RED)).envelope(down),
        "envelope_flag": framed(word.fill(BLUE)).envelope(wave, wave.translate(0, 30)),
        "envelope_amount_param": framed(word).envelope(up, up.translate(0, 30), amount=amount),
        "envelope_per_item_amount": framed(btvec.Drawing(word, word.translate(0, 20)).fill(GOLD)).envelope(
            up, amount=btvec.by_index(0.3, 1)
        ),
        "arch_param_bend": framed(word.fill(RED)).arch(bend),
        "arch_frame_closed_glyph": framed(
            btvec.Path.from_svg_path("M20 80 L20 20 L55 20 Q80 20 80 40 Q80 60 55 60 L40 60 L40 80 Z").fill(BLUE)
        ).arch(0.25, frame=(0, 0, 100, 100)),
        "arc_strokes": framed(
            btvec.Path.arc((50, 50), 35, -150, 60).stroke(INK, width=4, cap="round"),
            btvec.Path.arc((50, 50), 20, 100, -200).stroke(RED, width=3),
            btvec.Path.arc((50, 50), 8, 0, 360).stroke(BLUE, width=2),
        ),
        "arch_two_lines_one_frame": framed(
            btvec.text("Hi\nTo", font=FRAUNCES, size=38, leading=42, anchor=(10, 40)).fill(INK)
        ).arch(0.3),
    }
    draws: dict[str, list[Draw]] = {
        "envelope_amount_param": [(100, 100, {}), (200, 200, {"amount": 0.6})],
        "arch_param_bend": [(100, 100, {}), (200, 200, {"bend": -0.3})],
    }
    return [(name, drawing, draws.get(name, [(100, 100, {}), (50, 50, {})])) for name, drawing in drawings.items()]


def reverses() -> list[Entry]:
    """`reverse` on shapes: an open line, a closed outline, a curve, and the draw-on the other way round."""
    line = btvec.Path.from_svg_path("M15 70 L50 25 L85 70")
    circle = btvec.Path.circle((50, 50), 30)
    progress = btvec.Param("progress", default=0.5, min=0, max=1, unit="", meaning="how much is drawn")
    drawings = {
        "reverse_open": framed(line.stroke(INK, width=2)).reverse(),
        "reverse_closed": framed(btvec.Path.rect(20, 20, 60, 60).fill(RED)).reverse(),
        "reverse_curve": framed(circle.stroke(BLUE, width=3)).reverse(),
        "reverse_draw_on": framed(line.stroke(INK, width=2)).reverse().slice(0, progress),
    }
    draws: dict[str, list[Draw]] = {"reverse_draw_on": [(100, 100, {}), (200, 200, {"progress": 0.8})]}
    return [(name, drawing, draws.get(name, [(100, 100, {}), (50, 50, {})])) for name, drawing in drawings.items()]


def simplifies() -> list[Entry]:
    """`simplify` on dense polylines: a circle of points, a wave, a half disc with two corners, a per-item tolerance
    and a Param tolerance."""
    ring = btvec.Path.polyline(
        [(50 + 30 * math.cos(math.tau * i / 200), 50 + 30 * math.sin(math.tau * i / 200)) for i in range(200)],
        closed=True,
    )
    wave = btvec.Path.polyline([(10 + 80 * i / 120, 50 + 25 * math.sin(i / 12)) for i in range(121)])
    arc = [(50 + 35 * math.cos(math.pi * i / 60), 70 - 35 * math.sin(math.pi * i / 60)) for i in range(61)]
    disc = btvec.Path.polyline(arc + [(15 + 70 * i / 30, 70) for i in range(1, 30)], closed=True)
    tolerance = btvec.Param("tolerance", default=0.5, min=0.1, max=8, unit="px", meaning="how far a curve may stray")
    drawings = {
        "simplify_circle": framed(ring.fill(BLUE)).simplify(tolerance=0.5),
        "simplify_wave": framed(wave.stroke(INK, width=2)).simplify(tolerance=1),
        "simplify_corners": framed(disc.fill(RED)).simplify(tolerance=0.5),
        "simplify_per_item": framed(btvec.Drawing(ring, ring.scale(0.5)).fill(GOLD)).simplify(
            tolerance=btvec.by_index(4, 0.2)
        ),
        "simplify_param": framed(wave.stroke(INK, width=2)).simplify(tolerance=tolerance),
    }
    draws: dict[str, list[Draw]] = {"simplify_param": [(100, 100, {}), (200, 200, {"tolerance": 6})]}
    return [(name, drawing, draws.get(name, [(100, 100, {}), (50, 50, {})])) for name, drawing in drawings.items()]


def dashes() -> list[Entry]:
    """`dash` on shapes: an open line, a closed outline from its top and from `near`, a curve, an odd pattern and a
    Param phase (marching ants)."""
    line = btvec.Path.from_svg_path("M15 70 L50 25 L85 70")
    circle = btvec.Path.circle((50, 50), 30)
    march = btvec.Param("march", default=0.25, min=0, max=1, unit="cycle", meaning="how far the dashes have moved")
    drawings = {
        "dash_open": framed(line.stroke(INK, width=2)).dash([10, 6]),
        "dash_closed": framed(btvec.Path.rect(20, 20, 60, 60).stroke(RED, width=2)).dash([14, 5], phase=0.3),
        "dash_near": framed(btvec.Path.rect(20, 20, 60, 60).stroke(RED, width=2)).dash([14, 5], near=(80, 80)),
        "dash_curve": framed(circle.stroke(BLUE, width=2)).dash([9]),
        "dash_param": framed(line.stroke(INK, width=2)).dash([10, 6], phase=march),
    }
    draws: dict[str, list[Draw]] = {"dash_param": [(100, 100, {}), (200, 200, {"march": 0.5})]}
    return [(name, drawing, draws.get(name, [(100, 100, {}), (50, 50, {})])) for name, drawing in drawings.items()]


def placements() -> list[Entry]:
    """`place_along`: petals round a closed circle (a Param moves them, `by_order` sizes each copy), an arrow of two
    shapes every 40 pixels along an open wave, a word stamped on a circle, a copy that keeps its turn, and copies past
    both ends of an open path. Placing computes with `sqrt` only, so the SVG text and the mesh are exact."""
    ring = btvec.Path.circle((100, 100), 70)
    march = btvec.Param("march", default=0, min=0, max=1, unit="cycle", meaning="how far the copies have gone round")
    petal = btvec.Path.from_svg_path("M0 -14 C 10 -6 10 6 0 14 C -10 6 -10 -6 0 -14 Z").fill(RED)
    wave = btvec.Path.from_svg_path("M10 150 C 50 60 90 240 130 150 S 190 90 190 90")
    arrow = btvec.Drawing(btvec.Path.rect(0, -1, 14, 2).fill(INK), btvec.Path.rect(14, -4, 6, 8).fill(INK))
    word = btvec.text("Hif", font=FRAUNCES, size=30, anchor=(0, 0)).fill(BLUE)
    line = btvec.Path.line((20, 100), (180, 100))

    def sized(item: btvec.Drawing) -> btvec.Drawing:
        return btvec.Drawing(item, size=(200, 200))

    drawings = {
        "place_petals": sized(petal.place_along(ring, count=12, start=march).scale(btvec.by_order(0.5, 1.2))),
        "place_arrows": sized(arrow.place_along(wave, spacing=40)),
        "place_word": sized(word.place_along(ring, count=3, start=0.1)),
        "place_no_turn": sized(arrow.place_along(wave, count=4, turn=False)),
        "place_past_ends": sized(petal.place_along(line, count=4, start=0.4)),
    }
    draws: dict[str, list[Draw]] = {"place_petals": [(100, 100, {}), (200, 200, {"march": 0.25})]}
    return [(name, drawing, draws.get(name, [(200, 200, {}), (100, 100, {})])) for name, drawing in drawings.items()]


def morphs() -> list[Entry]:
    """`morph`: a letter into a circle at the middle and at both ends, a letter with counters into one without (a
    hole with no partner, subtracted), a glyph whose crossbar is a contour of its own, a word into a longer and a
    shorter one (glyph by glyph), a staggered Param, an overshooting fraction, open polylines and surplus shapes with
    their styles."""
    circle = btvec.Path.circle((50, 50), 30)

    def letter(content: str) -> btvec.Drawing:
        return framed(btvec.text(content, font=BRICOLAGE, size=90, anchor=(14, 84), axes={"wght": 700}).fill(INK))

    def word(content: str) -> btvec.Drawing:
        return btvec.Drawing(btvec.text(content, font=BRICOLAGE, size=90, anchor=(12, 110)).fill(INK), size=(400, 160))

    progress = btvec.Param("progress", default=0.3, min=0, max=1, unit="", meaning="how far the word has turned")
    polyline = btvec.Path.from_svg_path("M10 20 L40 80 L90 30")
    other = btvec.Path.from_svg_path("M10 80 L50 20 L90 70 L95 90")
    drawings = {
        "morph_letter_to_circle": letter("R").morph(circle, 0.5),
        "morph_letter_fraction_0": letter("R").morph(circle, 0),
        "morph_letter_fraction_1": letter("R").morph(circle, 1),
        "morph_counter_o_to_l": letter("O").morph(letter("l"), 0.9),
        "morph_counter_a_to_i": letter("a").morph(letter("i"), 0.9),
        "morph_unresolved_a_to_o": letter("A").morph(letter("O"), 0.6),
        "morph_overshoot": letter("R").morph(circle, 1.25),
        "morph_word_to_longer": word("cat").morph(word("horse"), 0.4),
        "morph_word_to_shorter": word("horse").morph(word("cat"), 0.4),
        "morph_word_stagger_param": word("cat").morph(word("horse"), btvec.stagger(progress)),
        "morph_polylines": framed(polyline.stroke(INK, width=3)).morph(other, 0.5),
        "morph_surplus_styles": framed(circle.fill(RED), btvec.Path.rect(10, 10, 20, 20).fill(BLUE)).morph(
            btvec.Drawing(btvec.Path.rect(40, 40, 50, 50).fill(GOLD)), 0.5
        ),
    }
    draws: dict[str, list[Draw]] = {"morph_word_stagger_param": [(400, 160, {}), (200, 80, {"progress": 0.8})]}
    return [
        (
            name,
            drawing,
            draws.get(name, [(400, 160, {}), (200, 80, {})] if "word" in name else [(100, 100, {}), (50, 50, {})]),
        )
        for name, drawing in drawings.items()
    ]


def hatches() -> list[Entry]:
    """`hatch`: straight lines, noise with a spacing read from a ramp, the contour of a ring (closed loops), a mix of an
    angle and noise, a distance field as the tone, and lines that avoid a circle. Every field and flow is a document
    value, so the native extension and the WebAssembly package read the same calls."""
    box = btvec.Path.rect(0, 0, 100, 100)
    ring_path = btvec.Path.from_svg_path(
        btvec.Path.circle((50, 50), 45).to_svg_path() + " " + btvec.Path.circle((50, 50), 15).reverse().to_svg_path()
    )
    ramp = btvec.Field.ramp((0, 0), (100, 0))
    noise = btvec.Flow.noise(wavelength=80, seed=3)
    photo = btvec.Field.map(16, 16, FONTS["tone_map"], box=(0, 0, 100, 100), name="tone_map")
    phase = btvec.Param("phase", default=0, min=0, max=4)
    level = btvec.Param("level", default=0.4, min=0, max=1)
    weight = btvec.Param("weight", default=800, min=200, max=800)
    word = btvec.text("Hi", font=BRICOLAGE, size=96, anchor=(8, 84), axes={"wght": weight}).resolve()
    drawings = {
        "hatch_angle": framed(btvec.Drawing(box).hatch(7, flow=btvec.Flow.angle(20)).stroke(INK, width=1)),
        "hatch_noise_ramp": framed(btvec.Drawing(box).hatch((3, 9), tone=ramp, flow=noise).stroke(INK, width=1)),
        "hatch_contour_ring": framed(
            btvec.Drawing(ring_path).hatch(5, flow=btvec.Flow.contour(ring_path)).stroke(INK, width=1)
        ),
        "hatch_mix": framed(btvec.Drawing(box).hatch(6, flow=btvec.Flow.angle(0).mix(noise, 0.5)).stroke(INK, width=1)),
        "hatch_distance_tone": framed(
            btvec.Drawing(ring_path)
            .hatch((3, 8), tone=btvec.Field.distance(ring_path, normalized=True), flow=btvec.Flow.angle(60))
            .stroke(INK, width=1)
        ),
        "hatch_avoid": framed(
            btvec.Drawing(box)
            .hatch(6, flow=btvec.Flow.angle(10), avoid=btvec.Path.circle((50, 50), 25))
            .stroke(INK, width=1)
        ),
    }
    draws: list[Draw] = [(100, 100, {}), (50, 50, {})]
    # V15b: values a step reads again with the values of the draw, a map as the tone, a level of tone
    by_values: dict[str, tuple[btvec.Drawing, list[Draw]]] = {
        "hatch_below_ramp": (
            framed(
                btvec.Drawing(box).hatch((6, 6), tone=ramp, flow=btvec.Flow.angle(30), below=0.5).stroke(INK, width=1)
            ),
            draws,
        ),
        "hatch_below_param": (
            framed(
                btvec.Drawing(box)
                .hatch((6, 6), tone=ramp, flow=btvec.Flow.angle(120), below=level)
                .stroke(INK, width=1)
            ),
            [(100, 100, {}), (100, 100, {"level": 0.8})],
        ),
        "hatch_map_tone": (
            framed(
                btvec.Drawing(box).hatch((3, 9), tone=photo, flow=btvec.Flow.angle(45), below=0.7).stroke(INK, width=1)
            ),
            draws,
        ),
        "hatch_noise_phase_param": (
            framed(
                btvec.Drawing(box)
                .hatch(7, flow=btvec.Flow.noise(wavelength=90, phase=phase, seed=5))
                .stroke(INK, width=1)
            ),
            [(100, 100, {}), (100, 100, {"phase": 0.5})],
        ),
        "hatch_text_weight_param": (
            framed(
                word.hatch(
                    (2.5, 7), tone=btvec.Field.distance(word, normalized=True), flow=btvec.Flow.contour(word)
                ).stroke(INK, width=0.8)
            ),
            [(100, 100, {}), (100, 100, {"weight": 300})],
        ),
    }
    return [(name, drawing, draws) for name, drawing in drawings.items()] + [
        (name, drawing, drawn) for name, (drawing, drawn) in by_values.items()
    ]


def line_families() -> list[Entry]:
    """V15c: ridgelines, bands of each carrier and stippled dots, over a map tone and a ramp, with Params."""
    box = btvec.Path.rect(0, 0, 100, 100)
    photo = btvec.Field.map(16, 16, FONTS["tone_map"], box=(0, 0, 100, 100), name="tone_map")
    ramp = btvec.Field.ramp((0, 0), (100, 0))
    lift = btvec.Param("lift", default=10, min=0, max=40)
    dot = btvec.Param("dot", default=1, min=0.5, max=2)
    drawings = {
        "rows_map": framed(btvec.Drawing(box).rows(8, tone=photo, amplitude=24).stroke(INK, width=1)),
        "rows_param": framed(btvec.Drawing(box).rows(8, tone=ramp, amplitude=lift).stroke(INK, width=1)),
        # spacing 7 at 20 degrees gave a pixel one level off between a window and the whole (design-decisions.md, V15c)
        "bands_lines": framed(btvec.Drawing(box).bands(8, tone=photo, width=(0.6, 5), angle=20).fill(INK)),
        "bands_waves": framed(
            btvec.Drawing(box).bands(9, tone=ramp, width=(0.6, 4), family="waves", wavelength=30, amplitude=3).fill(INK)
        ),
        "bands_rings": framed(btvec.Drawing(box).bands(8, tone=photo, width=(0.6, 5), family="rings").fill(INK)),
        "bands_spiral": framed(
            btvec.Drawing(box).bands(8, tone=ramp, width=(0.6, 5), family="spiral", centre=(30, 60)).fill(INK)
        ),
        "bands_rays": framed(btvec.Drawing(box).bands(8, tone=ramp, width=(0.6, 5), family="rays", angle=10).fill(INK)),
        "stipple_map": framed(btvec.Drawing(box).stipple(4, tone=photo, radius=dot, seed=3).fill(INK)),
    }
    draws: dict[str, list[Draw]] = {
        "rows_param": [(100, 100, {}), (100, 100, {"lift": 30})],
        "stipple_map": [(100, 100, {}), (100, 100, {"dot": 1.8})],
    }
    return [(name, drawing, draws.get(name, [(100, 100, {}), (50, 50, {})])) for name, drawing in drawings.items()]


def plots() -> list[dict[str, object]]:
    """V15c: the plotter SVG of drawings of strokes, fills and a closed outline, as the native extension writes it."""
    box = btvec.Path.rect(0, 0, 100, 100)
    ramp = btvec.Field.ramp((0, 0), (100, 0))
    art = btvec.Drawing(
        btvec.Drawing(box).rows(10, tone=ramp, amplitude=20).stroke(INK, width=1),
        btvec.Drawing(box).stipple(10, tone=0.3, radius=1.5).fill("#cc2200"),
        btvec.Path.circle((50, 50), 30).fill("#003399"),
        size=FRAME,
    )
    return [
        {
            "json": art.to_json(),
            "width": 400,
            "height": 400,
            "penWidth": 1.0,
            "tolerance": None,
            "svg": art.to_plot_svg(400, 400, pen_width=1.0),
        },
        {
            "json": art.to_json(),
            "width": 200,
            "height": 100,
            "penWidth": 0.5,
            "tolerance": 0.0,
            "svg": art.to_plot_svg(200, 100, pen_width=0.5, tolerance=0.0),
        },
    ]


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
    resamples,
    waves,
    radials,
    envelopes,
    morphs,
    hatches,
    line_families,
    wobbles,
    dashes,
    simplifies,
    reverses,
    placements,
    shadows,
    knockouts,
    halftones,
    gradients,
    cuts,
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


def window_of(width: float, height: float) -> tuple[int, int, int, int]:
    """A window inside a `width` by `height` output that no tile or blur grid lines up with: (x, y, width, height)."""
    x, y = int(width) // 3, int(height) // 4
    return x, y, max(1, min(int(width) // 2, int(width) - x)), max(1, min(int(height) // 3, int(height) - y))


def compared(drawing: btvec.Drawing, name: str, index: int, draw: Draw, pixels: Path) -> dict[str, Any]:
    """What the native extension makes of one draw of the corpus: hashes of the exact outputs, pixels in a file."""
    width, height, values = draw
    file = f"{name}-{index}.rgba"
    (pixels / file).write_bytes(drawing.to_rgba(width, height, **values))
    window = window_of(width, height)
    window_file = f"{name}-{index}.window.rgba"
    (pixels / window_file).write_bytes(drawing.to_rgba(width, height, window=window, **values))
    return {
        "width": width,
        "height": height,
        "values": values,
        "svg_sha256": sha256(drawing.to_svg(width, height, **values).encode("utf-8")),
        "mesh": refused_mesh(drawing, draw)
        if name in MESH_REFUSED
        else described(drawing.to_mesh(width, height, **values)),
        "rgba_file": file,
        "window": list(window),
        "window_rgba_file": window_file,
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


def example_everywhere(name: str) -> dict[str, Any]:
    """examples/<name>.py's frames as SVG text from Python and from the Rust example of the same name."""
    frames = runpy.run_path(str(ROOT / "examples" / f"{name}.py"))["frames"]
    command = ["cargo", "run", "--quiet", "--locked", "-p", "bullterrier-vector", "--example", name, "--"]
    return {
        "python": frames,
        "rust": {
            frame: subprocess.run([*command, frame], cwd=ROOT, capture_output=True, check=True).stdout.decode("utf-8")
            for frame in frames
        },
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
        "bent_type": example_everywhere("bent_type"),
        "glyph_morph": example_everywhere("glyph_morph"),
        "flow_hatch": example_everywhere("flow_hatch"),
        "kinetic_type": example_everywhere("kinetic_type"),
        "plots": plots(),
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
