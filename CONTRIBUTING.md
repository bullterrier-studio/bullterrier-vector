# Contributing

A Rust core (`crates/core`) with Python bindings (`crates/python`, `src/bullterrier_vector`). `CLAUDE.md` holds the rules
the code follows (also for people); `docs/` holds the design and the roadmap.

## Install from source

You need [rustup](https://rustup.rs) and [uv](https://docs.astral.sh/uv/). Python comes from uv; the Rust toolchain
(and the WebAssembly target) comes from rustup the first time `cargo` runs, as `rust-toolchain.toml` pins it. A C
compiler and linker must be there too (macOS: `xcode-select --install`; Linux: `build-essential` or your
distribution's equivalent; Windows: the Visual Studio Build Tools, which the rustup installer offers).

| | rustup | uv |
|---|---|---|
| macOS, Linux | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` | `curl -LsSf https://astral.sh/uv/install.sh \| sh` (or `brew install uv`) |
| Windows | `winget install Rustlang.Rustup` (or `rustup-init.exe` from rustup.rs) | `winget install astral-sh.uv` (or `powershell -c "irm https://astral.sh/uv/install.ps1 \| iex"`) |

Open a new shell afterwards so both are on the `PATH`. Then, in the clone:

```bash
uv sync && uv run poe setup
```

- `uv sync` creates `.venv` and builds the package, so it compiles the Rust core and PyO3 (the first time takes a
  while; later syncs rebuild only after Rust changes).
- `uv run poe setup` installs the git hooks (`pre-commit install`), sets the commit-message template, and installs the
  cargo tools the gate runs (cargo-deny, cargo-shear) at the versions `Cargo.toml` pins. They are compiled from source,
  which takes several minutes, unless [cargo-binstall](https://github.com/cargo-bins/cargo-binstall) is installed, which
  downloads them.
- `uv run poe` lists every task.

### A global `core.hooksPath`

`pre-commit install` refuses to run while git's `core.hooksPath` is set, in your global configuration or otherwise.
Hide the global configuration for the setup (`GIT_CONFIG_GLOBAL` needs git 2.32 or newer) and point this clone at its
own hooks, in the main checkout (bash or zsh):

```bash
GIT_CONFIG_GLOBAL=/dev/null uv run poe setup
git config --local core.hooksPath "$(git rev-parse --absolute-git-dir)/hooks"
```

In PowerShell, run `$env:GIT_CONFIG_GLOBAL = "NUL"` before `uv run poe setup`; it stays set for the whole session, so
remove it afterwards (`Remove-Item Env:GIT_CONFIG_GLOBAL`).

Your global hooks then no longer run in this clone. Once the local `core.hooksPath` is set, `poe setup` refuses again:
unset it first (`git config --local --unset core.hooksPath`). In a linked worktree, `--absolute-git-dir` names that
worktree's own folder, not the shared one where `pre-commit install` writes the hooks: run both commands in the main
checkout.

## Working

```bash
cargo test -p bullterrier-vector square          # the Rust tests near your change (a name filter)
uv run pytest tests/test_polygon.py -n 0         # the Python tests near your change, in one process
uv run poe fast                                  # the inner loop: format, lint, types, the quick tests
uv run poe check                                 # the whole gate: before you push or merge
```

`check` also builds the sdist and the wheel (`poe build-check`, into the git-ignored `dist/check`) and runs the tests
that prove each gate fails on bad input. The build runs in an isolated environment, so it needs the network or a warm
uv cache. `uv run poe audit` (known vulnerabilities in the locked dependencies) needs the network and is not part of
`check`; CI runs it.

The commit hooks check formatting and the commit message; the pre-push hook runs `check` for pushes that touch code
(`CLAUDE.md`, "Git conventions", says exactly when). Commits follow [Conventional Commits](https://www.conventionalcommits.org):
`type(scope): subject`, header at most 72 characters; the hook says which rule a message breaks. Never skip a hook.

## CI

`.github/workflows/ci.yml` runs on pushes to `main`, pull requests and merge-queue entries, and calls the others. Most
steps are the local gate's `uv run poe` tasks; the differences are noted:

- `lint.yml`: format, clippy, dead code, rustdoc, dependencies, WebAssembly build, types, stub, workflows: the commands
  of `check`, with ruff, deptry and mypy called directly, as `check` calls them.
- `test.yml`: the Rust tests and the Python suite on Linux, macOS and Windows. Linux runs `poe test`, gate proofs
  included; macOS and Windows run pytest without them.
- `security.yml` (also weekly): `poe audit`, which `check` leaves out; CodeQL and dependency review, which have no
  local equivalent.
- `pre-commit.yml`: the commit hooks over every file.
- `build.yml`: the sdist and a wheel per platform (maturin, in a manylinux container on Linux), checked by
  `tools/check_dists.py` as `build-check` does, and installed on their own. It runs only after everything else
  passed.

`ci-ok` is the only required check: it passes when every job it needs passed or was skipped. A pull request that changes
only documentation (top-level `*.md`, `docs/*.md`, pictures under `docs/images/`) skips lint, test, security and build;
pre-commit still runs. That set is written twice, in `tools/pre_push.py` (`is_documentation`) and in the `changes` job
of `ci.yml`: change both together.

Dependabot opens weekly update pull requests. Their titles (`ci: Bump ...`, `build(deps): Bump ...`) fail the commit-message
check and can be longer than 72 characters: edit the title when merging, so the merge commit follows `CLAUDE.md`.
