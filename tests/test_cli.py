"""`bt-vector`, called in process through `run`. The oracles: the PNG header bytes read here, the SVG parsed with
ElementTree, and the library's own `to_png` / `to_svg` called directly."""

import struct
from pathlib import Path
from xml.etree import ElementTree

import pytest

import bullterrier_vector as btvec
from bullterrier_vector.cli import run

PNG = b"\x89PNG\r\n\x1a\n"
BYTES = bytes([0, 100, 200, 255, 50, 0])  # a 3 by 2 map


def png_size(data: bytes) -> tuple[int, int]:
    assert data[:8] == PNG
    assert data[12:16] == b"IHDR"
    return struct.unpack(">II", data[16:24])


def map_document(directory: Path) -> Path:
    field = btvec.Field.map(3, 2, BYTES, name="photo")
    drawing = btvec.Drawing(btvec.Path.rect(0, 0, 3, 2)).hatch((0.5, 0.5), tone=field, below=0.5)
    path = directory / "map.json"
    path.write_text(drawing.to_json(), encoding="utf-8")
    return path


def test_a_preset_renders_to_a_png_of_its_own_size(tmp_path: Path) -> None:
    out = tmp_path / "ring.png"
    assert run(["render", "ring", "-o", str(out)]) == 0
    assert png_size(out.read_bytes()) == (400, 400)


def test_size_sets_the_png_dimensions(tmp_path: Path) -> None:
    out = tmp_path / "ring.png"
    assert run(["render", "ring", "-o", str(out), "--size", "200x100"]) == 0
    assert png_size(out.read_bytes()) == (200, 100)


def test_a_drawing_without_a_design_size_renders_at_800x600(tmp_path: Path) -> None:
    source = tmp_path / "plain.json"
    source.write_text(btvec.Drawing(btvec.Path.rect(0, 0, 3, 2)).to_json(), encoding="utf-8")
    out = tmp_path / "plain.png"
    assert run(["render", str(source), "-o", str(out)]) == 0
    assert png_size(out.read_bytes()) == (800, 600)


def test_svg_output_is_the_librarys_and_values_change_it(tmp_path: Path) -> None:
    outs = {}
    for width in ("2", "40"):
        out = tmp_path / f"ring{width}.svg"
        assert run(["render", "ring", "-o", str(out), f"half_width={width}"]) == 0
        outs[width] = out.read_text(encoding="utf-8")
        ElementTree.fromstring(outs[width])  # noqa: S314  our own output
        assert outs[width] == btvec.Drawing.preset("ring").to_svg(400, 400, half_width=float(width))
    assert outs["2"] != outs["40"]


def test_a_document_renders_as_the_library_draws_it(tmp_path: Path) -> None:
    drawing = btvec.Drawing.preset("gradient_type")
    source = tmp_path / "doc.json"
    source.write_text(drawing.to_json(), encoding="utf-8")
    out = tmp_path / "doc.png"
    assert run(["render", str(source), "-o", str(out), "--size", "300x140", "weight=500"]) == 0
    assert out.read_bytes() == drawing.to_png(300, 140, weight=500)


