"""The character grids and overlay marks of the WebAssembly comparison (see oracle.py): what the native extension
answers for them is what the Node tests expect of the package. Fonts are bundled ones, which a document names by digest
and loads without bytes."""

import bullterrier_vector as btvec

Draw = tuple[int, int, dict[str, float]]
Entry = tuple[str, btvec.Drawing, list[Draw]]

INK = "#1a1a1a"
RED = "#e2412e"
FRAME = (100, 100)


def glyphs() -> list[Entry]:
    """Grids (the default font, colours cell by cell, a proportional font, a size from a Param, ligature characters)
    and marks (boxes and points with labels, a box moved by a Param): the glyph shapes come from the one core, so the
    SVG text and the mesh are exact; the pixels are the rasteriser's, as for any shape."""
    scale = btvec.Param("scale", default=9, min=2, max=14, unit="px", meaning="the size of the characters")
    left = btvec.Param("left", default=10, min=0, max=40, unit="px", meaning="where the box starts")
    colors = [["#cc2200", "#1a1a1a", "#336699"], ["#1a1a1a", "#ffcc33", "#cc2200"]]
    hud = btvec.marks([(10, 12, 50, 36, "EAR L")], [(72, 70, "NOSE"), (30, 80)], bracket=0.3, cross=8, text_size=9)
    param_hud = btvec.marks([(left, 20, 40, 40, "BOX")], text_size=8)
    drawings = {
        "characters_grid": btvec.Drawing(btvec.characters("#@.:\n -=+\nHg%!", cell=(12, 16)).fill(INK), size=FRAME),
        "characters_colored": btvec.Drawing(
            btvec.characters("a#b\nc.d", cell=(14, 18), size=12, colors=colors), size=FRAME
        ),
        "characters_proportional": btvec.Drawing(
            btvec.characters(["Wim", "il."], cell=(16, 20), font=btvec.Font.bundled("fraunces")).fill(RED), size=FRAME
        ),
        "characters_param_size": btvec.Drawing(
            btvec.characters("HH\nHH", cell=(20, 24), size=scale).fill(INK), size=FRAME
        ),
        "characters_ligatures": btvec.Drawing(btvec.characters("->==!=<=", cell=(11, 14)).fill(INK), size=FRAME),
        "marks_hud": btvec.Drawing(
            hud.only(role="text").fill(INK), hud.without(role="text").stroke(RED, width=1), size=FRAME
        ),
        "marks_param_box": btvec.Drawing(
            param_hud.only(role="text").fill(INK), param_hud.without(role="text").stroke(INK, width=1), size=FRAME
        ),
    }
    draws: dict[str, list[Draw]] = {
        "characters_param_size": [(100, 100, {}), (100, 100, {"scale": 13})],
        "marks_param_box": [(100, 100, {}), (100, 100, {"left": 35})],
    }
    return [(name, drawing, draws.get(name, [(100, 100, {}), (50, 50, {})])) for name, drawing in drawings.items()]
