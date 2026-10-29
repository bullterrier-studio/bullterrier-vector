# CLAUDE.md: bullterrier-vector

Rust library with Python (and, later, WebAssembly) bindings: a **typography-first vector graphics scripting library**. You write
a picture in code (Bezier paths, type, booleans, offsets, deformers, repeaters, generative forms, tracing), and it is drawn as SVG
or pixels, identical from Rust, Python and the browser. `README.md` (status), `docs/design.md` (what and how),
`docs/design-decisions.md` (why), `docs/roadmap.md` (the batches, in order; effects in `docs/effects-roadmap.md`),
`docs/prototype-findings.md` (what the taste prototypes taught), `CONTRIBUTING.md` (setup and CI for people).

**Write docs tersely.** This file, the README, `docs/`, docstrings, reports and briefs: state the rule or fact, drop preamble,
repetition and justification a reader does not need to act. Reasons go in `docs/design-decisions.md`, once. Committed files are
public: no absolute paths, no references to folders outside the repository.

## Start here

```bash
uv sync && uv run poe setup                      # once per clone: environment (builds the Rust extension: needs rustup), git hooks, commit template, cargo tools
cargo test -p bullterrier-vector square          # the Rust tests near your change, while you work
uv run pytest tests/test_polygon.py -n 0         # the Python tests near your change
uv run poe fast                                  # the inner loop: seconds when warm
uv run poe check                                 # the whole gate, before you report or merge
```