def test_an_input_is_read_from_a_file(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    source = map_document(tmp_path)
    out = tmp_path / "map.svg"
    assert run(["render", str(source), "-o", str(out), "--size", "30x20"]) == 1
    assert "photo" in capsys.readouterr().err
    data = tmp_path / "photo.bin"
    data.write_bytes(BYTES)
    assert run(["render", str(source), "-o", str(out), "--size", "30x20", "--input", f"photo={data}"]) == 0
    expected = btvec.Drawing.from_json(source.read_text(encoding="utf-8"), inputs={"photo": BYTES})
    assert out.read_text(encoding="utf-8") == expected.to_svg(30, 20)


def test_list_prints_every_preset_once(capsys: pytest.CaptureFixture[str]) -> None:
    assert run(["list"]) == 0
    lines = capsys.readouterr().out.splitlines()
    assert [line.split()[0] for line in lines] == btvec.Drawing.preset_names()


def test_describe_prints_every_param(capsys: pytest.CaptureFixture[str]) -> None:
    assert run(["describe", "bent_type"]) == 0
    out = capsys.readouterr().out
    assert "1080" in out
    assert "560" in out
    lines = out.splitlines()
    for param in btvec.Drawing.preset("bent_type").parameters():
        row = next(line.split() for line in lines if line.split()[:1] == [param.name])
        for number in (param.default, param.min, param.max):
            assert f"{number:g}" in row
        assert param.meaning in " ".join(row)


def test_describe_reads_a_document(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    source = tmp_path / "ring.json"
    source.write_text(btvec.Drawing.preset("ring").to_json(), encoding="utf-8")
    assert run(["describe", str(source)]) == 0
    assert "half_width" in capsys.readouterr().out


def test_validate_reports_each_file(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    good = tmp_path / "good.json"
    good.write_text(btvec.Drawing.preset("ring").to_json(), encoding="utf-8")
    bad = tmp_path / "bad.json"
    bad.write_text('{"version": 99}', encoding="utf-8")
    broken = tmp_path / "broken.json"
    broken.write_text("{", encoding="utf-8")
    assert run(["validate", str(good)]) == 0
    assert capsys.readouterr().out == f"ok {good}\n"
    assert run(["validate", str(good), str(bad), str(broken)]) == 1
    captured = capsys.readouterr()
    assert f"ok {good}" in captured.out
    assert f"{bad}: /version" in captured.err
    assert f"{broken}: " in captured.err
    assert run(["validate", str(tmp_path / "missing.json")]) == 1
    assert "missing.json" in capsys.readouterr().err


def test_validate_takes_inputs(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    source = map_document(tmp_path)
    assert run(["validate", str(source)]) == 1
    data = tmp_path / "photo.bin"
    data.write_bytes(BYTES)
    capsys.readouterr()
    assert run(["validate", str(source), "--input", f"photo={data}"]) == 0


@pytest.mark.parametrize(
    ("argv", "mentions"),
    [
        pytest.param(["render", "nope", "-o", "{tmp}/a.png"], "ring", id="unknown preset lists the names"),
        pytest.param(["render", "{tmp}/missing.json", "-o", "{tmp}/a.png"], "missing.json", id="missing file"),
        pytest.param(["render", "ring", "-o", "{tmp}/a.gif"], ".svg", id="bad suffix names the two"),
        pytest.param(["render", "ring", "-o", "{tmp}/a.png", "half_width=1000"], "half_width", id="out of range"),
        pytest.param(["render", "ring", "-o", "{tmp}/a.png", "half_width="], "NAME=NUMBER", id="no number"),
        pytest.param(["render", "ring", "-o", "{tmp}/a.png", "--size", "big"], "WIDTHxHEIGHT", id="bad size"),
        pytest.param(["render", "ring", "-o", "{tmp}/no/such/a.png"], "a.png", id="unwritable folder"),
        pytest.param(["render", "ring", "-o", "{tmp}/a.png", "--input", "x={tmp}/gone"], "gone", id="missing input"),
        pytest.param(["describe", "nope"], "ring", id="describe unknown preset"),
    ],
)
def test_a_refusal_is_one_line_on_stderr_and_exit_1(
    argv: list[str], mentions: str, tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    assert run([a.replace("{tmp}", str(tmp_path)) for a in argv]) == 1
    captured = capsys.readouterr()
    assert captured.err.startswith("bt-vector: ")
    assert len(captured.err.splitlines()) == 1
    assert mentions in captured.err
    assert "Traceback" not in captured.err
    assert captured.out == ""


def test_a_file_that_is_not_utf8_is_a_refusal(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    source = tmp_path / "latin.json"
    source.write_bytes(b"\xff\xfe\x00")
    assert run(["describe", str(source)]) == 1
    assert "Traceback" not in capsys.readouterr().err


@pytest.mark.parametrize(
    "argv",
    [pytest.param(["frobnicate"], id="unknown command"), pytest.param(["render", "ring"], id="no output")],
)
def test_wrong_usage_exits_2(argv: list[str], capsys: pytest.CaptureFixture[str]) -> None:
    with pytest.raises(SystemExit) as stop:
        run(argv)
    assert stop.value.code == 2
    assert capsys.readouterr().err
