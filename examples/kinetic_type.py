"""NO SLEEP: kinetic type in two riso inks, 12 seconds of one drawing built once.

Two lines of type, each stretched by the width axis to fill the same measure, stand on a portrait page. What moves, in
order: hairlines (0 s); a blue plane rises behind the letters and is knocked out where they stand (2 to 4 s); the drop
at 4 s, when the letters are printed pink over the blue and a kick every half second (120 bpm) runs through them 0.045 s
apart, thickening each and scattering the whole word a little off its blue; the line arches, ripples and wobbles
(5.5 to 8.5 s); NO SLEEP turns into GO DREAM letter by letter (8 to 9.5 s); the print thins back to hairlines (10 to
12 s).

Every number that changes is a Param: `weight`, `rise`, `ink` (0: hairlines only, 1: the print), `burst`, `pulse`,
`bend`, `phase` (cycles, so a loop of 0 to 1 repeats) and `turn`. `pulse` is a list with one entry per character of the
text, the line break counting as one, and drives the weight only. `at_time` is the caller's function of time: easing
included, the library has none. The font is a subset of Anybody (SIL OFL 1.1, examples/fonts/OFL.txt) with its width
and weight axes. Draw a frame with `uv run poe look examples/kinetic_type.py`; write the clip with

    btvec.frames(at_time, width=WIDTH, height=HEIGHT, fps=30, seconds=12, out="frames/kinetic-####.png")

(a `.mov` or `.webm` for a clip with alpha: see examples/badge_frames.py).
"""

from pathlib import Path

import bullterrier_vector as btvec

WIDTH, HEIGHT = 1080, 1350
PAPER, INK, PINK, BLUE = "#f3eee4", "#16151a", "#ff4f9a", "#2a5bff"
MEASURE = 980  # the width each line is stretched to fill
BEAT = 0.5  # seconds: 120 bpm

font = btvec.Font((Path(__file__).parent / "fonts" / "anybody-subset.ttf").read_bytes(), name="anybody")

weight = btvec.Param("weight", default=300, min=100, max=900, meaning="how heavy the letters are, before a kick")
rise = btvec.Param("rise", default=0, min=0, max=1, unit="fraction", meaning="how far the blue plane has risen")
ink = btvec.Param("ink", default=0, min=0, max=1, unit="fraction", meaning="0: hairlines only; 1: the riso print")
burst = btvec.Param(
    "burst", default=0, min=0, max=1, unit="fraction", meaning="how hard the last hit scattered the word"
)
pulse = btvec.Param("pulse", default=0, min=0, max=1, unit="fraction", meaning="a kick, per character of the text")
bend = btvec.Param("bend", default=0, min=0, max=1, unit="fraction", meaning="how far the headline arches and ripples")
phase = btvec.Param("phase", default=0, min=0, max=1, unit="cycle", meaning="where the ripple and the shaking are")
turn = btvec.Param("turn", default=0, min=0, max=1, unit="fraction", meaning="how far NO SLEEP is GO DREAM")


def headline(content: str, wght: btvec.Expr) -> btvec.Drawing:
    """`content` set in the common layout. Each line gets the `wdth` at which it is MEASURE wide, once, at the calm
    pose (every Param at its default): a kick overflows the measure instead of shrinking the word. Both headlines
    share the layout, so each letter has a partner."""
    return btvec.text(
        content,
        font=font,
        size=800,
        axes={"wght": wght},
        leading=640,
        anchor=(WIDTH / 2, 620),
        align="center",
        width=MEASURE,
        fit="wdth",
    )


first = headline("NO\nSLEEP", weight + 400 * pulse)
second = headline("GO\nDREAM", weight)
CHARACTERS = len("NO\nSLEEP")  # `pulse` has one entry for each, the line break too: it counts characters, not shapes

# The arch is anchored to the calm bounds, or it would breathe with every kick.
calm = first.bounds()
letters = (
    first.resolve()
    .morph(second.resolve(), btvec.stagger(turn, spread=0.6))
    .arch(0.06 * bend, frame=(calm.x, calm.y, calm.width, calm.height))
    .wave(amplitude=16 * bend, wavelength=320, phase=phase)
    .wobble(amplitude=3 * bend, wavelength=60, phase=phase, seed=7)
    .rotate(btvec.random(-10, 10, seed=7) * burst)
    .translate(btvec.random(-30, 30, seed=8) * burst, btvec.random(-50, 50, seed=9) * burst)
)

page = btvec.Path.rect(0, 0, WIDTH, HEIGHT)
misregistration = 3 + 14 * burst  # the blue plate slips off the pink on a hit
plane = btvec.Drawing(page).translate(0, HEIGHT * (1 - rise))
blue = plane.subtract(letters.translate(misregistration, 0.6 * misregistration)).fill(BLUE).blend("multiply")
pink = letters.fill(PINK).blend("multiply").opacity(ink)
hairlines = letters.stroke(INK, width=1.5).opacity(1 - ink)
piece = btvec.Drawing(page.fill(PAPER), blue, pink, hairlines, size=(WIDTH, HEIGHT))


def smooth(x: float) -> float:
    """0 up to 1 over x from 0 to 1, easing in and out."""
    x = min(1.0, max(0.0, x))
    return x * x * (3 - 2 * x)


def kick(t: float) -> float:
    """1 on a beat, falling to 0 within 0.3 s, and again on every beat."""
    return max(0.0, 1 - (t % BEAT) / 0.3) ** 3


def at_time(t: float) -> btvec.Drawing:
    """The piece `t` seconds in (0 to 12), as the timeline in the module docstring."""
    printed = 4 <= t < 12
    hitting = printed and t < 8
    return piece.with_values(
        rise=smooth((t - 2) / 2),
        ink=1 - smooth((t - 11.5) / 0.5) if printed else 0,
        weight=300 - 200 * smooth((t - 10) / 1.5) if printed else 100,
        burst=kick(t - 4) if hitting else 0,
        pulse=[kick(t - 4 - 0.045 * i) if hitting else 0 for i in range(CHARACTERS)],
        bend=smooth(t - 5.5) * (1 - smooth(t - 7.5)),
        phase=(t * 0.25) % 1,
        turn=smooth((t - 8) / 1.5),
    )


drawing = at_time(6.5)  # one frame, for `uv run poe look examples/kinetic_type.py`
