# Roadmap

Status: draft for review, 2026-10-28. What the batches build: [design.md](design.md).

Small batches of work, in order. Each batch gets its own plan before work starts, ends merged into `main` with the
whole gate green and, for M and L batches, an independent review done, and leaves the worked examples it names passing. From V2, a batch
that adds calls is done only when they round-trip through JSON and the generated schema agrees with the hand-written
draft on them (formats.md, "Extension"). Sizes: **S** about one working session, **M** two or three, **L** four or
more.

## How we work

Short on purpose: a batch costs about two hours of agent time at the full process, so each rule below is a cut from
that. Everything else (trunk-based `main`, nothing pushed unless asked, tests through the public API, no hidden tunables)
stays as in CLAUDE.md.

1. **Plan:** one page per batch in the git-ignored `.superpowers/plans/`: the settled edge cases and the task list. An
   S batch needs only the task list.
2. **Branch:** one short-lived branch from `main`, fast-forward merged, then deleted.
3. **Build:** one implementer for the whole batch (two only if the context runs out), test first: see the test fail, then
   pass. Commit after every finished task or fix, never leaving more than one uncommitted, so an interruption loses
   nothing. While working run only the one test file; `uv run poe fast` once at the end of each part, `uv run poe
   check` once before the merge.
4. **Mutation checks:** at most about ten hand mutants per batch, on the settled edge cases and the arithmetic (a sign, a
   boundary, an ignored argument). Surviving mutants on core behaviour become tests; the rest are not chased.
5. **Review:** M and L batches get one independent review of the whole branch; S batches get the gate only. The reviewer
   proves each finding (a failing test or a reproduction; one it cannot prove is dropped) and in the same run fixes the
   Critical and Important ones it proved. Minor findings go to the controller, who ranks them, fixes the cheap useful
   ones without ceremony and drops the rest.
6. **Reports:** commits, public signatures, deviations from the plan, mutants killed or survived. No per-task narrative.

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
| V2a | Geometry: `Path` as a value (build, measure, split, slice, reverse, hit-test, SVG path data in and out), `Path.polygon`, `Path.star`, transforms; the `Error` redesign | M | |
| V2b | Drawings: shapes and style, attributes, the node trait and its contract, parameters and expressions, registry, evaluation with a cache, stacking, `with_values`, `parameters()`, `shapes()` | L | |
| V2c | JSON: `to_json`, `from_json`, the loader's checks, the schema generated from the registrations, the examples run | M | |
| V3 | Output: SVG, pixels, a mesh, `look` (draw a drawing and open it); the WebAssembly package; the same bytes on every target | L | **M1 a star everywhere** |
| X1 | Spike: a GPU display of the mesh in a page, measured | S | |
| X2 | Spike: live remeshing every frame, measured | S | |
| V4 | Frame export: a caller's function of time to a transparent PNG sequence or clip | S | |
| V5 | Fonts and shaping: glyphs as shapes, variable axes as parameters, bundled fonts | L | |
| V6 | Layout: wrap to a width, fit a line to a width by size or axis, text inside a shape; `along` (text on a path) | M | |
| V7 | Per-item variation: stagger, split, merge, per-item transforms, draw-on | M | **M2 type** |
| V8 | Booleans, with fuzzing and documented limits | L | |
| V9 | Offset, inset, stroke outline | M | |
| V10 | SVG fidelity: drop shadows and knockouts as style attributes, the same in SVG and pixels; a fast halftone fill | M | |
| V10d | Paint and compositing style: gradient fills and strokes, the full separable blend set, clip groups and soft masks | M | Done 2026-11-09 |
| V10e | Render a window: draw any rectangle of a drawing at the final scale | S | Done 2026-11-13 |
| V10f | Text runs: one line of text from runs that each have their own font, size and axes | M | Done 2026-11-13 |
| V11 | Path modifiers: dash march, place shapes along a path, simplify, resample; a width profile along a stroke (tapered strokes) | M | |
| V12 | Deformers: wobble, wave, twist, bulge; arch and envelope warps for type | M | **M3 kinetic type piece** |
| V13 | Morph, any shape to any shape | M | |
| V14 | Trace: grayscale levels and colour labels to outlines, steady over a sequence of maps | L | **M4 cut-paper piece** |
| V15 | Fields and lines: evenly spaced streamlines, direction and distance fields, hatching, plotter output | L | **M5 poster piece** |
| V16 | SVG import (logos) | S | Done 2026-11-11, trimmed |
| V17 | Presets as JSON | S | |
| V18 | Command line: `render`, `describe`, `list`, `validate` | S | |
| V19 | Presets and gallery | M | |
| V20 | Browser playground and a live page example | M | |
| V21 | Reference pieces rebuilt on the library | M | |
| V22 | First release: crates.io, PyPI, npm | S | **M6 release** |
| E | Effects track: one effect and its gallery piece per batch, open-ended, from V15 on ([effects-roadmap.md](effects-roadmap.md)) | S each | |

Order revised 2026-11-13: the missing "wow" capabilities (transforming type; lit, chrome-like type) come first among the
unbuilt batches. V10e (render a window) and V10f (text runs) are done; next: V11, then V12, V13, then the kinetic type piece (M3) as the first public demo; V15's distance fields
(see below) before V14. Reasons: [design-decisions.md](design-decisions.md), "Wow gaps (2026-11-13)".

Order decided 2026-10-31: after V3 the next batch is V5 (fonts). V4 and the spikes X1 and X2 were then done (2026-11-01), V5 the same day, V6 on 2026-11-04, V7 and V8 on 2026-11-07, V9 and V10 on 2026-11-08. `along`, listed under V11 until then, was built in V6 as the text-on-a-path piece (`design-decisions.md`, "Layout (V6)"); draw-on, listed under V11, was built in V7 as `Drawing.slice` ("Per-item variation (V7)"). V10, V10d, V10e and V10f are done; V11 follows.

