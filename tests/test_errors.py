"""The exceptions of the public package: what each is, and what it is not."""

import pytest

import bullterrier_vector as btvec


@pytest.mark.parametrize(
    ("error", "builtin_base"),
    [
        pytest.param(btvec.ParameterError, ValueError, id="parameter-is-a-value-error"),
        pytest.param(btvec.GeometryError, None, id="geometry-has-no-builtin-base"),
    ],
)
def test_each_error_is_a_package_error_and_inherits_only_what_is_documented(
    error: type[Exception], builtin_base: type[Exception] | None
) -> None:
    assert issubclass(error, btvec.BullterrierVectorError)
    assert issubclass(error, ValueError) == (builtin_base is ValueError)


def test_a_geometry_error_is_not_a_parameter_error() -> None:
    assert not issubclass(btvec.GeometryError, btvec.ParameterError)
    assert not issubclass(btvec.ParameterError, btvec.GeometryError)
