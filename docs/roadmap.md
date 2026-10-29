# Roadmap

Status: draft for review, 2026-10-28. What the batches build: [design.md](design.md).

Small batches of work, in order. Each batch gets its own plan before work starts, ends merged into `main` with the
whole gate green and an independent review done, and leaves the worked examples it names passing. Sizes: **S** about
one working session, **M** two or three, **L** four or more.

Order: a short feedback loop first (the gate, a walking skeleton through every target, a way to see a drawing, a way to
export frames), then the code API, then type, then the risky geometry, then the generative layer in the order the
reference pieces need it, then an open-ended effects track. The prototypes
([prototype-findings.md](prototype-findings.md)) set this order.

| # | Batch | Size | Milestone |
|---|---|---|---|
| V0 | Taste prototype: the three reference pieces, throwaway, outside the repository | M | |
| S0 | Scaffold and walking skeleton: workspace, gates, hooks, CLAUDE.md; one function from Rust core to Python, core built for WebAssembly | M | |
| S0b | CI and a wheel per platform (before `main` is first pushed) | S | |
| V1 | The code API (Rust, Python, JavaScript, examples first) and the JSON format | M | |
| V2 | Core: `Path` as a value, shapes, attributes, the node trait, parameters, registry, evaluation; `polygon`, `star`, `transform`, `group` | M | |
| V3 | Output: SVG, pixels, `look` (draw a drawing and open it); the WebAssembly package; the same bytes on every target | M | **M1 a star everywhere** |
| V4 | Frame export: a caller's function of time to a transparent PNG sequence or clip | S | |
| V5 | Fonts and shaping: glyphs as shapes, variable axes as parameters, bundled fonts | L | |
| V6 | Layout: wrap, align, tracking, leading, fit to a box; text on a path, text inside a shape | M | |
| V7 | Per-item variation: stagger, split, per-item transforms | M | **M2 type** |
| V8 | Booleans, with fuzzing and documented limits | L | |
| V9 | Offset, inset, stroke outline, offset rings | M | |
| V10 | SVG fidelity (shadows, halftone, knockouts) and a fast halftone fill | M | |
| V11 | Path modifiers: draw-on, dash march, along a path, simplify, resample | M | |
| V12 | Deformers: noise displace, wobble, twist, bulge, wave | M | **M3 kinetic type piece** |
| V13 | Morph, any shape to any shape | M | |
| V14 | Trace: grayscale levels and colour labels to outlines, steady over a sequence of maps | L | **M4 cut-paper piece** |
| V15 | Fields and lines: evenly spaced streamlines, direction and distance fields, hatching, plotter output | L | **M5 poster piece** |
| V16 | SVG import (logos) | S | |
| V17 | Presets as JSON | S | |
| V18 | Command line: `render`, `describe`, `list`, `validate` | S | |
| V19 | Presets and gallery | M | |
| V20 | Browser playground and a live page example | M | |
| V21 | Reference pieces rebuilt on the library | M | |
| V22 | First release: crates.io, PyPI, npm | S | **M6 release** |
| E | Effects track: one effect and its gallery piece per batch, open-ended, from V15 on ([effects-roadmap.md](effects-roadmap.md)) | S each | |

V13 and V16 can move once what they build on exists (V11, V3).

## V0 Taste prototype

Throwaway, outside the repository, in whatever is fastest. Build the three reference pieces far enough to judge them.

Done when: each piece exists as a still or a short clip; a written list of what each one needed (nodes, parameters,
which values varied per letter or per layer) and what made it look good. That list reorders or trims the batches after
V3.

State: done, in Python. What they taught, and the reordering it caused: [prototype-findings.md](prototype-findings.md).

## S0 Scaffold and walking skeleton

The feedback loop before any feature: the gate in two speeds (seconds while working, the whole gate before a merge),
hooks that stay fast, and one real function crossing every boundary (Rust core, Python package, WebAssembly build), so
toolchain problems show now, not in V3.

Done when: `uv run poe fast` and `uv run poe check` pass on a fresh clone after `uv sync && uv run poe setup`; each gate
has a test showing it fails on a known-bad input; the skeleton function behaves the same from Rust and Python.

State: done.

## S0b CI and wheels

GitHub workflows (pinned actions, minimal permissions, linted offline), wheels per platform, the build checks of the
Python distribution. Before `main` is first pushed.

State: done.

