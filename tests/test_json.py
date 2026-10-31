"""`Drawing.to_json` and `Drawing.from_json` through the public package: the two example drawings round trip to equal
canonical text and equal shapes, each loader check of docs/formats.md ("Checked by the loader, not the schema") that a
registered call can reach is a `FormatError` with the pointer in its message, `to_json` refuses what `from_json` would
refuse with `ParameterError`, and hostile text is refused without crashing. The expected pointers are written by hand
from the document's layout, not read from the library."""

import json
import runpy
from pathlib import Path

import pytest

import bullterrier_vector as btvec

EXAMPLES = Path(__file__).resolve().parents[1] / "examples"
JSON_EXAMPLES = Path(__file__).resolve().parents[1] / "docs" / "examples" / "json"
TRIANGLE = "M0 0 L10 0 L10 10 Z"


def example(name: str, drawing: str) -> btvec.Drawing:
    """The drawing a program in examples/ builds, named `drawing` in it."""
    found = runpy.run_path(str(EXAMPLES / f"{name}.py"), run_name="__main__")
    built = found[drawing]
    assert isinstance(built, btvec.Drawing)
    return built


@pytest.mark.parametrize(
    ("program", "name"), [pytest.param("star", "drawing", id="star"), pytest.param("series_bars", "chart", id="bars")]
)
def test_an_example_drawing_round_trips_to_equal_canonical_text_and_equal_shapes(program: str, name: str) -> None:
    drawing = example(program, name)
    text = drawing.to_json()
    loaded = btvec.Drawing.from_json(text)
    assert loaded.to_json() == text
    assert loaded.shapes() == drawing.shapes()
    assert loaded.size == drawing.size


def test_a_drawing_with_params_round_trips_with_its_params_and_their_values() -> None:
    spin = btvec.Param("spin", default=30, min=0, max=360, unit="deg", meaning="how far the square has turned")
    drawing = btvec.Drawing(btvec.Path.rect(150, 150, 100, 100).fill("#FF4F9A").rotate(spin, about=(200, 200)))
    loaded = btvec.Drawing.from_json(drawing.to_json())
    assert [(p.name, p.default, p.unit, p.meaning) for p in loaded.parameters()] == [
        ("spin", 30, "deg", "how far the square has turned")
    ]
    assert loaded.shapes(spin=90) == drawing.shapes(spin=90)


def test_to_json_writes_the_canonical_text_of_formats_md() -> None:
    drawing = btvec.Drawing(btvec.Path.rect(0, 0, 10, 10).fill("#FF4F9A"), size=(400, 400))
    assert drawing.to_json() == (
        '{"root":[{"stack":{"items":[[{"path":"M0 0 L10 0 L10 10 L0 10 Z"},{"fill":{"color":"#ff4f9a"}}]],'
        '"size":[400,400]}}],"version":1}'
    )


def test_from_json_is_a_static_method_and_reads_any_spelling() -> None:
    text = ' { "root" : [ {"path": "M0 0 L10 0"}, {"fill": {"color": "#FF0000FF"}} ], "version": 1.0 } '
    assert (
        btvec.Drawing.from_json(text).to_json()
        == '{"root":[{"path":"M0 0 L10 0"},{"fill":{"color":"#ff0000"}}],"version":1}'
    )


def test_the_generated_schema_is_text_that_is_json() -> None:
    schema = json.loads(btvec.drawing_schema())
    assert schema["title"].startswith("bullterrier-vector drawing")
    assert "stroke" in schema["$defs"]["step"]["properties"]


# FormatError ---------------------------------------------------------------------------------------------------------


def document(*steps: str, params: str = "", drawings: str = "") -> str:
    """A document whose root is the triangle followed by `steps` (JSON text of each)."""
    members = ['"version":1']
    if params:
        members.append(f'"params":[{params}]')
    if drawings:
        members.append(f'"drawings":{{{drawings}}}')
    root = ",".join([f'{{"path":"{TRIANGLE}"}}', *steps])
    members.append(f'"root":[{root}]')
    return "{" + ",".join(members) + "}"


def param(name: str, default: float = 0, low: float = 0, high: float = 10, unit: str = "") -> str:
    return f'{{"name":"{name}","default":{default},"min":{low},"max":{high},"unit":"{unit}","meaning":""}}'


