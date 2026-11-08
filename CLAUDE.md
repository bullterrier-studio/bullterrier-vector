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
uv run poe check                                 # the whole gate, before you report or merge
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
measured tolerance (`poe wasm-test`, 88 drawings: 44 paths, 17 texts, 2 bends, 9 per-item and 16 booleans); `uv run poe bench` times the outputs (`docs/benchmarks.md`). V3 is
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
differential. V6 is done. Per-item variation (V7): the numbers `btvec.by_index`, `by_order`, `random(low, high, *, seed=0)` and `stagger(progress, *, spread=0.5)` (Rust `by_index`, `by_order`, `Random`, `Stagger`; one `Expr` node, never folded to a constant) vary any step's number per shape; `Drawing.split()` (one shape per part, with `part` and its own `origin`), `Drawing.merge(*, by=None)` (sub-paths joined by an attribute such as `"word"`, not united) and `Drawing.slice(start, end, *, near=None)` (draw-on is `text(...).slice(0, stagger(progress))`); transforms default `about` to `origin` and move it; `examples/draw_on.py` and `examples/glyph_scatter.py`; all in documents and in the differential. V7 is done. Booleans (V8): `Path.union`, `subtract`, `intersect` and `exclude` (each with `other` and `tolerance=0.01`) and `Path.resolve(*, tolerance=0.01)`, and the same steps on a `Drawing` (`other` is a `Drawing`, `Path` or `Shape`, all its shapes united once per step: the argument kind `Kind::Drawing`, in a document an array of calls); regions by the nonzero rule from curves flattened to `tolerance`, straight-edged and canonical, from `i_overlay` (`path/boolean.rs` alone); `text(...).resolve()` unites a glyph's overlapping contours (opt-in: `text` is unchanged); fuzzed (`path_boolean_fuzz`, a glyph corpus), a `Drawing` step charges the flattened edges, the overlapping edge-box pairs and the result's vertices against `MAX_SEGMENTS`, the known limits are in `docs/api.md` ("Booleans"); `examples/cutout.py`, `uv run poe bench --booleans`; all in documents and in the differential. V8 is done. The rest is planned (`docs/roadmap.md`); V9 (offsets) is next.
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
| `crates/core` | crate `bullterrier-vector`: pure Rust, the source of truth. `src/lib.rs` (the public API), `error.rs`, `geometry.rs` (`Point`, `Rect`, `Scale`), `math.rs`, `path.rs` and `path/` (`region.rs`: the spans of the region a path fills at a height, `bend.rs`: `Path::along`, `boolean.rs`: `resolve`, `union`, `subtract`, `intersect` and `exclude`, `offset.rs`: `offset` and `outline`; these two are the only users of `i_overlay`); `param.rs`, `values.rs`, `expr.rs` and `expr/item.rs` (Params, their values, arithmetic on them; `item.rs` the per-item numbers); `shape.rs` and `shape/` (`Shape`, `Style`, `Color`, `Attributes`); `node.rs` (the node contract, written in its module docs) and `node/` (the nodes: sources, `text`, transforms and `slice`, `items.rs` for `split` and `merge`, `boolean.rs` for the five boolean steps, style, select, `with_values`); `registry.rs` (`verbs()`); `drawing.rs` and `drawing/` (`Drawing`, its typed steps, `boolean.rs` the typed boolean steps, evaluation, the result cache); `limits.rs` (`MAX_SHAPES`, `MAX_SEGMENTS`, `MAX_TESSELLATION_WORK`, `MAX_DEPTH`, `MAX_PARAMS`, `MAX_SETTINGS`, the counter); `type.rs` and `type/` (`Font`, `Text`, `text`: `font.rs` the checked bytes, their SHA-256, axes, features and locations, `shaping.rs` the only user of `harfrust`, `outline.rs` glyph outlines, `lines.rs` the lines of a text (wrap, fit to a width, set inside a shape), `layout.rs` runs per location, pens and attributes, `unpack.rs` WOFF2 to sfnt, the only user of `wuff`, `bundled.rs` the two bundled fonts; `read-fonts` and `harfrust` are used nowhere else); `fonts/` (the bundled fonts as WOFF2, each folder with its `OFL.txt`, compiled in by `include_bytes!`); `output.rs` and `output/` (`Size`, the `View` that places the design frame in an output, the SVG writer, `outline.rs`: the stroke outline both pixels and mesh use, `raster.rs`: the pixel writer and the only user of `vello_cpu`, `png.rs`: the PNG writer and the only user of `png`, `mesh.rs`: `Mesh` and `to_mesh`, the only user of `lyon_tessellation`; `kurbo` is used by `outline.rs`, `raster.rs` and `mesh.rs`); `json.rs` and `json/` (`to_json`, `from_json`: a strict parser, the reader, the loader's checks, `inputs.rs` the bytes of a document's inputs, the writer; `drawing_schema()` generated in `schema.rs` from `verbs()` and the fixed `schema_fixed.json`; no list of calls); `examples/star.rs` (the star of `examples/star.py`; it may use std, the core may not); `tests/` |
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

