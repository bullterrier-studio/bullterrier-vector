"""`tools/look.py` draws a drawing file to `dist/look/`: a PNG that decodes to the pixels the library returns and an
SVG that parses, the size from the drawing, from `--size` or the default, Params from NAME=VALUE; and it exits 1 with
one line on stderr, writing nothing, for every request it cannot draw.

Each case runs a copy of the tool in a scratch layout (`tools/`, `examples/`, `docs/examples/json/`) in a subprocess
with an explicit environment, so the real repository's `dist/` is never written and the tool is run as a person runs it.
"""

import runpy
import shutil
import subprocess
import sys
import xml.etree.ElementTree as ET
from collections.abc import Callable
from pathlib import Path
from typing import NamedTuple

import pytest

import bullterrier_vector as btvec

ROOT = Path(__file__).resolve().parents[2]
SVG = "{http://www.w3.org/2000/svg}"
SERIES_BARS = "docs/examples/json/series_bars.json"
SLIDER = """\
import bullterrier_vector as btvec

shift = btvec.Param("shift", default=0, min=0, max=100, unit="px")
drawing = btvec.Drawing(btvec.Path.rect(0, 0, 10, 10).fill("#00ff00")).translate(shift, 0)
"""
UNSIZED = (
    'import bullterrier_vector as btvec\n\ndrawing = btvec.Drawing(btvec.Path.rect(0, 0, 10, 10).fill("#00ff00"))\n'
)


class Run(NamedTuple):
    code: int
    out: str
    err: str


@pytest.fixture
def layout(tmp_path: Path) -> Path:
    """A scratch copy of the files the tool and its cases use, laid out as in the repository."""
    for name in ("tools/look.py", "examples/star.py", SERIES_BARS):
        (tmp_path / name).parent.mkdir(parents=True, exist_ok=True)
        shutil.copy(ROOT / name, tmp_path / name)
    return tmp_path


@pytest.fixture
def look(layout: Path, uv_run_env: dict[str, str]) -> Callable[..., Run]:
    """`look(*args, cwd=None)`: run the copy of the tool with `args`, from `cwd` (default: the layout)."""

    def run(*args: str, cwd: Path | None = None) -> Run:
        done = subprocess.run(
            [sys.executable, str(layout / "tools" / "look.py"), *args],
            cwd=cwd or layout,
            env=uv_run_env,
            capture_output=True,
            encoding="utf-8",
            check=False,
        )
        return Run(done.returncode, done.stdout, done.stderr)

    return run


def parse_svg(path: Path) -> ET.Element:
    """The root of the SVG document in the file `path`."""
    return ET.fromstring(path.read_text(encoding="utf-8"))  # noqa: S314  our own output


def pictures(layout: Path, name: str) -> tuple[Path, Path]:
    return layout / "dist" / "look" / f"{name}.png", layout / "dist" / "look" / f"{name}.svg"


def test_a_json_document_is_drawn_to_a_png_of_the_pixels_the_library_returns_and_an_svg(layout, look, decode_png):
    drawing = btvec.Drawing.from_json((layout / SERIES_BARS).read_text(encoding="utf-8"))
    assert drawing.size == (1080, 1080)
    result = look(SERIES_BARS)
    png, svg = pictures(layout, "series_bars")
    assert result == Run(0, f"{png}\n{svg}\n", "")
    assert decode_png(png.read_bytes()) == (1080, 1080, drawing.to_rgba(1080, 1080))
    root = parse_svg(svg)
    assert (root.tag, root.attrib["width"], root.attrib["height"]) == (f"{SVG}svg", "1080", "1080")
    assert len(root) > 0


def test_a_python_program_is_drawn_by_its_drawing_variable(layout, look, decode_png):
    result = look("examples/star.py")
    png, svg = pictures(layout, "star")
    assert result == Run(0, f"{png}\n{svg}\n", "")
    star = runpy.run_path(str(ROOT / "examples" / "star.py"))["drawing"]
    assert decode_png(png.read_bytes()) == (400, 400, star.to_rgba(400, 400))
    assert svg.read_text(encoding="utf-8") == star.to_svg(400, 400)
    # and the picture is the star: its centre is the yellow fill, the corner of the frame is empty
    pixels = decode_png(png.read_bytes())[2]
    assert tuple(pixels[(200 * 400 + 200) * 4 :][:4]) == (0xFF, 0xCC, 0x33, 255)
    assert tuple(pixels[:4]) == (0, 0, 0, 0)


