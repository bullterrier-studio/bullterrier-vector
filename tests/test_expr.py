"""`Param` and `Expr` through the public package: what a Param keeps, the text an expression prints as, equality and
hashing, and the errors. The printed forms are written by hand from the rules in docs/api.md (operators print infix
with the fewest parentheses that keep the shape); the values an expression reads are tested through a drawing, in
tests/test_drawing.py."""

from typing import Any

import pytest

import bullterrier_vector as btvec

Param = btvec.Param

a = Param("a", default=0.5, min=0, max=1)
b = Param("b", default=0.5, min=0, max=1)
c = Param("c", default=0.5, min=0, max=1)


# --- a Param keeps what it was made with --------------------------------------------------------------------------


def test_a_param_keeps_what_it_was_made_with():
    param = Param("wght", default=400, min=100, max=900, unit="px", meaning="how heavy the letters are")
    assert (param.name, param.default, param.min, param.max) == ("wght", 400.0, 100.0, 900.0)
    assert (param.unit, param.meaning) == ("px", "how heavy the letters are")


def test_unit_and_meaning_are_optional_and_empty_by_default():
    param = Param("seed", default=0, min=0, max=99)
    assert (param.unit, param.meaning) == ("", "")


@pytest.mark.parametrize("unit", ["", "px", "fraction"])
def test_a_bounded_unit_keeps_its_default_and_range(unit):
    param = Param("x", default=2, min=-5, max=5, unit=unit)
    assert (param.unit, param.default, param.min, param.max) == (unit, 2.0, -5.0, 5.0)


@pytest.mark.parametrize(
    ("unit", "turn"),
    [pytest.param("deg", 360, id="degrees"), pytest.param("cycle", 1, id="cycles")],
)
def test_a_cyclic_default_at_the_end_of_the_turn_is_read_as_its_start(unit, turn):
    param = Param("angle", default=turn, min=0, max=turn, unit=unit)
    assert param.default == 0.0


def test_a_param_is_an_expr():
    assert isinstance(a, btvec.Expr)
    assert isinstance(a + 1, btvec.Expr)
    assert not isinstance(a + 1, Param)


def test_a_param_cannot_be_changed():
    with pytest.raises(AttributeError):
        a.name = "z"  # type: ignore[misc]


def test_an_expr_is_made_by_arithmetic_only():
    with pytest.raises(TypeError):
        btvec.Expr()


def test_a_param_is_made_with_keywords():
    with pytest.raises(TypeError):
        Param("x", 0, 0, 1)  # type: ignore[call-arg]


@pytest.mark.parametrize(
    "arguments",
    [
        pytest.param({"name": 3, "default": 0, "min": 0, "max": 1}, id="name not text"),
        pytest.param({"name": "x", "default": "0", "min": 0, "max": 1}, id="default not a number"),
        pytest.param({"name": "x", "default": 0, "min": None, "max": 1}, id="min not a number"),
        pytest.param({"name": "x", "default": 0, "min": 0, "max": 1, "unit": 3}, id="unit not text"),
        pytest.param({"name": "x", "default": 0, "min": 0, "max": 1, "meaning": 3}, id="meaning not text"),
    ],
)
def test_a_param_of_the_wrong_type_is_a_type_error(arguments):
    with pytest.raises(TypeError):
        Param(**arguments)


# --- the errors, and what each names ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("arguments", "names", "shows"),
    [
        pytest.param({"name": ""}, "name", '""', id="empty name"),
        pytest.param({"name": "2x"}, "name", "2x", id="name starts with a digit"),
        pytest.param({"name": "a-b"}, "name", "a-b", id="name with a dash"),
        pytest.param({"name": "into"}, "name", "into", id="a name an output option takes"),
        pytest.param({"name": "pen_mm"}, "name", "pen_mm", id="another output option"),
        pytest.param({"min": 1}, "min", "1", id="min equal to max"),
        pytest.param({"min": 2}, "min", "2", id="min above max"),
        pytest.param({"default": 1.5}, "default", "1.5", id="default above max"),
        pytest.param({"default": -0.5}, "default", "-0.5", id="default below min"),
        pytest.param({"default": float("nan")}, "default", "NaN", id="default NaN"),
        pytest.param({"max": float("inf")}, "max", "inf", id="max infinite"),
        pytest.param({"unit": "inches"}, "unit", "inches", id="a unit that does not exist"),
        pytest.param({"unit": "deg"}, "unit", "deg", id="a cyclic unit over a range that is not one turn"),
        pytest.param({"unit": "cycle", "max": 2}, "unit", "cycle", id="cycle over two turns"),
    ],
)
def test_a_param_outside_what_is_accepted_raises_a_parameter_error_naming_the_argument(arguments, names, shows):
    given = {"name": "x", "default": 0, "min": 0, "max": 1, **arguments}
    with pytest.raises(btvec.ParameterError, match=names) as raised:
        Param(**given)
    assert shows in str(raised.value)


def test_a_parameter_error_is_a_value_error_and_a_package_error():
    with pytest.raises(btvec.BullterrierVectorError) as raised:
        Param("", default=0, min=0, max=1)
    assert isinstance(raised.value, ValueError)


