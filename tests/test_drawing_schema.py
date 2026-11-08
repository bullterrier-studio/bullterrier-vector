"""The draft JSON Schema of the drawing format (docs/formats.md): it accepts the example drawings and rejects bad ones.

Each bad document is rejected for its own rule: the test names the schema keyword and the JSON Pointer of the value
it expects to fail, so a row whose rule is loosened fails. Rules that relate values across a document are the loader's
(docs/formats.md, "Checked by the loader, not the schema") and are not tested here.

The schema the library generates from its registered calls (`drawing_schema()`) must agree with the draft: the same
verdict on every example and every test document whose calls are all registered. The draft stays the specification for
the calls not yet registered (docs/formats.md, "Extension").
"""

import json
from pathlib import Path
from typing import Any

import pytest
from jsonschema import Draft202012Validator
from jsonschema.exceptions import ValidationError

from bullterrier_vector import drawing_schema

ROOT = Path(__file__).resolve().parents[1]
SCHEMA = json.loads((ROOT / "schemas" / "drawing-1.schema.json").read_text(encoding="utf-8"))
EXAMPLES = sorted((ROOT / "docs" / "examples" / "json").glob("*.json"))
SQUARE = "M0 0 L10 0 L10 10 L0 10 Z"
MASK = {"Field.map": {"width": 2, "height": 2, "data": "mask"}}


def failures(document: Any, schema: dict[str, Any] = SCHEMA) -> set[tuple[str, str]]:
    """Every (keyword, JSON Pointer) the schema reports for `document`, including those inside `anyOf` branches."""
    found: set[tuple[str, str]] = set()
    pending: list[ValidationError] = list(Draft202012Validator(schema).iter_errors(document))
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


def text_with(arguments: dict[str, Any]) -> dict[str, Any]:
    """A document whose root is one `text` in the font input `a`, with `arguments` added."""
    return {
        "version": 1,
        "inputs": {"a": {"kind": "font"}},
        "root": [{"text": {"content": "HI", "font": "a", **arguments}}],
    }


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
    pytest.param(
        square_with({"stroke": {"color": "#000000", "join": "miter", "miter_limit": 2.5}}), id="a miter limit"
    ),
    pytest.param(square_with({"repeat": {"count": 3.0}}), id="a whole number written with a fraction part of zero"),
    pytest.param({"version": 1, "root": [{"stack": {"items": [], "size": [100, 100]}}]}, id="a blank frame"),
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
    pytest.param(
        square_with({"stroke": {"color": "#000000", "miter_limit": 0.5}}),
        "minimum",
        "/root/1/stroke/miter_limit",
        id="a miter limit below 1",
    ),
    pytest.param(
        square_with({"fill": {"color": "#ff0000\n"}}),
        "pattern",
        "/root/1/fill/color",
        id="a colour followed by a newline",
    ),
    pytest.param(
        square_with({"only": {"glyph": 1e300}}),
        "maximum",
        "/root/1/only/glyph",
        id="an attribute index beyond 2**53 - 1",
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
            "params": [
                {"name": f"p{i}", "default": 0, "min": 0, "max": 1, "unit": "", "meaning": ""} for i in range(1001)
            ],
            "root": [{"path": SQUARE}],
        },
        "maxItems",
        "/params",
        id="more Params than MAX_PARAMS",
    ),
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


# The generated schema ------------------------------------------------------------------------------------------------

GENERATED = json.loads(drawing_schema())

