# Prototype findings

What the taste prototypes (roadmap V0) taught. The design: [design.md](design.md); the batches:
[roadmap.md](roadmap.md).

On 2026-10-28 the three reference pieces were built as throwaway Python prototypes, outside this repository, on existing
libraries (skia-python for drawing and path booleans, shapely for offsets, fontTools for glyph outlines, numpy). Their
code is not kept and nothing here comes from it; this page is what they showed. The kinetic type and the cut paper were
judged as stills and a short 1080x1350 clip set to music, the poster as stills.

## The pieces

| Piece | What was built | Verdict |
|---|---|---|
| Kinetic type | Two lines of condensed type in a variable font (width and weight axes) flicker in as hairlines, are knocked out by a rising colour plane, burst into a two-colour riso print on the drop, pulse letter by letter, and thin back to hairlines. | Worth posting; the build, drop and final hold read as real posters. The groove is one pose with a pulse and gets samey. |
| Cut paper | A segmentation mask of a dancer traced into a black cut-out on stacked paper: a blobby mount with halftone, two shifted echoes, drop shadows, a line drawing on around the figure on each beat. | The stills are worth posting. The clip is uneven: the source's poses decide half the result, and the mask merges two people who touch. |
| Hatched poster | Two words in a heavy, condensed variable font, filled with evenly spaced flow-field hatching that grows denser towards the bottom until it closes into solid ink; one letter drawn as concentric rings in a second ink. As PNG and as plotter SVG. | Reads as a designed print poster, not a code demo. The upper half is evenly ruled and a little like an engraving. |

## What looked good

- **Per-letter stagger:** one value delayed by index (each letter a little later) is the signature of kinetic type.
  Seeded per-letter scatter and rotation work on a hit; more than about ±11° of tilt reads as a ransom note.
- **A width axis:** it makes type kinetic and fills a portrait frame; a font without one (tried) made a prettier still
  but could not fill the frame.
- **Knockouts:** a flat plane rising behind the letters and cutting them out (a boolean) was the best idea in the
  kinetic piece.
- **Riso inks:** each ink a shape built with booleans, inks multiplied over paper, misregistration that grows on a hit.
  A constant offset looked like a bug.
- **Cut paper as one fat, blobby mount** (closing radius 60-80 px on the outer layers) **plus two echoes the size of the
  figure, shifted in opposite directions, plus drop shadows.** Layers slide out and tuck back; they do not fade.
- **Evenly spaced streamlines** (the Jobard-Lefer method) with spacing driven by tone, 14 px down to 2.4 px, and line
  width 2 to 3 px: they close into a solid whose edge follows the field, the best moment of the poster.
- **One accent letter** in a second ink gave the poster its focal point.
- **Type under the paper layers**, as on a pasted-up poster, rather than on top.

## What did not

- **Halftone clipped inside letters:** dots of one ink on another went muddy and killed the contrast.
- **Thin outline echoes** of fat letters: read as wireframe scribble.
- **Large outline wobble:** melty, off-brief, with spikes at sharp corners where normals flip. A small wobble on one
  hit was kept, and nobody would miss it.
- **Concentric, evenly spaced offsets:** four even halos look like a contour map, a generic offset demo. So do iso-lines
  at several levels of one mask.
- **Fading layers:** translucent paper looks wrong.
- **Seeded random streamlines:** they bunch into ropes, leave holes, and cannot make a tone ramp.
- **Thin lines at the light end** (1-1.8 px at 8 px spacing): banknote engraving, not print poster.
- **Flow lines bending towards letter edges:** fussy hooks at every edge.
- **Re-placing streamlines on every frame:** most regions hold for a few frames, then whole bundles jump (design.md,
  Open decisions, D7).

## Numbers

All at 1080x1350, one process unless stated, in Python.

