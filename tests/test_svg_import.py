"""`Drawing.from_svg` through the public package. Oracles: the SVG that `to_svg` writes (exported, then imported),
transforms worked out by hand, and hand-written documents with ids and nested groups."""

import pytest

import bullterrier_vector as btvec

Drawing = btvec.Drawing
OPEN = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100">'


def svg(body: str) -> str:
    return f"{OPEN}{body}</svg>"


def logo() -> str:
    """42 contours: 30 in `logo/mark`, 10 in an unnamed group inside `logo`, and two outside any group."""

    def rect(index: int) -> str:
        return f'<rect id="part-{index}" class="c{index % 3}" x="{index * 10}" y="0" width="5" height="5"/>'

    mark = "".join(rect(i) for i in range(30))
    behind = "".join(rect(i) for i in range(30, 40))
    return svg(f'<g id="logo"><g id="mark">{mark}</g><g>{behind}</g></g>{rect(40)}<rect x="900" width="5" height="5"/>')


def test_a_drawing_exported_to_svg_is_imported_with_its_paths_and_paints():
    drawing = Drawing(
        btvec.Path.rect(10, 20, 30, 40).fill("#336699"),
        btvec.Path.line((5, 6), (70, 80)).stroke("#ff0000", width=3),
        btvec.Path.circle((100, 50), 12).fill("#00ff0080").opacity(0.5),
        size=(200, 100),
    )
    imported = Drawing.from_svg(drawing.to_svg(200, 100))
    assert imported.size == (200, 100)
    for was, now in zip(drawing.shapes(), imported.shapes(), strict=True):
        assert now.path.to_svg_path() == was.path.to_svg_path()
        assert now.style.fill == was.style.fill
        assert now.style.stroke == was.style.stroke
        assert now.style.stroke_width == was.style.stroke_width
        assert now.style.opacity == pytest.approx(was.style.opacity)


def test_only_id_picks_each_part_of_a_logo_of_42_contours_and_group_gives_its_path():
    imported = Drawing.from_svg(logo())
    assert len(imported.shapes()) == 42
    for index in range(41):
        (part,) = imported.only(id=f"part-{index}").shapes()
        assert part.path.bounds.x == index * 10
        assert part.attributes["class"] == f"c{index % 3}"
        group = "logo/mark" if index < 30 else "logo" if index < 40 else None
        assert part.attributes.get("group") == group


def test_group_and_class_match_the_whole_text_and_an_id_of_one_character_is_text():
    imported = Drawing.from_svg(logo())
    assert len(imported.only(group="logo/mark").shapes()) == 30
    assert len(imported.only(group="logo").shapes()) == 10
    assert len(imported.only(group="mark").shapes()) == 0
    assert len(imported.only(**{"class": ["c0", "c1"]}).shapes()) == 28
    one = Drawing.from_svg(svg('<rect id="a" width="1" height="1"/>'))
    assert len(one.only(id="a").shapes()) == 1


def test_a_shape_of_an_element_without_attributes_has_its_index_and_no_text_attributes():
    shape = Drawing.from_svg(logo()).shapes()[41]
    assert shape.attributes["index"] == 41
    assert {"id", "class", "group"}.isdisjoint(shape.attributes)


def test_split_then_only_id_and_merge_by_id_keep_the_ids():
    imported = Drawing.from_svg(
        svg(
            '<g id="two"><path id="pair" d="M0 0 L5 0 L5 5 Z M20 0 L25 0 L25 5 Z"/>'
            '<rect id="solo" width="5" height="5"/></g>'
        )
    )
    parts = imported.split().only(id="pair").shapes()
    assert len(parts) == 2
    assert parts[0].attributes["group"] == "two"
    merged = imported.split().merge(by="id").shapes()
    assert [shape.attributes["id"] for shape in merged] == ["pair", "solo"]


def test_the_text_attributes_survive_json():
    imported = Drawing.from_svg(logo())
    again = Drawing.from_json(imported.to_json())
    assert [s.attributes for s in again.shapes()] == [s.attributes for s in imported.shapes()]
    assert again.only(id="part-7").shapes()[0].attributes["group"] == "logo/mark"


@pytest.mark.parametrize(
    ("transform", "start", "end"),
    [
        pytest.param("translate(5 7)", (6, 9), (8, 11), id="translate"),
        pytest.param("scale(2 3)", (2, 6), (6, 12), id="scale-pair"),
        pytest.param("scale(2)", (2, 4), (6, 8), id="scale"),
        pytest.param("rotate(90)", (-2, 1), (-4, 3), id="rotate"),
        pytest.param("rotate(90 1 2)", (1, 2), (-1, 4), id="rotate-about"),
        pytest.param("skewX(45)", (3, 2), (7, 4), id="skew-x"),
        pytest.param("skewY(45)", (1, 3), (3, 7), id="skew-y"),
        pytest.param("matrix(1 0 0 1 10 20)", (11, 22), (13, 24), id="matrix"),
        pytest.param("translate(10 0) scale(2)", (12, 4), (16, 8), id="list-first-is-outermost"),
    ],
)
def test_transforms_are_worked_into_the_points_of_the_line_from_1_2_to_3_4(transform, start, end):
    (line,) = Drawing.from_svg(svg(f'<path transform="{transform}" fill="none" d="M1 2 L3 4"/>')).shapes()
    got = (line.path.point_at(0), line.path.point_at(1))
    assert (got[0].x, got[0].y, got[1].x, got[1].y) == pytest.approx((*start, *end), abs=1e-9)


