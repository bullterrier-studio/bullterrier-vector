"""Kinetic type, as far as the calls built so far reach: a variable font breathing letter by letter in two riso inks.

Two lines of condensed type stand on a portrait frame. A blue plane rises behind them, the print (pink letters over the
blue plane, multiplied over paper) fades in over hairlines, and a kick runs through the letters one after another: each
letter's weight is its own number, `pulse`, given as a list with one entry per character of the text.

The drawing is built once. Every number that changes is a Param; `at_time` is the caller's function of time, which gives
the drawing its values for a frame (give it to `btvec.frames` for a clip, as examples/badge_frames.py does). The font is
a subset of Anybody (SIL OFL 1.1, examples/fonts/OFL.txt) with its width and weight axes. Draw a frame with
`uv run poe look examples/kinetic_type.py`.
"""

from pathlib import Path

import bullterrier_vector as btvec

WIDTH, HEIGHT = 1080, 1350
PAPER, INK, PINK, BLUE = "#f3eee4", "#16151a", "#ff4f9a", "#2a5bff"
TEXT = "NO\nSLEEP"

font = btvec.Font((Path(__file__).parent / "fonts" / "anybody-subset.ttf").read_bytes(), name="anybody")

weight = btvec.Param("weight", default=300, min=100, max=500, meaning="how heavy the letters are, before a kick")
rise = btvec.Param("rise", default=0, min=0, max=1, unit="fraction", meaning="how far the blue plane has risen")
ink = btvec.Param("ink", default=0, min=0, max=1, unit="fraction", meaning="0: hairlines only; 1: the riso print")
pulse = btvec.Param("pulse", default=0, min=0, max=1, unit="fraction", meaning="a kick, per character of the text")

# One shape per letter; the line break and nothing else gives none. `wght` reads `pulse` per character.
letters = btvec.text(
    TEXT,
    font=font,
    size=800,
    axes={"wdth": 50, "wght": weight + 400 * pulse},
    leading=640,
    anchor=(WIDTH / 2, 620),
    align="center",
)

frame = btvec.Path.rect(0, 0, WIDTH, HEIGHT)
plane = btvec.Drawing(frame).translate(0, HEIGHT * (1 - rise)).fill(BLUE).blend("multiply")
pink = letters.fill(PINK).blend("multiply").opacity(ink)
hairlines = letters.stroke(INK, width=1.5).opacity(1 - ink)

piece = btvec.Drawing(frame.fill(PAPER), plane, pink, hairlines, size=(WIDTH, HEIGHT))
CHARACTERS = len(TEXT)  # every character counts for a per-character value, the line break too


def kick(t: float) -> float:
    """A kick on every beat at 120 bpm: 1 on the beat, falling to 0 within 0.3 s."""
    return max(0.0, 1 - (t % 0.5) / 0.3) ** 3


def at_time(t: float) -> btvec.Drawing:
    """The piece `t` seconds in: the plane rises from 1 s to 3 s, the print arrives at 3 s, the kick runs through the
    letters 0.045 s apart from then on."""
    printed = t >= 3
    return piece.with_values(
        rise=min(1.0, max(0.0, (t - 1) / 2)),
        ink=1 if printed else 0,
        pulse=[kick(t - 0.045 * i) if printed else 0 for i in range(CHARACTERS)],
    )


drawing = at_time(4.0)  # one frame, for `uv run poe look examples/kinetic_type.py`