def reading(name: str) -> str:
    return f'{{"translate":{{"dx":{{"param":"{name}"}},"dy":0}}}}'


REFUSED = [
    pytest.param("{", "document", id="json-text: not JSON"),
    pytest.param('{"version":1,"root":[],"root":[]}', "/root", id="json-text: a duplicate key"),
    pytest.param(
        '{"version":1,"root":[{"path":"M0 0"}],"x":"\\ud800"}',
        "/x",
        id="json-text: a lone surrogate written as an escape",
    ),
    pytest.param('{"version":2,"root":[{"path":"M0 0"}]}', "/version", id="schema: an unknown version"),
    pytest.param(document('{"glow":{}}'), "/root/1/glow", id="schema: an unknown call"),
    pytest.param(document('{"fill":{"color":"red"}}'), "/root/1/fill/color", id="schema: a colour by name"),
    pytest.param(document('{"opacity":{"alpha":1.5}}'), "/root/1/opacity/alpha", id="schema: a literal out of range"),
    pytest.param(
        document(*[reading(f"p{n}") for n in range(1001)], params=",".join(param(f"p{n}") for n in range(1001))),
        "/params/1000",
        id="limits: the first Param past MAX_PARAMS",
    ),
    pytest.param(
        document(*['{"translate":{"dx":1,"dy":1}}'] * 100_000),
        "document",
        id="limits: calls past MAX_CALLS",
    ),
    pytest.param(
        '{"version":1,"root":[{"stack":{"items":[[{"drawing":"b"}]]}}]}',
        "/root/0/stack/items/0/0/drawing",
        id="drawing-declared",
    ),
    pytest.param(
        '{"version":1,"drawings":{"a":[{"drawing":"a"}]},"root":[{"stack":{"items":[[{"drawing":"a"}]]}}]}',
        "/drawings/a/0/drawing",
        id="drawing-acyclic",
    ),
    pytest.param(document(reading("x"), params=param("x", default=-1)), "/params/0/default", id="param-range"),
    pytest.param(document(reading("x"), params=param("x") + "," + param("x")), "/params/1/name", id="param-unique"),
    pytest.param(
        document(reading("x"), reading("y"), params=param("x")), "/root/2/translate/dx/param", id="param-declared"
    ),
    pytest.param(
        document(reading("b"), reading("a"), params=param("a") + "," + param("b")), "/params", id="param-order"
    ),
    pytest.param(document(reading("x"), params=param("x") + "," + param("y")), "/params/1", id="param-used"),
    pytest.param(
        f'{{"version":1,"inputs":{{"anybody":{{"kind":"font"}}}},"root":[{{"path":"{TRIANGLE}"}}]}}',
        "/inputs/anybody",
        id="input-used",
    ),
    pytest.param(
        f'{{"version":1,"drawings":{{"a":[{{"path":"{TRIANGLE}"}}],"b":[{{"path":"{TRIANGLE}"}}]}},'
        '"root":[{"stack":{"items":[[{"drawing":"a"}]]}}]}',
        "/drawings/b",
        id="drawing-used",
    ),
    pytest.param('{"version":1,"root":[{"path":"M0 0 L10 10 L5"}]}', "/root/0/path", id="path-data"),
    pytest.param(
        document(reading("x"), '{"with_values":{"y":1}}', reading("y"), params=param("x") + "," + param("y")),
        "/root/2/with_values/y",
        id="with-values: a key the drawing before it does not read",
    ),
    pytest.param(
        document(reading("x"), '{"with_values":{"x":99}}', params=param("x")),
        "/root/2/with_values/x",
        id="with-values: a value outside the range",
    ),
    pytest.param(document('{"stroke":{"color":"none"}}'), "/root/1/stroke/color", id="call-checks: a stroke of none"),
    pytest.param(document('{"only":{}}'), "/root/1/only", id="call-checks: a selection of no attribute"),
    pytest.param(
        '{"version":1,"root":[{"stack":{"items":['
        '[{"stack":{"items":[],"size":[10,10]}}],[{"stack":{"items":[],"size":[20,20]}}]]}}]}',
        "/root/0/stack/items",
        id="call-checks: stack items of different sizes",
    ),
]


