"""Write the preset files: run each program `examples/presets/<name>.py` and write the `to_json` of the `drawing` it
leaves to `crates/core/presets/<name>.json`, with no trailing newline.

    uv run poe presets

A preset is added by writing its program, running this and adding the name and description to the table in
`crates/core/src/preset.rs`. `tests/test_preset.py` fails when a file and its program differ. The folder is wholly
generated: a JSON with no program is deleted. A program that leaves no `drawing` stops the tool (exit 1, naming it)
before anything is written.

Interface:
    write(programs, out) -> None: write `<name>.json` into `out` for each `<name>.py` in `programs`, delete the others
    main()               -> write into the repository's own folders
"""

import runpy
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def write(programs: Path, out: Path) -> None:
    """Write `<name>.json` into `out` for each program `<name>.py` in `programs`, and delete every other JSON there."""
    texts = {}
    for program in sorted(programs.glob("*.py")):
        namespace = runpy.run_path(str(program), run_name="__main__")
        if "drawing" not in namespace:
            raise SystemExit(f"{program.name}: the program leaves no `drawing`")
        texts[program.stem] = namespace["drawing"].to_json()
    for stale in out.glob("*.json"):
        if stale.stem not in texts:
            stale.unlink()
    for name, text in texts.items():
        (out / f"{name}.json").write_text(text, encoding="utf-8")


def main() -> None:
    """Write the repository's preset files."""
    write(ROOT / "examples" / "presets", ROOT / "crates" / "core" / "presets")


if __name__ == "__main__":
    main()