Needs [rustup](https://rustup.rs): the first `cargo` command in the checkout fetches the toolchain `rust-toolchain.toml` pins and
the `wasm32-unknown-unknown` target. `uv run poe` lists every task.

**State:** the scaffold (workspace, gates, hooks, CI workflows) and one walking-skeleton function, `polygon_svg(sides, radius)`, from the Rust
core to Python; the core also builds for WebAssembly. The rest is planned (`docs/roadmap.md`); the skeleton is replaced in V2.
Maintainers may keep machine-specific notes in a git-ignored `.local/` folder and batch plans in the git-ignored
`.superpowers/plans/`: read them when present; never commit or cite them.

## What it is and is not

| Responsible for (planned: `docs/roadmap.md`) | Not responsible for |
|---|---|
| Bezier paths as values: build, measure, split, slice, hit-test, resample, simplify, fit curves; booleans, offsets, stroke outlines | Timelines, keyframes, easing over time, an `onFrame` loop: values over time are the caller's |
| Drawings written as code: chainable, typed, discoverable from the editor (JSON is a layer on top); `Path`, `Shape`, `Drawing`, `Param`, the same verbs on a `Path` (computed at once) and a `Drawing` (a step) | Writing a rasteriser, a shaper or a font parser: existing crates do that |
| Type as geometry: any TrueType or OpenType font from bytes, shaping, variable-font axes as parameters, every glyph an editable path | Producing font files; colour and bitmap fonts in v1 |
| A generative layer: generators, deformers, repeaters, booleans, offsets, morph, hatching, tracing | An editor application (the library gives it geometry, hit testing, text layout) |
| Effects, each added with a gallery piece (the effects track) | Photo effects and compositing over footage |
| Describing every parameter: name, unit, range, default, meaning | Stateful simulation (particles, springs) in v1: if it comes, it produces points that a node draws |
| The same drawing giving the same bytes on every target; SVG that keeps every effect, plotter-ready SVG, RGBA pixels | Reading other tools' file formats: inputs are plain data (width, height, bytes; lists of numbers) |
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
`bullterrier-vector-python` (the bindings crate; builds `bullterrier_vector._native._core`), `bt-vector` (command line; planned,
V18), `BullterrierVectorError` (base exception), `ParameterError` (a value outside what the library accepts; also a
`ValueError`), `GeometryError` (geometry that fails, such as a boolean on an outline that cannot be resolved), `btvec` (the
alias the docs import the library as). Full name for identity, `bt-` for what a person types. Names inside the package carry
no prefix.

Licence: MIT, in `LICENSE` (named by `license-files` in `pyproject.toml`; `build-check` fails a distribution without it).

## Architecture

Two crates in one Cargo workspace; the package version is the workspace's.

| Path | What |
|---|---|
| `crates/core` | crate `bullterrier-vector`: pure Rust, the source of truth. `src/lib.rs` (the public API), `error.rs`, `math.rs`, `polygon.rs`; `tests/` |
| `crates/python` | PyO3 bindings building the extension `bullterrier_vector._native._core`; generic, passes values to the core unchecked; no Rust tests |
| `src/bullterrier_vector/` | the Python package: `errors.py` (the bottom layer, imports nothing from the package), `_native/` (the extension and its stub `_core.pyi`), `__init__.py` (the public API) |
| `tests/` | the Python suite (`test_*.py`); `tests/tools/` tests the tooling and the gates |
| `tools/` | scripts the hooks and gates run (`check_commit_message.py`, `check_one_config_home.py`, `pre_push.py`, `rust_tools.py`, `lint_workflows.py`, `check_dists.py`, `check_installed.py`) |

Rust is the source of truth: parameters are declared once, in Rust; Python and (from V3) WebAssembly are thin layers over it.
The core does no IO of its own: callers pass bytes. `pub` only for what the bindings use, else `pub(crate)` (`unreachable_pub`
fails the gate). Python reaches the core only through the bindings crate (`deny.toml` bans `pyo3` everywhere else). The Rust
`Error` maps to the Python exceptions in the bindings; the core owns every range check, so every language gets the same error.

The design's planned modules (geometry, type, scene, parameters, nodes, output, json), each hiding one decision, are in the table in
`docs/design.md`, "Shape of the library"; only `math`, `error` and the skeleton exist yet. A crate that does one of these jobs is never exposed in the API.

## The rules

Gated (the gate fails): rules 3, 4 (the bans), 5, and 8 (no printing). Convention (reviewed, not gated): 1, 2, 4's one number
formatter, 6 and 7, and "Fixing defects".

1. **No hidden tunables.** Every value that shapes a result is a named, documented parameter in the field's own terms. Exempt:
   a named, justified constant (a mathematical or standard constant, or a tolerance below any visible effect), carrying
   `// constant: <reason>`.
2. **Determinism.** Same inputs and `seed` give the same bytes on every target. Randomness comes only from the seed. No global
   random state.
3. **Transcendental maths only through `crate::math`** (`libm`); `f64::sin`, `exp`, `powf` and the rest are banned by
   `clippy.toml`. `sqrt` and `mul_add` are allowed.
4. **No clock, no hash-ordered collections, no IO in the core**, as banned in `clippy.toml` (the ban list lives there,
   with the reason beside each entry): use `BTreeMap`/`BTreeSet` and take bytes from the caller. One formatter writes every
   number into SVG (now `number` in `polygon.rs`): no other `format!` of a coordinate.
5. **The core builds for `wasm32-unknown-unknown` in every gate run** (`rust-wasm`): a native-only API in it fails there.
6. **Errors by kind.** Callers catch `BullterrierVectorError` (`bullterrier_vector.errors`). Library code raises a subclass
   (`ParameterError`), never a bare `ValueError`, for anything a caller can get wrong; each also inherits the matching builtin.
   In Rust, `bullterrier_vector::Error` is `#[non_exhaustive]`: a new kind is a new variant.
7. **No backwards-compatibility shims, aliases or re-exports before 1.0** unless the owner asks: change the callers.
8. The library never prints and never configures logging.

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
| `fast` | `rust-fmt`, `ruff format --check`, `rust-lint`, `ruff check`, `rust-test`, `rust-wasm`, `mypy`, `pytest -m 'not gate'` |
| `check` | `ruff format --check`, `rust-fmt`, `ruff check`, `rust-lint`, `lint-workflows`, `dead-code`, `rust-doc`, `deptry`, `rust-tools`, `rust-shear`, `rust-deny`, `build-check`, `rust-wasm`, `mypy`, `stubtest`, `rust-test`, `test` (gate proofs included); stops at the first failure |

`rust-lint` is clippy with `-D warnings` over the workspace and all targets (pedantic, the determinism rules of `clippy.toml`, dead
code, missing docs, unreachable `pub`; `[workspace.lints]` in `Cargo.toml`). `rust-tools` checks that cargo-shear and cargo-deny are
installed at the versions `Cargo.toml` pins (`poe setup` installs them). `rust-deny` is offline (bans, licences, sources;
`deny.toml`); `uv run poe audit` adds the advisories and needs the network (CI runs it with every code change and weekly).
`lint-workflows` runs actionlint and zizmor (offline) on `.github` (`tools/lint_workflows.py`; no `.github` folder, nothing to
lint; an empty one fails). `dead-code` is vulture on
`src`. `build-check` builds the sdist and wheel into `dist/check` (`dist/` is git-ignored, as is `audit`'s `dist/audit`)
and checks them as PyPI would see them (`tools/check_dists.py`: twine, the sdist's Rust sources, the licence files, one
extension, check-wheel-contents); `tools/check_installed.py` smoke-checks an installed artifact (its file, version, import
location, the documented square). `stubtest` checks `_native/_core.pyi` against the built extension. `uv run poe fix`
applies every automatic fix (`ruff check --fix`, `ruff format`, `cargo fmt`).

Each gate has a test proving it fails on a known-bad input (`tests/tools/`, marker `gate`: builds copies of the workspace, slow,
left out of `fast`, run by `check`). A new gate gets its proof, and a line in `tests/tools/test_gate_wiring.py`, which fails
when `check` does not run a proven gate.

`ruff` wants type hints and a docstring on every public function, class, method and module, and no `print` outside `tools/`.
`mypy` checks `src`, `tests` and `tools`. Fix findings properly; for a false positive use a targeted `# noqa: CODE  <reason>` or
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
local gate tasks (`uv run --locked poe <task>`); `lint.yml` calls ruff, deptry and mypy directly, as `check` does; macOS and
Windows run `rust-test` and plain pytest without the gate proofs; `security.yml` adds `poe audit`, CodeQL and dependency
review (the last two have no local equivalent). `ci-ok` is the only required check: every job is in its `needs`, and a
skipped job passes. The `changes` job skips lint, test, security and build for a pull request that changes only
documentation; the set is in two places, `is_documentation` in `tools/pre_push.py` and the `changes` filter in `ci.yml`:
change both. `build` runs after the checks passed and produces the sdist and a wheel per platform
(`docs/design-decisions.md`, "Platforms"). A job or a poe task added to the gate is added to the other too. Third-party
actions are pinned by commit SHA; `lint-workflows` rejects the rest.

## Testing

Follow the `testing-best-practices` skill. A test coupled to implementation or otherwise poor (weak sole assertion, tautology,
golden file, sleep, mock of our own code) is worse than none: delete it. Reviewers treat such tests as blocking.

- **Rust first.** Rust tests live in `crates/core/tests/` and use the public API (`use bullterrier_vector::...`); no unit
  tests in `src/`. The Python suite tests the bindings through the
  package's public API only (`import bullterrier_vector as btvec`).
- **Behaviour through the public contract; properties, not golden files.** `proptest` for broad input spaces. Expected values come
  from an independent oracle (hand computation, a standard, a brute-force reference parsed back from the output, e.g. `roxmltree`
  for SVG), never the code's own formula. No committed golden files: "the same bytes on every target" is a differential
  test (native against WebAssembly, V3), which needs no reviewed baseline.
- Every checker/validator gets a known-good and a known-bad input. Seeds pin determinism. No sleeps, live network or wall-clock
  assertions.
- Python tests run in random order (pytest-randomly; rerun with `-p randomly --randomly-seed=N`, disable with `-p no:randomly`);
  none may depend on another. Every warning fails the test that raised it.
- **Mutation-check your tests, by hand:** break the code on purpose (a sign, a boundary, an ignored argument) and see a test
  fail; no mutation tool is set up. Surviving mutants on core behaviour are review findings. Run Rust mutants with
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
- **Running:** one Rust test file: `cargo test -p bullterrier-vector --test polygon`; one Python file:
  `uv run pytest tests/test_polygon.py -n 0` (`-p no:randomly` for fixed order); all Python tests: `uv run poe test`.
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
branch other than `main` while `main` has no unpushed code, and for a push whose changes (and `main`'s unpushed ones) are
documentation only.
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
