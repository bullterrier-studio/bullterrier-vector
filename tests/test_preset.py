"""The drawings that come with the library (`Drawing.preset`). The oracles are the shipped text files
(`crates/core/presets/`) read directly, and ElementTree parsing the SVG."""

from pathlib import Path
from xml.etree import ElementTree

import pytest

import bullterrier_vector as btvec

FILES = Path(__file__).resolve().parents[1] / "crates" / "core" / "presets"
NAMES = sorted(p.stem for p in FILES.glob("*.json"))


def paths(drawing: btvec.Drawing, **values: float) -> int:
    root = ElementTree.fromstring(drawing.to_svg(200, 200, **values))  # noqa: S314  our own output
    return sum(1 for element in root.iter() if element.tag.endswith("}path") or element.tag == "path")


def test_the_presets_are_listed_sorted_by_name() -> None:
    assert btvec.Drawing.preset_names() == NAMES


@pytest.mark.parametrize("name", NAMES)
def test_a_preset_loads_draws_and_is_the_shipped_text(name: str) -> None:
    drawing = btvec.Drawing.preset(name)
    assert paths(drawing) > 0
    assert drawing.to_json() == (FILES / f"{name}.json").read_text(encoding="utf-8")


@pytest.mark.parametrize("name", NAMES)
def test_every_param_has_a_meaning_and_draws_at_its_minimum_default_and_maximum(name: str) -> None:
    drawing = btvec.Drawing.preset(name)
    params = drawing.parameters()
    assert params, f"{name} has no Param to play with"
    for param in params:
        assert param.meaning, param.name
        for value in (param.min, param.default, param.max):
            assert paths(drawing, **{param.name: value}) > 0, (param.name, value)
    # A phase loops, so it can look alike at both ends; some Param of the preset must change the picture.
    assert any(
        drawing.to_svg(200, 200, **{p.name: p.min}) != drawing.to_svg(200, 200, **{p.name: p.max}) for p in params
    )


def test_an_unknown_name_is_refused_naming_every_preset() -> None:
    with pytest.raises(btvec.ParameterError, match='"nope"') as raised:
        btvec.Drawing.preset("nope")
    assert ", ".join(NAMES) in str(raised.value)


@pytest.mark.parametrize("name", NAMES)
def test_a_description_is_one_sentence_ending_in_a_full_stop(name: str) -> None:
    text = btvec.Drawing.preset_description(name)
    assert text.endswith(".")
    assert "\n" not in text
    assert len(text.strip()) > 1
    assert ". " not in text[:-1]


def test_an_unknown_name_has_no_description_and_the_same_refusal_as_preset() -> None:
    with pytest.raises(btvec.ParameterError) as asked:
        btvec.Drawing.preset("nope")
    with pytest.raises(btvec.ParameterError) as described:
        btvec.Drawing.preset_description("nope")
    assert str(described.value) == str(asked.value)
