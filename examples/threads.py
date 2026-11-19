"""Threads: forking threads grown out of the edges of a heavy letter, tapered and drawn on from their roots.

`branch` grows a root every 70 pixels along the letter's outline (its counter too), each wandering out and forking
twice; `spread` makes some roots' trees shorter. Every path runs from its root to a tip, so `slice(0, growth)` grows the
threads out of the letter, a fork appearing where its parent passes; `outline` with a profile tapers each to a point,
and the trunks are drawn heavier. The frame "half" is the growth halfway.
Draw it with `uv run poe look examples/threads.py` (PNG and SVG in `dist/look/`).
The same program in Rust and JavaScript: `crates/core/examples/threads.rs`, `crates/wasm/examples/threads.mjs`.
"""

import bullterrier_vector as btvec

WIDTH, HEIGHT = 800, 800
PAPER, INK = "#f2c230", "#141414"
TAPER = [(0, 1), (1, 0.15)]

font = btvec.Font.bundled("fraunces")
letter = btvec.text("R", font=font, size=560, axes={"wght": 900}, anchor=(400, 600), align="center").resolve()
growth = btvec.Param("growth", default=1, min=0, max=1, meaning="how far the threads have grown")
threads = letter.branch(70, length=220, fork_spacing=60, forks=2, spread=0.5, angle=40, wander=35, seed=7)
forked = threads.without(forks=0).slice(0, growth).outline(3, profile=TAPER)
trunks = threads.only(forks=0).slice(0, growth).outline(6, profile=TAPER)
drawing = btvec.Drawing(
    btvec.Path.rect(0, 0, WIDTH, HEIGHT).fill(PAPER),
    btvec.Drawing(letter, forked, trunks).fill(INK),
    size=(WIDTH, HEIGHT),
)

# The frames of the piece, as SVG text.
frames = {"grown": drawing.to_svg(WIDTH, HEIGHT), "half": drawing.to_svg(WIDTH, HEIGHT, growth=0.5)}