def test_the_view_box_corner_is_the_origin_and_its_size_the_drawings():
    imported = Drawing.from_svg(
        '<svg xmlns="http://www.w3.org/2000/svg" width="999" height="9" viewBox="10 20 100 50">'
        '<rect x="15" y="25" width="5" height="6"/></svg>'
    )
    assert imported.size == (100, 50)
    (shape,) = imported.shapes()
    bounds = shape.path.bounds
    assert (bounds.x, bounds.y, bounds.width, bounds.height) == (5, 5, 5, 6)


def test_an_element_with_no_paint_is_filled_black():
    (shape,) = Drawing.from_svg(svg('<rect width="4" height="4"/>')).shapes()
    assert shape.style.fill == "#000000ff"
    assert shape.style.stroke is None


def test_paint_is_handed_down_and_the_style_attribute_wins():
    (shape,) = Drawing.from_svg(
        svg(
            '<g fill="#f00" stroke="#00f" stroke-width="3">'
            '<rect width="4" height="4" style="fill:#102030;stroke-width:5"/></g>'
        )
    ).shapes()
    assert (shape.style.fill, shape.style.stroke, shape.style.stroke_width) == ("#102030ff", "#0000ffff", 5)


GOOD = ['<rect width="1" height="1" fill-rule="nonzero"/>', "<defs/>", "<title>t</title><desc>d</desc><!-- c -->"]


@pytest.mark.parametrize("body", GOOD)
def test_what_does_not_change_the_picture_is_accepted(body):
    Drawing.from_svg(svg(body))


@pytest.mark.parametrize(
    ("body", "words"),
    [
        pytest.param(
            '<path id="ring" fill-rule="evenodd" d="M0 0 L5 0 L5 5 Z"/>',
            ['<path id="ring">', 'fill-rule="evenodd"', "nonzero"],
            id="evenodd",
        ),
        pytest.param('<use href="#a"/>', ["<use>"], id="use"),
        pytest.param("<text>hi</text>", ["<text>"], id="text"),
        pytest.param("<style>.a{fill:red}</style>", ["<style>", "CSS"], id="stylesheet"),
        pytest.param('<linearGradient id="g"/>', ["linearGradient", "gradients"], id="gradient-element"),
        pytest.param('<rect width="1" height="1" fill="url(#g)"/>', ['fill="url(#g)"'], id="gradient-fill"),
        pytest.param('<rect width="1" height="1" clip-path="url(#c)"/>', ["clip-path"], id="clip-path"),
        pytest.param('<rect width="1" height="1" mask="url(#m)"/>', ["mask"], id="mask"),
        pytest.param('<g opacity="0.5"><rect width="1" height="1"/></g>', ["<g>", "opacity"], id="group-opacity"),
        pytest.param('<rect width="1" height="1" fill="red"/>', ['fill="red"', "#rrggbb"], id="colour-name"),
        pytest.param('<rect width="50%" height="1"/>', ['width="50%"'], id="percent"),
        pytest.param('<rect width="1" height="1" paint-order="stroke"/>', ["paint-order"], id="paint-order"),
        pytest.param(
            '<rect width="1" height="1" style="transform:rotate(45deg)"/>',
            ["transform", "style"],
            id="css-transform",
        ),
    ],
)
def test_what_is_not_read_raises_a_parameter_error_naming_it(body, words):
    with pytest.raises(btvec.ParameterError) as raised:
        Drawing.from_svg(svg(body))
    for word in words:
        assert word in str(raised.value)


def test_a_text_that_is_not_an_svg_raises():
    with pytest.raises(btvec.ParameterError, match="not XML"):
        Drawing.from_svg("<svg")


def test_the_most_elements_are_read_and_one_more_raises():
    def with_elements(count):
        return svg('<rect width="1" height="1"/>' * (count - 1))

    assert len(Drawing.from_svg(with_elements(20_000)).shapes()) == 19_999
    with pytest.raises(btvec.ParameterError, match="MAX_SVG_ELEMENTS"):
        Drawing.from_svg(with_elements(20_001))


def test_the_deepest_nesting_is_read_and_one_deeper_raises():
    def nested(levels):
        return svg("<g>" * (levels - 2) + '<rect width="1" height="1"/>' + "</g>" * (levels - 2))

    assert len(Drawing.from_svg(nested(128)).shapes()) == 1
    with pytest.raises(btvec.ParameterError, match="MAX_SVG_DEPTH"):
        Drawing.from_svg(nested(129))


def test_a_stylesheet_instruction_before_the_root_raises():
    text = '<?xml-stylesheet type="text/css" href="a.css"?>' + svg("")
    with pytest.raises(btvec.ParameterError, match="xml-stylesheet"):
        Drawing.from_svg(text)


def test_a_text_with_a_lone_surrogate_raises_a_parameter_error():
    with pytest.raises(btvec.ParameterError, match="UTF-8"):
        Drawing.from_svg("\ud800")


def test_one_character_selects_by_char_and_by_id_alike():
    assert len(btvec.text("O", font=btvec.Font.bundled("fraunces"), size=50).only(char="O").shapes()) >= 1
    assert len(Drawing.from_svg(svg('<rect id="O" width="1" height="1"/>')).only(id="O").shapes()) == 1