@pytest.mark.parametrize(("text", "pointer"), REFUSED)
def test_a_bad_document_is_a_format_error_whose_message_starts_with_the_pointer(text: str, pointer: str) -> None:
    with pytest.raises(btvec.FormatError) as refused:
        btvec.Drawing.from_json(text)
    assert str(refused.value).startswith(f"{pointer}: ")
    assert isinstance(refused.value, btvec.BullterrierVectorError)
    assert isinstance(refused.value, ValueError)
    assert not isinstance(refused.value, btvec.ParameterError)


def test_nesting_past_max_depth_is_a_format_error_at_the_value_that_passes_it() -> None:
    with pytest.raises(btvec.FormatError, match=r"^/root/0/0/[0-9/]+: .*MAX_DEPTH 128"):
        btvec.Drawing.from_json('{"version":1,"root":' + "[" * 200 + "]" * 200 + "}")


# Hostile text --------------------------------------------------------------------------------------------------------

HOSTILE = [
    pytest.param("[" * 1_000_000, id="a million open brackets"),
    pytest.param('{"a":' * 100_000, id="a hundred thousand open objects"),
    pytest.param('{"version":' + "1" * 5_000 + ',"root":[{"path":"M0 0"}]}', id="a number of five thousand digits"),
    pytest.param('{"version":1e400,"root":[{"path":"M0 0"}]}', id="a number beyond a double"),
    pytest.param(
        '{"version":1,"root":[{"path":"M0 0"},{"translate":{"dx":1e309,"dy":0}}]}', id="an exponent past a double"
    ),
    pytest.param(" " * (16 * 1024 * 1024 + 1), id="a document past MAX_DOCUMENT_BYTES"),
    pytest.param("\x00", id="a NUL"),
    pytest.param("", id="nothing"),
    pytest.param("﻿{}", id="a byte order mark"),
]


@pytest.mark.parametrize("text", HOSTILE)
def test_hostile_text_is_refused_with_a_format_error_and_does_not_crash(text: str) -> None:
    with pytest.raises(btvec.FormatError):
        btvec.Drawing.from_json(text)


# ParameterError ------------------------------------------------------------------------------------------------------


def test_a_literal_divisor_of_zero_has_no_spelling_so_to_json_raises_parameter_error() -> None:
    x = btvec.Param("x", default=1, min=0, max=10)
    drawing = btvec.Drawing(btvec.Path.rect(0, 0, 1, 1)).translate(x / 0, 0)
    with pytest.raises(btvec.ParameterError, match="divides by a literal 0"):
        drawing.to_json()


def test_to_json_refuses_a_drawing_past_max_calls_as_a_parameter_error_naming_the_limit() -> None:
    drawing = btvec.Drawing(btvec.Path.rect(0, 0, 1, 1))
    for _ in range(100_000):
        drawing = drawing.translate(1, 1)
    with pytest.raises(btvec.ParameterError, match="MAX_CALLS 100000"):
        drawing.to_json()


def test_a_drawing_that_reads_more_than_max_params_raises_at_the_call_that_passes_it() -> None:
    drawing = btvec.Drawing(btvec.Path.rect(0, 0, 1, 1))
    for number in range(1000):
        drawing = drawing.translate(btvec.Param(f"p{number}", default=0, min=0, max=1), 0)
    assert len(drawing.parameters()) == 1000
    with pytest.raises(btvec.ParameterError, match="MAX_PARAMS 1000"):
        drawing.translate(btvec.Param("one_too_many", default=0, min=0, max=1), 0)


def test_the_series_bars_document_is_written_back_as_python_json_would_write_it() -> None:
    # an oracle that shares no code with the writer: sorted keys, compact, the one shared drawing named d1
    document = (JSON_EXAMPLES / "series_bars.json").read_text(encoding="utf-8")
    expected = json.dumps(json.loads(document), sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    assert btvec.Drawing.from_json(document).to_json() == expected.replace('"bars"', '"d1"')


def test_a_string_with_a_lone_surrogate_is_a_format_error_not_a_unicode_error() -> None:
    lone = "\ud800"  # a str Python allows and UTF-8 cannot hold
    for text in (lone, '{"version":1,"root":[{"path":"' + lone + '"}]}'):
        with pytest.raises(btvec.FormatError, match=r"^document: .*UTF-8"):
            btvec.Drawing.from_json(text)
