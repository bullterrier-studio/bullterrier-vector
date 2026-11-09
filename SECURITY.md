# Security policy

## Reporting a vulnerability

Please report security problems privately, not in a public issue: use GitHub's private vulnerability reporting at
<https://github.com/bullterrier-studio/bullterrier-vector/security/advisories/new> ("Report a vulnerability" on the repository's
Security tab). Say what is affected, how to reproduce it (a small file helps), and what an attacker could do with it.

You can expect an acknowledgement within a week. Once a fix is released, the advisory is published with credit to you,
unless you prefer otherwise.

## Supported versions

bullterrier-vector is before 1.0: only the latest release receives security fixes.

| Version | Supported |
| --- | --- |
| latest 0.x release | yes |
| older releases | no |

## What bullterrier-vector does to stay safe

- **Audited code and dependencies.** ruff's security rules (flake8-bandit) are part of every `poe check`, and
  `uv run poe audit` checks every release pinned in `uv.lock` with pip-audit and every crate in `Cargo.lock` with
  cargo-deny. No known advisory is open; one that has no fixed release yet would be listed here by its id (and, for a
  crate, in `deny.toml`).
- **The Rust core** is safe Rust (`#![forbid(unsafe_code)]`), and cargo-deny admits only crates from crates.io under
  permissive licences, with no PyO3 in the core (`deny.toml`, in every `poe check`).
- **Release files** are built in CI without a build cache. Each wheel is installed on its own platform from the built
  file alone and run, and the sdist is built and run, before anything is published.
- **No Python runtime dependency**, so there is nothing to audit there and nothing that downloads or executes at
  import.