# The documents that the draft and the generated schema must give one verdict on, beyond GOOD, BAD and the examples:
# each states a rule the registered calls declare and the draft was loosened or tightened to match.
AGREED = [
    pytest.param(
        square_with({"translate": {"dx": {"by_index": {"first": 0, "last": 20}}, "dy": 0}}), True, id="by_index"
    ),
    pytest.param(
        square_with({"translate": {"dx": {"random": {"low": 0, "high": 1}}, "dy": 0}}),
        True,
        id="random with its seed left out",
    ),
    pytest.param(
        square_with({"translate": {"dx": {"random": {"low": 0, "high": 1, "seed": 1.5}}, "dy": 0}}),
        False,
        id="a seed with a fraction",
    ),
    pytest.param(
        square_with({"translate": {"dx": {"stagger": {"progress": 0.5, "spread": 0.999}}, "dy": 0}}),
        True,
        id="a spread below 1",
    ),
    pytest.param(
        square_with({"translate": {"dx": {"stagger": {"progress": 0.5, "spread": 1}}, "dy": 0}}),
        False,
        id="a spread of 1",
    ),
    pytest.param(
        square_with({"translate": {"dx": {"stagger": {"spread": 0.5}}, "dy": 0}}), False, id="stagger without progress"
    ),
    pytest.param(
        square_with({"translate": {"dx": {"by_order": {"first": 0, "last": 1, "step": 2}}, "dy": 0}}),
        False,
        id="a field by_order does not have",
    ),
    pytest.param(
        square_with({"translate": {"dx": {"by_index": {"first": 0}}, "dy": 0}}), False, id="by_index without last"
    ),
    pytest.param(
        text_with({"size": {"mul": [{"random": {"low": 1, "high": 2}}, 10]}}),
        False,
        id="a per-item text size inside a product",
    ),
    pytest.param(
        text_with({"axes": {"wght": {"by_index": {"first": 300, "last": 700}}}}), True, id="a per-item axis value"
    ),
    pytest.param(square_with({"only": {"index": []}}), True, id="an empty list of indexes selects nothing"),
    pytest.param(square_with({"without": {"glyph": []}}), True, id="an empty list of glyphs drops nothing"),
    pytest.param(square_with({"only": {"origin": [1, 2]}}), True, id="selecting by one origin"),
    pytest.param(square_with({"only": {"origin": [[1, 2], [3, 4]]}}), True, id="selecting by a list of origins"),
    pytest.param(square_with({"only": {"origin": []}}), True, id="an empty list of origins"),
    pytest.param(square_with({"only": {"origin": [1, 2, 3]}}), False, id="an origin of three numbers"),
    pytest.param(square_with({"only": {"glyph": 2**32 - 1}}), True, id="the largest glyph"),
    pytest.param(square_with({"only": {"glyph": 2**32}}), False, id="a glyph past u32"),
    pytest.param(square_with({"only": {"char": "O"}}), True, id="selecting by one character"),
    pytest.param(square_with({"only": {"char": "ab"}}), False, id="selecting by two characters"),
    pytest.param(square_with({"only": {"char": ["a", ""]}}), False, id="selecting by an empty character"),
    pytest.param(square_with({"only": {"colour": 1}}), False, id="selecting by no attribute"),
    pytest.param(
        {
            "version": 1,
            "root": [{"shape": {"path": SQUARE, "attributes": {"glyph": 3, "char": "O", "origin": [1, 2]}}}],
        },
        True,
        id="a shape with attributes",
    ),
    pytest.param(
        {"version": 1, "root": [{"shape": {"path": SQUARE, "attributes": {"glyph": 2**32}}}]},
        False,
        id="a shape with a glyph past u32",
    ),
    pytest.param(
        {"version": 1, "root": [{"shape": {"path": SQUARE, "attributes": {"char": "ab"}}}]},
        False,
        id="a shape with two characters",
    ),
    pytest.param(
        {"version": 1, "root": [{"shape": {"path": SQUARE, "attributes": {"index": 1}}}]},
        False,
        id="a shape carrying an index",
    ),
    pytest.param({"version": 1, "root": [{"shape": {"attributes": {}}}]}, False, id="a shape without a path"),
    pytest.param(square_with({"translate": {"dx": 1e100, "dy": -1e100}}), True, id="the largest coordinates"),
    pytest.param(square_with({"translate": {"dx": 1.1e100, "dy": 0}}), False, id="a coordinate past 1e100"),
    pytest.param(square_with({"rotate": {"angle": 1e300}}), True, id="any finite angle"),
    pytest.param(square_with({"rotate": {"angle": 45, "about": [1e101, 0]}}), False, id="a pivot past 1e100"),
    pytest.param(
        square_with({"along": {"path": "M0 0 L100 0", "start": 0.5, "near": [1, 2]}}),
        True,
        id="an along step with every argument",
    ),
    pytest.param(square_with({"along": {"path": "M0 0 L100 0"}}), True, id="an along step with its path only"),
    pytest.param(square_with({"along": {"path": "M0 0 L100 0", "start": 1e300}}), True, id="any finite along start"),
    pytest.param(square_with({"along": {"start": 0.5}}), False, id="an along step without a path"),
    pytest.param(
        square_with({"along": {"path": "M0 0 L100 0", "start": "x"}}), False, id="an along start that is text"
    ),
    pytest.param(
        square_with({"along": {"path": "M0 0 L100 0", "near": [1e101, 0]}}), False, id="an along near past 1e100"
    ),
    pytest.param(
        square_with({"along": {"path": "M0 0 L100 0", "near": [1, 2, 3]}}), False, id="an along near of three"
    ),
    pytest.param(square_with({"along": {"path": "M0 0 L100 0", "speed": 1}}), False, id="an along argument it lacks"),
    pytest.param(
        square_with({"slice": {"start": 0, "end": 1, "near": [1, 2]}}), True, id="a slice with every argument"
    ),
    pytest.param(square_with({"slice": {"start": 0.25, "end": 0.25}}), True, id="a slice to a point"),
    pytest.param(square_with({"slice": {"start": 0}}), False, id="a slice without an end"),
    pytest.param(square_with({"slice": {"start": 0, "end": 1.1}}), False, id="a slice end past 1"),
    pytest.param(square_with({"slice": {"start": -0.1, "end": 1}}), False, id="a slice start below 0"),
    pytest.param(
        square_with({"slice": {"start": 0, "end": 1, "near": [1e101, 0]}}), False, id="a slice near past 1e100"
    ),
    pytest.param(square_with({"scale": {"by": [2, -3], "about": [5, 5]}}), True, id="a scale pair about a point"),
    pytest.param(square_with({"scale": {"by": [2, 0]}}), False, id="a scale pair with a zero"),
    pytest.param(square_with({"fill": {"color": "none"}}), True, id="a fill of none"),
    pytest.param(square_with({"stroke": {"color": "none"}}), False, id="a stroke of none"),
    pytest.param(square_with({"stroke": {"color": "#000000", "width": 0}}), False, id="a stroke of no width"),
    pytest.param(square_with({"stroke": {"color": "#000000", "cap": "flat"}}), False, id="a cap that is not one"),
    pytest.param(square_with({"blend": {"mode": "multiply"}}), True, id="a blend mode"),
    pytest.param(square_with({"blend": {}}), False, id="a blend without its mode"),
    pytest.param(square_with({"opacity": {}}), False, id="an opacity without its alpha"),
    pytest.param(
        {"version": 1, "root": [{"stack": {"items": [], "size": [100000, 1]}}]}, True, id="the largest design frame"
    ),
    pytest.param(
        {"version": 1, "root": [{"stack": {"items": [], "size": [100001, 1]}}]}, False, id="a frame past the bound"
    ),
    pytest.param({"version": 1, "root": [{"stack": {"size": [1, 1]}}]}, False, id="a stack without items"),
    pytest.param(
        {
            "version": 1,
            "params": [
                {"name": f"p{i}", "default": 0, "min": 0, "max": 1, "unit": "", "meaning": ""} for i in range(1000)
            ],
            "root": [{"path": SQUARE}],
        },
        True,
        id="exactly MAX_PARAMS Params",
    ),
    pytest.param(with_param({"opacity": {"alpha": {"neg": {"param": "x"}}}}), True, id="an alpha read from a Param"),
    pytest.param(text_with({"axes": {"wght": 700, "wdth": -1e300}}), True, id="axis values"),
    pytest.param(text_with({"axes": {}}), True, id="no axes"),
    pytest.param(text_with({"axes": {"wght": "bold"}}), False, id="an axis value that is a string"),
    pytest.param(text_with({"axes": {"wg t": 1}}), True, id="a tag with a space"),
    pytest.param(text_with({"axes": {"wght ": 1}}), False, id="a tag of five characters"),
    pytest.param(text_with({"axes": {"w\u00e9ht": 1}}), False, id="a tag with a letter past ASCII"),
    pytest.param(text_with({"axes": [700]}), False, id="axes as an array"),
    pytest.param(text_with({"features": {"liga": 0, "ss01": 4294967295}}), True, id="the largest feature value"),
    pytest.param(text_with({"features": {"liga": 4294967296}}), False, id="a feature value past u32"),
    pytest.param(text_with({"features": {"liga": 1.5}}), False, id="a feature value that is not whole"),
    pytest.param(text_with({"features": {"liga": -1}}), False, id="a negative feature value"),
    pytest.param(text_with({"features": {"lig": 1}}), False, id="a feature tag of three letters"),
    pytest.param(
        {
            "version": 1,
            "params": [{"name": "x", "default": 400, "min": 300, "max": 700, "unit": "", "meaning": ""}],
            "inputs": {"a": {"kind": "font"}},
            "root": [{"text": {"content": "HI", "font": "a", "axes": {"wght": {"mul": [2, {"param": "x"}]}}}}],
        },
        True,
        id="an axis read from a Param",
    ),
]

