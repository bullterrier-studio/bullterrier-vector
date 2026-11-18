"""`Field.smooth_maps` through the public package: maps in, maps out, each a `Field.map` that traces as any
other. Values are worked out by hand (a flat byte's weighted mean)."""

import math

import pytest

import bullterrier_vector as btvec

W, H = 12, 8


def flat(byte: int) -> btvec.Field:
    return btvec.Field.map(W, H, bytes([byte]) * (W * H))


def holed(x: int = 4) -> btvec.Field:
    """200 everywhere but a 3 by 2 hole of 10 at column `x`."""
    return btvec.Field.map(W, H, bytes(10 if x <= i % W < x + 3 and 3 <= i // W < 5 else 200 for i in range(W * H)))


def bytes_of(field: btvec.Field) -> bytes:
    return field.to_map(W, H)


def test_a_frame_is_the_gaussian_weighted_mean_of_its_neighbours() -> None:
    values = [10, 20, 40, 80, 160, 30, 60]
    got = btvec.Field.smooth_maps([flat(v) for v in values], sigma=1.0)
    assert len(got) == 7
    weights = [math.exp(-(d * d) / 2) for d in range(4)]
    for at, field in enumerate(got):
        taps = [(weights[abs(at - j)], v) for j, v in enumerate(values) if abs(at - j) <= 3]
        want = round(sum(w * v for w, v in taps) / sum(w for w, _ in taps))
        assert bytes_of(field) == bytes([want]) * (W * H), at


def test_a_static_sequence_traces_as_the_single_map() -> None:
    one = holed()
    drawing = lambda m: btvec.Drawing.trace_levels(m, [0.5], smooth=1, tolerance=0.5)  # noqa: E731
    want = drawing(one).to_svg(W, H)
    for field in btvec.Field.smooth_maps([one] * 5, sigma=1.5):
        assert drawing(field).to_svg(W, H) == want


def test_refusals_name_the_argument() -> None:
    with pytest.raises(btvec.ParameterError, match="sigma"):
        btvec.Field.smooth_maps([flat(1)], sigma=-1)
    with pytest.raises(btvec.ParameterError, match="maps"):
        btvec.Field.smooth_maps([flat(1), btvec.Field.map(2, 2, bytes(4))], sigma=1)