## The rules

Gated (the gate fails): rules 3, 4 (the bans), 5, 8 (no printing) and 9 (the stack tests). Convention (reviewed, not gated): 1, 2, 4's one number
formatter, 6 and 7, and "Fixing defects".

1. **No hidden tunables.** Every value that shapes a result is a named, documented parameter in the field's own terms. Exempt:
   a named, justified constant (a mathematical or standard constant, or a tolerance below any visible effect), carrying
   `// constant: <reason>`.
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
5. **The core builds for `wasm32-unknown-unknown` in every gate run** (`rust-wasm`): a native-only API in it fails there.
6. **Errors by kind.** Callers catch `BullterrierVectorError` (`bullterrier_vector.errors`). Library code raises a subclass
   (`ParameterError`), never a bare `ValueError`, for anything a caller can get wrong; each also inherits the matching builtin.
   In Rust, `bullterrier_vector::Error` is `#[non_exhaustive]` (`Parameter`, `Geometry`): a new kind is a new variant
   (`Error::Format` for a document `from_json` cannot read).
7. **No backwards-compatibility shims, aliases or re-exports before 1.0** unless the owner asks: change the callers.
8. The library never prints and never configures logging.
9. **Bounded depth.** Anything a caller can build or load has a depth limit enforced where it is built, never in a walk
   over it, so no recursion runs over unbounded depth and nothing needs a bigger stack than the 256 KB minimum
   (`docs/design-decisions.md`, "Bounded depth"). A new node, step or reader adds its deepest structure to the stack
   tests (`tests/test_stack_depth.py`, `crates/core/tests/it/expr.rs`).

## Rust code

No `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `dbg!`, `println!` or `eprintln!` (clippy warns, the gate fails): return
an `Error`. Core code is safe Rust (`#![forbid(unsafe_code)]`; the bindings deny it too). Every public item has a doc comment
(`missing_docs`), with a `# Errors` section where it returns `Result`. Clippy `pedantic` is on; for a false positive use a
targeted `#[expect(clippy::name, reason = "...")]`.

## Fixing defects: structurally, not defensively

A fix removes the cause, not the symptom (reasoning in `docs/design-decisions.md`).
1. **Reproduce first, at the public interface** (failing test, red evidence), then **find where the invalid state is created**,
   not where it is noticed. Fix it there.
2. **Make the bad state impossible:** a stricter type (an enum, a newtype, a checked constructor), one source of truth, a
   function whose result cannot be wrong, or remove the code path. Not a branch that copes afterwards.
