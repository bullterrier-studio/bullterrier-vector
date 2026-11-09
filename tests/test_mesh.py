"""`Drawing.to_mesh` through the public package: the `Mesh` it returns, the typed copies its buffers are, refilling with
`into=`, the sizes it refuses and the Param values it takes. Expected values are worked out by hand."""

import math
from array import array
from typing import Any

import pytest

import bullterrier_vector as btvec

SQUARE = btvec.Drawing(btvec.Path.rect(10, 20, 30, 40).fill("#336699"))
# the same class, untyped: the wrong types below are what is tested
LOOSE: Any = SQUARE


def test_a_new_mesh_is_empty_and_generation_0():
    mesh = btvec.Mesh()
    assert (len(mesh.vertices), len(mesh.indices), len(mesh.ranges), mesh.generation) == (0, 0, 0, 0)


def test_to_mesh_returns_a_mesh_of_a_rectangles_four_vertices_and_two_triangles():
    mesh = SQUARE.to_mesh(100, 100)
    assert isinstance(mesh, btvec.Mesh)
    assert len(mesh.vertices) == 4 * 8
    assert len(mesh.indices) == 6
    # one range: first index 0, six indices, blend 0 (normal)
    assert list(mesh.ranges) == [0, 6, 0]
    assert mesh.generation == 1
    corners = {(x, y) for x, y in zip(mesh.vertices[0::8], mesh.vertices[1::8], strict=True)}
    assert corners == {(10.0, 20.0), (40.0, 20.0), (10.0, 60.0), (40.0, 60.0)}


@pytest.mark.parametrize(("word", "range_blend"), [("normal", 0), ("multiply", 1)])
def test_a_mesh_has_ranges_for_normal_and_multiply(word, range_blend):
    assert list(SQUARE.blend(word).to_mesh(100, 100).ranges) == [0, 6, range_blend]


@pytest.mark.parametrize("word", ["screen", "overlay", "color-dodge", "soft-light", "exclusion"])
def test_a_mesh_refuses_the_other_blends_naming_blend_and_the_mode(word):
    with pytest.raises(btvec.ParameterError, match="blend") as raised:
        SQUARE.blend(word).to_mesh(100, 100)
    assert word in str(raised.value)


def test_the_buffers_are_float32_and_uint32_that_the_buffer_protocol_shows_as_such():
    # numpy.asarray reads exactly this: a format and an item size
    mesh = SQUARE.to_mesh(100, 100)
    buffers: list[tuple[array[Any], str, int]] = [
        (mesh.vertices, "f", 4),
        (mesh.indices, "I", 4),
        (mesh.ranges, "I", 4),
    ]
    for buffer, format_, size in buffers:
        view = memoryview(buffer)
        assert (view.format, view.itemsize, view.ndim) == (format_, size, 1)
    assert isinstance(mesh.vertices, array)


def test_a_vertex_is_position_then_premultiplied_colour_then_item_and_order():
    drawing = btvec.Drawing(
        btvec.Path.rect(0, 0, 10, 10).fill("#33669980"),
        btvec.Path.rect(20, 0, 10, 10).fill("#ff0000"),
    )
    mesh = drawing.to_mesh(100, 100)
    vertices = [mesh.vertices[at : at + 8] for at in range(0, len(mesh.vertices), 8)]
    first, second = vertices[0], vertices[-1]
    alpha = 128 / 255
    assert list(first[2:6]) == pytest.approx([0.2 * alpha, 0.4 * alpha, 0.6 * alpha, alpha])
    assert (first[6], first[7]) == (0.0, 0.0)
    # the second of two shapes is item 1, at the top of the order from 0 to 1
    assert list(second[2:]) == [1.0, 0.0, 0.0, 1.0, 1.0, 1.0]


def test_each_read_of_a_buffer_is_a_copy_that_changing_does_not_change_the_mesh():
    mesh = SQUARE.to_mesh(100, 100)
    first = mesh.vertices
    assert first is not mesh.vertices
    first[0] = 12345.0
    assert mesh.vertices[0] == 10.0
    taken = mesh.indices
    SQUARE.translate(5, 0).to_mesh(100, 100, into=mesh)
    # the copy taken before the refill is as it was
    assert list(taken) == list(mesh.indices)
    old = mesh.vertices
    btvec.Drawing(btvec.Path.circle((50, 50), 20).fill("#000000")).to_mesh(100, 100, into=mesh)
    assert old[0] == 15.0


