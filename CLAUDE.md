# CLAUDE.md: bullterrier-vector

Rust library with Python (and, later, WebAssembly) bindings: a **typography-first vector graphics scripting library**. You write
a picture in code (Bezier paths, type, booleans, offsets, deformers, repeaters, generative forms, tracing), and it is drawn as SVG
or pixels, looking the same from Rust, Python and the browser. `README.md` (status), `docs/design.md` (what and how),
`docs/design-decisions.md` (why), `docs/roadmap.md` (the batches, in order; effects in `docs/effects-roadmap.md`),
`docs/prototype-findings.md` (what the taste prototypes taught), `CONTRIBUTING.md` (setup and CI for people).

**Write docs tersely.** This file, the README, `docs/`, docstrings, reports and briefs: state the rule or fact, drop preamble,
repetition and justification a reader does not need to act. Reasons go in `docs/design-decisions.md`, once. Committed files are
public: no absolute paths, no references to folders outside the repository.

## Start here

```bash
uv sync && uv run poe setup                      # once per clone: environment (builds the Rust extension: needs rustup), git hooks, commit template, cargo tools
cargo test -p bullterrier-vector --test it path::   # the Rust tests near your change (one module), while you work
uv run pytest tests/test_path.py -n 0            # the Python tests near your change
uv run poe look examples/star.py                 # draw a drawing file to dist/look/ (PNG and SVG) to see it
uv run poe fast                                  # the inner loop: seconds when warm
uv run poe check                                 # the gate, before you report or merge (the slow proofs only when due)
uv run poe full                                  # check with the proofs forced: before a tag (CI runs it)
```

