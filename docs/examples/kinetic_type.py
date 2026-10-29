"""Kinetic type, the first reference piece: a variable font breathing letter by letter, printed as two riso inks.

Two lines of condensed type fill a portrait frame. They start as hairlines, a blue plane rises with the letters cut out
of it, the drop bursts them into a two-ink riso print (pink letters over the blue plane, multiplied over paper and
misregistered more on each hit), the letters pulse one after another, and the print thins back to hairlines.

The drawing is built once. Every number that changes is a Param; the clip is the caller's function of time, which
gives the drawing its values for each frame. `pulse` is the caller's signal per letter, so it is passed as a list.
"""

from pathlib import Path

import bullterrier_vector as btvec

WIDTH, HEIGHT = 1080, 1350
PAPER, INK, PINK, BLUE = "#f3eee4", "#16151a", "#ff4f9a", "#2a5bff"

font = btvec.Font(Path("fonts/Anybody[wdth,wght].ttf").read_bytes())

weight = btvec.Param("weight", default=600, min=100, max=780, meaning="how heavy the letters are, before a pulse")
rise = btvec.Param("rise", default=0, min=0, max=1, unit="fraction", meaning="how far the blue plane has risen")
ink = btvec.Param("ink", default=0, min=0, max=1, unit="fraction", meaning="0: hairlines only; 1: the riso print")
burst = btvec.Param("burst", default=0, min=0, max=1, unit="fraction", meaning="how hard the last hit was")
pulse = btvec.Param("pulse", default=0, min=0, max=1, unit="fraction", meaning="a kick, per letter")

# One shape per letter. Each line is fitted to the width by the width axis once, at the calm pose (every Param at its
# default), so a pulse overflows the margin instead of shrinking the word.
letters = (
    btvec.text(
        "NO\nSLEEP",
        font=font,
        size=560,
        axes={"wght": weight + 120 * pulse},
        leading=520,
        at=(WIDTH / 2, 600),
        align="center",
        width=980,
        fit="wdth",
    )
    .scale(1 + 0.08 * pulse)
    .rotate(btvec.random(-10, 10, seed=7) * burst)
    .translate(btvec.random(-30, 30, seed=8) * burst, btvec.random(-50, 50, seed=9) * burst)
)

frame = btvec.Path.rect(0, 0, WIDTH, HEIGHT)
misregistration = 3 + 14 * burst  # a constant offset looks like a mistake; it grows on a hit

paper = frame.fill(PAPER)
plane = btvec.Drawing(frame).translate(0, HEIGHT * (1 - rise))
blue = plane.subtract(letters.translate(misregistration, 0.6 * misregistration)).fill(BLUE).blend("multiply")
pink = letters.fill(PINK).blend("multiply").opacity(ink)
hairlines = letters.stroke(INK, width=1.5).opacity(1 - ink)

piece = btvec.Drawing(paper, blue, pink, hairlines, size=(WIDTH, HEIGHT))
LETTERS = len(letters.shapes())  # 7: a space or a line break gives no shape


def kick(t: float) -> float:
    """A kick on every beat at 120 bpm: 1 on the beat, falling to 0 within 0.3 s."""
    return 1 - btvec.ease.out_cubic((t % 0.5) / 0.3)


def at_time(t: float) -> btvec.Drawing:
    """The piece `t` seconds in: hairlines, the plane rises from 4 s, the drop at 6 s, thinning out from 10 s."""
    printed = 6 <= t < 11
    heavy = btvec.ease.out_back((t - 6) / 0.3) - btvec.ease.in_out_cubic((t - 10) / 1.5)
    return piece.with_values(
        rise=btvec.ease.in_out_cubic((t - 4) / 2),
        ink=1 if printed else 0,
        weight=100 + 500 * heavy,
        burst=kick(t) if printed else 0,
        pulse=[kick(t - 0.045 * i) if printed else 0 for i in range(LETTERS)],
    )


Path("no-sleep.png").write_bytes(at_time(8.0).to_png(WIDTH, HEIGHT))
btvec.frames(at_time, width=WIDTH, height=HEIGHT, fps=30, seconds=12, out="no-sleep.mov")
