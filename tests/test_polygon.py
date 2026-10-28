"""The walking skeleton from Python, through the public API only."""

import pytest

import bullterrier_vector as bv


def test_a_square_from_python_is_the_square_from_rust() -> None:
    assert bv.polygon_svg(4, 10.0) == (
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="-10 -10 20 20">'
        '<polygon points="0,-10 10,0 0,10 -10,0"/></svg>'
    )


@pytest.mark.parametrize(
    ("sides", "radius", "name", "value"),
    [
        (2, 10.0, "sides", "got 2"),
        (-1, 10.0, "sides", "-1"),
        (2**40, 10.0, "sides", "1099511627776"),
        (65_539, 10.0, "sides", "got 65539"),
        (5, float("nan"), "radius", "nan"),
        (5, 0.0, "radius", "got 0"),
    ],
)
def test_invalid_values_raise_parameter_error_naming_them(sides: int, radius: float, name: str, value: str) -> None:
    with pytest.raises(bv.ParameterError) as caught:
        bv.polygon_svg(sides, radius)
    message = str(caught.value).lower()
    assert name in message
    assert value in message


def test_parameter_error_is_a_value_error_and_the_package_error() -> None:
    with pytest.raises(ValueError, match="sides"):
        bv.polygon_svg(2, 1.0)
    with pytest.raises(bv.BullterrierVectorError, match="sides"):
        bv.polygon_svg(2, 1.0)


def test_a_float_for_sides_is_a_type_error() -> None:
    with pytest.raises(TypeError):
        bv.polygon_svg(4.0, 10.0)  # type: ignore[arg-type]
