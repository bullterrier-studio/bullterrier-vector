"""`btvec.frames` writing a PNG sequence, and the errors every kind of `out` shares. Every written frame is decoded with
the standard-library decoder of `conftest.py` and compared with `Drawing.to_rgba` of the same drawing drawn directly
with that frame's values; the expected values (how far the badge has moved at frame i) are worked out by hand."""

from pathlib import Path

import pytest

import bullterrier_vector as btvec

WIDTH, HEIGHT = 32, 16
# a red square, 8 by 8, that moves right by `x` pixels, so a frame drawn at the wrong time shows
X = btvec.Param("x", default=0, min=0, max=1000)
BADGE = btvec.Drawing(btvec.Path.rect(0, 0, 8, 8).fill("#e2412e").translate(X, 4), size=(WIDTH, HEIGHT))


def badge_at(t: float) -> btvec.Drawing:
    """The badge at `t` seconds: it moves 16 pixels a second."""
    return BADGE.with_values(x=16 * t)


class Clock:
    """A scene that remembers the times it was asked for."""

    def __init__(self) -> None:
        self.times: list[float] = []

    def __call__(self, t: float) -> btvec.Drawing:
        self.times.append(t)
        return badge_at(t)


def names(folder: Path) -> list[str]:
    """The file names in `folder`, sorted."""
    return sorted(path.name for path in folder.iterdir())


def test_each_frame_is_the_drawing_with_that_frames_values_exactly(tmp_path, decode_png):
    # 4 frames a second for 1.5 seconds: six frames, the badge 4 pixels further right each
    btvec.frames(badge_at, width=WIDTH, height=HEIGHT, fps=4, seconds=1.5, out=tmp_path / "badge-###.png")

    assert names(tmp_path) == [f"badge-{i:03d}.png" for i in range(6)]
    for i in range(6):
        width, height, pixels = decode_png((tmp_path / f"badge-{i:03d}.png").read_bytes())
        assert (width, height) == (WIDTH, HEIGHT)
        assert pixels == BADGE.to_rgba(WIDTH, HEIGHT, x=4 * i)


def test_the_first_frame_is_drawn_where_nothing_has_moved_and_the_last_frame_is_before_the_end(tmp_path, decode_png):
    btvec.frames(badge_at, width=WIDTH, height=HEIGHT, fps=4, seconds=1, out=tmp_path / "f-#.png")

    # t = 0, 0.25, 0.5, 0.75: the badge at x = 0 and x = 12; the end (t = 1, x = 16) is not drawn
    first = decode_png((tmp_path / "f-0.png").read_bytes())[2]
    last = decode_png((tmp_path / "f-3.png").read_bytes())[2]
    assert first == BADGE.to_rgba(WIDTH, HEIGHT, x=0)
    assert last == BADGE.to_rgba(WIDTH, HEIGHT, x=12)
    assert names(tmp_path) == ["f-0.png", "f-1.png", "f-2.png", "f-3.png"]


@pytest.mark.parametrize(
    ("fps", "seconds", "times"),
    [
        pytest.param(4, 1, [0.0, 0.25, 0.5, 0.75], id="quarter-seconds"),
        pytest.param(30, 0.1, [0.0, 1 / 30, 2 / 30], id="thirtieths-of-a-second"),
        pytest.param(0.5, 4, [0.0, 2.0], id="a-frame-every-two-seconds"),
    ],
)
def test_the_scene_is_asked_for_t_equal_to_the_index_over_fps_in_order(tmp_path, fps, seconds, times):
    clock = Clock()
    btvec.frames(clock, width=WIDTH, height=HEIGHT, fps=fps, seconds=seconds, out=tmp_path / "f-#.png")
    assert clock.times == times