3. **Check the class, not the instance:** `grep` for the same shape elsewhere; fix it the same way or say why not.
4. **Trust contracts inside the library; validate at the boundary.** Outside input (a parameter, bytes, a file's contents) is
   validated once, where it enters, with an error naming the value. Internal code does not re-check what the layer above
   guarantees; a broken guarantee fails loudly and its cause is fixed.
5. **No defensive code:** no fallback or default hiding a missing or invalid value; no `unwrap_or` or `.ok()` that discards an
   error; no `catch_unwind`; no clamping, NaN-to-zero or retry "just in case"; no `try/except` that continues (`except
   Exception`, bare `except`, `contextlib.suppress`); no widening a type to silence an error; no skipped, ignored, xfailed or
   loosened tests.
6. **A fix that needs a design change says so:** propose it instead of patching around the design.
7. **One decision, one place.** A table, constant, name or rule that exists twice is a defect. For tests: look for existing
   coverage first; add a row to the existing table; setup two files need is a fixture in the nearest `conftest.py`.
8. **KISS outranks these rules.** If applying one makes the code longer, more indirect or harder to read than the problem, don't:
   note the finding and move on.
9. Where a skill (`systematic-debugging`'s defence-in-depth, "add validation at every layer") disagrees with this section, this
   section wins.

## Lint, types and gates

Two speeds, both through `uv run poe`:

| Task | Runs, cheapest first |
|---|---|
| `fast` | `rust-fmt`, `ruff format --check`, `rust-lint`, `ruff check`, `rust-test`, `rust-wasm`, `mypy`, `file-size`, `pytest -m 'not gate'` |
| `check` | `ruff format --check`, `rust-fmt`, `ruff check`, `rust-test-layout`, `rust-lint`, `file-size`, `lint-workflows`, `dead-code`, `rust-doc`, `deptry`, `rust-tools`, `rust-shear`, `rust-deny`, `rust-fonts`, `build-check`, `rust-wasm`, `mypy`, `stubtest`, `rust-test`, `wasm-test`, `test` (gate proofs included); stops at the first failure |

`rust-lint` is clippy with `-D warnings` over the workspace and all targets (pedantic, the determinism rules of `clippy.toml`, dead
code, missing docs, unreachable `pub`; `[workspace.lints]` in `Cargo.toml`). `rust-tools` checks that cargo-shear, cargo-deny and the `wasm-bindgen` command are
installed at the versions `Cargo.toml` pins (`poe setup` installs them) and that the command's version is that of the
`wasm-bindgen` crate in `Cargo.lock`. `rust-wasm` builds the core and the bindings crate for WebAssembly. `wasm-test` (needs Node)
builds the package in release with `RUSTFLAGS=-C target-feature=+simd128` (set by the task, for that build alone),
runs `wasm-bindgen --target nodejs` into the ignored `pkg/` and runs `tests/wasm` in Node (with a 2 GB JavaScript heap cap) against `tests/wasm/oracle.py`'s
native answers (exact SVG text, mesh buffers, JSON, Param descriptions, error names and messages; pixels and PNG within a
measured tolerance, `tests/wasm/compare.mjs`; the star from Python, Rust and Node); `fast` leaves it out. `rust-fonts` fails when `read-fonts` or `harfrust` is built with `std` (their maths would be the platform's; `tools/check_font_features.py`). `rust-deny` is offline (bans, licences, sources;
`deny.toml`); `uv run poe audit` adds the advisories and needs the network (CI runs it with every code change and weekly).
`lint-workflows` runs actionlint and zizmor (offline) on `.github` (`tools/lint_workflows.py`; no `.github` folder, nothing to
lint; an empty one fails). `dead-code` is vulture on
`src`. `build-check` builds the sdist and wheel into `dist/check` (`dist/` is git-ignored, as is `audit`'s `dist/audit`)
and checks them as PyPI would see them (`tools/check_dists.py`: twine, the sdist's Rust sources and bundled fonts, the licence files with each bundled font's `OFL.txt`, one
extension, check-wheel-contents); `tools/check_installed.py` smoke-checks an installed artifact (its file, version, import
location, the documented exceptions). `stubtest` checks `_native/_core.pyi` against the built extension. `uv run poe fix`
applies every automatic fix (`ruff check --fix`, `ruff format`, `cargo fmt`).

`file-size` (`tools/check_file_size.py`) fails a tracked `.py` or `.rs` file over 1000 lines (test files: 1500): split it by topic, never
raise the budget. An agent reads a file in chunks, and one under the budget stays navigable. A file already over is listed in
`[tool.file-size.exempt]` (path = current lines): it may shrink but never grow, and a stale entry (file within budget or gone) fails.

Each gate has a test proving it fails on a known-bad input (`tests/tools/`, marker `gate`: builds copies of the workspace, slow,
left out of `fast`, run by `check`). A new gate gets its proof, and a line in `tests/tools/test_gate_wiring.py`, which fails
when `check` does not run a proven gate.

`ruff` wants type hints and a docstring on every public function, class, method and module, and no `print` outside `tools/`.
`mypy` checks `src`, `tests`, `tools` and `examples`. Fix findings properly; for a false positive use a targeted `# noqa: CODE  <reason>` or
`# type: ignore[code]  # reason` (a bare or unused one fails). A subprocess call carries `# noqa: S603  <why its argv is safe>`.
Tests may not reach private names (`SLF001`, `PLC2701`).

Config homes: Python tooling in `pyproject.toml` (ruff, mypy, pytest, poe, commit-message, maturin, vulture, deptry,
check-wheel-contents), Rust tooling in `Cargo.toml` (workspace lints, pinned cargo tools). The Rust tools that cannot read `Cargo.toml` have one file each, at the root
only: `rust-toolchain.toml`, `rustfmt.toml`, `deny.toml`, `clippy.toml`. Do not add `ruff.toml`, `mypy.ini`, `setup.cfg`,
`pytest.ini`, `.coveragerc`, a second `clippy.toml`, `.cargo/config.toml` or similar (a test fails). Other config files:
`.pre-commit-config.yaml` (hook wiring only), `.editorconfig`, `.gitmessage`.

## CI

`.github/workflows/`: `ci.yml` (pushes to `main`, pull requests, merge queue; calls the rest), `lint.yml`, `test.yml`,
`security.yml` (also weekly), `pre-commit.yml`, `build.yml`; `.github/dependabot.yml` for weekly updates. Most steps are
local gate tasks (`uv run --locked poe <task>`); `lint.yml` calls ruff, deptry and mypy directly, as `check` does; Linux also runs `wasm-test` (Node);
macOS and Windows run `rust-test` and plain pytest without the gate proofs; `security.yml` adds `poe audit`, CodeQL and dependency
review (the last two have no local equivalent). `ci-ok` is the only required check: every job is in its `needs`, and a
skipped job passes. The `changes` job skips lint, test, security and build for a pull request that changes only
documentation; the set is in two places, `is_documentation` in `tools/pre_push.py` and the `changes` filter in `ci.yml`:
change both. `build` runs after the checks passed and produces the sdist and a wheel per platform
(`docs/design-decisions.md`, "Platforms"). A job or a poe task added to the gate is added to the other too. Third-party
actions are pinned by commit SHA; `lint-workflows` rejects the rest.

## Testing

Follow the `testing-best-practices` skill. A test coupled to implementation or otherwise poor (weak sole assertion, tautology,
golden file, sleep, mock of our own code) is worse than none: delete it. Reviewers treat such tests as blocking.

- **Rust first.** Rust tests live in `crates/core/tests/it/` and use the public API (`use bullterrier_vector::...`); no unit
  tests in `src/` unless no public route reaches the behaviour (never widen an item to `pub` to move a test out).
  **One integration binary:** `tests/it/main.rs` declares a `mod` per area, helpers are in `tests/it/common/`; a new test
  file is a module there, not a `tests/<name>.rs` (each is an executable that macOS scans on first launch, about 12 s). A
  file is a binary of its own only for a process-wide need (a `#[global_allocator]`; `tests/drawing_limits.rs`, whose
  million-shape tests peak at several GB and a process keeps what it allocated): first line `//! Own binary: <why>`,
  listed in `[tool.rust-tests.own-binaries]`; `poe rust-test-layout` fails otherwise. `poe rust-test` runs under a memory cap.
  The Python suite tests the bindings through the
  package's public API only (`import bullterrier_vector as btvec`).
- **Behaviour through the public contract; properties, not golden files.** `proptest` for broad input spaces. Expected values come
  from an independent oracle (hand computation, a standard, a brute-force reference parsed back from the output, e.g. `roxmltree`
  for SVG), never the code's own formula. No committed golden files: "the same on every target" is a differential
  test (native against WebAssembly, V3), which needs no reviewed baseline.
- Every checker/validator gets a known-good and a known-bad input. Seeds pin determinism. No sleeps, live network or wall-clock
  assertions.
- Python tests run in random order (pytest-randomly; rerun with `-p randomly --randomly-seed=N`, disable with `-p no:randomly`);
  none may depend on another. Every warning fails the test that raised it.
- **Mutation-check your tests, by hand:** break the code on purpose (a sign, a boundary, an ignored argument) and see a test
  fail; about ten mutants per batch, on the settled edge cases and the arithmetic (docs/roadmap.md, "How we work"); no
  mutation tool is set up. A surviving mutant on core behaviour becomes a test. Run Rust mutants with
  `PROPTEST_DISABLE_FAILURE_PERSISTENCE=1` and never commit a `*.proptest-regressions` file a mutant created; for Python use
  `PYTHONDONTWRITEBYTECODE=1` and clear `__pycache__` between mutants.
- **Hook-run tests must not touch the enclosing repository:** the autouse `no_enclosing_git` fixture (`tests/conftest.py`) removes
  every `GIT_*` variable, and tests that need git use the throwaway `git_repo` fixture (`tests/tools/conftest.py`). Never run
  `git` against the real repository from a test. The commands of a gate proof get an explicit environment (the `explicit_env`
  fixture: `PATH`, `HOME`, the cargo and rustup homes, the temporary folder, what Windows needs to start a program),
  never a copy of `os.environ`; the autouse `no_enclosing_hook` fixture removes `PRE_COMMIT_*` the same way as `GIT_*`.
- Refactors prove behaviour is preserved by comparing outputs before and after (byte-identical), in the report, not committed.

**Tests describe behaviour, not implementation:** they must survive any behaviour-preserving refactor.
- Never import underscore names, read production constants or private attributes, or call private functions. Worth testing means
  worth a public name; otherwise test it through the behaviour it serves.
- Never monkeypatch or mock our own code; never assert call counts or order on it. Substitute only true external boundaries.
- Assert observable outputs only: returned values, files written, errors raised. For errors, the kind plus a short substring naming
  the argument and value, never the whole sentence.
- Hard to test through the public interface is a design signal, in Rust too: change the interface, don't reach inside.

Writing tests:
- **Parametrise by default:** cases differing by literals are one table (`pytest.param(..., id=...)`; in Rust a table of tuples
  with the rule in each row's name) with good and bad rows together.
- **DAMP, not DRY:** each case and its expectation are visible; helpers build input or name an assertion, never hide what is checked.
- **No change-detectors:** test a rule with a known-bad input; never restate configuration.
- **Slow tests in groups:** a module whose module-scoped fixture spawns tools (ruff, mypy, poe, cargo, git) carries
  `pytestmark = pytest.mark.xdist_group("<module>")`.
- **Running:** one Rust test module: `cargo test -p bullterrier-vector --test it path::`; one Python file:
  `uv run pytest tests/test_path.py -n 0` (`-p no:randomly` for fixed order); all Python tests: `uv run poe test`.
- **Fast feedback (agents):** while iterating or doing red-green TDD run only the tests covering your change (a filter, one file),
  and `uv run poe fast` between steps. Run `uv run poe check` once when the work is finished, before you report. If it fails, fix
  and rerun only the failing tests, then `check` once more.

## Skills

If available: `superpowers:writing-plans`, `superpowers:test-driven-development`,
`superpowers:verification-before-completion` (process); `testing-best-practices` (tests); `software-design-philosophy` (deep
modules, no pass-throughs, no information leakage; interface docs state promises).
`superpowers:*` skills are process aids, not authority: they tend to verbose, over-engineered, over-defensive output. Where one
asks for more validation, layers, documents or steps than this file, follow this file. Tests follow this file (no goldens); fixes
follow "Fixing defects"; design follows the skill.

## Working style

- **Read narrowly:** the files your task names, `CLAUDE.md`, one reference module. If you need more, say why in the report.
- **Propose before refactoring** outside your brief's file scope; inside it, execute (plans are pre-approved).
- Report honestly: red/green evidence, exact commands run, anything skipped or unverified.
- Scratch space for plans, briefs, reports: `.superpowers/` (git-ignored; never commit it).

## Git conventions

Once per clone: `uv run poe setup` (hooks and commit template). Before every push or merge: `uv run poe check`. The commit hooks
check formatting (rustfmt, ruff) and the message; compiling and testing wait for the gate. The pre-push hook runs the whole gate
for a push to `main` or a tag, and for any push while local `main` has unpushed code or git cannot say. (Pre-commit does not
call the hook at all for a tag that points at a commit the remote already has: "nothing to push".) It skips the gate for a
branch other than `main` while `main` has no unpushed code, for a push whose changes (and `main`'s unpushed ones) are
documentation only, and for a push of exactly the tree a passing `uv run poe check` ran on (it must have started and
ended on a clean working tree; the record is in the git folder, so commit before you run `check`).
- **Trunk-based:** short-lived branch `<type>/<short-description>` (`feat/polygon-rotation`), merged into `main` and deleted when
  the work is complete; nothing unmerged is left behind. No worktrees unless the owner asks. Rebase on `main` before reporting.
  Never push.
- **Conventional Commits 1.0:** `type(scope)!: subject`. Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`,
  `build`, `ci`, `chore`, `revert`. Scope optional, lowercase: `core`, `python`, `docs`, `deps`. `!` or a `BREAKING CHANGE:`
  footer marks a breaking change.
- **Subject:** imperative present tense, lowercase start (acronyms and identifiers excepted), no final period.
- **Body** (after a blank line): why, not what. **One logical change per commit.**
- **Agents** end the message with `Co-Authored-By: <your model name> <noreply@anthropic.com>`.
- The `commit-msg` hook (`tools/check_commit_message.py`) rejects a malformed message and says which rule failed. Limits and
  scope pattern: `[tool.commit-message]` in `pyproject.toml`. Merge, revert and fixup commits git writes pass unchanged.
- **Never `--no-verify`, skip, silence or loosen a hook:** fix the cause. If a rule is wrong, change it in its one config home
  (`pyproject.toml` for the commit-message limits) in its own commit and say why.
