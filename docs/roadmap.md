# Roadmap

This file records what shipped. Planned work is tracked outside this repository.

What the batches built: [design.md](design.md).

Small batches of work. Each batch gets its own plan before work starts, ends merged into `main` with the
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
geometry, then the generative layer in the order the reference pieces need it, then an effects track. The
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
| V3 | Output: SVG, pixels, a mesh, `look` (draw a drawing and open it); the WebAssembly package; the same look on every target | L | **M1 a star everywhere** |
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
| V11 | Path modifiers: dash march, place shapes along a path, simplify, resample; a width profile along a stroke (tapered strokes) | M | Done 2026-11-16 |
| V12 | Deformers: wobble, wave, twist, bulge; arch and envelope warps for type | M | Done 2026-11-16 |
| V13 | Morph, any shape to any shape | M | Done 2026-11-16 |
| M3 | The kinetic type piece: `examples/kinetic_type.{py,rs,mjs}`, the first public demo | S | **M3 kinetic type piece**, done 2026-11-16 |
| V14 | Trace: grayscale levels and colour labels to outlines, steady over a sequence of maps | L | **M4 cut-paper piece** |
| V15 | Fields and lines: evenly spaced streamlines, direction and distance fields, hatching, plotter output (V15a, V15b and V15c done) | L | **M5 poster piece** |
| V16 | SVG import (logos) | S | Done 2026-11-11, trimmed |
| V17 | Presets as JSON | S | Done 2026-11-18 |
| V18 | Command line: `render`, `describe`, `list`, `validate` | S | Done 2026-11-18 |
| V19 | Presets and gallery | M | Done 2026-11-18 |
| V20 | Browser playground | S | Done 2026-11-18 |
| E | Effects track: one effect and its gallery piece per batch, from V15 on; 4a and 4b are done ([effects-roadmap.md](effects-roadmap.md)) | S each | |

Order revised 2026-11-13: the missing "wow" capabilities (transforming type; lit, chrome-like type) come first among the
unbuilt batches. V10e (render a window), V10f (text runs), V11 (path modifiers), V12 (deformers), V13 (morph) and the kinetic type piece (M3, the first public demo) and V15a (fields and streamlines), V15b (fields and lines) and V15c (line families, plotter output and the poster piece, M5) are done; V15 is done and V14a (trace of a single map) is done; V14 (with V14b) is done and M4 is reached; the effects track's 4a (halftone screens) and 4b (character grids and overlay marks) are done; V17 (presets as JSON), V18 (command line) and V19 (presets and gallery) are done. Reasons: [design-decisions.md](design-decisions.md), "Wow gaps (2026-11-13)".

Order decided 2026-10-31: after V3 the next batch is V5 (fonts). V4 and the spikes X1 and X2 were then done (2026-11-01), V5 the same day, V6 on 2026-11-04, V7 and V8 on 2026-11-07, V9 and V10 on 2026-11-08. `along`, listed under V11 until then, was built in V6 as the text-on-a-path piece (`design-decisions.md`, "Layout (V6)"); draw-on, listed under V11, was built in V7 as `Drawing.slice` ("Per-item variation (V7)"). V10, V10d, V10e, V10f, V11, V12, V13 and M3 are done.

"V2" in the documents means V2a to V2c together.

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

## V1a The code API

Design the API first, as the examples a user would write: the three reference pieces and a dozen small drawings, in
Python, the kinetic type piece also in Rust and JavaScript, reviewed for how they read. The four ideas (`Path`, `Shape`,
`Drawing`, `Param`) are decided (design.md, "The code API"); this batch settles their method names.
- `docs/api.md`: the ideas, a reference table per type, outputs, errors, conventions, the idioms of each language, the
  prototypes' needs mapped to methods, open questions.
- `docs/examples/`: the example programs, written before their calls existed.

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
round-trip.

## V3 Output, WebAssembly, the same look everywhere

