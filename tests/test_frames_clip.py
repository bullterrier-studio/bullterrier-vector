"""`btvec.frames` writing a clip. A clip is lossy, so it is read back by ffprobe (frame count, duration, frame rate,
alpha) and decoded by ffmpeg to raw RGBA, and compared with `Drawing.to_rgba` within a measured tolerance. The tests
that need ffmpeg skip with a stated reason when it, or the encoder, is missing; the ones that fail a stand-in ffmpeg do
not need it."""

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

import bullterrier_vector as btvec

WIDTH, HEIGHT = 64, 48
# a red square, 16 by 16, that moves right by `x` pixels, so a frame drawn at the wrong time shows
X = btvec.Param("x", default=0, min=0, max=1000)
BADGE = btvec.Drawing(btvec.Path.rect(0, 0, 16, 16).fill("#e2412e").translate(X, 8), size=(WIDTH, HEIGHT))

# Tolerances for a lossy clip, in levels of a channel (0 to 255). Measured on this badge with ffmpeg 8.0.1: ProRes 4444
# decoded exactly, VP9 (constant quality 15, 4:2:0 chroma) was at most 4 levels off in colour and 0 in alpha; the limits
# are twice what was measured, far below the 255 levels by which a frame drawn at the wrong time differs.
COLOUR_TOLERANCE = 8
ALPHA_TOLERANCE = 8
# Pixels within this many pixels of an edge are not compared: 4:2:0 chroma blends colours across an edge.
EDGE_MARGIN = 2

ENCODERS = {".mov": "prores_ks", ".webm": "libvpx-vp9"}
# ffmpeg needs the VP9 decoder named to read the alpha of a .webm; its default decoder drops it
DECODER = {".mov": [], ".webm": ["-c:v", "libvpx-vp9"]}


def has_encoder(name: str) -> bool:
    """Whether the ffmpeg on the PATH lists the encoder `name`."""
    path = shutil.which("ffmpeg")
    if path is None:
        return False
    listing = subprocess.run(
        [path, "-hide_banner", "-encoders"], capture_output=True, text=True, encoding="utf-8", check=False
    )
    return name in listing.stdout


def needs(suffix: str):
    """A skip marker, with the reason, for a test that needs ffmpeg with the encoder of `suffix`."""
    return pytest.mark.skipif(
        not has_encoder(ENCODERS[suffix]), reason=f"ffmpeg with the {ENCODERS[suffix]} encoder is not on the PATH"
    )


CLIPS = [
    pytest.param(".mov", marks=needs(".mov"), id="prores-4444"),
    pytest.param(".webm", marks=needs(".webm"), id="vp9"),
]


def probe(clip: Path) -> dict:
    """What ffprobe says about the video stream and the container of `clip`: `stream` (with `nb_read_frames`, counted by
    decoding) and `duration`."""
    command = [
        "ffprobe", "-v", "error", "-count_frames", "-select_streams", "v:0", "-show_streams", "-show_format",
        "-of", "json", str(clip),
    ]  # fmt: skip
    answer = json.loads(subprocess.run(command, capture_output=True, text=True, encoding="utf-8", check=True).stdout)
    return {"stream": answer["streams"][0], "duration": float(answer["format"]["duration"])}


def decode(clip: Path) -> bytes:
    """Every frame of `clip` as raw RGBA, one after the other."""
    command = [
        "ffmpeg",
        "-v",
        "error",
        *DECODER[clip.suffix],
        "-i",
        str(clip),
        "-f",
        "rawvideo",
        "-pix_fmt",
        "rgba",
        "-",
    ]
    return subprocess.run(command, capture_output=True, check=True).stdout


def deviation(source: bytes, decoded: bytes, width: int, height: int) -> tuple[int, int, int]:
    """How far the decoded frame is from the source frame, away from the edges of what is drawn:
    `(colour error where the source is opaque, alpha missing where it is opaque, alpha present where it is clear)`."""

    def alpha(x: int, y: int) -> int | None:
        return source[(y * width + x) * 4 + 3] if 0 <= x < width and 0 <= y < height else None

    colour = missing = stray = 0
    for y in range(height):
        for x in range(width):
            around = {
                alpha(x + dx, y + dy)
                for dx in range(-EDGE_MARGIN, EDGE_MARGIN + 1)
                for dy in range(-EDGE_MARGIN, EDGE_MARGIN + 1)
            }
            at = (y * width + x) * 4
            if around - {None} == {255}:
                colour = max(colour, *(abs(source[at + c] - decoded[at + c]) for c in range(3)))
                missing = max(missing, 255 - decoded[at + 3])
            elif around - {None} == {0}:
                stray = max(stray, decoded[at + 3])
    return colour, missing, stray


