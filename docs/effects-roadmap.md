# Effects roadmap

This file records what shipped. Planned work is tracked outside this repository.

The effects track ([roadmap.md](roadmap.md), "E Effects track"): the effects built so far, each added with a gallery piece.

An effect is a finished look: named, with described parameters and good defaults, built from the library's primitives,
and added only with a gallery piece that uses it ([design-decisions.md](design-decisions.md), "Effects earn their place
with a gallery piece"). The primitive comes from its batch; the effect adds the look.

| Effect | What it does | Built on |
|---|---|---|
| Draw-on, write-on, marching dashes | an outline drawing itself, text appearing as if written, dashes moving along a path (presets `draw_on`, `marching_dashes`) | V7, V11 |
| Wobble, wave, twist, bulge | deformations with tuned defaults; small amplitudes (a large wobble did not earn a place) | V12 |
| Arch and envelope type | a line of text bent on an arch or fitted into an envelope shape (preset `bent_type`) | V5, V12 |
| Glyph-to-shape morph | a letter turning into a shape and back | V5, V13 |
| Halftone dots | a tone drawn as a dot screen: dot shapes and screen geometry ([dither-and-print.md](dither-and-print.md)) | V10, 4a |
| Radial and linear repeaters | copies along a line or around a center, each with its copy index (`repeat`); `examples/glyph_scatter.py` scatters by a seeded random | V2, V7 |
| Tapered strokes | a stroke whose width varies along its length, thinning at free ends (preset `brush_taper`) | V11 |
| Series input | bars and waveform paths from a list of numbers (`examples/series_bars.py`) | V2 |

Halftone dots are the vector-native version of an effect usually applied to pixels: a colour and geometry operation on
shapes, so the output stays vector ([design.md](design.md), "Effects").

Tried in the prototypes and not taken: halftone clipped inside letters, thin outline echoes, a large outline wobble,
concentric evenly spaced offsets ([prototype-findings.md](prototype-findings.md)).