def test_into_refills_that_mesh_and_returns_it():
    mesh = btvec.Mesh()
    again = SQUARE.to_mesh(100, 100, into=mesh)
    assert again is mesh
    assert mesh.generation == 1
    assert list(mesh.ranges) == [0, 6, 0]
    circle = btvec.Drawing(btvec.Path.circle((50, 50), 20).fill("#000000"))
    assert circle.to_mesh(100, 100, into=mesh) is mesh
    assert len(mesh.indices) > 6
    assert mesh.generation == 2
    # a refill with the buffers it has leaves the number
    assert circle.to_mesh(100, 100, into=mesh).generation == 2


def test_generation_moves_exactly_when_the_values_change_the_contents():
    shift = btvec.Param("shift", default=0, min=0, max=50)
    drawing = btvec.Drawing(btvec.Path.rect(0, 0, 10, 10).fill("#ff0000")).translate(shift, 0)
    mesh = drawing.to_mesh(100, 100)
    assert mesh.generation == 1
    assert drawing.to_mesh(100, 100, into=mesh).generation == 1
    assert drawing.to_mesh(100, 100, into=mesh, shift=0).generation == 1
    assert drawing.to_mesh(100, 100, into=mesh, shift=5).generation == 2
    assert drawing.to_mesh(100, 100, into=mesh, shift=5).generation == 2
    assert sorted(set(mesh.vertices[0::8])) == [5.0, 15.0]


def test_a_sized_drawing_at_twice_the_size_has_exactly_twice_the_positions():
    drawing = btvec.Drawing(btvec.Path.circle((30, 25), 12).fill("#336699"), size=(100, 50))
    single, double = drawing.to_mesh(200, 100), drawing.to_mesh(400, 200)
    assert list(double.indices) == list(single.indices)
    assert len(single.vertices) > 8 * 20
    for axis in (0, 1):
        assert [2 * one for one in single.vertices[axis::8]] == list(double.vertices[axis::8])


def test_the_same_drawing_gives_the_same_buffers():
    one, two = SQUARE.to_mesh(100, 100), SQUARE.to_mesh(100, 100)
    assert (one.vertices, one.indices, one.ranges) == (two.vertices, two.indices, two.ranges)


@pytest.mark.parametrize(
    ("width", "height", "name", "shown"),
    [
        pytest.param(0, 10, "width", "0", id="zero width"),
        pytest.param(10, 0, "height", "0", id="zero height"),
        pytest.param(0.5, 10, "width", "0.5", id="half a pixel"),
        pytest.param(10, 1080.5, "height", "1080.5", id="a fraction"),
        pytest.param(math.nan, 10, "width", "NaN", id="not a number"),
        pytest.param(10, -4, "height", "-4", id="negative"),
        pytest.param(100_001, 10, "width", "100001", id="one past the largest side"),
    ],
)
def test_a_size_that_is_not_whole_pixels_from_1_to_100000_raises_naming_the_side(
    width: float, height: float, name: str, shown: str
):
    with pytest.raises(btvec.ParameterError, match=name) as raised:
        SQUARE.to_mesh(width, height)
    assert shown in str(raised.value)


def test_the_largest_size_is_a_mesh_and_has_no_pixel_count_limit():
    assert len(SQUARE.to_mesh(100_000, 100_000).indices) == 6


def test_a_draw_that_raises_leaves_the_mesh_as_it_was():
    mesh = SQUARE.to_mesh(100, 100)
    before = (mesh.vertices, mesh.indices, mesh.ranges, mesh.generation)
    with pytest.raises(btvec.ParameterError, match="width"):
        SQUARE.to_mesh(0, 100, into=mesh)
    assert (mesh.vertices, mesh.indices, mesh.ranges, mesh.generation) == before


def test_a_value_for_no_param_raises_and_names_it():
    with pytest.raises(btvec.ParameterError, match="speed"):
        SQUARE.to_mesh(10, 10, speed=1)


def test_into_must_be_a_mesh():
    with pytest.raises(TypeError, match="into"):
        LOOSE.to_mesh(10, 10, into="not a mesh")


def test_the_sizes_are_positional_only():
    with pytest.raises(TypeError):
        LOOSE.to_mesh(width=10, height=10)
