"""The draft JSON Schema of the drawing format (docs/formats.md): it accepts the example drawings and rejects bad ones.

Each bad document is rejected for its own rule: the test names the schema keyword and the JSON Pointer of the value
it expects to fail, so a row whose rule is loosened fails. Rules that relate values across a document are the loader's
(docs/formats.md, "Checked by the loader, not the schema") and are not tested here.
"""

import json
from pathlib import Path
from typing import Any

import pytest
from jsonschema import Draft202012Validator
from jsonschema.exceptions import ValidationError

ROOT = Path(__file__).resolve().parents[1]
SCHEMA = json.loads((ROOT / "schemas" / "drawing-1.schema.json").read_text(encoding="utf-8"))
EXAMPLES = sorted((ROOT / "docs" / "examples" / "json").glob("*.json"))
SQUARE = "M0 0 L10 0 L10 10 L0 10 Z"
MASK = {"Field.map": {"width": 2, "height": 2, "data": "mask"}}


def failures(document: Any) -> set[tuple[str, str]]:
    """Every (keyword, JSON Pointer) the schema reports for `document`, including those inside `anyOf` branches."""
    found: set[tuple[str, str]] = set()
    pending: list[ValidationError] = list(Draft202012Validator(SCHEMA).iter_errors(document))
    while pending:
        error = pending.pop()
        found.add((str(error.validator), "".join(f"/{part}" for part in error.absolute_path)))
        pending.extend(error.context or [])
    return found


def square_with(*steps: Any) -> dict[str, Any]:
    """A document whose root is a square path followed by `steps`."""
    return {"version": 1, "root": [{"path": SQUARE}, *steps]}


def with_param(step: Any, **param: Any) -> dict[str, Any]:
    """A square followed by `step`, with a Param `x` (in 0..1 unless `param` says otherwise) declared."""
    description = {"name": "x", "default": 0, "min": 0, "max": 1, "unit": "", "meaning": "", **param}
    return {"version": 1, "params": [description], "root": [{"path": SQUARE}, step]}


def test_the_schema_is_a_valid_draft_2020_12_schema() -> None:
    Draft202012Validator.check_schema(SCHEMA)


def test_there_is_an_example_for_each_piece_and_five_small_drawings() -> None:
    assert len(EXAMPLES) >= 8


@pytest.mark.parametrize("path", EXAMPLES, ids=lambda path: path.name)
def test_every_example_drawing_is_valid(path: Path) -> None:
    assert failures(json.loads(path.read_text(encoding="utf-8"))) == set()


GOOD = [
    pytest.param(square_with(), id="a path alone"),
    pytest.param(square_with({"fill": {"color": "#FF4F9A80"}}), id="a colour in upper case with alpha"),
    pytest.param(with_param({"rotate": {"angle": {"neg": {"param": "x"}}}}), id="negation of a Param"),
    pytest.param(
        with_param({"opacity": {"alpha": {"mul": [{"by_index": {"first": 0, "last": 1}}, {"param": "x"}]}}}),
        id="a per-item number times a Param",
    ),
    pytest.param(with_param({"repeat": {"count": {"add": [2, {"param": "x"}]}}}), id="a whole number as expression"),
    pytest.param(square_with({"with_values": {}}), id="with_values fixing nothing"),
    pytest.param(
        {
            "version": 1,
            "drawings": {"cell": [{"path": SQUARE}]},
            "root": [
                {"stack": {"items": [[{"drawing": "cell"}], [{"drawing": "cell"}, {"translate": {"dx": 10, "dy": 0}}]]}}
            ],
        },
        id="a stack with a copy",
    ),
    pytest.param(
        {
            "version": 1,
            "inputs": {"mask": {"kind": "map", "sha256": "0" * 64}},
            "root": [{"trace": {"field": {"Field.map": {"width": 2, "height": 2, "data": "mask"}}}}],
        },
        id="a map input with its digest",
    ),
    pytest.param(
        {
            "version": 1,
            "params": [{"name": "x", "default": 0.5, "min": 0, "max": 1, "unit": "", "meaning": ""}],
            "root": [{"trace": {"field": MASK, "levels": [{"param": "x"}]}}],
        },
        id="a trace level read from a Param",
    ),
    pytest.param(
        square_with({"hatch": {"spacing": [2.4, 14], "tone": {"Field.ramp": {"start": [0, 10], "end": [0, 0]}}}}),
        id="a spacing pair with its tone",
    ),
]

