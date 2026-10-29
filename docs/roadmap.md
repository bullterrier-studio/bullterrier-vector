# Roadmap

Status: draft for review, 2026-10-28. What the batches build: [design.md](design.md).

Small batches of work, in order. Each batch gets its own plan before work starts, ends merged into `main` with the
whole gate green and an independent review done, and leaves the worked examples it names passing. Sizes: **S** about
one working session, **M** two or three, **L** four or more.

Order: a short feedback loop first (the gate, a walking skeleton through every target), then the code API and the core,
a way to see a drawing (with two spikes that measure a GPU display), a way to export frames, then type, then the risky
geometry, then the generative layer in the order the reference pieces need it, then an open-ended effects track. The
prototypes ([prototype-findings.md](prototype-findings.md)) set this order.

| # | Batch | Size | Milestone |
|---|---|---|---|
| V0 | Taste prototype: the three reference pieces, throwaway, outside the repository | M | |
| S0 | Scaffold and walking skeleton: workspace, gates, hooks, CLAUDE.md; one function from Rust core to Python, core built for WebAssembly | M | |
| S0b | CI and a wheel per platform (before `main` is first pushed) | S | |
| V1a | The code API (Rust, Python, JavaScript, examples first) | M | |
| V1b | The JSON format: the same tree written down | M | |
| V2 | Core: `Path` as a value, shapes, attributes, the node trait, parameters, registry, evaluation; `Path.polygon`, `Path.star`, transforms, stacking | L | |
| V3 | Output: SVG, pixels, a mesh, `look` (draw a drawing and open it); the WebAssembly package; the same bytes on every target | L | **M1 a star everywhere** |
| X1 | Spike: a GPU display of the mesh in a page, measured | S | |
| X2 | Spike: live remeshing every frame, measured | S | |
| V4 | Frame export: a caller's function of time to a transparent PNG sequence or clip | S | |
| V5 | Fonts and shaping: glyphs as shapes, variable axes as parameters, bundled fonts | L | |
| V6 | Layout: wrap, align, tracking, leading, fit to a box; text on a path, text inside a shape | M | |
| V7 | Per-item variation: stagger, split, per-item transforms | M | **M2 type** |
| V8 | Booleans, with fuzzing and documented limits | L | |
| V9 | Offset, inset, stroke outline | M | |
| V10 | SVG fidelity: drop shadows and knockouts as style attributes, the same in SVG and pixels; a fast halftone fill | M | |
| V11 | Path modifiers: draw-on, dash march, along a path, simplify, resample | M | |
| V12 | Deformers: wobble, wave, twist, bulge | M | **M3 kinetic type piece** |
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

V13 (morph) builds on V2 and V5, and V16 (SVG import) on V2; either can move earlier if a piece needs it.

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
- After the first CI run: the `build.yml` items above, and an edge case of the `changes` filter, `docs/images/.png` (a bare
  extension; unchecked whether the `*.png` exclusion matches it).
- V2, from the review of the gates:
  - A stray non-test file in the package passes check-wheel-contents: add an allow-list of entry kinds when the package grows.
  - `_git` raises `FileNotFoundError` when git is missing (it fails safe).
  - `test_gate_wiring` matches commands by exact string.
  - `check_dists` reads sdist members and `license-files` literally; globs would false-fail.
  - Parity between `check` and the workflows is a convention only: no test compares `lint.yml` and `test.yml` with
    `check`. Consider one.
  - pip-audit has no place to record ignored advisories (cargo-deny has `deny.toml`).
  - Dependabot does not update the `rev:` pins in `.pre-commit-config.yaml`, `rust-toolchain.toml` or
    `[workspace.metadata.tools]` in `Cargo.toml`.
- A test that every path in `clippy.toml` resolves (clippy only warns for a mistyped path) and that every entry has a
  proof row: V2.
- More unbanned IO and clock sources (`env::args`, `current_dir`, `fs::read_dir`, `metadata`, `net`, `thread::sleep`,
  `process`): V2, when the core reads bytes.

What to expect from the first CI run, most likely first:
1. The Windows test legs: never run.
2. `build.yml`: maturin-action and the pinned toolchain inside the manylinux container; adding the macOS cross target to the
   pinned toolchain; Windows paths in the smoke job; the sdist smoke relying on rustup.
