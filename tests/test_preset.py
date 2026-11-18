"""The drawings that come with the library (`Drawing.preset`). The oracles are the shipped text files
(`crates/core/presets/`) read directly, and the pixels of the PNG, read by the decoder in conftest."""

import re
import runpy
from pathlib import Path

import pytest

import bullterrier_vector as btvec

ROOT = Path(__file__).resolve().parents[1]
FILES = ROOT / "crates" / "core" / "presets"
PROGRAMS = ROOT / "examples" / "presets"
NAMES = sorted(p.stem for p in FILES.glob("*.json"))


MIN_INK = 0.01  # a drawing has a subject if at least this share of its pixels differs from the frame's corner
CYCLIC_UNITS = ("cycle", "deg")  # a cyclic Param's maximum is the minimum again: one turn, not a span


def ink(drawing: btvec.Drawing, decode_png, **values: float) -> float:
    """The share of pixels, at a quarter of the design frame, that differ from the pixel at its top left corner."""
    assert drawing.size is not None  # a preset has a frame
    width, height = (round(side / 4) for side in drawing.size)
    _, _, pixels = decode_png(drawing.with_values(**values).to_png(width, height))
    corner = pixels[:4]
    return sum(pixels[i : i + 4] != corner for i in range(0, len(pixels), 4)) / (width * height)


SENTENCE_BREAK = re.compile(r"[!?]|\.\s+[A-Z]")


def is_one_sentence(text: str) -> bool:
    return text.endswith(".") and "\n" not in text and len(text.strip()) > 1 and not SENTENCE_BREAK.search(text[:-1])


def test_the_presets_are_listed_sorted_by_name() -> None:
    assert btvec.Drawing.preset_names() == NAMES


@pytest.mark.parametrize("name", NAMES)
def test_a_preset_loads_draws_its_subject_and_is_the_shipped_text(name: str, decode_png) -> None:
    drawing = btvec.Drawing.preset(name)
    assert ink(drawing, decode_png) >= MIN_INK
    assert drawing.to_json() == (FILES / f"{name}.json").read_text(encoding="utf-8")


@pytest.mark.parametrize("name", NAMES)
def test_every_param_has_a_meaning_draws_its_subject_at_default_and_maximum_and_moves_the_picture(
    name: str, decode_png
) -> None:
    drawing = btvec.Drawing.preset(name)
    params = drawing.parameters()
    assert params, f"{name} has no Param to play with"
    for param in params:
        assert param.meaning, param.name
        # Not at the minimum: a reveal (`draw_on`'s progress) is bare there, by design.
        for value in (param.default, param.max):
            assert ink(drawing, decode_png, **{param.name: value}) >= MIN_INK, (param.name, value)
        # A cyclic Param ends where it starts, so it is compared with a quarter turn; any other, with its far end.
        far = param.min + (param.max - param.min) / 4 if param.unit in CYCLIC_UNITS else param.max
        assert drawing.to_svg(200, 200, **{param.name: param.min}) != drawing.to_svg(200, 200, **{param.name: far}), (
            f"{name}: {param.name} does not change the picture"
        )


def test_an_unknown_name_is_refused_naming_every_preset() -> None:
    with pytest.raises(btvec.ParameterError, match='"nope"') as raised:
        btvec.Drawing.preset("nope")
    assert ", ".join(NAMES) in str(raised.value)


@pytest.mark.parametrize("name", NAMES)
def test_a_description_is_one_sentence_ending_in_a_full_stop(name: str) -> None:
    assert is_one_sentence(btvec.Drawing.preset_description(name))


@pytest.mark.parametrize(
    ("text", "one_sentence"),
    [
        ("A ring.", True),
        ("Type set at approx. 200 px.", True),
        ("A ring, with a slider; a wide one.", True),
        ("A ring! Its width is a slider.", False),
        ("A ring? Yes, a wide one.", False),
        ("A ring. Its width is a slider.", False),
        ("A ring!", False),
        ("A ring", False),
        ("A ring.\n", False),
        ("A ring\nand a disc.", False),
        (" .", False),
    ],
)
def test_the_one_sentence_rule_tells_known_good_from_known_bad(text: str, one_sentence: bool) -> None:
    assert is_one_sentence(text) is one_sentence


def test_an_unknown_name_has_no_description_and_the_same_refusal_as_preset() -> None:
    with pytest.raises(btvec.ParameterError) as asked:
        btvec.Drawing.preset("nope")
    with pytest.raises(btvec.ParameterError) as described:
        btvec.Drawing.preset_description("nope")
    assert str(described.value) == str(asked.value)


def test_every_shipped_file_has_a_program_and_every_program_a_file() -> None:
    assert sorted(p.stem for p in PROGRAMS.glob("*.py")) == NAMES


@pytest.mark.parametrize("name", NAMES)
def test_a_shipped_file_is_the_json_of_its_program(name: str) -> None:
    drawing = runpy.run_path(str(PROGRAMS / f"{name}.py"), run_name="__main__")["drawing"]
    assert (FILES / f"{name}.json").read_text(encoding="utf-8") == drawing.to_json()


def test_at_least_twelve_presets_ship() -> None:
    assert len(btvec.Drawing.preset_names()) >= 12  # the promise of the gallery batch
