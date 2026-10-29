"""The pre-push hook: run the whole local gate (`uv run poe check`) when `main` or a tag is pushed, or when local
`main` has commits its remote-tracking branch lacks, and skip it for any other push.

Why: a feature branch is checked by CI when its pull request opens, so running the full gate on every push to it only
slows the work down. A direct push to `main` has no pull request in front of it, so the gate runs first, locally,
instead of leaving a red `main` for CI to find.

pre-commit reports one pushed ref to the hook, the first of a multi-ref push (`git push origin feat/x main`, `git push
--all`), in PRE_COMMIT_REMOTE_BRANCH (for example `refs/heads/main`). Which ref comes first must not decide: the hook
also asks git whether local `main` has commits `<remote>/main` lacks (`<remote>` is PRE_COMMIT_REMOTE_NAME, default
`origin`), and runs the gate if they hold code, however the push is spelt. A feature branch is therefore skipped only
while `main` has no unpushed code (level with `<remote>/main`, behind it, or ahead by documentation only); the
trade-off, accepted, is that while local `main` has unpushed code every push runs the gate. The gate also runs when the
reported ref is `main`, is missing, or is not a branch ref (a tag), and in every state git cannot read (no
`<remote>/main`, no local `main`, a failing command): skipping is only for what is clearly not main.

A push that changes only documentation (the top-level and `docs/*.md` pages, the pictures in `docs/images/`: png, jpg,
jpeg, gif, svg, webp) skips the gate too: no code is in it. For the reported ref the pushed range comes from
PRE_COMMIT_FROM_REF and PRE_COMMIT_TO_REF; when it cannot be read the gate runs. When the reported ref is not `main`,
the commits of `main` that `<remote>/main` lacks count as well, and any code in them runs the gate. Renames are listed
as a deletion and an addition (`--no-renames`), so moving code into `docs/` still counts as changing code.

Limit: besides the first ref and `main`, no ref is looked at. A tag or a branch later in a multi-ref push that holds
code `main` lacks goes unseen, and the gate may be skipped for it.
"""

import os
import shutil
import subprocess
import sys
from pathlib import PurePosixPath

PICTURE_SUFFIXES = frozenset({".png", ".jpg", ".jpeg", ".gif", ".svg", ".webp"})


def must_run(remote_ref: str, remote: str = "origin") -> bool:
    """Whether a push whose first ref is `remote_ref` (as pre-commit reports it) must pass the local gate first.

    True for `main`, for anything that is not a branch, and for any branch while local `main` has code (or something
    git cannot read) that `<remote>/main` lacks. Run in the repository, as the hook is."""
    if not remote_ref.startswith("refs/heads/") or remote_ref == "refs/heads/main":
        return True
    return unpushed_main_needs_the_gate(remote)


def unpushed_main_needs_the_gate(remote: str) -> bool:
    """Whether local `main` has commits `<remote>/main` lacks that are more than documentation, or git cannot say."""
    changed = unpushed_main(remote)
    return changed is None or (bool(changed) and not all(is_documentation(path) for path in changed))


def is_documentation(path: str) -> bool:
    """Whether a changed path (relative to the repository, `/`-separated) is a page or picture that holds no code."""
    parts = PurePosixPath(path).parts
    if len(parts) == 1:
        return parts[0].endswith(".md")
    if parts[0] != "docs":
        return False
    if parts[1] == "images":
        return PurePosixPath(path).suffix.lower() in PICTURE_SUFFIXES  # a script in docs/images is code
    return len(parts) == 2 and parts[1].endswith(".md")


def _git(*args: str) -> str | None:
    """git's output, or None when it fails (a ref this clone lacks, not a repository)."""
    result = subprocess.run(  # noqa: S603  fixed argv; refs and paths are arguments, never a shell
        ["git", *args],  # noqa: S607  git is on the PATH of any clone
        capture_output=True,
        encoding="utf-8",
        check=False,
    )
    return result.stdout if result.returncode == 0 else None


def pushed_files(from_ref: str, to_ref: str) -> list[str] | None:
    """The paths the push changes, or None when git cannot say (a new branch, a ref this clone lacks)."""
    if not from_ref or not to_ref:
        return None
    diff = _git("diff", "--name-only", "--no-renames", f"{from_ref}..{to_ref}")
    return None if diff is None else diff.splitlines()


def unpushed_main(remote: str) -> list[str] | None:
    """The paths local `main` changes beyond `<remote>/main`: empty when `main` is level with it (or behind), None
    when git cannot say (no `<remote>/main` or no `main` here) or `main` has commits that change no path (that is no
    proof they hold only documentation)."""
    tracking, local = f"refs/remotes/{remote}/main", "refs/heads/main"
    ahead = _git("rev-list", "--count", f"{tracking}..{local}")
    if ahead is None:
        return None
    if int(ahead) == 0:
        return []
    diff = _git("diff", "--name-only", "--no-renames", f"{tracking}...{local}")
    return (diff.splitlines() or None) if diff is not None else None


def main() -> int:
    """Run the gate when the push needs it; return the exit status git should see."""
    remote_ref = os.environ.get("PRE_COMMIT_REMOTE_BRANCH", "")
    remote = os.environ.get("PRE_COMMIT_REMOTE_NAME") or "origin"
    if not must_run(remote_ref, remote):
        print(
            f"pre-push: {remote_ref} is a branch other than main, and main has no unpushed code (it is level with "
            f"{remote}/main, behind it, or ahead of it by documentation only), so the full gate is left to CI, which "
            "checks the pull request (`uv run poe check` runs it now)"
        )
        return 0
    changed = pushed_files(os.environ.get("PRE_COMMIT_FROM_REF", ""), os.environ.get("PRE_COMMIT_TO_REF", ""))
    # for a pushed main, the range pre-commit reports is main's own; otherwise main's unpushed commits count as well
    main_needs_it = remote_ref != "refs/heads/main" and unpushed_main_needs_the_gate(remote)
    if changed and all(is_documentation(path) for path in changed) and not main_needs_it:
        print("pre-push: only documentation changed, so the full gate is skipped (`uv run poe check` runs it now)")
        return 0
    uv = shutil.which("uv")
    if uv is None:
        print("pre-push: uv is not on the PATH, so the gate cannot run; install uv (https://docs.astral.sh/uv/)")
        return 1
    return subprocess.run([uv, "run", "poe", "check"], check=False).returncode  # noqa: S603  fixed argv, no input


if __name__ == "__main__":
    sys.exit(main())