3. The gate proofs on a cold Linux runner: the timeouts are guesses.
4. CodeQL: turn off the organisation's "default setup", or the upload is rejected.
5. The install-action version lookup.
6. The arm and Intel macOS runners.
7. An advisory published since the last local audit.
8. The repository must be created empty for the first push.
9. Dependabot's first pull requests: edit their titles.
10. The `wasm32` target on a clean runner.

## V1a The code API

Design the API first, as the examples a user would write: the three reference pieces and a dozen small drawings, in
Python, the kinetic type piece also in Rust and JavaScript, reviewed for how they read. The four ideas (`Path`, `Shape`,
`Drawing`, `Param`) are decided (design.md, "The code API"); this batch settles their method names.
- `docs/api.md`: the ideas, a reference table per type, outputs, errors, conventions, the idioms of each language, the
  prototypes' needs mapped to methods, open questions.
- `docs/examples/`: the example programs, not yet run by anything.

State: done, waiting for the owner's review.

## V1b The JSON format

The drawing as the same tree written down; nothing in it the code API cannot say.
- `docs/formats.md`: the drawing (node tree, values, expressions, version; how fonts and maps are referred to),
  parameter descriptions (name, unit, range, default, meaning, cyclic or bounded), map input (width, height, bytes).
- JSON Schema in `schemas/`, generated from the Rust declarations and committed.

Done when: the format is agreed; each example program round-trips through JSON in a test (as soon as V2 can run them);
the schema rejects known-bad drawings.

## V2 Core

- `Path` as a value: build, measure (length, point and tangent at a fraction of the length), split, slice, reverse,
  hit-test. Each later batch adds its verbs to `Path` (computed now) and `Drawing` (a step) together.
- Shapes: paths, style (fill, stroke, opacity, blend mode: normal or multiply), attributes.
- The node trait: inputs in, shapes out, parameters declared once; the chainable builder methods of V1a over it.
- Evaluation of a tree; a seed reaches every node; a cache so that changing one value re-evaluates only what depends
  on it.
- Parameters: describe, validate, scale design pixels by output size, wrap cyclic angles.
- Registry by name. Built-ins: the `Path` constructors (`Path.polygon`, `Path.star` and the rest), `translate`,
  `rotate`, `scale`, stacking (`Drawing(...)`). Replaces S0's skeleton function.
- Contract tests over every registered node: determinism, values within range, the same result at any size.
- The example programs of `docs/examples/` start to run here: the Python ones move to a tested top-level `examples/`,
  the Rust one to `crates/core/examples/`, the JavaScript ones under the npm package. `docs/examples/` is never added to
  the documentation filter (`is_documentation`, the `changes` filter in `ci.yml`): its programs are code.
- Every internal sampling step (trace on an analytic field, resampling before a deformer, curve flattening, hatch
  seeding) in design pixels, independent of the output size, each a named constant (`// constant:`; api.md,
  "Conventions").

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
- Mesh output (`to_mesh`, api.md "Mesh"): triangles with positions, colours and per-item attributes, and draw ranges
  with their blend mode, in three flat buffers; knockouts resolved by booleans; a blurred shadow raises. A pointer, a
  length and a generation across WebAssembly; a view without a copy in Python. Covered by the differential tests
  below.
- `uv run poe look <drawing>`: draws to PNG and SVG in a git-ignored folder, for a person or an agent to look at.
- The WebAssembly package (wasm-bindgen): build, a smoke test in Node.
- Differential tests: the core's drawings rendered natively and in WebAssembly give identical bytes; and a drawing
  drawn at 1x and at 2x gives the same geometry (SVG and mesh coordinates at 2x are exactly twice those at 1x).
- Benchmark: time to draw a reference drawing at 1080p, tracked from here on.

## X1 Spike: GPU display

A measured experiment, not a promise. Draw V3's mesh with WebGL in a page and measure, on drawings from the reference
pieces: frame time, upload cost, and the difference from the CPU pixels (the reference). Done when the numbers are
written here with a proposal: what a GPU display would add to the roadmap, if anything, and a shader for the one step
with no triangle form, a blurred shadow (multiply already draws through the draw ranges, knockouts through booleans).

## X2 Spike: live remeshing

