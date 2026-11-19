"""One spelling per concept in the Python interface: the stub names no argument by the second spelling of a pair."""

import ast
from pathlib import Path

import pytest

STUB = Path(__file__).parent.parent / "src" / "bullterrier_vector" / "_native" / "_core.pyi"

# The spelling the library does not use, and the one it does.
SECOND_SPELLINGS = {"centre": "center", "colour": "color", "grey": "gray"}


def argument_names(source: str) -> set[str]:
    """Every parameter and method name in `source`."""
    names: set[str] = set()
    for node in ast.walk(ast.parse(source)):
        if isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef):
            names.add(node.name)
            arguments = node.args
            every = [*arguments.posonlyargs, *arguments.args, *arguments.kwonlyargs, arguments.vararg, arguments.kwarg]
            names.update(argument.arg for argument in every if argument is not None)
    return names


def second_spellings(source: str) -> dict[str, str]:
    """The arguments of `source` spelled the way the library does not, each with the spelling it uses."""
    return {name: SECOND_SPELLINGS[name] for name in sorted(argument_names(source) & SECOND_SPELLINGS.keys())}


@pytest.mark.parametrize(
    ("source", "found"),
    [
        pytest.param("def circle(center, radius): ...", {}, id="the library's spelling passes"),
        pytest.param("def bands(*, centre=None): ...", {"centre": "center"}, id="centre is found"),
        pytest.param(
            "class A:\n    def f(self, *, grey=0, colour=1): ...", {"colour": "color", "grey": "gray"}, id="in a method"
        ),
    ],
)
def test_a_second_spelling_is_found_in_a_known_bad_stub_and_not_in_a_good_one(
    source: str, found: dict[str, str]
) -> None:
    assert second_spellings(source) == found


def test_the_stub_names_every_argument_one_way() -> None:
    assert second_spellings(STUB.read_text(encoding="utf-8")) == {}
