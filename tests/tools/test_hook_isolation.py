"""A test never takes its input from the pre-commit hook that runs it. The pre-push hook runs the gate, and pytest with
it, with `PRE_COMMIT_REMOTE_NAME` (the remote pushed to), `PRE_COMMIT_REMOTE_BRANCH`, `PRE_COMMIT_FROM_REF` and
`PRE_COMMIT_TO_REF` set, which `tools/pre_push.py` reads: without the autouse `no_enclosing_hook` fixture a push to a
remote not named `origin` would fail the gate's own tests of that hook.

The proof copies the pre-push tests next to a probe conftest and runs them in a pytest process of their own with those
variables set to a hook's values; a control run without the fixture shows they fail when nothing defends them. It
never touches this repository: the probe runs in tmp_path.
"""

import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
NO_FIXTURE = '"""The control: nothing defends the tests from an enclosing hook."""\n'
# the rows that read the hook's variables or the remote's name; the other rows set what they need or none
ROWS = "multi_ref or main_level or of_a_tag"


def run_pre_push_tests(tmp_path: Path, tests_conftest: str, explicit_env) -> subprocess.CompletedProcess[str]:
    """Run the pre-push tests under `tests_conftest`, in a process whose environment is a push to `github`'s main."""
    probe = tmp_path / "probe"
    (probe / "tools").mkdir(parents=True)
    (probe / "conftest.py").write_text(tests_conftest, encoding="utf-8")
    shutil.copy(ROOT / "tests" / "tools" / "conftest.py", probe / "tools" / "conftest.py")
    module = (ROOT / "tests" / "tools" / "test_pre_push.py").read_text(encoding="utf-8")
    where = "ROOT = Path(__file__).resolve().parents[2]"
    assert where in module
    (probe / "tools" / "test_pre_push.py").write_text(module.replace(where, f"ROOT = Path({str(ROOT)!r})"), "utf-8")
    env = explicit_env(
        PRE_COMMIT_REMOTE_NAME="github",
        PRE_COMMIT_REMOTE_BRANCH="refs/heads/main",
        PRE_COMMIT_FROM_REF="f" * 40,
        PRE_COMMIT_TO_REF="e" * 40,
    )
    pytest_args = ["-q", "-n", "0", "-p", "no:randomly", "-p", "no:cacheprovider", "--import-mode=importlib"]
    return subprocess.run(
        [sys.executable, "-m", "pytest", *pytest_args, "-k", ROWS, "--rootdir", str(probe), str(probe)],
        cwd=probe,
        env=env,
        capture_output=True,
        encoding="utf-8",
        check=False,
    )


def test_the_pre_push_tests_pass_inside_a_push_to_a_remote_not_named_origin(tmp_path, explicit_env):
    done = run_pre_push_tests(tmp_path, (ROOT / "tests" / "conftest.py").read_text(encoding="utf-8"), explicit_env)
    assert done.returncode == 0, done.stdout + done.stderr
    assert "passed" in done.stdout


def test_without_the_fixture_the_hooks_variables_do_fail_those_tests(tmp_path, explicit_env):
    """The control: this is what the fixture prevents, so the test above can fail."""
    done = run_pre_push_tests(tmp_path, NO_FIXTURE, explicit_env)
    assert done.returncode != 0, done.stdout + done.stderr
    assert "test_a_feature_branch_push_with_main_level" in done.stdout