# The calls the library registers, written out by hand: a call registered later is added here, and from then on the
# documents that use it are compared. (Reading the names from the generated schema would drop a document from the
# comparison whenever the generator dropped a call.)
REGISTERED = {
    "drawing",
    "path",
    "shape",
    "stack",
    "translate",
    "rotate",
    "scale",
    "along",
    "slice",
    "fill",
    "stroke",
    "opacity",
    "blend",
    "only",
    "without",
    "split",
    "merge",
    "with_values",
    "union",
    "subtract",
    "intersect",
    "exclude",
    "resolve",
    "offset",
    "outline",
    "text",
    "param",
    "add",
    "sub",
    "mul",
    "div",
    "neg",
    "by_index",
    "by_order",
    "random",
    "stagger",
}
# Documents the two schemas are expected to give different verdicts on, by id, each with the reason. Empty: the registry
# expresses every rule of the registered calls.
KNOWN_DIFFERENCES: dict[str, str] = {}


def call_names(schema: dict[str, Any]) -> set[str]:
    """Every call and expression word a schema names."""
    defs = schema["$defs"]
    names: set[str] = set()
    for kind in ("source", "step", "field", "flow", "expression"):
        if kind in defs:
            names |= set(defs[kind]["propertyNames"]["enum"])
    return names


