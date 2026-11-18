"""The halftone drawings of the WebAssembly comparison (see oracle.py): what the native extension answers for them is
what the Node tests expect of the package."""

import bullterrier_vector as btvec

Draw = tuple[int, int, dict[str, float]]
Entry = tuple[str, btvec.Drawing, list[Draw]]

GOLD = "#ffcc33"
INK = "#1a1a1a"
RED = "#e2412e"
BLUE = "#336699"
FRAME = (100, 100)


def framed(*items: btvec.Path | btvec.Drawing) -> btvec.Drawing:
    """The items in the 100 by 100 design frame."""
    return btvec.Drawing(*items, size=FRAME)


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
        "halftone_ellipse_rings": framed(
            disc.halftone(INK, tone=0.55, pitch=6, shape="ellipse", aspect=2, screen="rings", centre=(40, 45))
        ),
        "halftone_euclidean_spiral": framed(disc.halftone(RED, tone=0.4, pitch=7, shape="euclidean", screen="spiral")),
        "halftone_diamond_wave": framed(
            ring.halftone(
                BLUE, tone=0.6, pitch=7, angle=15, shape="diamond", screen="wave", amplitude=1.5, wavelength=7
            )
        ),
        "halftone_square_rays": framed(disc.halftone(INK, tone=0.5, pitch=8, angle=20, shape="square", screen="rays")),
        "halftone_path_grid": framed(
            disc.halftone(RED, tone=0.45, pitch=9, angle=30, shape=btvec.Path.from_svg_path("M0 0 L4 0 L0 2 Z"))
        ),
        "halftone_ramp_field": framed(
            disc.halftone(INK, tone=btvec.Field.ramp((10, 10), (90, 90)), pitch=6, angle=0, shape="square")
        ),
        "halftone_distance_field": framed(
            btvec.Path.rect(5, 5, 90, 90).halftone(
                BLUE, tone=btvec.Field.distance(disc, normalized=False), pitch=5, shape="dot", screen="rings"
            )
        ),
    }
    draws: dict[str, list[Draw]] = {
        "halftone_param": [(100, 100, {"tone": 0.3}), (200, 200, {"tone": 0.8})],
        "halftone_disc": [(100, 100, {}), (200, 200, {})],
    }
    return [(name, drawing, draws.get(name, [(100, 100, {})])) for name, drawing in drawings.items()]
