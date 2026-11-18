"""Preset `glyph_morph`: a letter turning into a circle, and one word into another, on one `progress`."""

import bullterrier_vector as btvec

WIDTH, HEIGHT = 1080, 560
PAPER, INK, ACCENT = "#f3eee4", "#16151a", "#d4452b"

progress = btvec.Param("progress", default=0, min=0, max=1, unit="fraction", meaning="how far the shapes have turned")
font = btvec.Font.bundled("bricolage_grotesque")


def word(content: str) -> btvec.Drawing:
    """The glyphs of `content`, each one outline."""
    glyphs = btvec.text(
        content, font=font, size=190, axes={"opsz": 96, "wdth": 75, "wght": 800}, anchor=(790, 350), align="center"
    )
    return glyphs.resolve()


letter = btvec.text("R", font=font, size=440, axes={"opsz": 96, "wght": 800}, anchor=(270, 430), align="center")
drawing = btvec.Drawing(
    btvec.Path.rect(0, 0, WIDTH, HEIGHT).fill(PAPER),
    letter.morph(btvec.Path.circle((270, 285), 150), progress).fill(INK),
    word("CAT").morph(word("HORSE"), btvec.stagger(progress)).fill(ACCENT),
    size=(WIDTH, HEIGHT),
)
