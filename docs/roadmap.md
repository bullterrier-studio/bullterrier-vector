# Roadmap

Status: draft for review, 2026-10-28. What the batches build: [design.md](design.md).

Small batches of work, in order. Each batch gets its own plan before work starts, ends merged into `main` with the
whole gate green and an independent review done, and leaves the worked examples it names passing. Sizes: **S** about
one working session, **M** two or three, **L** four or more.

Order: a short feedback loop first (the gate, a walking skeleton through every target, a way to see a drawing), then
the code API, then type, then the risky geometry, then the generative layer in the order the reference pieces need it.

| # | Batch | Size | Milestone |
|---|---|---|---|
| V0 | Taste prototype: the three reference pieces, throwaway, outside the repository | M | |
| S0 | Scaffold and walking skeleton: workspace, gates, hooks, CLAUDE.md; one function from Rust core to Python, core built for WebAssembly | M | |
| S0b | CI and a wheel per platform (before `main` is first pushed) | S | |
| V1 | The code API (Rust, Python, JavaScript, examples first) and the JSON format | M | |
| V2 | Core: items, attributes, the node trait, parameters, registry, evaluation; `polygon`, `star`, `transform`, `group` | M | |
| V3 | Output: SVG, pixels, `look` (draw a drawing and open it); the WebAssembly package; the same bytes on every target | M | **M1 a star everywhere** |
| V4 | Fonts and shaping: glyphs as items, variable axes as parameters, bundled fonts | L | |
| V5 | Layout: wrap, align, tracking, leading; text on a path, text inside a shape | M | |
| V6 | Per-item variation: stagger, split, per-item transforms | M | **M2 type** |
| V7 | Booleans, with fuzzing and documented limits | L | |
| V8 | Offset, inset, stroke outline, offset rings | M | |
| V9 | Path modifiers: draw-on, dash march, along a path, simplify, resample | M | |
| V10 | Deformers: noise displace, wobble, twist, bulge, wave | M | **M3 kinetic type piece** |
| V11 | Trace: grayscale image to outlines at levels | M | **M4 cut-paper piece** |
| V12 | Fields and lines: flow-field lines, hatching inside shapes, contours | L | **M5 poster piece** |
| V13 | Repeaters and generators: radial, grid, along a path, scatter; Voronoi, truchet, superformula, blob | M | |
| V14 | Morph, any shape to any shape | M | |
| V15 | Series input (bars, waveform paths) and SVG import (logos) | S | |
| V16 | Presets as JSON | S | |
| V17 | Command line: `render`, `describe`, `list`, `validate` | S | |
| V18 | Presets and gallery | M | |
| V19 | Browser playground | M | |
| V20 | Reference pieces rebuilt on the library | M | |
| V21 | First release: crates.io, PyPI, npm | S | **M6 release** |

V0 runs alongside S0 (it touches nothing here). V13 to V15 can run in any order after V9.

## V0 Taste prototype

Throwaway, outside the repository, in whatever is fastest (Paper.js, p5, Python). Build the three reference pieces far
enough to judge them.

Done when: each piece exists as a still or a short clip; a written list of what each one needed (nodes, parameters,
which values varied per letter or per layer) and what made it look good. That list reorders or trims V4 to V15.

## S0 Scaffold and walking skeleton

The feedback loop before any feature: the gate in two speeds (seconds while working, the whole gate before a merge),
hooks that stay fast, and one real function crossing every boundary (Rust core, Python package, WebAssembly build), so
toolchain problems show now, not in V3.

Done when: `uv run poe fast` and `uv run poe check` pass on a fresh clone after `uv sync && uv run poe setup`; each gate
has a test showing it fails on a known-bad input; the skeleton function behaves the same from Rust and Python.

## S0b CI and wheels

GitHub workflows (pinned actions, minimal permissions, linted offline), wheels per platform, the build checks of the
Python distribution. Before `main` is first pushed.

## V1 The code API and the JSON format

Design the API first, as the examples a user would write: the three reference pieces and a dozen small drawings, in
Rust, Python and JavaScript, reviewed for how they read. Then the JSON format as the same tree written down.
- `docs/api.md`: building a drawing, naming nodes, setting values, listing parameters, rendering.
- `docs/formats.md`: the drawing (node tree, values, version), parameter descriptions (name, unit, range, default,
  meaning, cyclic or bounded), image input (width, height, bytes), series input.