Carried to later:
- First real CI run on GitHub (the owner pushes): fix whatever it shows, and verify that rustup adds the `wasm32` target
  of `rust-toolchain.toml` to an already installed toolchain on a clean runner. Not run before: the macOS and Windows legs,
  the arm runners (`ubuntu-24.04-arm`, `macos-15-intel`), the whole of `build.yml` (the manylinux container, the macOS
  cross-compile, the smoke installs), CodeQL and dependency review, the `changes` filter and `ci-ok` on real events.
  Also unproven: Dependabot's uv and cargo ecosystems (only schema-checked), the job timeouts (guesses), install-action
  given the version strings `tools/rust_tools.py specs` prints, maturin picking up the pinned toolchain inside the
  manylinux container, `merge_group` runs, Windows paths in the smoke install.
- A test that every path in `clippy.toml` resolves (clippy only warns for a mistyped path) and that every entry has a
  proof row: V2.
- More unbanned IO and clock sources (`env::args`, `current_dir`, `fs::read_dir`, `metadata`, `net`, `thread::sleep`,
  `process`): V2, when the core reads bytes.

## V1 The code API and the JSON format

Design the API first, as the examples a user would write: the three reference pieces and a dozen small drawings, in
Rust, Python and JavaScript, reviewed for how they read; the owner reviews a first draft before the batch starts. Then
the JSON format as the same tree written down. The four ideas (`Path`, `Shape`, `Drawing`, `Param`) are decided
(design.md, "The code API"); this batch settles their method names.
- `docs/api.md`: building a drawing, naming nodes, setting values, listing parameters, rendering.
- `docs/formats.md`: the drawing (node tree, values, expressions, version), parameter descriptions (name, unit, range,
  default, meaning, cyclic or bounded), map input (width, height, bytes).
- JSON Schema in `schemas/`, generated from the Rust declarations and committed.

Done when: the example programs are written and agreed; each round-trips through JSON in a test (as soon as V2 can run
them); the schema rejects known-bad drawings.

## V2 Core

- `Path` as a value: build, measure (length, point and tangent at a distance), split, slice, reverse, hit-test. Each
  later batch adds its verbs to `Path` (computed now) and `Drawing` (a step) together.
- Shapes: paths, style (fill, stroke, opacity), attributes.
- The node trait: inputs in, shapes out, parameters declared once; the chainable builder methods of V1 over it.
- Evaluation of a tree; a seed reaches every node; a cache so that changing one value re-evaluates only what depends
  on it.
- Parameters: describe, validate, scale design pixels by output size, wrap cyclic angles.
- Registry by name. Built-ins: `polygon`, `star`, `transform`, `group`. Replaces S0's skeleton function.
- Contract tests over every registered node: determinism, values within range, the same result at any size.

Carried from the S0 review:
- `polygon_svg` accepts radii that draw nothing (viewBox `0 0 0 0` at radius 1e-9) and rounds to 6 fixed decimals, which
  does not scale with size: V2 needs relative precision or a real lower bound.
- The proptest radii are uniform, so small radii are almost never sampled: use log-uniform.
- The `Error` shape (`parameter: &'static str`, public fields, derived `Eq`) will not fit nested node paths: mark it
  non-exhaustive or redesign it in V2.
- Clippy checks only this workspace's code, so the curve, boolean and raster crates chosen in V2 must use `libm` for
  their maths; otherwise the V3 native-against-WebAssembly differential test is the only guard.

## V3 Output, WebAssembly, the same bytes everywhere

- SVG writer; pixels at any size on the CPU.
- `uv run poe look <drawing>`: draws to PNG and SVG in a git-ignored folder, for a person or an agent to look at.
- The WebAssembly package (wasm-bindgen): build, a smoke test in Node.
- Differential test: the core's drawings rendered natively and in WebAssembly give identical bytes.
- Benchmark: time to draw a reference drawing at 1080p, tracked from here on.

## V4 Frame export

A small helper outside the core: it loops a caller's function (time in, parameter values out) over frames at a frame
rate and writes a transparent PNG sequence, or ProRes 4444 or WebM with alpha through ffmpeg. No timeline, keyframes or
mixing.

Done when: a worked example writes a PNG sequence and a clip with alpha, and each frame equals the drawing drawn
directly with that frame's values.

## V5 Fonts and shaping

- Fonts from bytes: OpenType, TrueType, variable fonts; two or three open-licence fonts bundled. Whether WOFF and WOFF2
  load, or need unpacking first, is checked here.