- SVG writer; pixels at any size on the CPU.
- Mesh output (`to_mesh`, api.md "Mesh"): triangles with positions, colours and per-item attributes, and draw ranges
  with their blend mode, in three flat buffers; knockouts resolved by booleans; a blurred shadow raises. A pointer, a
  length and a generation across WebAssembly; typed copies in Python (done: V3d, V3f). Covered by the differential
  tests below.
- `uv run poe look <drawing>`: draws to PNG and SVG in a git-ignored folder, for a person or an agent to look at (done:
  V3c).
- The WebAssembly package (wasm-bindgen): build, a smoke test in Node (done: V3e).
- Differential tests: the core's drawings rendered natively and in WebAssembly look the same (SVG, mesh and pixels within the one
  tolerance of `docs/design-decisions.md`); and a drawing
  drawn at 1x and at 2x gives the same geometry (SVG and mesh coordinates at 2x are exactly twice those at 1x). Done:
  V3f, 44 drawings, in `poe wasm-test`.
- Benchmark: time to draw a reference drawing at 1080p, tracked from here on (done: V3f, `uv run poe bench`, numbers in
  [benchmarks.md](benchmarks.md)).

State: V3a done in Rust and Python (`Size`, the view, `Drawing.to_svg`, the star example writes SVG); V3e done: the
WebAssembly package loads a document and writes its SVG, looking the same as the native text, in Node (`poe wasm-test`); V3b done in Rust and
Python (`Drawing.to_rgba`, through `vello_cpu`); V3c done in Rust and Python (`Drawing.to_png`, through the `png` crate,
and `uv run poe look`); V3d done in Rust and Python (`Drawing.to_mesh`, `Mesh`, through `lyon_tessellation`); V3f done:
`toRgba`, `toPng` and `toMesh` in the WebAssembly package, the native-against-WebAssembly tests, `uv run poe bench`, and
the star example in Rust and Node. **V3 is done and M1, a star everywhere, is reached**: one star as SVG and as pixels
from Python, Rust and Node (`tests/wasm/star.test.mjs`).

## X1 Spike: GPU display

A measured experiment, not a promise. Draw V3's mesh with WebGL in a page and measure, on drawings from the reference
pieces: frame time, upload cost, and the difference from the CPU pixels (the reference).

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

## X2 Spike: live remeshing

A measured experiment, not a promise. Re-evaluate and re-mesh a drawing from the kinetic type piece every frame in
WebAssembly, with new values each frame, and measure the time per frame at 1080p against a 60 Hz budget, knockouts
resolved by booleans on every frame; find what dominates (shaping, booleans, tessellation, the cache).

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
drawings in the native-against-WebAssembly differential (SVG text and mesh buffers that look the same), `examples/kinetic_type.py`,
and the per-frame cost of type in `docs/benchmarks.md`. V5d is done: `Font::new` reads WOFF2 (unpacked with `wuff`, a
decompression bomb refused at `MAX_FONT_BYTES`) and refuses WOFF, answering the question above; `Font::bundled(name)` and
`Font::bundled_names()` (`btvec.Font.bundled`) give Bricolage Grotesque and Fraunces (OFL 1.1, compiled in as WOFF2), a
document names them by `sha256` and loads without bytes, the licences ship in the crate, the sdist and the wheel
(`build-check` fails without them), `examples/bundled_type.py`. V5 is done.

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
  rotate, scale by index or by a seeded random) about a typographic origin (cap-height center, not the bounding box).

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
`examples/cut_paper_layers.py` stacks offset layers of a headline under an outline cut line, `examples/presets/ring.py` makes a
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
`resvg` in the core tests; multiply was found to agree. A blurred shadow is a `ParameterError` in `to_mesh`: the mesh
format has no draw-range kind for it. The halftone `tone` was a number until 4a, which reads a `Field` too; a screen of 3,000 dots takes 20 ms in pixels and about 70 ms as SVG or mesh,
against about 500 ms for a boolean per dot (`benchmarks.md`).

## V10d Paint and compositing style

Parts 1 (gradient fills and strokes), 2 (the separable blend set) and 3 (clip groups and soft masks) are built; done 2026-11-09: see `design-decisions.md`, "Paint and compositing style (V10d)".