A measured experiment, not a promise. Re-evaluate and re-mesh a drawing from the kinetic type piece every frame in
WebAssembly, with new values each frame, and measure the time per frame at 1080p against a 60 Hz budget, knockouts
resolved by booleans on every frame; find what dominates (shaping, booleans, tessellation, the cache). Done when the
numbers are written here with a proposal.

## V4 Frame export

A small helper outside the core: it loops a caller's function (time in, a drawing with its values out; api.md, "Frame
export") over frames at a frame rate and writes a transparent PNG sequence, or ProRes 4444 or WebM with alpha through
ffmpeg. No timeline, keyframes or mixing.

Done when: a worked example writes a PNG sequence and a clip with alpha, and each frame equals the drawing drawn
directly with that frame's values.

## V5 Fonts and shaping

- Fonts from bytes: OpenType, TrueType, variable fonts; two or three open-licence fonts bundled. Whether WOFF and WOFF2
  load, or need unpacking first, is checked here.
- Shaping with ligatures, kerning, complex scripts and OpenType features; glyph outlines as shapes with attributes
  (index in text, word, line; order 0..1); overlapping contours removed, so every glyph is a valid region.
- Variable axes (weight, width, slant, custom) as parameters of the text node; per-glyph axis values, with advances that
  follow them.

## V6 Layout

Wrap to a width, alignment, tracking, leading; fit a line to a width by a variable axis, at a reference pose separate
from the live values, saying when the axis runs out; text along a path; text filling a shape.

## V7 Per-item variation

- Stagger: one value (say `progress`) becomes a per-item value spread by an attribute, for any modifier.
- Text as one shape per glyph; `split` into parts, `merge(by=...)` into words or lines; per-item transforms (translate,
  rotate, scale by index or by a seeded random) about a typographic origin (cap-height centre, not the bounding box).

Done when: a word draws on letter by letter and scatters by index with no text-specific modifier.

## V8 Booleans

Union, subtract, intersect, exclude, on any shapes. The riskiest batch: build on an existing crate, fuzz with random and
degenerate paths (overlaps, touching edges, tiny segments), document what fails.

## V9 Offsets

Offset and inset with round, miter and bevel joins; outline a stroke into a fill.

## V10 SVG fidelity and a fast halftone fill

- Drop shadows and knockouts, built here as style attributes of a shape, with the same result in SVG and in pixels.
  Multiply blending (V2's style model) is written into SVG as it is drawn in pixels, so inks multiply over paper.
- Every effect so far written into SVG as drawn in pixels. An effect with no SVG form fails a test.
- Halftone as a fill (a screen: pitch, angle, dot size from a tone) clipped to any shape, without a boolean per dot;
  timed by the V3 benchmark.

## V11 Path modifiers

Draw-on (partial path by length, from a canonical start point and direction), dash with a moving phase, place shapes
along a path, simplify, resample to even spacing.

## V12 Deformers

Wobble (seeded noise displacement, moving by `phase`), wave, twist, bulge. Paths are resampled first so deformation is
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
- Helpers for a sequence of maps from the caller: smoothing over neighbouring maps (a Gaussian, σ in maps), and hole
  persistence (a hole shows only if neighbouring maps have it). The library keeps no sequence itself.

With V9 this is the cut-paper piece.

## V15 Fields and lines

- Evenly spaced streamlines (the Jobard-Lefer method) with spacing that varies with a tone, region masks, obstacles and
  closed loops.
- Line fields (directions without sign: noise, a shape's contour tangents, and mixes of them, blended in doubled-angle
  space so a direction and its opposite agree) and distance fields of shapes (signed, and normalised for letters with
  counters).
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

Also:
- The `Development Status :: 3 - Alpha` classifier overstates a skeleton: revisit.
- The README's relative links will not work on the PyPI page: use absolute URLs.
- The crate packages must include `LICENSE`.

## Later, with a trigger

- GPU rendering: when the CPU path is too slow for a live preview.
- Text-editing support (cursor, selection, hit testing within text): when an editor is started.
- Multi-threaded rendering: when batch rendering needs it.
- Custom nodes in Python or JavaScript (D6): when a user wants one.
- Colour and bitmap fonts: after v1, when a piece needs them.
- Stateful simulation (D5).
