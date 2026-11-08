"""The pre-push hook: run the local gate when `main` or a tag is pushed, or when local `main` has commits its
remote-tracking branch lacks, and skip it for any other push. The gate has three tiers (`fast`, `check`, `full`;
CLAUDE.md); the hook needs `check` for `main` and `full` for a tag (or a ref that is not clearly a branch).

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

A gate that already passed on exactly the pushed tree is not repeated. `uv run poe check` and `full` begin with
`python tools/pre_push.py start TIER` and end with `python tools/pre_push.py pass TIER`: the second records the tree of
`HEAD` in the git folder (never committed), one record per tier, only if the working tree was clean and `HEAD`'s tree
was the same at both moments, so the record names contents that were checked in full. A push whose tip has that tree
skips the gate if a pass of the needed tier or a higher one is recorded (a `full` pass satisfies `check`, a `check` pass
never satisfies a tag); any other tree runs it. CI checks everything again. `start` says when the tree already passed.

The gate proofs (tests/tools, marker `gate`) are slow, so `check` runs them only when they are due: `python
tools/pre_push.py proofs` runs them when a path of the trigger list (`[tool.gate-proofs]` in pyproject.toml: everything
the proofs read, copy, edit or anchor on) changed since the commit recorded when they last passed (`full`, or a
`check` that ran them, on a clean tree), counting uncommitted and untracked files. With no record, a record git cannot
read, a shallow clone or no repository they are due: the cost of a wrong guess is a skipped proof, so any doubt
runs them.

Limit: besides the first ref and `main`, no ref is looked at. A tag or a branch later in a multi-ref push that holds
code `main` lacks goes unseen, and the gate may be skipped for it.
One gate runs per checkout: `start` also records the PID of the `poe` process running the gate, and refuses to begin
while a live `poe` process holds that record (two gates in one checkout share its build folder and double the memory
they take). A record left by a gate that died names a dead process, or one that is no longer `poe`, and is ignored.
Where `ps` is missing (Windows) nothing is recorded and nothing is refused.
"""

import os
import re
import shutil
import subprocess
import sys
import tomllib
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[1]
PICTURE_SUFFIXES = frozenset({".png", ".jpg", ".jpeg", ".gif", ".svg", ".webp"})
STARTED, RUNNING, PROOFS_PASSED = (
    "gate-started",
    "gate-running",
    "gate-proofs-passed",
)  # files in the git folder: the tree a gate began on, the PID of the `poe` running it, the commit the proofs passed on
# the tiers that stand for each needed tier: a pass of a higher tier covers the lower one
COVERED_BY = {"check": ("check", "full"), "full": ("full",)}
POE = re.compile(r"\bpoe(thepoet)?\b")


def _passed_record(tier: str) -> Path | None:
    """The git-folder file that holds the tree a gate of `tier` passed on."""
    return _record_path(f"gate-passed-{tier}")


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


def _record_path(name: str) -> Path | None:
    """Where the git folder keeps the record `name` (right in a worktree too), None outside a repository."""
    path = _git("rev-parse", "--git-path", name)
    return Path(path.strip()) if path else None


def _tree_of(rev: str) -> str | None:
    """The hash of the tree `rev` holds (equal for two commits with the same files), None when git cannot say."""
    tree = _git("rev-parse", f"{rev}^{{tree}}")
    return tree.strip() if tree else None


def _clean_tree() -> bool:
    """Whether the working tree has no change, staged, unstaged or untracked (ignored files do not count)."""
    return _git("status", "--porcelain") == ""


def _process(pid: int) -> tuple[int, str] | None:
    """(parent PID, command line) of process `pid`, None when it is not running."""
    row = subprocess.run(  # noqa: S603  fixed argv; the PID is a number
        ["ps", "-o", "ppid=,command=", "-p", str(pid)],  # noqa: S607  ps is a system tool
        capture_output=True,
        encoding="utf-8",
        check=False,
    ).stdout.split(None, 1)
    return (int(row[0]), row[1]) if len(row) == 2 else None


def _gate_owner() -> int | None:
    """The PID of the `poe` process running this step: the nearest ancestor whose command line names poe."""
    pid = os.getppid()
    while (found := _process(pid)) is not None:
        if POE.search(found[1]):
            return pid
        pid = found[0]
    return None


def record_gate_start(tier: str) -> None:
    """Note the tree a gate of `tier` begins on: that of `HEAD` if the working tree is clean, nothing if not or outside
    a repository, and the PID of the `poe` running the gate. Exit with a message when another gate is running here; say
    so when the tree already passed. The first step of `uv run poe check` and `full`."""
    path, running = _record_path(STARTED), _record_path(RUNNING)
    if running is not None and shutil.which("ps"):
        holder = int(running.read_text(encoding="utf-8")) if running.exists() else None
        found = _process(holder) if holder else None
        if found is not None and POE.search(found[1]):
            sys.exit(f"gate: another gate is already running in this checkout (pid {holder}); wait for it or stop it")
        owner = _gate_owner()
        if owner is not None:
            running.write_text(str(owner), encoding="utf-8")
    if path is not None:
        path.write_text((_tree_of("HEAD") if _clean_tree() else None) or "", encoding="utf-8")
    if _clean_tree() and gate_passed_on("HEAD", tier):
        print(f"gate: ALREADY PASSED on this tree ({tier} or higher): running it again re-verifies nothing; stop here")