The same style model as V10, extended; the result is the same in SVG and in pixels, and the mesh raises as it does for a
blurred shadow.

- Linear and radial gradient fills, on fill and on stroke (stops with colour and position, a start and end or a center
  and radius in design pixels). The fill of a shape is a paint, not only a colour.
- The separable blend modes of the W3C compositing specification beyond `normal` and `multiply` (built: all ten).
  Blending stays in sRGB values, with straight alpha out of `to_rgba`.
- Clip groups (shapes drawn only inside a path) and soft masks (a drawing whose luminance sets the opacity of another)
  (built: `clip` and `mask`, each shape on its own).

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

## V11 Path modifiers

Dash with a moving phase, place shapes along a path, simplify, resample to even spacing. Resample comes first inside this
batch because V12 deforms resampled paths. V11 then V12 then V13 is the order of the transforming-type capability. `along` (V6) bends shapes along a
path; `place_along` places copies on it without bending them (at even spacing, each turned to the path's direction).
Draw-on (partial path by length from a canonical start point and direction) was built in V7 as `Drawing.slice`;
`reverse` is a draw-on the other way round.
A width profile along a stroke (tapered strokes: a width that varies along the length, thinning at free ends) is added
here; it needs a stroke the rasteriser and the mesh can draw as a filled outline.

State: done 2026-11-16, in Rust, Python and WebAssembly (`design-decisions.md`, "Path modifiers (V11)"): `resample`, `dash`,
`simplify`, `reverse`, `place_along` (`Drawing` only), `outline(profile=)` and `stroke(profile=)`, `examples/marching_dashes.py`,
`examples/beads_on_a_path.py`, `examples/brush_taper.py`.

## V12 Deformers

Wobble (seeded noise displacement, moving by `phase`), wave, twist, bulge. Paths are resampled first so deformation is
smooth at any size.
Arch and envelope warps for type (a line of text bent on an arch, or fitted into an envelope shape) join them.

Done when: `examples/wobble_outline.py` and `examples/bent_type.{py,rs,mjs}` run as shipped examples, with a letter
line that bends, ripples, twists and wobbles on `phase` and `progress`. `docs/examples/kinetic_type.*` had no deformer
calls, so the deformer half of the kinetic piece is the new `bent_type` trio; the riso reference pieces stay in
`docs/examples`.

State: done 2026-11-16, in Rust, Python and WebAssembly (`design-decisions.md`, "Deformers (V12)"): `wave`, `wobble`,
`twist`, `bulge`, `envelope`, `arch` and `Path.arc`, one warp kernel with one error budget of 0.05 design pixels,
`examples/wobble_outline.py`, `examples/bent_type.py`, `.rs` and `.mjs`.

## V13 Morph

Any shape to any shape, matching points when the counts differ. Spike first (as X1 and X2 did): a letter into a
circle, a word into another word of a different length, a letter with counters (holes) into one without. Measure what
pairing of points looks right; the rest of the batch follows the finding.

Done when: a letter turns into a shape and back, and one word into another, by a single `progress` value, in a gallery
piece. With V12 this is the transforming-type capability and the headline of the first demo (M3).

State: done 2026-11-16, in Rust, Python and WebAssembly (`design-decisions.md`, "Morph (V13)"): `Path.morph(target,
fraction)` and the `Drawing` step (shapes pair by index, contours by role and place, unmatched ones shrink or grow, an
unmatched hole only cuts the body and never paints outside it), `examples/glyph_morph.py`, `.rs` and `.mjs` (a letter into a circle and CAT into HORSE on
one `progress`; "and back" is `progress` going 1 to 0).

## M3 The kinetic type piece

The first public demo: "NO SLEEP" in a variable font, as one drawing built once with every number that changes a Param
and the caller's function of time (`at_time(t)`) giving the values of a frame. Hairlines; a blue plane rises and knocks
the letters out; the drop prints them in two inks (misregistered more on each hit) with a kick running through the
letters (weight a letter, scatter); the line arches, ripples and wobbles; NO SLEEP turns into GO DREAM letter by letter;
the print thins back to hairlines.

Done when: `examples/kinetic_type.py`, `crates/core/examples/kinetic_type.rs` and `crates/wasm/examples/kinetic_type.mjs`
run, draw four frames that are the same SVG text in all three (`poe wasm-test`), and `btvec.frames` writes the clip
(Python).

State: done 2026-11-16 (`design-decisions.md`, "Kinetic type piece (M3)"). A frame costs about 25 ms at 1080x1350.

## V14 Trace

**V14a, done 2026-11-17.** The single-map part, as three source verbs on a `Field.map` (`Drawing.trace_levels`, `trace_labels`,
`trace_cells`, in Rust, Python and documents so WebAssembly; `examples/trace.py` in the three languages): filled shapes per level
with the level as an attribute, per label, and an exactly traced dithered cell map (one shape per colour, a cap on colours);
curve fitting, edge padding, `open`/`close` in design pixels, `min_area`/`min_hole`. Reasons and limits: `design-decisions.md`,
"Trace (V14a)".

**V14b and M4, done 2026-11-18; V14 done and M4 reached.** `Field.smooth_maps` (Rust and Python; maps in, maps out, no
document verb), `examples/trace_sequence.{py,rs}`, and the M4 piece: the owner's dog as a cut-paper poster
(`design-decisions.md`, "Trace sequences (V14b)" and "Cut-paper piece (M4)").

- A grayscale map in (width, height, bytes), outlines at chosen levels out; each level a shape with its level as an
  attribute.
- Multi-colour trace: a map of quantised colour labels to one set of outlines per colour.
- A dithered label map (a palette index per pixel, square cells of a known size) traced exactly: cells of one colour
  merged into straight-edged regions, no curve fitting, one group per colour, a cap on the number of colours: dither and
  posterize to SVG ([dither-and-print.md](dither-and-print.md)).
- Curves fitted through the traced edge (a raw polyline looks digital); edge padding, so a subject cut by the map's
  edge gets no false closed edge.
- Per-layer shrink and grow (open, close) in output units; dropping parts and filling holes below an area.
- A helper for a sequence of maps from the caller: smoothing over neighbouring maps (a Gaussian, σ in maps). The library
  keeps no sequence itself. (Hole persistence was built and removed: it made the flicker worse, `design-decisions.md`, "Trace sequences (V14b)".)

With V9 this is the cut-paper piece.

## V15 Fields and lines

Size L, built in three slices.

**V15a, done 2026-11-17.** `Flow` (`angle`, `noise`, `contour(path)`, `mix`: directions without sign, mixed in doubled-angle
space so a direction and its opposite agree), `Field` (`ramp`, `distance(path, normalized=)`: signed, and normalised for letters
with counters, or a plain number) and `hatch(spacing, *, tone, flow, avoid)`: Jobard-Lefer streamlines inside each shape, spacing
varying with a tone, an avoided drawing, closed loops, on a `Path` and as a step, in Rust, Python and documents (so WebAssembly);
`examples/flow_hatch.py` in the three languages. Straight hatching clipped to a shape is `hatch` with `Flow.angle`. Reasons and
limits: `design-decisions.md`, "Fields and streamlines (V15a)".

**V15b, done 2026-11-17.** `Field.map` (8-bit grayscale bytes in, bilinear, placed by a `box`; an input of kind `map` in
documents) and `field.to_map` (a distance field out for image's `relight`, over a `low` to `high` range), `field.contours` (marching
squares: open lines at the frame, closed loops inside), Params in the numbers of a `Field` or a `Flow` (an angle, a `phase`, a
`fraction`) and a drawing in `Field.distance` and `Flow.contour` (a `hatch` step reads both with the values it draws with),
`hatch(below=)` (crosshatch and tone levels are stacked steps), and a `Flow.contour` direction blended across the middle of a
stroke; in Rust, Python and documents (so WebAssembly), with `examples/flow_hatch.py` in the three languages following the
headline's weight. Reasons and limits: `design-decisions.md`, "Fields and lines (V15b)". The halftone `tone` reads a field since 4a.

**V15c, done 2026-11-17; V15 done and M5 reached.** `rows` (ridgelines with hidden lines removed), `bands` (lines whose width
carries a tone along straight lines, waves, rings, a spiral or rays) and `stipple` (dots whose density follows a tone), each
on a `Path` and as a `Drawing` step, in Rust, Python and documents (so WebAssembly); `Path.within(region)` (lines kept inside a
region); `Field.contours(.., smooth=, min_length=)` with lines running the high side on the left (a matte's `contours(0.5)` is
its silhouette); `Drawing.to_plot_svg` (strokes only, a layer per pen, nearest-neighbour order, simplified, solids hatched at the
pen width, its sides in px, mm or in; `toPlotSvg` in JavaScript); a font's name in its cache key; `examples/line_families.py` in the three languages. The
M5 piece is a poster of the owner's dog: his outline from image's subject matte, drawn as rings whose width
is his tone, a headline hatched along its own outline keeping clear of him, a stippled shadow, and the plotter SVG of it (50,000
lines in three pens). Reasons and limits: `design-decisions.md`, "Line families, plotter output and tidy contours (V15c)".
After the owner's review (2026-11-17): `rows` keeps to its region once lifted and reads the tone over each point's share of
the rows, so a photo's fine detail no longer chops them.

**4a, done 2026-11-18 (effects track).** `halftone` reads a `Field` tone (a number stays a number per shape), takes a dot
`shape` (dot, square, ellipse with `aspect`, diamond, euclidean or a caller's `Path`) and a `screen` (grid, wave, rings,
spiral, rays; `center`, `amplitude`, `wavelength`), each dot covering `1 - tone` of its own cell on every screen; `bands` and
the screens share one layout module; in Rust, Python and documents (so WebAssembly), `examples/halftone.py` in the three
languages. Reasons and limits: `design-decisions.md`, "Halftone screens (4a)". Image's `line` and `matrix` shapes have no
counterpart: `bands` and a caller's `Path` do the same.

**4b, done 2026-11-18 (effects track).** `characters(content, *, cell, font, size, colors)` (a character grid as outlined glyphs, with
`char`, `row`, `column`, image's `colors` accepted as they come in Python and JSON) and `marks(boxes, points, *, bracket, cross, text_size, font)`
(corner brackets, crosshairs, labels on leader lines; open strokes; `role` and `mark`), with the monospaced JetBrains Mono
bundled (`jetbrains_mono`); in Rust, Python and documents (so WebAssembly), `examples/overlay.py` in the three languages.
Reasons and limits: `design-decisions.md`, "Character grids and overlay marks (4b)".

Distance fields have a second job: a signed, normalised distance field of letters or shapes is the relief that
image's `relight` lights. Written out as a grayscale map (`field.to_map`, V15b: bytes, the input form of `Field.map`), it lets scene give a headline chrome, emboss or a lit bevel with no 3D engine and no new link
between image and vector. Chrome type already works without it: scene's C5 example lights the headline's own brightness (`relight` with `field="light"`, a light fill).

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
`docs/api.md`, "SVG import". It does not read `evenodd`, `use`, gradients, clip paths, masks, text or CSS stylesheets, and names the one it meets.

Done when: an SVG with ids on a logo of 40 or more contours imports, `only(id=...)` selects each part, nested groups give the
group path, and the JSON round trip keeps the attributes.

## V17 to V19

**V17, done 2026-11-18.** Presets as JSON: `Drawing.preset(name)` and `Drawing.preset_names()` (JavaScript `Drawing.preset`,
`presetNames`; Rust `Drawing::preset`) give whole drawings that ship with the library, each a JSON document written from a
program in `examples/presets/`. Reasons: `design-decisions.md`, "Presets (V17)".

**V18, done 2026-11-18.** The command line, `bt-vector`, in the Python package: `list`, `describe`, `render` and `validate`.
Reasons and what it leaves out: `design-decisions.md`, "Command line (V18)".

**V19, done 2026-11-18.** Presets and a gallery that look good with no tuning: 13 presets (`bent_type`, `brush_taper`,
`draw_on`, `flow_hatch`, `glyph_scatter`, `gradient_type`, `halftone_screen`, `marching_dashes`, `ring`, `stipple`,
`text_on_path`, `two_fonts`, `waves`), each with a one-sentence description (`Drawing.preset_description(name)`,
`presetDescription` in JavaScript) kept in the `PRESETS` table and shown by `bt-vector list` and `describe`; each program in
`examples/presets/` is checked against its JSON file, its subject, its Params and the sentence rule; demo 36 is a gallery
sheet of the 13. Reasons: `design-decisions.md`, "Preset descriptions (V19)".

## V20 Browser playground

**V20, done 2026-11-18.** `playground/`: a static page (`index.html`, `playground.mjs`, no framework, no build step beyond the
WebAssembly glue) over the package built by `uv run poe playground` (the release simd128 build of `wasm-test`, then
`wasm-bindgen --target web` into the ignored `playground/pkg/`). A preset picker with the descriptions, a canvas drawn with
`toRgba` at its CSS size times the device pixel ratio (at most 2), a size select, one slider per Param (name, unit, meaning),
a source for each slider (manual, clock, pointer x, pointer y, scroll), the render and draw times, and a JSON document opened by
paste or file (an input it needs is named and refused). Renders coalesce per animation frame; library and document text goes
into the page as text. `playground/params.mjs` holds the slider maths; `tests/wasm/playground.test.mjs` (in `wasm-test`)
checks it on every preset of the real package, each slider drawn at its min, default and max. `uv run poe playground-serve`
serves the folder; `uv run poe playground-check` (`playground/check.mjs`, headless Chrome over the DevTools protocol, no
packages; not part of `check`) moves every slider of every preset to both ends, binds the pointer, scroll and clock, opens bad
documents and writes a screenshot. Reasons and what it leaves out: `design-decisions.md`, "Browser playground (V20)".

## E Effects track

One effect per small batch, merged with its gallery piece, from V15 on. What shipped, and the effects the track draws
from: [effects-roadmap.md](effects-roadmap.md).

## Asks from scene (2026-11-14), as shipped

What `bullterrier-scene` asked of vector while building real pieces, and where each stands.

- **`Path.dash` (V11)**: built (`crates/core/src/path/dash.rs`, `examples/marching_dashes.py`). The Conan poster had drawn
  stitching as many `slice` pieces.
- **A gradient fill for text, reachable from scene**: vector already fills text with a gradient (`examples/gradient_type.py`),
  and a gradient is plain data in a document (`formats.md`, the `fill` and `stroke` rows); scene passes the paint on its own
  side, so vector changed nothing.
- **`#rgb` colours on fills and strokes** (done 2026-11-16). `Color::parse` is the one parser (the SVG reader calls it) and
  reads `#rgb`, `#rrggbb`, `#rrggbbaa` and `none`; image and the SVG reader have no `#rgba`, so neither has vector.
- **Window render (V10e)** (done 2026-11-13, Rust, Python and WebAssembly): for every drawing, what looks the same as the whole
  render's rows and columns.
- **WebAssembly builds** already, so a scene editor or a web export can draw vector in the page.

## vello_cpu 0.4 and the painter (BTVEC-0013, 2026-11-22)

`vello_cpu` 0.3.0 to 0.4.0 with `vello_common` 0.4.0 pinned beside it; the pixel drawing rules (`output/raster.rs`) paint
through the crate-private `Painter` (`output/painter.rs`), `vello_cpu` its one implementation, so a GPU painter on
`vello_gpu` needs no second copy of them. 17 of 1,079 measured renders moved by 1 to 4 levels on translucent pixels, no
closer to or farther from `resvg`; the painter moved no byte. No API change. Measured in `docs/design-decisions.md`
("`vello_cpu` 0.4", "One painter for the drawing rules") and `docs/benchmarks.md`.