"V2" in the documents means V2a to V2c together. V13 (morph) builds on V2 and V5, and V16 (SVG import) on V2; either can move earlier if a piece needs it.

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

State: done; independently reviewed 2026-10-31 and the proven findings fixed (design-decisions.md).

## V1b The JSON format

The drawing as the same tree written down; nothing in it the code API cannot say.
- `docs/formats.md`: the document, drawings as chains of calls, expressions, the parameter description format, inputs
  by name, canonical form, versioning, errors, limits, the checks only a loader can make.
- `docs/examples/json/`: nine example programs as JSON drawings.
- `schemas/drawing-1.schema.json`: a hand-written draft schema; `tests/test_drawing_schema.py` checks the examples and
  good and bad documents against it. From V2 a schema generated from the Rust declarations replaces it call by call.

Done when: the format is agreed; the schema accepts the examples and rejects known-bad drawings.

State: done; independently reviewed 2026-10-31 and the proven findings fixed (formats.md, the schema).

## V2 Core

Three batches, cut by the decision each hides: geometry, drawings, JSON. The unit tests, contract tests and properties
follow `CLAUDE.md`, "Testing"; the oracle of every geometry property is independent of the code (closed forms,
brute-force sampling, metamorphic relations).

### V2a Geometry

- `Path` as a value: build, measure (length, point and tangent at a fraction of the length), split, slice, reverse,
  hit-test (nonzero rule). The edge cases of `slice` (start above end, `slice(0, 0)`, a closed result), `tangent_at` at
  a corner and `locate` ties are settled in api.md ("Path") and tested.
- `Path.polygon`, `Path.star`, `Path.from_svg_path` and `to_svg_path` (arcs to cubics by one fixed rule), `translate`,
  `rotate`, `scale` on a `Path`. Replaced S0's skeleton function.
- Every internal sampling step (curve flattening for measuring) in design pixels, independent of the output size, each a
  named constant (`// constant:`; api.md, "Conventions").
- The `Error` redesign: `Error::Parameter` and `Error::Geometry` with owned strings, non-exhaustive (design-decisions.md,
  "Geometry (V2a)").

Done when: properties pass (length against closed forms, `reverse` twice is the identity, `point_at(t)` equals
`reverse().point_at(1 - t)`, the length of `slice(a, b)` is `(b - a)` times the length, a split's parts sum to the
whole, `contains` against a brute-force sampler); the bindings expose the same calls.

State: done; independently reviewed 2026-10-31 and the proven findings fixed (design-decisions.md).

### V2b Drawings

- Shapes: paths, style (fill, stroke, opacity, blend mode: normal, multiply or another separable CSS mode), attributes.
- The node trait and its contract, written first (inputs, the evaluation context, each argument's kind: expression,
  whole number, per-item, read once, literal; shape-by-shape or whole-list; how an error names its verb and place), and
  the registry vocabulary that can generate the draft schema (read-once, literal-only, nonzero, cross-argument rules, a
  per-call check hook). Each verb is declared once; the declaration gives its call-time and draw-time validation, its
  evaluation and the registry. The typed builders and the binding methods and stubs are written by hand over it until
  generating them pays (design-decisions.md, "Registry vocabulary"); V2c generates the schema entry.
