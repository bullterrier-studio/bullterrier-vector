"""The commands the proofs run get an explicit environment: a variable of the developer's shell or of a CI runner that
is not on the allow-list (`RUSTFLAGS` making clippy fail or cargo rebuild, `GIT_DIR` sending git elsewhere, a
`CARGO_BUILD_TARGET`) reaches none of them, and the variables a command needs to start do."""

import os

import pytest

OUTSIDE = {
    "RUSTFLAGS": "-D warnings",
    "RUSTDOCFLAGS": "-D warnings",
    "CARGO_BUILD_TARGET": "wasm32-unknown-unknown",
    "CARGO_TARGET_DIR": "/nonexistent/target",
    "GIT_DIR": "/nonexistent/.git",
    "GIT_WORK_TREE": "/nonexistent",
    "UV_PROJECT_ENVIRONMENT": "/nonexistent/venv",
    "PYTHONWARNINGS": "error",
    "PRE_COMMIT_REMOTE_NAME": "github",
}
# PATH is not faked: git_repo runs git in the environment it builds, and git is found through the real PATH
PATH_MARKER = "/nonexistent/tools"
INHERITED = {
    "HOME": "/home/me",
    "CARGO_HOME": "/cargo",
    "RUSTUP_HOME": "/rustup",
    "TMPDIR": "/var/scratch",
    "SYSTEMROOT": r"C:\Windows",
    "USERPROFILE": r"C:\Users\me",
    "TEMP": r"C:\Temp",
    "TMP": r"C:\Temp",
    "APPDATA": r"C:\Users\me\AppData\Roaming",
    "LOCALAPPDATA": r"C:\Users\me\AppData\Local",
    "COMSPEC": r"C:\Windows\System32\cmd.exe",
    "PATHEXT": ".COM;.EXE;.BAT",
}

ENVIRONMENTS = ["explicit_env", "uv_run_env", "copy_env", "cargo_env", "git_repo"]


def environment_of(request, fixture):
    """The environment fixture `fixture` builds, made now (after the test has set the outer environment)."""
    built = request.getfixturevalue(fixture)
    return built.env if fixture == "git_repo" else built() if fixture == "explicit_env" else built


@pytest.mark.parametrize("fixture", ENVIRONMENTS)
def test_a_variable_outside_the_allow_list_does_not_reach_the_proofs(request, monkeypatch, fixture):
    for name, value in OUTSIDE.items():
        monkeypatch.setenv(name, value)
    env = environment_of(request, fixture)
    assert {name: value for name, value in OUTSIDE.items() if env.get(name) == value} == {}


# not cargo_env: it is built once per module, by the first test that asks, and the outer environment is set per test
@pytest.mark.parametrize("fixture", [name for name in ENVIRONMENTS if name != "cargo_env"])
def test_the_variables_a_command_needs_to_start_reach_the_proofs(request, monkeypatch, fixture):
    for name, value in INHERITED.items():
        monkeypatch.setenv(name, value)
    monkeypatch.setenv("PATH", os.pathsep.join([os.environ["PATH"], PATH_MARKER]))  # the real PATH still starts git
    env = environment_of(request, fixture)
    assert {name: env.get(name) for name in INHERITED} == INHERITED
    assert env["PATH"].endswith(PATH_MARKER)  # the interpreter's folder comes first in some


def test_a_variable_that_is_not_set_is_not_made_up(explicit_env, monkeypatch):
    for name in [*INHERITED, "PATH"]:
        monkeypatch.delenv(name, raising=False)
    assert explicit_env() == {}
