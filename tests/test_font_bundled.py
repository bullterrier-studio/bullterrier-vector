"""The fonts that come with the library (`Font.bundled`). The oracle is fontTools reading the shipped WOFF2 files
(`crates/core/fonts/`), code that shares nothing with the library: the axes (`fvar`), the feature tags (`GSUB`, `GPOS`)
and the advance of `H` (`hmtx`) it reads are the expected values."""

from pathlib import Path

import pytest
from fontTools.ttLib import TTFont

import bullterrier_vector as btvec

FONTS = Path(__file__).resolve().parents[1] / "crates" / "core" / "fonts"
NAMES = ["bricolage_grotesque", "fraunces"]


def reference(name: str) -> TTFont:
    return TTFont(FONTS / name / f"{name}.woff2")


def test_the_bundled_fonts_are_listed_by_name() -> None:
    assert btvec.Font.bundled_names() == NAMES


@pytest.mark.parametrize("name", NAMES)
def test_a_bundled_font_has_the_name_axes_and_features_of_its_font_file(name: str) -> None:
    font = btvec.Font.bundled(name)
    file = reference(name)
    assert font.name == name
    wanted = {
        axis.axisTag: (axis.minValue, axis.defaultValue, axis.maxValue, file["name"].getDebugName(axis.axisNameID))
        for axis in file["fvar"].axes
    }
    found = {tag: (axis.min, axis.default, axis.max, axis.meaning) for tag, axis in font.axes.items()}
    assert found == wanted
    assert list(font.axes) == list(wanted)  # the order of the fvar table
    tags = {record.FeatureTag for table in ("GSUB", "GPOS") for record in file[table].table.FeatureList.FeatureRecord}
    assert font.features == sorted(tags)


@pytest.mark.parametrize("name", NAMES)
def test_a_bundled_font_shapes_a_string_with_the_advances_of_its_font_file(name: str) -> None:
    file = reference(name)
    em = file["head"].unitsPerEm
    advance = file["hmtx"][file.getBestCmap()[ord("H")]][0]
    shapes = btvec.text("HH", font=btvec.Font.bundled(name), size=em).shapes()
    origins = []
    for shape in shapes:
        origin = shape.attributes["origin"]
        assert isinstance(origin, btvec.Point)
        origins.append(origin.x)
    assert origins[1] - origins[0] == pytest.approx(advance)


@pytest.mark.parametrize("name", NAMES)
def test_a_saved_drawing_in_a_bundled_font_loads_again_with_no_bytes_supplied(name: str) -> None:
    drawing = btvec.text("Hi", font=btvec.Font.bundled(name))
    text = drawing.to_json()
    assert btvec.Drawing.from_json(text).to_json() == text


@pytest.mark.parametrize("wrong", ["Fraunces", "roboto_flex", ""])
def test_a_name_that_is_not_bundled_is_refused_naming_it_and_the_choices(wrong: str) -> None:
    with pytest.raises(btvec.ParameterError, match=rf'name.*"{wrong}".*bricolage_grotesque, fraunces'):
        btvec.Font.bundled(wrong)