| What | Measured |
|---|---|
| Kinetic type, per frame | median 23 ms; 9-21 ms in the quiet sections; 320 ms with the noise wobble; 510 ms with the halftone |
| Cut paper, per frame | 160-250 ms: about 50 ms smoothing the mask over neighbouring frames, 30 ms offsets, 60-100 ms drawing shadows and about 3,000 halftone dots |
| Hatched poster, per still | 3-8 s, of which 1.2-4 s placing streamlines (shared machine, varying 2-3x with load) |
| Halftone built as thousands of shapes through general booleans | about 500 ms a frame |
| Traced edge jitter (distance from the average of its neighbours) | 2.5 px mean (5.4 px p95) raw; unchanged by blurring each frame; 0.57 px (1.2 px p95) after averaging over neighbouring frames (Gaussian, σ 1.2 frames) |
| Frame pairs where a part or a hole appeared or vanished | 46 of 191; 21 with hole persistence (a hole shows only if neighbouring frames have it). σ 2 frames alone gave 34, but ghosted fast arms |
| Pen-up travel of the plotter file | 27.7 m before, 2.55 m after nearest-neighbour ordering with end flipping (732 paths, 26.7 m pen-down); a shuffled order would be 80-95 m. Two other words: 24.1 to 2.15 m, 23.8 to 2.55 m |
| Plotter SVG size | 2 MB before, 140-160 KB after simplifying at 0.02 mm |
| One word at width 50 of a variable font | 387 units wide at weight 100, 1698 at weight 900 |
| Boolean and offset failures | none, once glyph outlines had their overlaps removed |

## Lessons

- **Fit at a calm pose.** Fitting a line to a width at the live axis values shrinks the word on every pulse; fitting
  once lets pulses overflow. Fit at a reference pose, separate from the live values.
- **Glyphs must become a valid region.** Variable-font outlines keep overlapping contours, and winding differs between
  fonts; a first heuristic broke a digit in another font. Removing overlaps (nonzero fill) first fixed it.
- **Line fields, not vector fields:** hatching directions have no sign, so they blend in doubled-angle space. Noise, a
  glyph's contour tangents and their mixes are one type.
- **Distance fields of shapes:** a normalised distance (outside over outside plus inside) gives seam-free rings in
  letters with counters; plain offsets crease along the medial axis. A letter with two counters still gives a saddle.
- **Draw-on needs a canonical start** (topmost point, fixed orientation), or it jumps between frames.
- **Shrink and grow per layer, in output pixels** (open and close), plus dropping small parts and filling small holes:
  the biggest single lever between "traced" and "designed".
- **Curves through the traced polyline:** a raw polyline edge looks digital.
- **Pad the map's edge**, or a subject cut by the frame gets a false closed edge when offset inward.
- **The SVG must carry every effect.** We found that skia-python 144.1's SVG canvas left out every shape drawn with a
  shadow filter in our test, so all shadowed layers vanished; we did not check other versions. A hand-written SVG writer
  replaced it.
- Noise on a coarse lattice with bilinear interpolation looked identical to full resolution and was 20x faster.

## What the library needs, ranked

Ordered by the pain each caused, merged across the three pieces (each prototype ranked its own); the batch that answers
it in brackets.

1. Per-glyph variable-font layout that re-flows: axis values per glyph, advances that follow them, tracking, stacked
   lines, fitting to a width at a reference pose, glyph outlines cached by axis location (V5 to V7).
2. Evenly spaced streamlines with spacing from a tone, region masks, obstacles and closed-loop detection, fast enough to
   be native; line fields and distance fields of shapes (V15).
3. Steady tracing over a sequence of maps: smoothing over neighbouring maps and hole persistence (V14).
4. Booleans that hold on overlapping, rotated glyphs, and ink layers built from them (V8; riso inks in the effects
   track).
5. Shrink and grow per layer in output units; dropping small parts and filling small holes (V14).
6. Per-item transforms about a typographic origin (cap-height centre, not the bounding box), seeded randomness and
   stagger by index as one mechanism (V7).
7. Curve fitting through dense polylines (V14).
8. One picture to both pixels and SVG, every effect kept (V10).
9. A fast halftone fill (V10).
10. Plotter export: stroke only, ordered, simplified, solids as pen-width hatching, one layer per pen (V15).
11. A variable-width stroke, tapered at free ends (effects track).
12. Partial path by length from a stable start (V11).
13. Fitting text to a box by a variable axis, with a clear signal when the axis runs out (V6).

## Not needed by any piece

Kerning and shaping (all capitals, even tracking; still in scope for real text), Voronoi, truchet, superformula and
series input (moved to the effects track), contours at several levels, re-simplifying per frame, opacity fades. Morph
was not used, but is wanted.

Left out of the library for now: the tracking crop that kept a subject framed, picking the main subject when people
touch (it needs a segmenter's instance labels), and the score helpers that turned beats into values over time (the
caller's: design.md, Non-goals; pure easing functions of a number are open, D8).