def test_the_size_option_sets_the_pictures_size(layout, look, decode_png):
    assert look("examples/star.py", "--size", "100x50").code == 0
    png, svg = pictures(layout, "star")
    width, height, pixels = decode_png(png.read_bytes())
    assert (width, height, len(pixels)) == (100, 50, 100 * 50 * 4)
    root = parse_svg(svg)
    assert (root.attrib["width"], root.attrib["height"]) == ("100", "50")


def test_a_drawing_without_a_design_size_is_drawn_at_800_by_600(layout, look, decode_png):
    (layout / "unsized.py").write_text(UNSIZED, encoding="utf-8")
    assert look("unsized.py").code == 0
    png, svg = pictures(layout, "unsized")
    assert decode_png(png.read_bytes())[:2] == (800, 600)
    assert parse_svg(svg).attrib["width"] == "800"


def test_name_value_arguments_set_params(layout, look, decode_png):
    (layout / "slider.py").write_text(SLIDER, encoding="utf-8")
    assert look("slider.py", "--size", "30x10", "shift=20").code == 0
    png, svg = pictures(layout, "slider")
    drawing = btvec.Drawing(btvec.Path.rect(0, 0, 10, 10).fill("#00ff00")).translate(
        btvec.Param("shift", default=0, min=0, max=100, unit="px"), 0
    )
    assert decode_png(png.read_bytes()) == (30, 10, drawing.to_rgba(30, 10, shift=20))
    assert decode_png(png.read_bytes())[2] != drawing.to_rgba(30, 10)
    assert svg.read_text(encoding="utf-8") == drawing.to_svg(30, 10, shift=20)


def test_pictures_go_to_the_tools_dist_folder_whatever_the_working_folder(layout, look, tmp_path_factory):
    elsewhere = tmp_path_factory.mktemp("elsewhere")
    assert look(str(layout / "examples" / "star.py"), cwd=elsewhere).code == 0
    assert list(elsewhere.iterdir()) == []
    assert all(path.is_file() for path in pictures(layout, "star"))


def test_a_picture_of_the_same_name_is_overwritten(layout, look, decode_png):
    assert look("examples/star.py", "--size", "20x20").code == 0
    assert look("examples/star.py", "--size", "30x10").code == 0
    png, _ = pictures(layout, "star")
    assert decode_png(png.read_bytes())[:2] == (30, 10)
    assert sorted(path.name for path in png.parent.iterdir()) == ["star.png", "star.svg"]


@pytest.mark.parametrize(
    ("setup", "args", "message"),
    [
        pytest.param({}, ["missing.json"], "missing.json: no such file", id="a missing file"),
        pytest.param({"notes.txt": "x"}, ["notes.txt"], ".json document or a .py program", id="another suffix"),
        pytest.param({"bare": "x"}, ["bare"], "no suffix", id="no suffix"),
        pytest.param({"empty.py": "x = 1\n"}, ["empty.py"], "`drawing`", id="a program without drawing"),
        pytest.param({"wrong.py": "drawing = 3\n"}, ["wrong.py"], "`drawing`", id="a drawing that is not a Drawing"),
        pytest.param({"broken.json": "{"}, ["broken.json"], "", id="a document the library cannot read"),
        pytest.param({}, [SERIES_BARS, "--size", "10by10"], "WIDTHxHEIGHT", id="a size not written WxH"),
        pytest.param({}, [SERIES_BARS, "--size", "0x10"], "width", id="a size of zero"),
        pytest.param({}, [SERIES_BARS, "elsewhere=3"], "elsewhere", id="a value for no Param"),
        pytest.param({}, [SERIES_BARS, "elsewhere"], "NAME=NUMBER", id="a value with no number"),
        pytest.param({"slider.py": SLIDER}, ["slider.py", "shift=900"], "shift", id="a value out of range"),
    ],
)
def test_a_request_that_cannot_be_drawn_exits_1_with_one_line_and_writes_nothing(layout, look, setup, args, message):
    for name, text in setup.items():
        (layout / name).write_text(text, encoding="utf-8")
    result = look(*args)
    assert (result.code, result.out) == (1, "")
    assert result.err.startswith("look: ")
    assert len(result.err.splitlines()) == 1
    assert message in result.err
    assert not (layout / "dist").exists()