@pytest.mark.parametrize(
    ("seconds", "fps", "count"),
    [
        pytest.param(1, 30, 30, id="a-whole-number"),
        pytest.param(2, 24, 48, id="two-seconds-of-film"),
        pytest.param(0.5, 30, 15, id="half-a-second"),
        pytest.param(1, 29.97, 30, id="rounds-up-from-29.97"),
        pytest.param(1, 29.4, 29, id="rounds-down-from-29.4"),
        pytest.param(0.1, 25, 2, id="2.5-rounds-to-even-as-round-does"),
        pytest.param(0.3, 25, 8, id="7.5-rounds-to-even-as-round-does"),
        pytest.param(0.024, 25, 1, id="more-than-half-a-frame-is-a-frame"),
    ],
)
def test_the_frame_count_is_seconds_times_fps_rounded(tmp_path, seconds, fps, count):
    btvec.frames(badge_at, width=4, height=4, fps=fps, seconds=seconds, out=tmp_path / "f-#####.png")
    assert len(names(tmp_path)) == count


def test_fps_defaults_to_30(tmp_path):
    btvec.frames(badge_at, width=4, height=4, seconds=1, out=tmp_path / "f-###.png")
    assert len(names(tmp_path)) == 30


@pytest.mark.parametrize(
    ("pattern", "count", "expected_first", "expected_last"),
    [
        pytest.param("f-####.png", 12, "f-0000.png", "f-0011.png", id="four-digits-zero-padded"),
        pytest.param("f-#.png", 12, "f-0.png", "f-11.png", id="one-digit-is-a-minimum-not-a-width"),
        pytest.param("f-##.png", 101, "f-00.png", "f-100.png", id="an-index-that-needs-more-digits-widens"),
        pytest.param("f-#.PNG", 3, "f-0.PNG", "f-2.PNG", id="the-suffix-is-matched-in-any-case"),
        pytest.param("#.png", 3, "0.png", "2.png", id="the-name-may-be-only-the-run"),
        pytest.param("a#b.png", 3, "a0b.png", "a2b.png", id="text-on-both-sides"),
    ],
)
def test_the_run_of_hashes_is_the_minimum_digits_of_the_zero_based_index(
    tmp_path, pattern, count, expected_first, expected_last
):
    btvec.frames(badge_at, width=4, height=4, fps=count, seconds=1, out=tmp_path / pattern)
    found = names(tmp_path)
    assert (len(found), min(found), max(found, key=lambda name: (len(name), name))) == (
        count,
        expected_first,
        expected_last,
    )


def test_a_hash_in_a_folder_name_is_literal_only_the_file_name_is_numbered(tmp_path):
    btvec.frames(badge_at, width=4, height=4, fps=2, seconds=1, out=tmp_path / "take#1" / "f-##.png")
    assert names(tmp_path / "take#1") == ["f-00.png", "f-01.png"]


def test_the_folder_is_created_with_its_parents(tmp_path):
    btvec.frames(badge_at, width=4, height=4, fps=2, seconds=1, out=tmp_path / "a" / "b" / "f-#.png")
    assert names(tmp_path / "a" / "b") == ["f-0.png", "f-1.png"]


def test_a_string_path_works_as_well_as_a_path_object(tmp_path):
    btvec.frames(badge_at, width=4, height=4, fps=2, seconds=1, out=str(tmp_path / "f-#.png"))
    assert names(tmp_path) == ["f-0.png", "f-1.png"]


def test_existing_frames_are_overwritten_and_frames_of_a_longer_earlier_run_stay(tmp_path, decode_png):
    (tmp_path / "f-0.png").write_bytes(b"not a png")
    (tmp_path / "f-9.png").write_bytes(b"left from a longer run")

    btvec.frames(badge_at, width=WIDTH, height=HEIGHT, fps=2, seconds=1, out=tmp_path / "f-#.png")

    assert decode_png((tmp_path / "f-0.png").read_bytes())[2] == BADGE.to_rgba(WIDTH, HEIGHT, x=0)
    assert (tmp_path / "f-9.png").read_bytes() == b"left from a longer run"