- JSON Schema in `schemas/`, generated from the Rust declarations and committed.

Done when: the example programs are written and agreed; each round-trips through JSON in a test (as soon as V2 can run
them); the schema rejects known-bad drawings.

## V2 Core

- Items: path, style (fill, stroke, opacity), attributes.
- The node trait: inputs in, items out, parameters declared once; the chainable builder methods of V1 over it.
- Evaluation of a tree; a seed reaches every node; a cache so that changing one value re-evaluates only what depends
  on it.
- Parameters: describe, validate, scale design pixels by output size, wrap cyclic angles.
- Registry by name. Built-ins: `polygon`, `star`, `transform`, `group`. Replaces S0's skeleton function.
- Contract tests over every registered node: determinism, values within range, the same result at any size.

## V3 Output, WebAssembly, the same bytes everywhere

- SVG writer; pixels at any size on the CPU.
- `uv run poe look <drawing>`: draws to PNG and SVG in a git-ignored folder, for a person or an agent to look at.
- The WebAssembly package (wasm-bindgen): build, a smoke test in Node.
- Differential test: the core's drawings rendered natively and in WebAssembly give identical bytes.
- Benchmark: time to draw a reference drawing at 1080p, tracked from here on.

## V4 Fonts and shaping

- Fonts from bytes: OpenType, TrueType, variable fonts; two or three open-licence fonts bundled.
- Shaping with ligatures, kerning and complex scripts; glyph outlines as items with attributes (index in text, word,
  line; position 0..1).
- Variable axes (weight, width, slant, custom) as parameters of the text node.

## V5 Layout

Wrap to a width, alignment, tracking, leading; text along a path; text filling a shape.

## V6 Per-item variation

- Stagger: one value (say `progress`) becomes a per-item value spread by an attribute, for any modifier.
- Split a text into letters, words or lines as separate items; per-item transforms (offset, rotate, scale by index or
  by a seeded random).

Done when: a word draws on letter by letter and scatters by index with no text-specific modifier.

## V7 Booleans

Union, subtract, intersect, exclude, on any items. The riskiest batch: build on an existing crate, fuzz with random and
degenerate paths (overlaps, touching edges, tiny segments), document what fails.

## V8 Offsets

Offset and inset with round, miter and bevel joins; outline a stroke into a fill; offset rings (repeated offsets).

## V9 Path modifiers

Draw-on (partial path by length), dash with a moving phase, place items along a path, simplify, resample to even
spacing.

## V10 Deformers

Noise displace (seeded, moving by `phase`), wobble, twist, bulge, wave. Paths are resampled first so deformation is
smooth at any size.

## V11 Trace

A grayscale image in, outlines at chosen levels out, smoothed and simplified; each level an item with its level as an
attribute. With V8 this is the cut-paper piece.

## V12 Fields and lines

Flow-field lines (seeded, evenly spaced), hatching clipped to shapes, contour lines of a field or an image.
Plotter-ready: lines only, ordered to reduce pen travel.

## V13 to V15

Repeaters (radial, grid, along a path, scatter with even spacing), generators (Voronoi, Delaunay, truchet,
superformula, blob), morph between any two shapes (matching points when the counts differ), series input, SVG import.

## V16 to V19

Presets as JSON; the command line; presets and a gallery that look good with no tuning; a browser page that loads a
drawing and builds sliders from its parameter descriptions.

## V20 Reference pieces

The three pieces rebuilt on the library, animated by a script that sets values per frame.

## V21 First release

Names reserved and published on crates.io, PyPI and npm; changelog; README with a scenario and a "what gap does it
fill" section, claims about other projects checked again.

## Later, with a trigger

- GPU rendering: when the CPU path is too slow for a live preview.
- Hit testing and text-editing support (cursor, selection): when an editor is started.
- Multi-threaded rendering: when batch rendering needs it.
- Custom nodes in Python or JavaScript (D6): when a user wants one.
- Stateful simulation (D5).