def record_gate_pass(tier: str) -> None:
    """Remember that the gate of `tier` passed on the tree of `HEAD`, if it began on that very tree with a clean working
    tree and the working tree is clean now; a `full` pass also remembers the proofs (they ran in it). The last step of
    `uv run poe check` and `full`, reached only when every step passed."""
    started, passed = _record_path(STARTED), _passed_record(tier)
    if started is None or passed is None or not started.exists():
        print("gate: no record that a gate began here, so this pass is not remembered for the push")
        return
    began, now = started.read_text(encoding="utf-8"), _tree_of("HEAD")
    started.unlink()
    if (running := _record_path(RUNNING)) is not None:
        running.unlink(missing_ok=True)
    if began and began == now and _clean_tree():
        passed.write_text(began, encoding="utf-8")
        print(f"gate: {tier} passed on tree {began[:12]}; a push of exactly this tree will not repeat it")
        if tier == "full":
            record_proofs_pass()
    else:
        print("gate: the tree was not clean and unchanged from start to end, so this pass is not remembered")


def gate_passed_on(rev: str, tier: str) -> bool:
    """Whether a gate of `tier` or a higher one passed, as `record_gate_pass` remembers it, on the tree of `rev`."""
    tree = _tree_of(rev)
    for covering in COVERED_BY[tier]:
        passed = _passed_record(covering)
        if tree and passed is not None and passed.exists() and passed.read_text(encoding="utf-8") == tree:
            return True
    return False


def triggers_proofs(path: str, triggers: list[str]) -> bool:
    """Whether a changed path (relative to the repository, `/`-separated) is one the proofs read: a trigger is a file's
    path, or a folder's path ending in `/`."""
    return any(path == trigger or (trigger.endswith("/") and path.startswith(trigger)) for trigger in triggers)


def record_proofs_pass() -> None:
    """Remember the commit the proofs passed on, if the working tree is clean (else none stands for what they saw)."""
    record = _record_path(PROOFS_PASSED)
    head = _git("rev-parse", "HEAD")
    if record is not None and head and _clean_tree():
        record.write_text(head.strip(), encoding="utf-8")


def proofs_due(triggers: list[str]) -> str | None:
    """Why the gate proofs must run now, None when no path of `triggers` changed since they last passed. Any doubt (no
    record, a commit git lacks, a shallow clone, no repository) is a reason."""
    record = _record_path(PROOFS_PASSED)
    if record is None or not record.exists():
        return "no record that the proofs ever passed here"
    baseline = record.read_text(encoding="utf-8").strip()
    if _git("rev-parse", "--is-shallow-repository") != "false\n":
        return "the history is shallow or unreadable, so what changed since the last pass cannot be told"
    tracked = _git("diff", "--name-only", "--no-renames", baseline)  # the working tree against the commit
    untracked = _git("ls-files", "--others", "--exclude-standard")
    if tracked is None or untracked is None:
        return f"git cannot say what changed since {baseline[:12]}"
    touched = [path for path in [*tracked.splitlines(), *untracked.splitlines()] if triggers_proofs(path, triggers)]
    return f"changed since the last pass ({baseline[:12]}): {', '.join(touched[:5])}" if touched else None


def run_proofs_if_due(triggers: list[str]) -> int:
    """The step of `poe check` that runs `poe proofs` when they are due and remembers a pass; its exit status."""
    reason = proofs_due(triggers)
    if reason is None:
        print("proofs: skipped, nothing they read changed since they last passed (`uv run poe full` forces them)")
        return 0
    print(f"proofs: running, {reason}")
    status = subprocess.run([sys.executable, "-m", "poethepoet", "proofs"], check=False).returncode
    if status == 0:
        record_proofs_pass()
    return status


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
    tier = (
        "check" if remote_ref.startswith("refs/heads/") else "full"
    )  # a tag, or a ref not clearly a branch, needs full
    if gate_passed_on(os.environ.get("PRE_COMMIT_TO_REF", ""), tier):
        print(f"pre-push: the {tier} gate already passed on exactly the pushed tree, so it is not repeated")
        return 0
    uv = shutil.which("uv")
    if uv is None:
        print("pre-push: uv is not on the PATH, so the gate cannot run; install uv (https://docs.astral.sh/uv/)")
        return 1
    return subprocess.run([uv, "run", "poe", tier], check=False).returncode  # noqa: S603  fixed argv, no input


if __name__ == "__main__":
    match sys.argv[1:]:
        case ["start", ("check" | "full") as tier]:
            record_gate_start(tier)
        case ["pass", ("check" | "full") as tier]:
            record_gate_pass(tier)
        case ["proofs"]:
            sys.exit(
                run_proofs_if_due(
                    tomllib.loads((ROOT / "pyproject.toml").read_text(encoding="utf-8"))["tool"]["gate-proofs"][
                        "triggers"
                    ]
                )
            )
        case []:
            sys.exit(main())
        case other:
            sys.exit(f"usage: pre_push.py [start TIER | pass TIER | proofs] (TIER: check or full), got {other}")