- Shaping with ligatures, kerning, complex scripts and OpenType features; glyph outlines as shapes with attributes
  (index in text, word, line; position 0..1); overlapping contours removed, so every glyph is a valid region.
- Variable axes (weight, width, slant, custom) as parameters of the text node; per-glyph axis values, with advances that
  follow them.

## V6 Layout

Wrap to a width, alignment, tracking, leading; fit a line to a width by a variable axis, at a reference pose separate
from the live values, saying when the axis runs out; text along a path; text filling a shape.

## V7 Per-item variation

- Stagger: one value (say `progress`) becomes a per-item value spread by an attribute, for any modifier.
- Split a text into letters, words or lines as separate shapes; per-item transforms (offset, rotate, scale by index or
  by a seeded random) about a typographic origin (cap-height centre, not the bounding box).

Done when: a word draws on letter by letter and scatters by index with no text-specific modifier.

## V8 Booleans

Union, subtract, intersect, exclude, on any shapes. The riskiest batch: build on an existing crate, fuzz with random and
degenerate paths (overlaps, touching edges, tiny segments), document what fails.

## V9 Offsets

Offset and inset with round, miter and bevel joins; outline a stroke into a fill; offset rings (repeated offsets).

## V10 SVG fidelity and a fast halftone fill

- Every effect so far written into SVG as drawn in pixels: drop shadows, halftone, knockouts, inks multiplied over
  paper. An effect with no SVG form fails a test.
- Halftone as a fill (a screen: pitch, angle, dot size from a tone) clipped to any shape, without a boolean per dot;
  timed by the V3 benchmark.

## V11 Path modifiers

Draw-on (partial path by length, from a canonical start point and direction), dash with a moving phase, place shapes
along a path, simplify, resample to even spacing.

## V12 Deformers

Noise displace (seeded, moving by `phase`), wobble, twist, bulge, wave. Paths are resampled first so deformation is
smooth at any size.

## V13 Morph

Any shape to any shape, matching points when the counts differ.

## V14 Trace

- A grayscale map in (width, height, bytes), outlines at chosen levels out; each level a shape with its level as an
  attribute.
- Multi-colour trace: a map of quantised colour labels to one set of outlines per colour.
- Curves fitted through the traced edge (a raw polyline looks digital); edge padding, so a subject cut by the map's
  edge gets no false closed edge.
- Per-layer shrink and grow (open, close) in output units; dropping parts and filling holes below an area.
- Helpers for a sequence of maps from the caller: smoothing over neighbouring maps, and hole persistence (a hole shows
  only if neighbouring maps have it). The library keeps no sequence itself.

With V9 this is the cut-paper piece.

## V15 Fields and lines

- Evenly spaced streamlines (the Jobard-Lefer method) with spacing that varies with a tone, region masks, obstacles and
  closed loops.
- Line fields (directions without sign: noise, a shape's contour tangents, and mixes of them) and distance fields of
  shapes (signed, and normalised for letters with counters).
- Hatching clipped to shapes; contour lines of a field or a map.
- Plotter output: stroke only, ordered to reduce pen travel (nearest neighbour, ends flipped), simplified, solids as
  pen-width hatching, one layer per pen.

Placement that stays stable as values change (design.md, D7) is needed before hatching is animated; the poster is a
still.

## V16 SVG import

Logos and other SVG paths in as shapes.

## V17 to V20

Presets as JSON; the command line; presets and a gallery that look good with no tuning; a browser page that loads a
drawing and builds sliders from its parameter descriptions, and a live page example: a drawing redrawn on a canvas from
values the page supplies (the clock, scroll, pointer).

## V21 Reference pieces

The three pieces rebuilt on the library, animated by a script that sets values per frame, exported with V4.

## E Effects track

Open-ended, from V15 on, alongside V16 to V22. One effect per small batch, merged with its gallery piece. Candidates,
and what each builds on: [effects-roadmap.md](effects-roadmap.md).

## V22 First release

Names reserved and published on crates.io, PyPI and npm; changelog; README with a scenario and a "what gap does it
fill" section, claims about other projects checked again.

## Later, with a trigger

- GPU rendering: when the CPU path is too slow for a live preview.
- Text-editing support (cursor, selection, hit testing within text): when an editor is started.
- Multi-threaded rendering: when batch rendering needs it.
- Custom nodes in Python or JavaScript (D6): when a user wants one.
- Colour and bitmap fonts: after v1, when a piece needs them.
- Stateful simulation (D5).