def badge_at(t: float) -> btvec.Drawing:
    """The badge at `t` seconds: it moves 16 pixels a second."""
    return BADGE.with_values(x=16 * t)


@pytest.mark.parametrize("suffix", CLIPS)
@pytest.mark.parametrize(
    ("fps", "seconds", "frames"),
    [
        pytest.param(4, 1.5, 6, id="4-fps"),
        pytest.param(24, 0.25, 6, id="24-fps"),
        pytest.param(30, 1, 30, id="30-fps"),
    ],
)
def test_a_clip_has_the_frame_count_frame_rate_duration_size_and_an_alpha_channel(
    tmp_path, suffix, fps, seconds, frames
):
    clip = tmp_path / f"badge{suffix}"
    btvec.frames(badge_at, width=WIDTH, height=HEIGHT, fps=fps, seconds=seconds, out=clip)

    found = probe(clip)
    stream = found["stream"]
    assert int(stream["nb_read_frames"]) == frames
    assert stream["r_frame_rate"] == f"{fps}/1"
    assert (stream["width"], stream["height"]) == (WIDTH, HEIGHT)
    assert found["duration"] == pytest.approx(frames / fps, abs=0.5 / fps)
    if suffix == ".mov":
        assert stream["codec_name"] == "prores"
        assert stream["pix_fmt"].startswith("yuva"), "ProRes 4444 with alpha is a yuva pixel format"
    else:
        # ffprobe's default VP9 decoder shows yuv420p; the file says its second stream is alpha in this tag
        assert stream["codec_name"] == "vp9"
        assert stream["tags"]["alpha_mode"] == "1"


@pytest.mark.parametrize("suffix", CLIPS)
def test_every_decoded_frame_is_the_drawing_at_its_time_opaque_where_drawn_and_clear_elsewhere(tmp_path, suffix):
    clip = tmp_path / f"badge{suffix}"
    btvec.frames(badge_at, width=WIDTH, height=HEIGHT, fps=4, seconds=1.5, out=clip)

    decoded = decode(clip)
    size = WIDTH * HEIGHT * 4
    assert len(decoded) == 6 * size
    for i in range(6):
        # at 4 frames a second the badge is 4 pixels further right each frame
        source = BADGE.to_rgba(WIDTH, HEIGHT, x=4 * i)
        colour, missing, stray = deviation(source, decoded[i * size : (i + 1) * size], WIDTH, HEIGHT)
        assert colour <= COLOUR_TOLERANCE, f"frame {i}: colour error {colour}"
        assert missing <= ALPHA_TOLERANCE, f"frame {i}: opaque pixels lack alpha by {missing}"
        assert stray <= ALPHA_TOLERANCE, f"frame {i}: clear pixels have alpha {stray}"


def test_the_comparison_fails_for_a_frame_drawn_at_the_wrong_time(tmp_path):
    # the known-bad input of `deviation`: the frame of the badge 8 pixels to the right of where it should be
    drawn = BADGE.to_rgba(WIDTH, HEIGHT, x=0)
    shifted = BADGE.to_rgba(WIDTH, HEIGHT, x=8)
    _colour, missing, stray = deviation(drawn, shifted, WIDTH, HEIGHT)
    assert max(missing, stray) == 255
    assert deviation(drawn, drawn, WIDTH, HEIGHT) == (0, 0, 0)


@pytest.mark.parametrize("suffix", CLIPS)
def test_a_clip_of_an_odd_size_is_written_at_that_size(tmp_path, suffix):
    clip = tmp_path / f"odd{suffix}"
    btvec.frames(badge_at, width=33, height=17, fps=4, seconds=1, out=clip)
    stream = probe(clip)["stream"]
    assert (stream["width"], stream["height"], int(stream["nb_read_frames"])) == (33, 17, 4)


