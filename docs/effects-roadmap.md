# Effects roadmap

Candidates for the effects track ([roadmap.md](roadmap.md), "E Effects track"). Status: a list, not an order or a
promise; the next effect is the one a piece wants.

An effect is a finished look: named, with described parameters and good defaults, built from the library's primitives,
and added only with a gallery piece that uses it ([design-decisions.md](design-decisions.md), "Effects earn their place
with a gallery piece"). The primitive comes from its batch; the effect adds the look.

| Effect | What it does | Builds on |
|---|---|---|
| Draw-on, write-on, marching dashes | an outline drawing itself, text appearing as if written, dashes moving along a path | V7, V11 |
| Wobble, wave, twist, bulge | deformations with tuned defaults; small amplitudes (a large wobble did not earn a place) | V12 |
| Arch and envelope type | a line of text bent on an arch or fitted into an envelope shape | V5, V12 |
| Glyph-to-shape morph | a letter turning into a shape and back | V5, V13 |
| Glitch slice-and-shift | shapes cut into bands, the bands shifted sideways | V8 |
| Duotone and gradient maps | fill and stroke colours mapped by lightness onto two colours or a gradient | V2 |
| Posterize | fill and stroke colours reduced to a few levels | V2 |
| Halftone dots | a tone drawn as a dot screen | V10 (built: `halftone`) |
| Scramble and flip letters | letters cycling through other glyphs, or flipping, before they settle | V5, V7 |
| Die-cut sticker outlines with shadows | a thick, rounded outline cut around a shape, lifted by a drop shadow | V9, V10 (both built) |
| Ink-bleed edges | edges roughened like ink spreading into paper; taken up after V12 | V9, V12 |
| Radial, grid and scatter repeaters | copies around a centre, on a grid, or scattered with even spacing, each with its copy index | V2, V7 |
| Perspective floor | a grid, or any drawing, laid on a ground plane receding to a horizon: rows close up toward the horizon, rays meet at a vanishing point (the synthwave floor); a projective deformer under the grid repeater. A prototype wrote this as hand maths | V2, V7, V12 |
| Smooth blob merging | shapes that melt into each other where they come close | V8, V9 |
| Tapered strokes | a stroke whose width varies along its length, thinning at free ends | V11 |
| Riso inks | inks multiplied over paper, each misregistered by its own offset | V8, V10 |
| Voronoi and Delaunay, truchet, superformula | generators | V2 |
| Series input | bars and waveform paths from a list of numbers | V2 |

Duotone, gradient maps, posterize, halftone dots, glitch slices and misregistration (the offsets of riso inks) are the
vector-native versions of effects usually applied to pixels: colour and geometry operations on shapes, so the output
stays vector ([design.md](design.md), "Effects").

Tried in the prototypes and not taken: halftone clipped inside letters, thin outline echoes, a large outline wobble,
concentric evenly spaced offsets ([prototype-findings.md](prototype-findings.md)).