BAD = [
    pytest.param(
        square_with({"repeat": {"count": 2**53}}), "maximum", "/root/1/repeat/count", id="a count beyond 2**53 - 1"
    ),
    pytest.param(
        square_with({"rotate": {"angle": {"random": {"low": -1, "high": 1, "seed": 2**60}}}}),
        "maximum",
        "/root/1/rotate/angle/random/seed",
        id="a seed beyond 2**53 - 1",
    ),
    pytest.param(
        {"version": 1, "root": [{"trace": {"field": MASK, "levels": [{"by_index": {"first": 0.2, "last": 0.8}}]}}]},
        "enum",
        "/root/0/trace/levels/0",
        id="a per-item number in an argument read once",
    ),
    pytest.param(
        square_with({"hatch": {"flow": {"Flow.noise": {"seed": {"random": {"low": 0, "high": 9}}}}}}),
        "enum",
        "/root/1/hatch/flow/Flow.noise/seed",
        id="a per-item seed for a flow",
    ),
    pytest.param(
        {
            "version": 1,
            "root": [{"text": {"content": "HI", "font": "a", "size": {"by_order": {"first": 9, "last": 90}}}}],
        },
        "enum",
        "/root/0/text/size",
        id="a per-item text size",
    ),
    pytest.param(
        {
            "version": 1,
            "params": [{"name": "into", "default": 0, "min": 0, "max": 1, "unit": "", "meaning": ""}],
            "root": [{"path": SQUARE}],
        },
        "not",
        "/params/0/name",
        id="a Param named like an output option",
    ),
    pytest.param(
        square_with({"with_values": {"pen_mm": 0.5}}), "not", "/root/1/with_values", id="fixing an output option"
    ),
    pytest.param(
        square_with({"hatch": {"spacing": [2.4, 14]}}), "required", "/root/1/hatch", id="a spacing pair without tone"
    ),
    pytest.param(
        square_with({"posterize": {"levels": 1000}}),
        "maximum",
        "/root/1/posterize/levels",
        id="posterize levels above 256",
    ),
    pytest.param(
        {"version": 1, "root": [{"stack": {"items": [], "size": [1e9, 1350]}}]},
        "maximum",
        "/root/0/stack/size/0",
        id="a design size beyond the bound",
    ),
    pytest.param(
        {"version": 1, "root": [{"trace": {"field": MASK, "levels": [i / 300 for i in range(1, 300)]}}]},
        "maxItems",
        "/root/0/trace/levels",
        id="too many trace levels",
    ),
    pytest.param(
        {"version": 1, "root": [{"text": {"content": "HI", "font": "a", "features": {"liga": {"param": "x"}}}}]},
        "type",
        "/root/0/text/features/liga",
        id="a feature as an expression",
    ),
    pytest.param({"root": [{"path": SQUARE}]}, "required", "", id="no version"),
    pytest.param({"version": 2, "root": [{"path": SQUARE}]}, "const", "/version", id="an unknown version"),
    pytest.param({"version": "1", "root": [{"path": SQUARE}]}, "const", "/version", id="the version as a string"),
    pytest.param(
        {"version": 1, "root": [{"path": SQUARE}], "note": "hello"}, "additionalProperties", "", id="an unknown key"
    ),
    pytest.param(square_with({"glow": {"radius": 4}}), "enum", "/root/1", id="an unknown call"),
    pytest.param(
        square_with({"fill": {"color": "#ff0000", "note": "red"}}),
        "additionalProperties",
        "/root/1/fill",
        id="an unknown argument",
    ),
    pytest.param(
        square_with({"fill": {"color": "#ff0000"}, "blend": {"mode": "multiply"}}),
        "maxProperties",
        "/root/1",
        id="two calls in one object",
    ),
    pytest.param({"version": 1, "root": []}, "minItems", "/root", id="an empty drawing"),
    pytest.param(
        {"version": 1, "root": [{"fill": {"color": "#ff0000"}}]}, "enum", "/root/0", id="a step where a source must be"
    ),
    pytest.param(square_with({"path": SQUARE}), "enum", "/root/1", id="a source where a step must be"),
    pytest.param(square_with({"stroke": {"width": 2}}), "required", "/root/1/stroke", id="a required argument missing"),
    pytest.param(square_with({"fill": {"color": "red"}}), "pattern", "/root/1/fill/color", id="a colour by name"),
    pytest.param(square_with({"fill": {"color": "#f00"}}), "pattern", "/root/1/fill/color", id="a three-digit colour"),
    pytest.param(
        square_with({"translate": {"dx": "10", "dy": 0}}), "type", "/root/1/translate/dx", id="a string number"
    ),
    pytest.param(
        square_with({"opacity": {"alpha": 1.5}}), "maximum", "/root/1/opacity/alpha", id="a literal alpha above 1"
    ),
    pytest.param(square_with({"scale": {"by": 0}}), "not", "/root/1/scale/by", id="a scale of zero"),
    pytest.param(square_with({"repeat": {"count": 2.5}}), "type", "/root/1/repeat/count", id="a fractional count"),
    pytest.param(square_with({"repeat": {"count": 0}}), "minimum", "/root/1/repeat/count", id="no copies"),
    pytest.param(
        square_with({"offset": {"distance": 4, "join": "rounded"}}), "enum", "/root/1/offset/join", id="a join"
    ),
    pytest.param(
        {"version": 1, "root": [{"stack": {"items": [], "size": [-1080, 1350]}}]},
        "exclusiveMinimum",
        "/root/0/stack/size/0",
        id="a negative design size",
    ),
    pytest.param(
        {"version": 1, "root": [{"stack": {"items": [], "size": [{"param": "x"}, 1350]}}]},
        "type",
        "/root/0/stack/size/0",
        id="a design size as an expression",
    ),
    pytest.param(
        with_param({"opacity": {"alpha": {"clamp": [{"param": "x"}, 0, 1]}}}),
        "enum",
        "/root/1/opacity/alpha",
        id="an unknown function",
    ),
    pytest.param(
        with_param({"translate": {"dx": {"div": [{"param": "x"}, 0]}, "dy": 0}}),
        "not",
        "/root/1/translate/dx/div/1",
        id="a division by a literal zero",
    ),
    pytest.param(
        square_with({"translate": {"dx": {"add": [1, 2]}, "dy": 0}}),
        "not",
        "/root/1/translate/dx/add",
        id="a constant expression",
    ),
    pytest.param(
        square_with({"translate": {"dx": {"neg": 3}, "dy": 0}}),
        "not",
        "/root/1/translate/dx/neg",
        id="a negated literal",
    ),
    pytest.param(
        with_param({"rotate": {"angle": {"mul": [{"param": "x"}]}}}),
        "minItems",
        "/root/1/rotate/angle/mul",
        id="an operator with one operand",
    ),
    pytest.param(
        square_with({"rotate": {"angle": {"random": {"low": -10, "seed": 7}}}}),
        "required",
        "/root/1/rotate/angle/random",
        id="a per-item number missing an argument",
    ),
    pytest.param(
        square_with({"rotate": {"angle": {"random": {"low": -10, "high": 10, "seed": 0.5}}}}),
        "type",
        "/root/1/rotate/angle/random/seed",
        id="a fractional seed",
    ),
    pytest.param(
        {
            "version": 1,
            "params": [{"name": "1x", "default": 0, "min": 0, "max": 1, "unit": "", "meaning": ""}],
            "root": [{"path": SQUARE}],
        },
        "pattern",
        "/params/0/name",
        id="a Param name starting with a digit",
    ),
    pytest.param(
        with_param({"rotate": {"angle": {"param": "x"}}}, unit="ms"), "enum", "/params/0/unit", id="a time unit"
    ),
    pytest.param(
        {"version": 1, "params": [{"name": "x", "default": 0, "min": 0, "max": 1}], "root": [{"path": SQUARE}]},
        "required",
        "/params/0",
        id="a Param description missing fields",
    ),
    pytest.param({"version": 1, "params": [], "root": [{"path": SQUARE}]}, "minItems", "/params", id="empty params"),
    pytest.param(
        {
            "version": 1,
            "params": {"x": {"default": 0, "min": 0, "max": 1, "unit": "", "meaning": ""}},
            "root": [{"path": SQUARE}],
        },
        "type",
        "/params",
        id="params keyed by name instead of in order",
    ),
    pytest.param(
        {"version": 1, "inputs": {"anybody": {"kind": "image"}}, "root": [{"path": SQUARE}]},
        "enum",
        "/inputs/anybody/kind",
        id="an unknown input kind",
    ),
    pytest.param(
        {"version": 1, "inputs": {"anybody": {"kind": "font", "sha256": "abc"}}, "root": [{"path": SQUARE}]},
        "pattern",
        "/inputs/anybody/sha256",
        id="a digest that is not SHA-256",
    ),
    pytest.param(
        {
            "version": 1,
            "inputs": {"anybody": {"kind": "font", "data": "AAEAAAAS"}},
            "root": [{"text": {"content": "HI", "font": "anybody"}}],
        },
        "additionalProperties",
        "/inputs/anybody",
        id="input bytes embedded in the document",
    ),
    pytest.param(
        {"version": 1, "root": [{"text": {"content": "HI", "font": "anybody", "axes": {"wgt": 700}}}]},
        "pattern",
        "/root/0/text/axes",
        id="an axis tag of three letters",
    ),
    pytest.param(
        {"version": 1, "root": [{"trace": {"field": {"Field.map": {"width": 0, "height": 2, "data": "mask"}}}}]},
        "minimum",
        "/root/0/trace/field/Field.map/width",
        id="a map without width",
    ),
    pytest.param(
        {"version": 1, "root": [{"drawing": "my-letters"}]},
        "pattern",
        "/root/0/drawing",
        id="a reference name with a dash",
    ),
    pytest.param(
        {"version": 1, "root": [{"path": "M0 0 X10 0"}]}, "pattern", "/root/0/path", id="unknown path command"
    ),
    pytest.param(
        square_with({"with_values": {"x": "high"}}), "type", "/root/1/with_values/x", id="a value that is not a number"
    ),
    pytest.param(
        square_with({"gradient_map": {"colors": ["#1b1446"]}}),
        "minItems",
        "/root/1/gradient_map/colors",
        id="a gradient of one colour",
    ),
]


@pytest.mark.parametrize("document", GOOD)
def test_a_valid_document_passes(document: Any) -> None:
    assert failures(document) == set()


@pytest.mark.parametrize(("document", "keyword", "pointer"), BAD)
def test_a_bad_document_fails_for_its_rule(document: Any, keyword: str, pointer: str) -> None:
    assert (keyword, pointer) in failures(document)