- Evaluation of a tree on the merged graph of formats.md ("Shared drawings"), a chain's steps walked in a loop; a cache
  so that changing one value re-evaluates only what depends on it; the evaluation counters of formats.md ("Limits and
  security"), checked as the work is done.
- Parameters and expression arithmetic, `with_values`, `parameters()`, `shapes()`; validate, wrap cyclic angles.
- Built-ins: `translate`, `rotate`, `scale`, the style steps (`fill`, `stroke`, `opacity`, `blend`), `only` and
  `without` (the first readers of attributes), stacking (`Drawing(...)`), over the V2a `Path` constructors.
- Contract tests over every registered node, enrolled exactly (a node registered without a row fails): determinism, values
  within range, the same result at any design size (the 1x against 2x output check needs outputs: V3), a cached
  evaluation equal to a fresh one after random edits.
- The example programs start to run here, as far as `shapes()`: the Python ones move to a tested top-level `examples/`
  (`star.py` and `series_bars.py` now, the rest as their calls arrive), the Rust one to `crates/core/examples/`, the
  JavaScript ones under the npm package. `examples/` is never added to the documentation filter (`is_documentation`, the
  `changes` filter in `ci.yml`): its programs are code.

Done when: the example programs that use only V2's calls run as far as `shapes()`; each `Path` verb has its `Drawing` step
and the two agree; each evaluation counter a V2 node can reach raises `ParameterError` when passed, in a test (the
others arrive with the first step that produces what they count).

State: V2b-1 done in Rust, independently reviewed 2026-10-31 and the proven findings fixed (shapes, style, `Param`/`Expr`,
the node contract and registry, `translate`, `rotate`, `scale`, the style steps, `only`, `without`, stacking, `with_values`, `parameters()`, `shapes()`, `MAX_SHAPES`
and `MAX_DEPTH`, the contract table). V2b-2 built (the result cache, the Python names `Drawing`, `Shape`, `Style`, `Param`
and `Expr`, `star.py` and `series_bars.py` in a tested `examples/`), independently reviewed 2026-10-31 and the proven
findings fixed. V2b is done.

### V2c JSON

- `to_json` writing the canonical form and `from_json` reading it, over the registry; the limits; every check of
  formats.md "Checked by the loader, not the schema" whose calls are registered, in the order listed there, each with a
  test named after it; `FormatError`. Every later call brings its own checks and tests.
- The schema generated from the registrations. It agrees with the hand-written draft when both give the same verdict
  on every example and every test document whose calls are all registered; the draft stays the specification for the
  calls not yet registered and is retired call by call.
- `from_json` is a parser of hostile bytes: a property test feeds it arbitrary bytes and arbitrary JSON, and it returns
  a value or an `Error`, never panics.

Done when: each V2 example whose calls are registered round-trips through JSON to equal canonical text and equal
`shapes()` (V3 adds equal render bytes; the others as their calls arrive); a property test round-trips random trees from
the registry to the same canonical text; the generated schema agrees with the draft; the loader rejects a document for
each check whose calls are registered.

State: done; independently reviewed 2026-10-31 and the proven findings fixed. In Rust and Python: `Drawing::to_json`, `Drawing::from_json` (a strict in-crate
parser), `drawing_schema()` (generated from `verbs()`, agreeing with the draft on every example and test document whose
calls are registered), `Error::Format` and `FormatError`, `MAX_PARAMS`; the loader's checks whose calls are registered,
each tested by name and in order; `series_bars.json`, the two Python examples and random trees from the registry
round-trip. The other example documents round-trip as their calls arrive. These checks wait for the calls that reach them:

| Check | Waits for |
|---|---|
| `input-declared` | the first call that reads an input (`text`, `Field.map`) |
| `font-checks` | `text` |
| `inputs-supplied` | `text`, `Field.map` (and `from_json`'s `inputs`) |
| `call-checks`: `trace` levels and neighbours, `random`, `text` `fit` and `inside`, `Field.ramp` | `trace`, `random`, `text`, `Field.ramp` |

Carried from the S0 review:
- Clippy checks only this workspace's code, so the boolean and raster crates chosen in V2 must use `libm` for their
  maths; otherwise the V3 native-against-WebAssembly differential test is the only guard. (V2a wrote its own curve
  code on `libm`; see design-decisions.md, "No curve crate". The V8 boolean engine reaches the output through `libm` and integer
  arithmetic only: "Booleans (V8)".)

## V3 Output, WebAssembly, the same bytes everywhere

- SVG writer; pixels at any size on the CPU.
- Mesh output (`to_mesh`, api.md "Mesh"): triangles with positions, colours and per-item attributes, and draw ranges
  with their blend mode, in three flat buffers; knockouts resolved by booleans; a blurred shadow raises. A pointer, a
  length and a generation across WebAssembly; typed copies in Python (done: V3d, V3f). Covered by the differential
  tests below.
- `uv run poe look <drawing>`: draws to PNG and SVG in a git-ignored folder, for a person or an agent to look at (done:
  V3c).
- The WebAssembly package (wasm-bindgen): build, a smoke test in Node (done: V3e).
- Differential tests: the core's drawings rendered natively and in WebAssembly give identical bytes (pixels: within a
  measured tolerance, as the rasteriser crate is required to look the same, not to match byte for byte); and a drawing
  drawn at 1x and at 2x gives the same geometry (SVG and mesh coordinates at 2x are exactly twice those at 1x). Done:
  V3f, 44 drawings, in `poe wasm-test`.
- Benchmark: time to draw a reference drawing at 1080p, tracked from here on (done: V3f, `uv run poe bench`, numbers in
  [benchmarks.md](benchmarks.md)).

State: V3a done in Rust and Python (`Size`, the view, `Drawing.to_svg`, the star example writes SVG); V3e done: the
WebAssembly package loads a document and writes its SVG, equal to the native text, in Node (`poe wasm-test`); V3b done in Rust and
Python (`Drawing.to_rgba`, through `vello_cpu`); V3c done in Rust and Python (`Drawing.to_png`, through the `png` crate,
and `uv run poe look`); V3d done in Rust and Python (`Drawing.to_mesh`, `Mesh`, through `lyon_tessellation`); V3f done:
`toRgba`, `toPng` and `toMesh` in the WebAssembly package, the native-against-WebAssembly tests, `uv run poe bench`, and
the star example in Rust and Node. **V3 is done and M1, a star everywhere, is reached**: one star as SVG and as pixels
from Python, Rust and Node (`tests/wasm/star.test.mjs`).

## X1 Spike: GPU display

A measured experiment, not a promise. Draw V3's mesh with WebGL in a page and measure, on drawings from the reference
pieces: frame time, upload cost, and the difference from the CPU pixels (the reference). Done when the numbers are
written here with a proposal: what a GPU display would add to the roadmap, if anything, and a shader for the one step
with no triangle form, a blurred shadow (multiply already draws through the draw ranges, knockouts through booleans).

State: measured 2026-11-01.

Set-up: Apple M4 laptop, macOS, Google Chrome 156 (`--headless=new`), WebGL 2 on the real GPU (`UNMASKED_RENDERER`: ANGLE, Apple M4, Metal), the release WebAssembly package (simd128). Scenes: the 200-shape seeded drawing of `tools/bench.py` at 1920x1080 (also 960x540 and 3840x2160), the poster and the star from `examples/`. Mesh drawn with a 20-line shader (position and premultiplied colour), `ONE, ONE_MINUS_SRC_ALPHA` for normal ranges, canvas premultiplied. Reference: `toRgba` of the same drawing, compared in premultiplied space after flipping the read-back rows. Another job ran on the machine, so times move by about 30% between runs; each number is a median of 12 to 100 frames.

| Per frame, 1080p, 200-shape drawing (134k triangles, 60 ranges, 5.6 MB) | ms |
|---|---|
| `toMeshInto` (WebAssembly in the page) | 34 to 51 |
| `toRgba` (WebAssembly in the page) | 33 to 51 |
| `bufferData` call, vertices and indices | 0.2 |
| `bufferData` landed on the GPU (call, a one-triangle draw, a 1-pixel read-back, minus the empty read-back) | about 1 |
| GPU draw of the cached mesh, 20 frames per sync (4x MSAA / no MSAA) | 0.6 / 0.4 |
| one draw with a 1-pixel read-back as the sync (4x MSAA / no MSAA) | 2.5 to 3.6 / 1.4 to 1.5 (an empty clear with the same sync: 1.3 / 0.5) |
| mesh, upload and draw together, as a changed drawing needs | 37 to 56 |
| `putImageData` of the CPU pixels (the CPU path's display cost) | 1.7 |

At 960x540 the draw is 0.3 ms and `toRgba` 30 ms; at 3840x2160 the draw is 0.6 ms (no MSAA) to 1.7 ms (4x), `toRgba` 83 ms, `toMeshInto` 39 ms (the mesh does not depend on the size). Poster at 800x600: mesh 1.4 ms, draw 0.1 to 0.3 ms, `toRgba` 5.8 ms. Star: everything under 1 ms.

**Frame time.** Drawing a mesh is cheap (under 1 ms of GPU time at 1080p, 4K included); making it is not. Re-meshing costs as much as rasterising on the CPU, so a drawing that changes every frame gains nothing from a GPU display (37 to 56 ms against 35 to 53 ms). It pays only when the mesh is kept (`generation` unchanged: pan, zoom, a per-item effect in the shader from `item` and `order`), or at large sizes where the pixel path grows with the area and the mesh does not. `gl.finish()` did not wait in Chrome (0 ms); the table syncs with `readPixels`. `performance.now` ticks at 0.1 ms in headless.

**Difference from the CPU pixels** (1080p, 200 shapes on a white ground, 2.07 M pixels, 4x MSAA): 823k pixels differ (40%), 156k by more than 2/255, 92k by more than 8, 19k by more than 32 (0.9%), max 128, mean 0.7/255 per channel. Histogram of the largest channel difference: 0: 1.25 M, 1: 558k, 2 to 3: 126k, 4 to 7: 39k, 8 to 15: 43k, 16 to 31: 35k, 32 to 63: 18k, 64 to 127: 3.3k, 128 and up: 26. Where, each cause replicated on a scene that isolates it:

- Edges (anti-aliasing), 18% of the pixels: mean 2.5/255, 11k pixels over 32. Plain opaque shapes (no alpha, opacity or multiply) are exact inside (0 differing interior pixels) and differ only on edges. 4x MSAA is the most this GPU gives (`MAX_SAMPLES` 4); without MSAA the edge mean doubles (5.4, 48k pixels over 32, max 167). Rendering at 2x2 gives the same as 4x MSAA; at 4x4 and averaging down, the edge mean drops to 1.6 (star: 1.0, max 48): the rest is the CPU rasteriser's own coverage, not a position offset.
- Translucent colours (a colour with alpha, no group): interior differs by at most 2/255 (8-bit blending).
- A shape with fill and stroke under opacity or multiply (the mesh draws two layers, the pixels one group, as `api.md` says): interior differs up to 56 (opacity) and 136 (multiply); the same shapes with one layer differ at most 2 (opacity) and 22 (multiply, one pixel). This is the only large interior difference.
- Multiply: `gl.blendFunc(DST_COLOR, ONE_MINUS_SRC_ALPHA)` (the `mesh_upload.js` example) is right only over an opaque ground. With no background shape it gives 0 where the pixels give the source colour: 150k pixels over 32, max 255, against 22k and max 191 with an exact two-pass form (draw the range twice: `blendFuncSeparate(DST_COLOR, ONE_MINUS_SRC_ALPHA, ZERO, ONE)` then `blendFuncSeparate(ONE_MINUS_DST_ALPHA, ONE, ONE, ONE_MINUS_SRC_ALPHA)`). Over a white ground both forms are identical.
- Poster 800x600: interior differs by at most 1; edges mean 3.5, 1.4k of 110k differing pixels over 32. Star: 2631 pixels differ (1.6% of 160k), all on edges.

Not measured: a blurred shadow (no step for it yet, so no shader), other GPUs, other browsers, a windowed page with the compositor and vsync, a rendering of the mesh to a texture and back, and any 2D-canvas comparison other than `putImageData`. One machine, headless.

**Proposal.**
- Add no GPU display to the core. It is about 60 lines in the page (shaders, one buffer pair, a loop over the ranges); the mesh format needs no change. Put it in the V20 playground and give `mesh_upload.js` a complete shader and the multiply form above.
- Say in `api.md` that a mesh drawn with `blendFunc` is a preview: the pixels stay the reference. Differences: edge anti-aliasing (a few levels of 255 on average, more at hairlines), and two-layer shapes under opacity or multiply (by design).
- Fix the multiply line in `api.md` and `mesh_upload.js`: the single `blendFunc` is exact only over an opaque ground; use the two-pass form otherwise.
- A GPU display helps the live preview only where the mesh is kept between frames; for a drawing that changes every frame the time is the re-meshing, which X2 measures.
- Blurred shadow: keep the `ParameterError` in `to_mesh`. It needs an offscreen layer and two blur passes (a different kind of step from a draw range), which this spike did not build or time; decide it with V10, where the CPU blur is the reference.

## X2 Spike: live remeshing

A measured experiment, not a promise. Re-evaluate and re-mesh a drawing from the kinetic type piece every frame in
WebAssembly, with new values each frame, and measure the time per frame at 1080p against a 60 Hz budget, knockouts
resolved by booleans on every frame; find what dominates (shaping, booleans, tessellation, the cache). Done when the
numbers are written here with a proposal.

Measured 2026-11-01 on an Apple M4 (10 cores) laptop, macOS, Node 26.11, release WebAssembly with `simd128`, one thread; native
numbers are the same core compiled for the machine (release). The machine was shared and busy (load average 3 to 6 during
the runs reported, up to 15 at other times), so read every figure as plus or minus 30%.

**Scene (what exists today).** N shapes with the mix of `tools/bench.py` (circle, polygon, star, rounded rectangle, curve; fill,
stroke or both; 40% translucent; 20% multiply; radius 20 to 160 px) at 1920x1080. On the stack, five steps read Params whose
values are a list with one number per shape: `translate(dx, dy)`, `rotate(rot)`, `scale(sc)`, `opacity(alpha)`. Each frame the
caller builds the five lists (sine signals) and calls `drawing.toMeshInto(mesh, 1920, 1080, values)` on one reused `Mesh`; then
makes fresh typed-array views and copies vertices and indices into a preallocated buffer (a stand-in for a GPU upload; a real
upload was not timed). 200 frames after 30 warm-up frames. About 650 vertices and 700 triangles a shape.

| Shapes | What changes | Median ms | p95 | Max | Frames over 16.7 ms |
|---|---|---|---|---|---|
| 100 | all five lists | 14.4 | 18.2 | 25.0 | 17 of 200 |
| 100 | `alpha` only (rest cache hits) | 13.3 | 15.5 | 28.6 | 5 |
| 100 | nothing (all cache hits) | 14.4 | 14.9 | 18.0 | 3 |
| 100, 10 of them live | five lists of 10 | 14.8 | 16.2 | 19.0 | 3 |
| 300 | all five lists | 42.9 | 54.1 | 87.2 | 200 |
| 1000 | all five lists | 134.8 | 157.4 | 182.6 | 200 |
| 1000 | `alpha` only | 143.7 | 190.3 | 264.2 | 200 |
| 1000 | nothing | 140.6 | 188.0 | 345.3 | 200 |
| 1000, 10 of them live | five lists of 10 | 141.5 | 197.6 | 340.0 | 200 |

Total per frame, including making the values (0.01 ms at 100 shapes, 0.09 ms at 1000) and the copy (0.07 and 0.7 ms). In
headless Chrome 156 (coarse 0.1 ms clock, busy machine, one run each): 20 ms at 100 shapes and 152 to 202 ms at 1000, the
same order as Node; not conclusive.

**Where the time goes** (native, same code, per frame; shares are the same at 100 and 1000 shapes):

| Stage | 1000 shapes, ms | Share |
|---|---|---|
| Tessellation (`lyon`, with the vertex sink) | 58.1 | 53% |
| `crossing_at_most` (the work-limit pre-pass over each layer's edges) | 21.0 | 19% |
| Flattening curves to 0.01 px | 12.2 | 11% |
| Stroke outlines (`kurbo`) | 10.0 | 9% |
| Evaluation (`shapes()`, cache) | 3.1 | 3% |
| Building tessellator input, joins and caps | 4.4 | 4% |
| Whole `to_mesh` | 104 to 110 | |

- **Meshing is 97% of a frame; evaluation is 3%.** Evaluating 1000 shapes with every list changed takes 2.5 ms; with only
  `alpha` changed 1.1 ms; with 10 of 1000 shapes live 1.0 ms; a draw served entirely from the cache 1.2 ms (that floor is probably the
  clone of the shape list `shapes()` returns; not isolated). At 100 shapes: 0.25, 0.07, 0.09 and 0.09 ms. The cache works as designed and is
  not the problem.
- **`to_mesh` caches nothing below the shape list.** A frame where no value changed, or 10 of 1000 shapes moved, costs as much
  as one where everything moved. Only `generation` stays put, so an upload can be skipped, not the work.
- **Strokes cost about five times a fill.** 1000 fills alone: 26 ms; 1000 strokes alone: 127 ms (their outlines overlap
  themselves, so the sweep has many edges alive).
- **Flattening tolerance 0.01 px makes the meshes large** (650 vertices a shape). Natively, 1000 shapes at tolerance 0.1:
  40 ms; at 0.25: 26 ms (100 shapes: 11.1, 4.1, 2.9 ms). Not run in WebAssembly.
- **WebAssembly costs 1.3 times native** (11.3 against 14.4 ms at 100 shapes; 104 against 135 ms at 1000), as in
  `benchmarks.md`. The boundary is not a cost: reading five lists of 1000 numbers takes 0.3 ms, a throw 0.01 ms, the copy of
  21 MB of vertices 0.7 ms.
- Cost is linear in shape count (about 0.11 ms a shape in Node), and in this scene independent of how many values change.

**Not measured.** (Shaping and variable-axis changes per frame were measured in V5c: `benchmarks.md`, "Text".) Knockouts per frame
(`knockout` is not a step yet; booleans per frame were measured in V8, natively only: `benchmarks.md`, "Booleans (V8)"), offsets and stroke-outline verbs (V9), deformers (V12), a real GPU upload and draw, a
mesh with a different flattening tolerance in WebAssembly, more than one machine or browser, several threads (the build is
single-threaded). Per-item fill colours cannot be Params today (a Param is a number): the per-shape lists drive transforms and
opacity only. The kinetic type piece itself was not measured: this is a stand-in with the same kind of work.

**Proposal.**
- Live remeshing at 1080p and 60 Hz is feasible for about 100 mixed filled and stroked shapes of this size today (median
  14 ms, p95 18 ms: no headroom), not for 300 or more, and for fills alone up to about 500 (26 ms for 1000 natively, times
  1.3). Typical type has fewer and simpler contours than this scene, but that is unmeasured.
- Do not remesh to move things. Evaluation is already cheap. Two cuts remove nearly all of it, in order of value:
  1. Cache the layers of a shape by content (flattened path, style) so a frame re-tessellates only the shapes that
     changed. This takes the 10-of-1000 case from 140 ms to about 2 ms plus the changed shapes, and a still frame to the
     evaluation floor.
  2. For motion that is only a transform or opacity per item (the common live case), mesh once and apply the per-item
     translate, rotate, scale and alpha in the vertex shader using the `item` attribute the mesh already carries; remeshing is
     then needed only when geometry changes (shape, stroke width, an axis, a boolean).
- Cheap tuning before either: loosen the mesh's flattening to a display tolerance (0.1 to 0.25 px is 3 to 4 times fewer
  vertices and 2.8 to 4 times faster), and replace the pre-pass `crossing_at_most` (19%) with a cheaper bound. Drawing strokes in
  the shader instead of tessellating their outlines is an idea only, not tried.
- Re-measure after V5 (done in V5c, `benchmarks.md` "Text": 0.015 ms to shape and outline a headline at a new weight, 0.5 ms for a
  page, 0.6 ms to mesh it at 1080x1350): shaping and glyph outlines per frame when a variable axis moves (cache by axis value; a glyph set
  with the same text and a moving axis is the kinetic type case), and the mesh of real glyphs. Booleans per frame (done in V8,
  `benchmarks.md` "Booleans (V8)": 1.8 ms to resolve a line of 35 capitals, 4.7 ms to cut it out of a plate, natively; not meshed,
  not in WebAssembly), against the 16.7 ms left over once meshing is cached. Re-measure on
  another machine, in a real browser with a real upload, and with the chosen caching in place, before promising 60 Hz.

## V4 Frame export

A small helper outside the core: it loops a caller's function (time in, a drawing with its values out; api.md, "Frame
export") over frames at a frame rate and writes a transparent PNG sequence, or ProRes 4444 or WebM with alpha through
ffmpeg. No timeline, keyframes or mixing.

Done when: a worked example writes a PNG sequence and a clip with alpha, and each frame equals the drawing drawn
directly with that frame's values.

State: done in Python (`btvec.frames`, `src/bullterrier_vector/frame_export.py`; `examples/badge_frames.py` writes a PNG
sequence; the tests write and read back PNG sequences exactly, and ProRes 4444 and VP9 clips with alpha within a measured
tolerance, skipping with a stated reason without ffmpeg).

## V5 Fonts and shaping

- Fonts from bytes: OpenType, TrueType, variable fonts; two or three open-licence fonts bundled. Whether WOFF and WOFF2
  load, or need unpacking first, is checked here.
- Shaping with ligatures, kerning, complex scripts and OpenType features; glyph outlines as shapes with attributes
  (index in text, word, line; order 0..1). `resolve` (V8) removes the overlapping contours on request (mesh and pixels
  fill nonzero, so unresolved overlaps draw right).
- Variable axes (weight, width, slant, custom) as parameters of the text node; per-glyph axis values, with advances that
  follow them.

State: V5a is done (Rust, through JSON): `Font`, `text(content, Text::new(&font).size(..).tracking(..).leading(..)
.align(..).anchor(..))` with the attributes, `Inputs` and `Drawing::from_json(text, &inputs)`, `to_json` writing `inputs`
with `sha256`. V5b is done (Rust, through JSON): `Font::axes()` (a `Param` per axis) and `Font::features()`,
`Text::axes(..)` (a number, `Param` or expression per tag, a value per character, shaped per run of equal location) and
`Text::features(..)`, the loader's `font-checks`. V5c is done: `btvec.Font`, `btvec.text` and `Drawing.from_json(...,
inputs=)` in Python, `Drawing.fromJson(text, inputs)` in WebAssembly, fontTools as the oracle of the Python tests, ten text
drawings in the native-against-WebAssembly differential (exact SVG text and mesh buffers), `examples/kinetic_type.py`,
and the per-frame cost of type in `docs/benchmarks.md`. V5d is done: `Font::new` reads WOFF2 (unpacked with `wuff`, a
decompression bomb refused at `MAX_FONT_BYTES`) and refuses WOFF, answering the question above; `Font::bundled(name)` and
`Font::bundled_names()` (`btvec.Font.bundled`) give Bricolage Grotesque and Fraunces (OFL 1.1, compiled in as WOFF2), a
document names them by `sha256` and loads without bytes, the licences ship in the crate, the sdist and the wheel
(`build-check` fails without them), `examples/bundled_type.py`. V5 is done. The npm package does not exist yet: V22
publishes it with the `OFL.txt` of each font in `files` and `license` `MIT AND OFL-1.1` (the `wasm-bindgen` output takes the
licence from the crate).

## V6 Layout

- Wrap to a width; fit each line to a width by size or by a variable axis, at a reference pose separate from the live
  values, saying when the axis runs out.
- Text set inside a shape.
- `along`: shapes carried along a guide path, so text on a path is `text(...).along(path)`. Alignment, tracking and leading
  were V5's.

Done when: a headline fills a measure by its width axis and a page wraps to a width, both from `text` arguments alone; a
text fills a circle and a frame with a hole; a text turns round a circle by a `Param`; each in Rust, Python and the
native-against-WebAssembly differential.

State: V6 is done (Rust, Python, through JSON): `text(..., width=, fit=, inside=)` (Rust `Text::width`, `Text::fit`,
`Text::inside`), `Path.along(path, *, start=0, near=None)` and `Drawing.along(path, *, start=0, near=None)` (Rust
`Path::along`, `Drawing::along(&guide, Along::new().start(..).near(..))`), with `along` registered and in the schema.
`examples/kinetic_type.py` fills its measure with `width` and `fit="wdth"`; `examples/text_on_path.py` turns a line round
a circle by a `Param`. The WebAssembly differential has 63 drawings (44 paths, 17 texts, 2 bends); five of the texts are
wrapped, fitted or set inside a shape. Costs are in `benchmarks.md`, "Text". Known limits (`api.md`, "text" and `along`):
no Unicode line-break rule, so text without spaces does not wrap; a fit by an axis finds only widths between the widths at
the axis's ends; right-to-left text inside a shape is not tested; an edge longer than half a closed guide can be bent
coarsely.

## V7 Per-item variation

- Stagger: one value (say `progress`) becomes a per-item value spread by an attribute, for any modifier.
- Text as one shape per glyph; `split` into parts, `merge(by=...)` into words or lines; per-item transforms (translate,
  rotate, scale by index or by a seeded random) about a typographic origin (cap-height centre, not the bounding box).

- Draw-on, from V11: `Drawing.slice(start, end)` read per shape, so `slice(0, stagger(progress))` draws a text on letter by
  letter.

Done when: a word draws on letter by letter and scatters by index with no text-specific modifier.

State: V7 is done (Rust, Python, through JSON): the per-item numbers `by_index`, `by_order`, `random(low, high, seed=)` and
`stagger(progress, spread=)` (one `Expr` node, `expr/item.rs`), the steps `split`, `merge(by=)` and `slice(start, end, near=)`
on a `Drawing`, and transforms whose `about` defaults to the shape's `origin` and that move it. `examples/draw_on.py` draws
a word on letter by letter by one `progress`; `examples/glyph_scatter.py` splits a word into parts and scatters them by
seeded `random` and a `scatter` Param. The WebAssembly differential has 72 drawings (44 paths, 17 texts, 2 bends, 9 per-item).
Known limits (`api.md`, "Per-item numbers" and "Items"): `merge` joins sub-paths and leaves overlaps (`union` or `resolve` unites them);
`random` is keyed by the shape's current `index`, so an `only` or `split` before it changes the numbers; `split` (and
`Path::split`) tests every closed sub-path against every other, so its time grows with the square of their number (8000 tiny
squares: 0.4 s; measured 4x per doubling).

## V8 Booleans

Union, subtract, intersect, exclude, on any shapes. The riskiest batch: build on an existing crate, fuzz with random and
degenerate paths (overlaps, touching edges, tiny segments), document what fails. Also here: removing the overlaps
between the contours of a glyph (`text` keeps the font's contours; `resolve` removes the overlaps), so that every glyph
outline can be a valid region in the canonical form of api.md.

State: V8 is done (Rust, Python, through JSON): `union`, `subtract`, `intersect` and `exclude` with `other` and
`tolerance`, and the unary `resolve`, on a `Path` and as steps on a `Drawing` (`path/boolean.rs` over `i_overlay`,
`node/boolean.rs`, `drawing/boolean.rs`; `other` is the new argument kind `Kind::Drawing`). Fuzzed with six properties at
10,000 cases each, 407 glyphs of both bundled fonts and the degenerate extremes; the six defects found are fixed
(design-decisions.md, "Booleans (V8)") and the limits are in api.md, "Booleans". `examples/cutout.py` cuts a headline out of a
plate; the WebAssembly differential has 88 drawings (44 paths, 17 texts, 2 bends, 9 per-item, 16 booleans);
`uv run poe bench --booleans` times the verbs (`benchmarks.md`, "Booleans (V8)").

## V9 Offsets

Offset and inset with round, miter and bevel joins; outline a stroke into a fill.

State: V9 is done (Rust, Python, through JSON): `offset(distance, join=, miter_limit=, tolerance=)` and `outline(width,
join=, cap=, miter_limit=, tolerance=)`, on a `Path` and as steps on a `Drawing` (`path/offset.rs` over `i_overlay`,
`node/offset.rs`, `drawing/offset.rs`; `distance` and `width` are per-item numbers). A `miter` past the limit is clipped
flat, where `stroke` bevels (design-decisions.md, "Offsets (V9)"); the limits are in api.md, "Offsets".
`examples/cut_paper_layers.py` stacks offset layers of a headline under an outline cut line, `examples/ring.py` makes a
ring of two offsets; the WebAssembly differential has 104 drawings (44 paths, 17 texts, 2 bends, 9 per-item, 16
booleans, 16 offsets).

## V10 SVG fidelity and a fast halftone fill

- Drop shadows and knockouts, built here as style attributes of a shape, with the same result in SVG and in pixels.
  Multiply blending (V2's style model) is written into SVG as it is drawn in pixels, so inks multiply over paper.
- Every effect so far written into SVG as drawn in pixels. An effect with no SVG form fails a test.
- Halftone as a fill (a screen: pitch, angle, dot size from a tone) clipped to any shape, without a boolean per dot;
  timed by the V3 benchmark.

Done 2026-11-08. `shadow`, `knockout` and `halftone` are style steps, in Rust, Python, documents and the WebAssembly
differential (128 drawings), written into SVG, drawn in pixels and cut into the mesh by the same rules (`api.md`,
"Style"; `design-decisions.md`, "Style effects (V10)"). The SVG is checked against the pixels by rendering it with
`resvg` in the core tests; multiply was found to agree. A blurred shadow stays a `ParameterError` in `to_mesh`: the mesh
format is untouched, so allowing it later adds a draw-range kind and breaks no caller. The halftone `tone` is a number
until the tone sources of V15 (`Field`); a screen of 3,000 dots takes 20 ms in pixels and about 70 ms as SVG or mesh,
against about 500 ms for a boolean per dot (`benchmarks.md`).

## V10d Paint and compositing style

Parts 1 (gradient fills and strokes), 2 (the separable blend set) and 3 (clip groups and soft masks) are built; done 2026-11-09: see `design-decisions.md`, "Paint and compositing style (V10d)".

The same style model as V10, extended; the result is the same in SVG and in pixels, and the mesh raises as it does for a
blurred shadow.

- Linear and radial gradient fills, on fill and on stroke (stops with colour and position, a start and end or a centre
  and radius in design pixels). The fill of a shape is a paint, not only a colour.
- The separable blend modes of the W3C compositing specification beyond `normal` and `multiply` (built: all ten).
  Blending stays in sRGB values, with straight alpha out of `to_rgba`.
- Clip groups (shapes drawn only inside a path) and soft masks (a drawing whose luminance sets the opacity of another)
  (built: `clip` and `mask`, each shape on its own; a group form is not built).

## V10e Render a window

Draw any rectangle of a drawing at the final scale, so a caller can render a large canvas in bands without a raster of the
whole. Property: a window's pixels equal the matching rows and columns of the whole render, for every output that draws
pixels. The limits of `to_rgba` (65,000 a side, 2^27 pixels) then apply to the window, not to the drawing.

State: done 2026-11-13, in Rust, Python and WebAssembly (`design-decisions.md`, "Render a window (V10e)").

## V10f Text runs

One line of text built from runs, each with its own font, size and axes, shaped per run on shared lines: wrapping,
fitting, alignment, `inside` and `along` work on the whole. Today `text` takes one font per call, and only the axis
values and the fitted size vary from character to character, so letters of different typefaces in a word (ransom-note
lettering) need a call per letter placed by hand.

State: done 2026-11-13, in Rust, Python and WebAssembly (`design-decisions.md`, "Text runs (V10f)"): `run(content, *, font,
size, axes, features, tracking)` and `text([run, ..], ...)`, the `run` attribute, `examples/ransom_note.py`.

Carried to later (review, minor):
- A `fit` axis that no font of a one-run text has is a `FormatError` in a document and a `ParameterError` from a plain text.
- A `fit` on an axis names no line when a line holds only runs whose font lacks the axis.
- `inside` plans its lines with the largest run size and tallest capital, an empty run included.
- The unoptimised 256 KB stack overflows on a plain text with a 126-level axis expression or 127 nested texts (not from runs).

## V11 Path modifiers

Dash with a moving phase, place shapes along a path, simplify, resample to even spacing. Resample comes first inside this
batch because V12 deforms resampled paths. V11 then V12 then V13 is the order of the transforming-type capability. `along` (V6) bends shapes along a
path; placing shapes on it without bending them (copies at even spacing, each turned to the path's direction) is not built
and stays here. Draw-on (partial path by length from a canonical start point and direction) was built in V7 as
`Drawing.slice`; `reverse`, a draw-on the other way round, stays here.
A width profile along a stroke (tapered strokes: a width that varies along the length, thinning at free ends) is added
here; it needs a stroke the rasteriser and the mesh can draw as a filled outline.

## V12 Deformers

Wobble (seeded noise displacement, moving by `phase`), wave, twist, bulge. Paths are resampled first so deformation is
smooth at any size.
Arch and envelope warps for type (a line of text bent on an arch, or fitted into an envelope shape) join them.

Done when: `docs/examples/wobble_outline.py` and the deformer half of `docs/examples/kinetic_type.*` run as shipped
examples, with a letter line that bends, ripples and twists on `phase` and `progress`.

## V13 Morph

Any shape to any shape, matching points when the counts differ. Spike first (as X1 and X2 did): a letter into a
circle, a word into another word of a different length, a letter with counters (holes) into one without. Measure what
pairing of points looks right; the rest of the batch follows the finding.

Done when: a letter turns into a shape and back, and one word into another, by a single `progress` value, in a gallery
piece. With V12 this is the transforming-type capability and the headline of the first demo (M3).

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

Distance fields have a second job: a signed, normalised distance field of letters or shapes is the relief that
image's `relight`, and later `glass`, light. Written out as a grayscale map (bytes; the call is not designed yet, and `Field.map` only reads maps in), it lets scene give a headline chrome, emboss or a lit bevel with no 3D engine and no new link
between image and vector. Chrome type already works without it: scene's C5 example lights the headline's own brightness (`relight` with `field="light"`, a light fill). The distance field is a later refinement, and gives a bevel that also reaches outside the letters only if image lets a look change alpha.

Placement that stays stable as values change (design.md, D7) is needed before hatching is animated; the poster is a
still.

## V16 SVG import

Logos and other SVG paths in as shapes. A logo is a set of named parts, so keep the names: every imported shape takes its source
element's `id` and `class`, and its group path, as attributes, so `only(id="ring")`, `split` and the per-shape numbers
(`stagger`, `by_index`) address parts of an imported drawing without counting contours. Fills, strokes, transforms and the
`viewBox` come too (the `viewBox` becomes the drawing's size); an element with no id still gets its `index`.

Why: a prototype animated a club logo part by part (a ring that draws on, then a triangle, then the monogram and letters). The
only import was `Path.from_svg_path` on the whole compound path, so which of its 42 contours was the ring was read off a picture
that coloured each contour and typed in as indices, which breaks when the logo is exported again. The roles belong in the SVG's own
ids and groups (a designer already names layers); vector adds no sidecar format and does not guess which contour is which.

**Done (2026-11-11), trimmed:** `Drawing.from_svg(text)` reads `svg`, `g` and the shape elements; the limits are in
`docs/api.md`, "SVG import". Not in the first version: `evenodd`, `use`, gradients, clip paths, masks, text, CSS stylesheets.

Deferred importer features, each refused with an error today, and what each needs:

- `fill-rule="evenodd"`: the rasteriser is hard-wired nonzero (`output/raster.rs`, `output/mesh.rs`); needs a `Style`
  fill-rule field, then the rasteriser, mesh, SVG writer, JSON and the WebAssembly oracle.
- CSS `<style>` and `class` rules: needs a CSS parser; minimal class and tag selectors are feasible.
- Gradients: the model exists (`Gradient`); needs `defs`, `url(#id)` lookup, `gradientUnits` and `gradientTransform`.
- `use`: needs an id map and a cycle guard.
- `<!DOCTYPE>` (Illustrator and Inkscape headers): refused because `roxmltree` does not cap entity expansion
  (`design-decisions.md`, "`roxmltree` reads SVG"); needs a pre-scan that refuses `<!ENTITY` and lets a bare DOCTYPE
  through, or a parser with an expansion cap.

Done when: an SVG with ids on a logo of 40 or more contours imports, `only(id=...)` selects each part, nested groups give the
group path, and the JSON round trip keeps the attributes.

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
- Memory per shape: measure the bytes of a shape and what the result cache keeps (the million-shape limit test in
  `drawing_limits.rs` peaks at 2 to 3 GB), and lower `MAX_SHAPES` or shrink a shape if a server or the browser needs
  less: before V22.
- Gate lints for hidden tunables and swallowed errors: ruff `PLR2004` and clippy's `let_underscore_must_use`,
  `map_err_ignore`, `allow_attributes`, tried on a short branch with the hits counted, keeping those that are quiet or
  find real problems; `// constant:` stays a reviewed rule (no tool tells a tunable from a harmless literal).
- Wrapping tests for right-to-left text, ligatures across a break and combining marks, and right-to-left text inside a
  shape: when a test font for them exists or a piece needs them.
