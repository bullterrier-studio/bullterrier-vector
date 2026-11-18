"""tools/write_presets.py writes `<name>.json` into the output folder for each program `<name>.py` of the programs
folder: the text the program's `drawing.to_json()` gives (the oracle is the program run here, by runpy), with no
trailing newline, and nothing else."""

import importlib.util
import runpy
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PROGRAMS = ROOT / "examples" / "presets"


def _load_tool():
    spec = importlib.util.spec_from_file_location("write_presets", ROOT / "tools" / "write_presets.py")
    assert spec is not None
    assert spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


tool = _load_tool()


def test_it_writes_the_json_of_every_program_and_no_other_file(tmp_path: Path) -> None:
    tool.write(PROGRAMS, tmp_path)
    assert sorted(p.name for p in tmp_path.iterdir()) == sorted(f"{p.stem}.json" for p in PROGRAMS.glob("*.py"))
    for program in PROGRAMS.glob("*.py"):
        expected = runpy.run_path(str(program), run_name="__main__")["drawing"].to_json()
        written = (tmp_path / f"{program.stem}.json").read_text(encoding="utf-8")
        assert written == expected
        assert not written.endswith("\n")


def test_it_replaces_a_stale_file(tmp_path: Path) -> None:
    (tmp_path / "ring.json").write_text("stale", encoding="utf-8")
    tool.write(PROGRAMS, tmp_path)
    assert (tmp_path / "ring.json").read_text(encoding="utf-8").startswith('{"drawings"')