Needs [rustup](https://rustup.rs): the first `cargo` command in the checkout fetches the toolchain `rust-toolchain.toml` pins and
the `wasm32-unknown-unknown` target. `uv run poe` lists every task.

**State:** the scaffold (workspace, gates, hooks, CI workflows) and the geometry value `Path` (with `Point` and `Rect`) from the
Rust core to Python (V2a: build, measure, cut, move, hit-test, SVG path data); the core also builds for WebAssembly. Drawings
(V2b): `Drawing`, `Shape`, `Style`, `Param`, `Expr` and the verbs listed by `verbs()`, evaluated with a result cache, in Rust
and Python; `examples/series_bars.py` runs as far as `shapes()`, and `examples/poster.py` draws a small poster (`uv run poe look examples/poster.py`). JSON (V2c): `Drawing.to_json`,
`Drawing.from_json` and `drawing_schema()` (the schema generated from the registrations) in Rust and Python, for the calls
registered so far. Output (V3a): `Size` and `Drawing.to_svg(size, values)` in Rust, `Drawing.to_svg(width, height, /, **values)` in
Python, and `examples/star.py` writes its SVG. Pixels (V3b): `Drawing::to_rgba(size, values)` in Rust and `Drawing.to_rgba(width,
height, /, **values)` in Python, straight-alpha bytes drawn by `vello_cpu`. PNG (V3c): `Drawing::to_png(size, values)` and
`Drawing.to_png(width, height, /, **values)`, the pixels of `to_rgba` as a file (the `png` crate), and `uv run poe look FILE`,
which draws a `.json` or `.py` drawing to `dist/look/` as PNG and SVG. Mesh (V3d): `Drawing::to_mesh(size, values, &mut Mesh)` in Rust and
`Drawing.to_mesh(width, height, /, *, into=None, **values)` in Python, triangles from `lyon_tessellation`. WebAssembly (V3e, V3f): the package built from `crates/wasm` loads a
document and writes `toSvg`, `toRgba`, `toPng`, `toMesh` and `toMeshInto` (also `toJson`, `parameters`, `size`, the `Mesh` class
and `wasmMemory()`) in Node: the SVG text and the mesh buffers equal the native ones, the pixels look the same within a
measured tolerance (`poe wasm-test`, 148 drawings: 44 paths, 17 texts, 2 bends, 9 per-item, 16 booleans, 16 offsets, 24 style effects and the gradient, blend, clip and mask rows); `uv run poe bench` times the outputs (`docs/benchmarks.md`). V3 is
done and M1 (a star everywhere: `examples/star.py`, `crates/core/examples/star.rs`, `crates/wasm/examples/star.mjs`) is reached.
Frame export (V4): `btvec.frames(scene, *, width, height, fps=30, seconds, out)` in Python only, a PNG sequence or a
ProRes 4444 or VP9 clip with alpha through ffmpeg (`examples/badge_frames.py`). Fonts (V5a to V5c): `Font::new(bytes)` / `btvec.Font(data, *, name=None)` with `axes` (a `Param` each) and `features`,
`btvec.text(content, *, font, size, axes, features, tracking, leading, align, anchor)` (Rust: `text(content,
Text::new(&font).size(..).axes([("wght", &weight)]).features([("liga", 0)]))`; one shape per glyph, shaped with kerning
and ligatures, with `glyph`, `char`, `word`, `line`, `order`, `origin`; a `Param` list gives each character its own axis
value, shaped per run of equal location), `Inputs` and `Drawing.from_json(text, *, inputs=None)` (in JavaScript
`Drawing.fromJson(text, inputs?)`, names to `Uint8Array`s; with `font-checks`); `to_json` writes `inputs` with `sha256`.
Text is in the WebAssembly differential (the SVG text and mesh buffers equal the native ones); `examples/kinetic_type.py`
draws a headline of it, `uv run poe bench --text` times it (`docs/benchmarks.md`). Web and bundled fonts (V5d):
`Font::new` unpacks WOFF2 and refuses WOFF; `Font::bundled(name)` and `Font::bundled_names()` (Python `btvec.Font.bundled`) give
Bricolage Grotesque and Fraunces (SIL OFL 1.1; `examples/bundled_type.py`), and a document names them by digest and loads without
bytes. V5 is done. Layout (V6): `text(..., width=, fit=, inside=)` (Rust `Text::width`, `fit`, `inside`) wraps lines at whitespace,
fits each line to the width by `"size"` or a font axis (at the reference pose, `ParameterError` when the axis cannot reach
it) or sets the text in the region a `Path` fills (`examples/kinetic_type.py` fills its measure with `fit="wdth"`);
`Path.along(path, *, start=0, near=None)` and `Drawing.along(...)` carry shapes along a guide, so text on a path is
`text(...).along(path)` (`examples/text_on_path.py`); `width`, `fit`, `inside` and `along` are in documents and in the
differential. V6 is done. Per-item variation (V7): the numbers `btvec.by_index`, `by_order`, `random(low, high, *, seed=0)` and `stagger(progress, *, spread=0.5)` (Rust `by_index`, `by_order`, `Random`, `Stagger`; one `Expr` node, never folded to a constant) vary any step's number per shape; `Drawing.split()` (one shape per part, with `part` and its own `origin`), `Drawing.merge(*, by=None)` (sub-paths joined by an attribute such as `"word"`, not united) and `Drawing.slice(start, end, *, near=None)` (draw-on is `text(...).slice(0, stagger(progress))`); transforms default `about` to `origin` and move it; `examples/draw_on.py` and `examples/glyph_scatter.py`; all in documents and in the differential. V7 is done. Booleans (V8): `Path.union`, `subtract`, `intersect` and `exclude` (each with `other` and `tolerance=0.01`) and `Path.resolve(*, tolerance=0.01)`, and the same steps on a `Drawing` (`other` is a `Drawing`, `Path` or `Shape`, all its shapes united once per step: the argument kind `Kind::Drawing`, in a document an array of calls); regions by the nonzero rule from curves flattened to `tolerance`, straight-edged and canonical, from `i_overlay` (`path/boolean.rs` alone); `text(...).resolve()` unites a glyph's overlapping contours (opt-in: `text` is unchanged); fuzzed (`path_boolean_fuzz`, a glyph corpus), a `Drawing` step charges the flattened edges, the overlapping edge-box pairs and the result's vertices against `MAX_SEGMENTS`, the known limits are in `docs/api.md` ("Booleans"); `examples/cutout.py`, `uv run poe bench --booleans`; all in documents and in the differential. V8 is done. Offsets (V9): `Path.offset(distance, *, join="round", miter_limit=4, tolerance=0.01)` (outward, inward when negative; holes shrink as outer contours grow) and `Path.outline(width, *, join="round", cap="round", miter_limit=4, tolerance=0.01)` (the region a stroke paints, as closed outlines), and the same steps on a `Drawing` (`distance` and `width` are per-item numbers, the rest literals); from `i_overlay` (`path/offset.rs`, with `boolean.rs` its only user), canonical and straight-edged as the booleans; a `miter` past `miter_limit` is clipped flat (SVG 2 `miter-clip`), where `stroke` bevels; a step charges `MAX_SEGMENTS` as the booleans do; the known limits are in `docs/api.md` ("Offsets"); `examples/cut_paper_layers.py` and `examples/ring.py`; all in documents and in the differential. V9 is done. The rest is planned (`docs/roadmap.md`); Style effects (V10): `shadow(*, dx=0, dy=8, blur=16, color)` (a drop shadow behind the shape, `blur` the CSS blur radius so sigma is half of it), `knockout()` (the shape removes what is below it instead of painting) and `halftone(color, *, tone, pitch=8, angle=45)` (a dot screen on a lattice anchored to the design frame, clipped to the shape; `tone` a per-item number, 0 solid to 1 none) are style attributes, written into SVG, drawn in pixels and cut into the mesh alike (`Style::shadow`, `knockout`, `halftone`; Python `Style.shadow`, `knockout`, `halftone`); a blurred shadow raises in `to_mesh`, a blur above 1000 device pixels, nesting past `MAX_KNOCKOUT_LAYERS` and a screen of too many dots raise `ParameterError`; the SVG is checked against the pixels by rendering it with `resvg` (a dev-dependency of the core tests only, `tests/it/common/svg_oracle.rs`); `examples/halftone.py`, `uv run poe bench --halftone`; all in documents and in the differential. V10 is done. Blend modes (V10d part 2): `blend(mode)` takes the ten separable CSS modes beyond `normal` and `multiply` (`screen`, `overlay`, `darken`, `lighten`, `color-dodge`, `color-burn`, `hard-light`, `soft-light`, `difference`, `exclusion`; `Blend::NAMES` is the one list), written as `mix-blend-mode` and drawn by `vello_cpu`, checked against the W3C formulas and `resvg`; the mesh refuses any mode but normal and multiply. Gradient paints (V10d part 1): `btvec.linear_gradient(stops, *, start, end)` and `radial_gradient(stops, *, center, radius)` (Rust `Gradient::linear`/`radial`, validated; `Paint` is a `Color` or a `Gradient`, the new `Kind::Paint` of `fill` and `stroke`) paint a fill or stroke with literals in design pixels of the frame, pad extend, sRGB stops interpolated unpremultiplied (`output/ramp.rs` places them for SVG and pixels); a shadow follows the paint's alpha, a knockout ignores it, a stroke whose pieces overlap is painted once (`SrcIn` layer), `to_mesh` raises `ParameterError`; `examples/gradient_type.py`; all in documents and in the differential. Clip and mask (V10d part 3): `clip(region)` and `mask(source)` (Rust `Drawing::clip`, `mask`; Python `Drawing` and `Path`; `Style.clip`, `Style.mask`) cut each shape and its shadow to the union of a `Drawing`, `Path` or `Shape` argument (`Kind::Drawing`, as a boolean's `other`) or fade it by that source's luminance as painted; each shape on its own, made once per step and shared as an `Arc` on `Style`, so one mask is one SVG `<mask>`, one pixel `Mask` and one charge of the canvas to `MAX_FILTER_PIXELS`; a mask holds no masks, a knockout ignores both, a mesh refuses both; ids inside a mask are prefixed with its own; `examples/cut_type.py`; in documents and in the differential. V10d is done. V10e (render a window) is next.
Maintainers may keep machine-specific notes in a git-ignored `.local/` folder and batch plans in the git-ignored
`.superpowers/plans/`: read them when present; never commit or cite them.

## What it is and is not

| Responsible for (planned: `docs/roadmap.md`) | Not responsible for |
|---|---|
| Bezier paths as values: build, measure, split, slice, hit-test, resample, simplify, fit curves; booleans, offsets, stroke outlines | Timelines, keyframes, easing over time, an `onFrame` loop: values over time are the caller's |
| Drawings written as code: chainable, typed, discoverable from the editor (JSON is a layer on top); `Path`, `Shape`, `Drawing`, `Param`, the same verbs on a `Path` (computed at once) and a `Drawing` (a step) | Writing a rasteriser from scratch, a shaper or a font parser: existing crates do that |
| Type as geometry: any TrueType or OpenType font from bytes, shaping, variable-font axes as parameters, every glyph an editable path | Producing font files; colour and bitmap fonts in v1 |
| A generative layer: generators, deformers, repeaters, booleans, offsets, morph, hatching, tracing | An editor application (the library gives it geometry, hit testing, text layout) |
| Effects, each added with a gallery piece (the effects track) | Photo effects and compositing over footage |
| Describing every parameter: name, unit, range, default, meaning | Stateful simulation (particles, springs) in v1: if it comes, it produces points that a node draws |
| The same drawing looking the same on every target (same bytes wherever that is cheap); SVG that keeps every effect, plotter-ready SVG, RGBA pixels | Reading other tools' file formats: inputs are plain data (width, height, bytes; lists of numbers) |
| Redrawing with new values cheaply: a live preview, live in a page through the WebAssembly package | |
| Frame export: a caller's function of time looped over frames into a transparent PNG sequence or clip | |

**State, not time:** no seconds, frames, clips, easing or fps in drawings, parameters or their docs; the one exception is the
frame-export helper (V4), which loops a caller's function of time over frames. A parameter is a *state* (`progress` 0..1, `phase`
in cycles, `seed`); whoever animates it sets it per frame and draws again.
**Code first, JSON second:** every drawing written in code saves to JSON and loads back to the same drawing, never the other way
round. Design each API with examples written first, then the format.
Works with other tools only through plain data: `docs/design.md`, "Works with".

## Names

`bullterrier-vector` (crate, PyPI distribution, repository), `bullterrier_vector` (Python import, Rust library),
`bullterrier-vector-python` (the bindings crate; builds `bullterrier_vector._native._core`), `bullterrier-vector-wasm` (the
WebAssembly bindings crate; builds `pkg/bullterrier_vector_wasm.js`), `bt-vector` (command line; planned,
V18), `BullterrierVectorError` (base exception), `ParameterError` (a value outside what the library accepts; also a
`ValueError`), `GeometryError` (geometry that fails, such as measuring an empty path; no boolean raises it), `FormatError`
(a document `from_json` cannot read; also a `ValueError`), `btvec` (the alias the docs import the library as). Full name
for identity, `bt-` for what a person types. Names inside the package carry no prefix.

Licence: MIT for the code, in `LICENSE`; SIL OFL 1.1 for the bundled fonts, in `crates/core/fonts/*/OFL.txt` (`MIT AND OFL-1.1`); both named by
`license-files` in `pyproject.toml`, and `build-check` fails a distribution without either.

## Architecture

Three crates in one Cargo workspace; the package version is the workspace's.

| Path | What |
|---|---|
| `crates/core` | crate `bullterrier-vector`: pure Rust, the source of truth. `src/lib.rs` (the public API), `error.rs`, `geometry.rs` (`Point`, `Rect`, `Scale`), `math.rs`, `path.rs` and `path/` (`region.rs`: the spans of the region a path fills at a height, `bend.rs`: `Path::along`, `boolean.rs`: `resolve`, `union`, `subtract`, `intersect` and `exclude`, `offset.rs`: `offset` and `outline`; these two are the only users of `i_overlay`); `param.rs`, `values.rs`, `expr.rs` and `expr/item.rs` (Params, their values, arithmetic on them; `item.rs` the per-item numbers); `shape.rs` and `shape/` (`Shape`, `Style`, `Color`, `Attributes`); `node.rs` (the node contract, written in its module docs) and `node/` (the nodes: sources, `text`, transforms and `slice`, `items.rs` for `split` and `merge`, `boolean.rs` for the five boolean steps, `offset.rs` for `offset` and `outline`, style, select, `with_values`); `registry.rs` (`verbs()`); `drawing.rs` and `drawing/` (`Drawing`, its typed steps, `boolean.rs` the typed boolean steps, `offset.rs` the typed offset steps, evaluation, the result cache); `limits.rs` (`MAX_SHAPES`, `MAX_SEGMENTS`, `MAX_TESSELLATION_WORK`, `MAX_DEPTH`, `MAX_PARAMS`, `MAX_SETTINGS`, the counter); `type.rs` and `type/` (`Font`, `Text`, `text`: `font.rs` the checked bytes, their SHA-256, axes, features and locations, `shaping.rs` the only user of `harfrust`, `outline.rs` glyph outlines, `lines.rs` the lines of a text (wrap, fit to a width, set inside a shape), `layout.rs` runs per location, pens and attributes, `unpack.rs` WOFF2 to sfnt, the only user of `wuff`, `bundled.rs` the two bundled fonts; `read-fonts` and `harfrust` are used nowhere else); `fonts/` (the bundled fonts as WOFF2, each folder with its `OFL.txt`, compiled in by `include_bytes!`); `output.rs` and `output/` (`Size`, the `View` that places the design frame in an output, the SVG writer, `outline.rs`: the stroke outline both pixels and mesh use, `raster.rs`: the pixel writer and the only user of `vello_cpu`, `screen.rs`: the halftone lattice, dots and their classification against a shape (no rasteriser types), `png.rs`: the PNG writer and the only user of `png`, `mesh.rs`: `Mesh` and `to_mesh`, the only user of `lyon_tessellation`; `kurbo` is used by `outline.rs`, `raster.rs` and `mesh.rs`); `json.rs` and `json/` (`to_json`, `from_json`: a strict parser, the reader, the loader's checks, `inputs.rs` the bytes of a document's inputs, the writer; `drawing_schema()` generated in `schema.rs` from `verbs()` and the fixed `schema_fixed.json`; no list of calls); `examples/star.rs` (the star of `examples/star.py`; it may use std, the core may not); `tests/` |
| `crates/python` | PyO3 bindings building the extension `bullterrier_vector._native._core`; generic, passes values to the core unchecked; no Rust tests. `src/lib.rs` (`Path`, `Point`, `Rect`, the error mapping), `expr.rs` (`Expr`, `Param`), `drawing.rs` (`Drawing`), `shape.rs` (`Shape`, `Style`), `text.rs` (`Font`, `text`) |
| `crates/wasm` | wasm-bindgen bindings building the WebAssembly package (`cdylib`); generic, passes values to the core unchecked; no Rust tests (Node tests the built package: `tests/wasm/`). `src/lib.rs` (`Drawing` with `fromJson(text, inputs?)`, `Mesh`, `wasmMemory`, the error mapping: a JavaScript `Error` whose `name` is the kind); `examples/star.mjs` |
| `src/bullterrier_vector/` | the Python package: `errors.py` (the bottom layer, imports nothing from the package), `_native/` (the extension and its stub `_core.pyi`), `frame_export.py` (`frames`: the one part that knows time; PNG files and an ffmpeg process, so outside the core), `__init__.py` (the public API) |
| `tests/` | the Python suite (`test_*.py`; fontTools is the oracle of the font tests and builds their test fonts); `tests/tools/` tests the tooling and the gates; `tests/wasm/` holds the Node tests of the WebAssembly package and `oracle.py`, which writes what the native extension answers for them (and the corpus of drawings they share) |
| `examples/` | example programs that run as far as the library can draw; `tests/test_examples.py` runs each. Code, never documentation (not in the `changes` filter). `examples/fonts/` holds the OFL font subset they read, with its licence. The others stay in `docs/examples/` until their calls exist |
| `tools/` | scripts the hooks and gates run (`check_commit_message.py`, `check_font_features.py`, `check_one_config_home.py`, `pre_push.py`, `rust_tools.py`, `lint_workflows.py`, `check_dists.py`, `check_installed.py`, `check_file_size.py`), `look.py` (`uv run poe look FILE [--size WxH] [name=value ...]`: not a gate; draws a drawing file to the ignored `dist/look/` as PNG and SVG, to open and look at), and `bench.py` with `bench.mjs` (`uv run poe bench`: not a gate, not in `check`; times `to_rgba` and `to_mesh` natively and in Node, `--booleans` the boolean verbs natively, nothing asserts on time) |

Rust is the source of truth: parameters are declared once, in Rust; Python and (from V3) WebAssembly are thin layers over it.
The core does no IO of its own: callers pass bytes. `pub` only for what the bindings use, else `pub(crate)` (`unreachable_pub`
fails the gate). Python and the browser reach the core only through their bindings crates (`deny.toml` bans `pyo3` and `wasm-bindgen`
everywhere else). The Rust `Error` maps to the Python exceptions and to JavaScript `Error`s (`name` is the kind) in the bindings; the core owns every range check, so every language gets the same error.

The design's planned modules (geometry, type, scene, parameters, nodes, output, json), each hiding one decision, are in the table in
`docs/design.md`, "Shape of the library"; `math`, `error`, geometry (`geometry`, `path`) and the core of scene, parameters and
nodes (`drawing`, `shape`, `node`, `param`, `expr`, `registry`), `json` and `output` (SVG so far) exist so far. A crate that does one of these jobs is never exposed in the API.

## Rules

@docs/rules.md

The shared rules are the template's (`docs/rules.md`, `docs/template-decisions.md` say why). Vector's own rules, on top:

Gated (the gate fails): the maths and clock bans (clippy.toml), the wasm build and the stack tests. Convention (reviewed, not gated): 1, 2, the
one number formatter and 6.

1. **Hidden tunables.** A value that shapes a result is a named, documented parameter in the field's own terms. The exempt
   constants (a mathematical or standard constant, or a tolerance below any visible effect) carry `// constant: <reason>`.
2. **Determinism, pragmatically.** Same inputs and `seed` give the same drawing on every target: it must look the same.
   We aim for the same bytes for what we compute (geometry, the view, SVG text, straight-alpha conversion) and the
   differential tests check it, but an unforeseen difference nobody can see (a last-digit coordinate, an edge pixel off by
   a level) is not a defect: never add complexity (a workaround, a special path, a dependency) to remove it; loosen that test's tolerance and record the cause in
   `docs/design-decisions.md`. A visible difference is a defect. Randomness comes only from the seed. No global random state.
3. **Transcendental maths only through `crate::math`** (`libm`); `f64::sin`, `exp`, `powf` and the rest are banned by
   `clippy.toml`. `sqrt` and `mul_add` are allowed.
4. **No clock, no hash-ordered collections, no IO in the core**, as banned in `clippy.toml` (the ban list lives there,
   with the reason beside each entry): use `BTreeMap`/`BTreeSet` and take bytes from the caller. One formatter writes every
   number into SVG (now `number` in `path/write.rs`): no other `format!` of a coordinate.
5. **The core builds for `wasm32-unknown-unknown` in every gate run** (`rust-wasm`, and the bindings with `wasm-build`): a
   native-only API in it fails there.
6. **Errors by kind.** In Rust, `bullterrier_vector::Error` is `#[non_exhaustive]` (`Parameter`, `Geometry`): a new kind is a new
   variant (`Error::Format` for a document `from_json` cannot read). `ParameterError`, `GeometryError` and `FormatError` are
   the Python classes; `FormatError` and `ParameterError` also inherit `ValueError`.
7. **Bounded depth.** Anything a caller can build or load has a depth limit enforced where it is built, never in a walk
   over it, so no recursion runs over unbounded depth and nothing needs a bigger stack than the 256 KB minimum
   (`docs/design-decisions.md`, "Bounded depth"). A new node, step or reader adds its deepest structure to the stack
   tests (`tests/test_stack_depth.py`, `crates/core/tests/stack.rs`: unoptimised, see `[profile.stack]`).

## Vector's gates

The template's gates are in `docs/rules.md`. Vector adds five, hung on `project-gates` in `pyproject.toml` (run last by `check`
and `full`; `tests/tools/test_gate_wiring.py` fails when a tier lacks one; each has a proof in `tests/tools/`):

