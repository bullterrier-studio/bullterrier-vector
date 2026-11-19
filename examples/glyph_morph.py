"""Glyph morph: a letter that turns into a circle, and one word that turns into another, on one value.

`progress` (0 to 1) is the only input. The letter R is carried to a circle, its counter closing as it goes. Beside it,
CAT turns into HORSE letter by letter: each glyph is paired with the glyph at the same place in the other word, a
spare glyph of the longer word grows from its own center, and `stagger(progress)` lets the letters start one after
the other, so at 0.5 the word is half way, each letter at its own point of the way. Every frame is the straight blend
of the two outlines, and 1 to 0 is the way back: the library has no time, so whoever animates sets `progress` and draws
again. The font comes with the library (SIL OFL 1.1).
Draw it with `uv run poe look examples/glyph_morph.py progress=0.5` (PNG and SVG in `dist/look/`).
The same program in Rust and JavaScript: `crates/core/examples/glyph_morph.rs`, `crates/wasm/examples/glyph_morph.mjs`.
"""

import bullterrier_vector as btvec

WIDTH, HEIGHT = 1080, 560
PAPER, INK, ACCENT = "#f3eee4", "#16151a", "#d4452b"

font = btvec.Font.bundled("bricolage_grotesque")
progress = btvec.Param("progress", default=0, min=0, max=1, unit="fraction", meaning="how far the shapes have turned")


def word(content: str) -> btvec.Drawing:
    """The glyphs of `content`, each one outline (an E made of four bars would grow as four bars)."""
    glyphs = btvec.text(content, font=font, size=190, axes={"opsz": 96, "wdth": 75, "wght": 800}, anchor=(570, 350))
    return glyphs.resolve()


letter = btvec.text("R", font=font, size=440, axes={"opsz": 96, "wght": 800}, anchor=(270, 440), align="center")
circle = btvec.Path.circle((270, 285), 150)

drawing = btvec.Drawing(
    btvec.Path.rect(0, 0, WIDTH, HEIGHT).fill(PAPER),
    letter.morph(circle, progress).fill(INK),
    word("CAT").morph(word("HORSE"), btvec.stagger(progress)).fill(ACCENT),
    size=(WIDTH, HEIGHT),
)

# Three frames of the piece: the letter and CAT, half way, the circle and HORSE.
FRAMES = {"start": 0, "half": 0.5, "end": 1}
frames = {name: drawing.to_svg(WIDTH, HEIGHT, progress=p) for name, p in FRAMES.items()}