@pytest.mark.parametrize("suffix", CLIPS)
def test_only_the_clip_is_left_in_the_new_folder_and_it_replaces_an_older_one(tmp_path, suffix):
    clip = tmp_path / "a" / "b" / f"badge{suffix}"
    clip.parent.mkdir(parents=True)
    clip.write_bytes(b"an older clip")

    btvec.frames(badge_at, width=WIDTH, height=HEIGHT, fps=4, seconds=1, out=clip)

    assert [path.name for path in clip.parent.iterdir()] == [clip.name]
    assert int(probe(clip)["stream"]["nb_read_frames"]) == 4


@pytest.mark.parametrize("suffix", CLIPS)
def test_a_scene_that_fails_midway_leaves_no_clip_and_an_older_clip_untouched(tmp_path, suffix):
    def scene(t: float) -> btvec.Drawing:
        if t >= 0.5:
            raise RuntimeError("the scene broke at 0.5")
        return badge_at(t)

    clip = tmp_path / f"badge{suffix}"
    with pytest.raises(RuntimeError, match=r"broke at 0\.5"):
        btvec.frames(scene, width=WIDTH, height=HEIGHT, fps=4, seconds=1, out=clip)
    assert list(tmp_path.iterdir()) == []

    clip.write_bytes(b"an older clip")
    with pytest.raises(RuntimeError, match=r"broke at 0\.5"):
        btvec.frames(scene, width=WIDTH, height=HEIGHT, fps=4, seconds=1, out=clip)
    assert [(path.name, path.read_bytes()) for path in tmp_path.iterdir()] == [(clip.name, b"an older clip")]


@pytest.mark.skipif(sys.platform == "win32", reason="the stand-in ffmpeg is a shell script")
@pytest.mark.parametrize("suffix", [".mov", ".webm"])
def test_a_size_the_outputs_refuse_is_a_parameter_error_and_leaves_no_folder(tmp_path, monkeypatch, suffix):
    # the stand-in ffmpeg is found, so the refusal is the size's and not a missing program
    stand_in_ffmpeg(tmp_path / "bin", monkeypatch, "exit 0")
    with pytest.raises(btvec.ParameterError, match=r"width|height|size"):
        btvec.frames(badge_at, width=0, height=4, fps=4, seconds=1, out=tmp_path / "out" / f"badge{suffix}")
    assert not (tmp_path / "out").exists()


def stand_in_ffmpeg(folder: Path, monkeypatch: pytest.MonkeyPatch, script: str) -> None:
    """Put a `ffmpeg` first on the PATH that is a shell script with the body `script`."""
    folder.mkdir()
    program = folder / "ffmpeg"
    program.write_text(f"#!/bin/sh\n{script}\n", encoding="utf-8")
    program.chmod(0o755)
    monkeypatch.setenv("PATH", f"{folder}{os.pathsep}/bin{os.pathsep}/usr/bin")  # the script needs `cat`


@pytest.mark.skipif(sys.platform == "win32", reason="the stand-in ffmpeg is a shell script")
@pytest.mark.parametrize(
    "script",
    [
        pytest.param("echo 'boom: no such codec' >&2; exit 3", id="exits-before-reading-a-frame"),
        pytest.param("cat > /dev/null; echo 'boom: no such codec' >&2; exit 3", id="fails-after-reading-all-frames"),
    ],
)
@pytest.mark.parametrize("suffix", [".mov", ".webm"])
def test_an_ffmpeg_that_fails_is_an_oserror_with_its_message_and_leaves_nothing(tmp_path, monkeypatch, script, suffix):
    stand_in_ffmpeg(tmp_path / "bin", monkeypatch, script)
    out = tmp_path / "out"

    with pytest.raises(OSError, match=r"exit status 3.*boom: no such codec"):
        btvec.frames(badge_at, width=WIDTH, height=HEIGHT, fps=4, seconds=1, out=out / f"badge{suffix}")

    assert list(out.iterdir()) == []


@pytest.mark.parametrize("suffix", [".mov", ".webm"])
def test_no_ffmpeg_on_the_path_is_an_oserror_naming_it_and_nothing_is_drawn_or_written(tmp_path, monkeypatch, suffix):
    monkeypatch.setenv("PATH", str(tmp_path))
    drawn: list[float] = []

    def scene(t: float) -> btvec.Drawing:
        drawn.append(t)
        return badge_at(t)

    with pytest.raises(OSError, match="ffmpeg"):
        btvec.frames(scene, width=WIDTH, height=HEIGHT, fps=4, seconds=1, out=tmp_path / "out" / f"badge{suffix}")

    assert drawn == []
    assert [path.name for path in tmp_path.iterdir()] == []