- `file-size` (`tools/check_file_size.py`) fails a tracked `.py` or `.rs` file over 1000 lines (test files: 1500): split it by
  topic, never raise the budget. A file already over is listed in `[tool.file-size.exempt]` (path = current lines): it may
  shrink but never grow, and a stale entry fails.
- `rust-fonts` fails when `read-fonts` or `harfrust` is built with `std` (their maths would be the platform's;
  `tools/check_font_features.py`).
- `wasm-build` builds the core and the bindings crate for WebAssembly (the template's `rust-wasm` checks the core alone).
- `rust-test-stack` runs the 256 KB stack tests unoptimised.
- `wasm-test` (needs Node) builds the package in release with `RUSTFLAGS=-C target-feature=+simd128` (for that build alone),
  runs `wasm-bindgen --target nodejs` into the ignored `pkg/` and runs `tests/wasm` in Node (2 GB heap cap) against
  `tests/wasm/oracle.py`'s native answers (exact SVG text, mesh buffers, JSON, Param descriptions, error names and messages;
  pixels and PNG within a measured tolerance, `tests/wasm/compare.mjs`).

`[tool.gate-proofs] project-triggers` lists what vector's own proofs read. `tools/check_installed.py` and `tools/check_dists.py`
(the bundled fonts' `OFL.txt` among the licence files) check an installed or built distribution; `uv run poe look FILE` draws a
drawing file to `dist/look/`; `uv run poe bench` times the outputs.

## CI

`.github/workflows/`: `ci.yml` (pushes to `main`, pull requests, merge queue; calls the rest), `lint.yml`, `test.yml`,
`security.yml` (also weekly), `pre-commit.yml`, `build.yml` and `project.yml`; `.github/dependabot.yml` for weekly updates. The
first six and dependabot are the template's; `project.yml` is vector's: one job runs `poe project-gates` (file size, fonts,
WebAssembly with Node, the stack tests) on Linux. `ci-ok` is the only required check: every job is in its `needs`, and a skipped
job passes. A pull request that changes only documentation skips the code jobs. The gate proofs run once, on Linux Python 3.12.
`build` runs after the checks passed and produces the sdist and a wheel per platform (`docs/template-decisions.md`, "Platforms").
Third-party actions are pinned by commit SHA; `lint-workflows` rejects the rest.

## Testing, vector's specifics

On top of the Testing section of `docs/rules.md`:

- **Rust first, black-box.** Rust tests live in `crates/core/tests/it/` and use the public API (`use bullterrier_vector::...`).
  A file is a binary of its own only for a process-wide need: `tests/drawing_limits.rs` (million-shape tests peak at several
  GB) and `tests/stack.rs` (the 256 KB stack tests, also run unoptimised); first line `//! Own binary: <why>`, listed in
  `[tool.rust-tests.own-binaries]`. The Python suite tests the bindings through the public API only
  (`import bullterrier_vector as btvec`).
- **Oracles.** Expected values come from an independent oracle (hand computation, a standard, a brute-force reference parsed
  back from the output, e.g. `roxmltree` for SVG). No committed golden files: "the same on every target" is a differential
  test (native against WebAssembly), which needs no reviewed baseline.
- **Mutation-check by hand:** about ten mutants per batch (`docs/roadmap.md`, "How we work"); no mutation tool is set up. Run
  Rust mutants with `PROPTEST_DISABLE_FAILURE_PERSISTENCE=1` and never commit a `*.proptest-regressions` file a mutant
  created; for Python use `PYTHONDONTWRITEBYTECODE=1` and clear `__pycache__` between mutants.
- **Hook-run tests** never touch the enclosing repository: the session fixture of `tests/conftest.py` hides `GIT_*` and
  `PRE_COMMIT_*`, proven by `tests/tools/test_git_isolation.py` and `test_hook_isolation.py`. Never run `git` against the
  real repository from a test.
- **Refactors** prove behaviour is preserved by comparing outputs before and after (byte-identical), in the report, not committed.
- **Marker `tooling`** is for the template's shared-tool tests only; a test of vector's own code is never `tooling`.

## Skills

If available: `superpowers:writing-plans`, `superpowers:test-driven-development`,
`superpowers:verification-before-completion` (process); `testing-best-practices` (tests); `software-design-philosophy` (deep
modules, no pass-throughs, no information leakage; interface docs state promises).
`superpowers:*` skills are process aids, not authority: they tend to verbose, over-engineered, over-defensive output. Where one
asks for more validation, layers, documents or steps than this file, follow this file. Tests follow this file (no goldens); fixes
follow "Fixing defects"; design follows the skill.