def keys_of(value: Any) -> set[str]:
    """Every object key at any depth."""
    if isinstance(value, dict):
        return set(value) | set().union(*(keys_of(inner) for inner in value.values()))
    if isinstance(value, list):
        return set().union(*(keys_of(inner) for inner in value))
    return set()


UNREGISTERED = call_names(SCHEMA) - REGISTERED


def waits_for_a_later_batch(document: Any) -> bool:
    """Whether the document uses a call that the library does not register yet."""
    if keys_of(document) & UNREGISTERED:
        return True
    if isinstance(document, dict):
        return any(waits_for_a_later_batch(inner) for inner in document.values())
    if isinstance(document, list):
        return any(waits_for_a_later_batch(inner) for inner in document)
    return False


def comparable() -> list[tuple[str, Any]]:
    """The examples and test documents none of whose calls is unregistered."""
    documents = [(path.name, json.loads(path.read_text(encoding="utf-8"))) for path in EXAMPLES]
    documents += [(str(row.id), row.values[0]) for row in GOOD + AGREED]
    documents += [(str(row.id), row.values[0]) for row in BAD]
    return [(name, document) for name, document in documents if not waits_for_a_later_batch(document)]


COMPARABLE = comparable()


def disagreements(generated: dict[str, Any]) -> list[str]:
    """The ids of the comparable documents `generated` and the draft give different verdicts on."""
    draft, other = Draft202012Validator(SCHEMA), Draft202012Validator(generated)
    return [name for name, document in COMPARABLE if draft.is_valid(document) != other.is_valid(document)]


def test_the_registered_calls_are_the_calls_the_generated_schema_names() -> None:
    assert call_names(GENERATED) == REGISTERED


def test_the_generated_schema_is_a_valid_draft_2020_12_schema() -> None:
    Draft202012Validator.check_schema(GENERATED)


def test_the_comparison_covers_the_registered_documents() -> None:
    names = {name for name, _ in COMPARABLE}
    assert {"series_bars.json", "kinetic_type.json"} <= names
    assert len(COMPARABLE) >= 50
    assert "halftone.json" not in names


def test_the_generated_schema_agrees_with_the_draft_on_every_registered_document() -> None:
    assert sorted(disagreements(GENERATED)) == sorted(KNOWN_DIFFERENCES)


def test_no_registered_document_is_expected_to_differ() -> None:
    assert KNOWN_DIFFERENCES == {}


@pytest.mark.parametrize(("document", "valid"), AGREED)
def test_both_schemas_give_the_verdict_the_registered_declaration_states(document: Any, valid: bool) -> None:
    assert Draft202012Validator(SCHEMA).is_valid(document) is valid
    assert Draft202012Validator(GENERATED).is_valid(document) is valid


@pytest.mark.parametrize(
    ("document", "keyword", "pointer"), [row for row in BAD if not waits_for_a_later_batch(row.values[0])]
)
def test_a_bad_registered_document_fails_in_the_generated_schema_where_the_draft_fails(
    document: Any, keyword: str, pointer: str
) -> None:
    assert pointer in {where for _, where in failures(document, GENERATED)}


def tampered(path: list[str | int], value: Any) -> dict[str, Any]:
    """A copy of the generated schema with the member at `path` set to `value`."""
    schema = json.loads(drawing_schema())
    inner: Any = schema
    for key in path[:-1]:
        inner = inner[key]
    inner[path[-1]] = value
    return schema


STACK = ["$defs", "source", "properties", "stack", "properties"]
STEPS = ["$defs", "step", "properties"]
TAMPERED = [
    pytest.param(tampered([*STACK, "size", "prefixItems", 0, "maximum"], 1e9), id="a looser frame bound"),
    pytest.param(tampered([*STEPS, "stroke", "properties", "color"], {"type": "string"}), id="any stroke colour"),
    pytest.param(tampered([*STEPS, "opacity", "required"], []), id="an optional alpha"),
    pytest.param(
        tampered([*STEPS, "scale", "properties", "by", "anyOf", 0, "anyOf", 0, "not"], {"const": "never"}),
        id="a scale of zero",
    ),
    pytest.param(tampered(["$defs", "expression", "properties", "div"], {}), id="a division by a literal zero"),
    pytest.param(tampered([*STEPS, "only", "properties", "glyph"], {}), id="any glyph"),
]


@pytest.mark.parametrize("schema", TAMPERED)
def test_the_agreement_test_goes_red_when_the_generated_schema_is_wrong(schema: dict[str, Any]) -> None:
    assert disagreements(schema) != []