# --- an expression prints infix, with the fewest parentheses that keep its shape -----------------------------------


@pytest.mark.parametrize(
    ("expression", "printed"),
    [
        pytest.param(a + b, "a + b", id="add"),
        pytest.param(a - b, "a - b", id="subtract"),
        pytest.param(a * b, "a * b", id="multiply"),
        pytest.param(a / b, "a / b", id="divide"),
        pytest.param(-a, "-(a)", id="negate"),
        pytest.param(a + 1, "a + 1", id="a number on the right"),
        pytest.param(a + 0.25, "a + 0.25", id="a fraction on the right"),
        pytest.param(1 + a, "1 + a", id="reflected add keeps the number on the left"),
        pytest.param(1 - a, "1 - a", id="reflected subtract is not a - 1"),
        pytest.param(2 * a, "2 * a", id="reflected multiply"),
        pytest.param(1 / a, "1 / a", id="reflected divide is not a / 1"),
        pytest.param(1 + 0.08 * a, "1 + 0.08 * a", id="a multiplication binds tighter than an addition"),
        pytest.param((a + b) * c, "(a + b) * c", id="an addition inside a product is parenthesised"),
        pytest.param(a * (b + c), "a * (b + c)", id="on the right too"),
        pytest.param(a - b - c, "a - b - c", id="subtraction reads left to right"),
        pytest.param(a - (b - c), "a - (b - c)", id="a right operand of the same rank is parenthesised"),
        pytest.param(a / (b * c), "a / (b * c)", id="a division by a product"),
        pytest.param(a * b / c, "a * b / c", id="a product then a division"),
        pytest.param(-(a + b), "-(a + b)", id="a negated sum"),
        pytest.param(-a * b, "-(a) * b", id="a negation is its own operand"),
        pytest.param(a + (-b), "a + -(b)", id="a negation as an operand"),
        pytest.param(a + 3 * 4, "a + 12", id="plain numbers are worked out by Python before the Param meets them"),
        pytest.param(1.0 + a, "1 + a", id="a whole float prints without a point"),
    ],
)
def test_an_expression_prints_as_the_arithmetic_that_made_it(expression, printed):
    assert str(expression) == printed
    assert repr(expression) == f"Expr('{printed}')"


def test_a_param_prints_as_its_name_in_an_expression_and_as_its_fields_on_its_own():
    param = Param("pulse", default=0.5, min=0, max=1, unit="fraction", meaning="beat")
    assert str(param) == "pulse"
    assert repr(param) == "Param('pulse', default=0.5, min=0.0, max=1.0, unit='fraction', meaning='beat')"


def test_an_expression_made_of_the_same_param_twice_prints_it_twice():
    assert str(a * a + a) == "a * a + a"


# --- an operand that is not a number or an expression ---------------------------------------------------------


@pytest.mark.parametrize(
    "operate",
    [
        pytest.param(lambda x: x + "1", id="add text"),
        pytest.param(lambda x: "1" + x, id="reflected add text"),
        pytest.param(lambda x: x * None, id="multiply by None"),
        pytest.param(lambda x: x - [1], id="subtract a list"),
        pytest.param(lambda x: (1, 2) / x, id="reflected divide a tuple"),
        pytest.param(lambda x: x**2, id="power is not an operator of expressions"),
        pytest.param(lambda x: x // 2, id="floor division is not either"),
    ],
)
def test_arithmetic_with_something_else_is_a_type_error(operate):
    with pytest.raises(TypeError):
        operate(a)


# --- equality and hashing -----------------------------------------------------------------------------------------


def test_params_are_equal_when_all_six_fields_are():
    made = Param("x", default=0.5, min=0, max=1, unit="px", meaning="m")
    again = Param("x", default=0.5, min=0, max=1, unit="px", meaning="m")
    assert made == again
    assert hash(made) == hash(again)
    assert len({made, again}) == 1


@pytest.mark.parametrize(
    "changed",
    [
        pytest.param({"name": "y"}, id="name"),
        pytest.param({"default": 0.25}, id="default"),
        pytest.param({"min": -1}, id="min"),
        pytest.param({"max": 2}, id="max"),
        pytest.param({"unit": "px"}, id="unit"),
        pytest.param({"meaning": "other"}, id="meaning"),
    ],
)
def test_params_that_differ_in_one_field_are_not_equal(changed):
    base: dict[str, Any] = {"name": "x", "default": 0.5, "min": 0, "max": 1, "unit": "", "meaning": "m"}
    assert Param(**base) != Param(**{**base, **changed})


def test_expressions_are_equal_when_built_the_same_way():
    assert a + 1 == a + 1
    assert hash(a + 1) == hash(a + 1)
    assert a + 1 != 1 + a
    assert a + 1 != a + 2
    assert (a + b) + c != a + (b + c)


def test_a_param_is_not_equal_to_a_number_or_to_an_expression_that_reads_it():
    assert a != 0.5
    assert a != a + 0
    assert a != "a"
    assert (a == None) is False  # noqa: E711  the comparison itself is what is tested


def test_a_param_is_a_dictionary_key():
    assert {a: 1, b: 2}[Param("a", default=0.5, min=0, max=1)] == 1
