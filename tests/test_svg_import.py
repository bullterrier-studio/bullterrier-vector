"""`Drawing.from_svg` through the Python package: only what the bindings can break. What the import reads and refuses,
its transforms, paint, limits and JSON are tested once, in `crates/core/tests/it/svg_import.rs`."""

import pytest

import bullterrier_vector as btvec

Drawing = btvec.Drawing
SVG = (
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100">'
    '<g id="logo"><g id="mark"><rect id="a" class="st0 st1" x="10" width="5" height="5"/></g></g>'
    '<rect id="O" class="st0" x="30" width="5" height="5"/><rect width="5" height="5"/></svg>'
)


def test_from_svg_returns_a_drawing_with_the_viewbox_as_its_size():
    imported = Drawing.from_svg(SVG)
    assert isinstance(imported, Drawing)
    assert imported.size == (200, 100)
    assert len(imported.shapes()) == 3


def test_the_text_attributes_are_visible_on_shape_attributes_as_strings():
    first, second, plain = Drawing.from_svg(SVG).shapes()
    found = (first.attributes["id"], first.attributes["class"], first.attributes["group"])
    assert found == ("a", "st0 st1", "logo/mark")
    assert second.attributes["id"] == "O"
    assert {"id", "class", "group"}.isdisjoint(plain.attributes)
    assert plain.attributes["index"] == 2


def test_only_takes_text_for_the_text_attributes_and_a_list_means_any_of():
    imported = Drawing.from_svg(SVG)
    assert [s.attributes["id"] for s in imported.only(id="a").shapes()] == ["a"]
    assert len(imported.only(group="logo/mark").shapes()) == 1
    assert len(imported.only(id=["a", "O"]).shapes()) == 2


def test_a_one_character_string_selects_by_id_and_by_char_alike():
    assert len(Drawing.from_svg(SVG).only(id="O").shapes()) == 1
    assert len(btvec.text("O", font=btvec.Font.bundled("fraunces"), size=50).only(char="O").shapes()) >= 1


def test_class_is_a_keyword_so_it_is_selected_through_a_dict_and_matches_the_whole_text():
    imported = Drawing.from_svg(SVG)
    assert [s.attributes["id"] for s in imported.only(**{"class": "st0 st1"}).shapes()] == ["a"]
    assert len(imported.only(**{"class": "st0"}).shapes()) == 1  # the shape whose whole class is "st0"
    assert len(imported.only(**{"class": ["st0", "st0 st1"]}).shapes()) == 2
    with pytest.raises(btvec.ParameterError, match="class_"):
        imported.only(class_="st0")


def test_something_not_read_reaches_python_as_a_parameter_error_naming_it():
    with pytest.raises(btvec.ParameterError, match='fill-rule="evenodd"'):
        Drawing.from_svg(SVG.replace('id="a"', 'id="a" fill-rule="evenodd"'))


def test_a_text_with_a_lone_surrogate_raises_a_parameter_error():
    with pytest.raises(btvec.ParameterError, match="UTF-8"):
        Drawing.from_svg("\ud800")