def test_two_runs_write_identical_files(tmp_path):
    first, second = tmp_path / "first", tmp_path / "second"
    for folder in (first, second):
        btvec.frames(badge_at, width=WIDTH, height=HEIGHT, fps=4, seconds=1, out=folder / "f-#.png")

    assert names(first) == names(second) == [f"f-{i}.png" for i in range(4)]
    assert [(first / name).read_bytes() for name in names(first)] == [
        (second / name).read_bytes() for name in names(second)
    ]


def test_frames_written_before_the_scene_fails_stay_and_its_error_reaches_the_caller(tmp_path):
    def scene(t: float) -> btvec.Drawing:
        if t >= 0.5:
            raise RuntimeError("the scene broke at 0.5")
        return badge_at(t)

    with pytest.raises(RuntimeError, match=r"broke at 0\.5"):
        btvec.frames(scene, width=4, height=4, fps=4, seconds=1, out=tmp_path / "f-#.png")

    assert names(tmp_path) == ["f-0.png", "f-1.png"]


@pytest.mark.parametrize(
    ("width", "height"),
    [
        pytest.param(0, 4, id="width-0"),
        pytest.param(4, 0, id="height-0"),
        pytest.param(65_001, 4, id="width-over-the-pixel-limit"),
    ],
)
def test_a_size_the_outputs_refuse_is_their_parameter_error_and_nothing_is_written(tmp_path, width, height):
    with pytest.raises(btvec.ParameterError, match=r"width|height|size"):
        btvec.frames(badge_at, width=width, height=height, fps=2, seconds=1, out=tmp_path / "out" / "f-#.png")
    assert list(tmp_path.iterdir()) == []


def test_a_folder_that_cannot_be_made_is_an_oserror(tmp_path):
    (tmp_path / "file").write_bytes(b"")
    with pytest.raises(OSError, match="file"):
        btvec.frames(badge_at, width=4, height=4, fps=2, seconds=1, out=tmp_path / "file" / "f-#.png")


@pytest.mark.parametrize(
    ("kwargs", "named"),
    [
        pytest.param({"fps": 0}, "fps", id="fps-0"),
        pytest.param({"fps": -30}, "fps", id="fps-negative"),
        pytest.param({"fps": float("nan")}, "fps", id="fps-nan"),
        pytest.param({"fps": float("inf")}, "fps", id="fps-infinite"),
        pytest.param({"seconds": 0}, "seconds", id="seconds-0"),
        pytest.param({"seconds": -1}, "seconds", id="seconds-negative"),
        pytest.param({"seconds": float("nan")}, "seconds", id="seconds-nan"),
        pytest.param({"seconds": float("inf")}, "seconds", id="seconds-infinite"),
        pytest.param({"seconds": 0.01}, "seconds", id="shorter-than-half-a-frame-is-no-frame"),
        pytest.param({"out": "frames/badge.png"}, "out", id="png-without-a-run-of-hashes"),
        pytest.param({"out": "frames/a#b#.png"}, "out", id="png-with-two-runs"),
        pytest.param({"out": "frames/take#1/badge.png"}, "out", id="a-hash-only-in-a-folder-is-no-run"),
        pytest.param({"out": "frames/badge-####.gif"}, "out", id="an-unknown-kind"),
        pytest.param({"out": "frames/badge"}, "out", id="no-suffix"),
        pytest.param({"out": ""}, "out", id="empty"),
    ],
)
def test_a_bad_argument_is_a_parameter_error_naming_it_and_nothing_is_drawn_or_written(
    tmp_path, monkeypatch, kwargs, named
):
    monkeypatch.chdir(tmp_path)
    clock = Clock()
    arguments = {"width": 4, "height": 4, "fps": 30, "seconds": 1, "out": "frames/badge-####.png"} | kwargs

    with pytest.raises(btvec.ParameterError, match=f"{named} must"):
        btvec.frames(clock, **arguments)

    assert clock.times == []
    assert list(tmp_path.iterdir()) == []
