"""Draw a drawing to look at it: write `dist/look/<name>.png` and `dist/look/<name>.svg` and print their paths.

    uv run poe look FILE [--size WxH] [NAME=VALUE ...]

FILE is a `.json` document (read with `Drawing.from_json`) or a `.py` program (run; its module-level variable
`drawing` is the drawing, as in examples/star.py). The size is the drawing's design frame when it has one, else 800x600.
Each NAME=VALUE sets a Param. A picture of the same name is overwritten. Exit code 1, with one line on stderr, for a
file that is missing or of another kind, a program without `drawing`, a bad size or value, or an error from the
library. The pictures are for a person or an agent to open: the tool opens nothing.

Interface:
    main(argv) -> exit code: 0 the pictures were written, 1 they were not (the reason is on stderr)
"""

import argparse
import re
import runpy
import sys
from pathlib import Path

import bullterrier_vector as btvec
from bullterrier_vector.errors import BullterrierVectorError

OUTPUT = Path(__file__).resolve().parent.parent / "dist" / "look"
# constant: the size of a drawing with no design frame, which has none of its own to draw at: 4:3, large enough to see
DEFAULT_SIZE = (800, 600)


class LookError(Exception):
    """Something the caller asked for cannot be drawn; the message says what, in one line."""


def load(path: Path) -> btvec.Drawing:
    """The drawing in the `.json` document or `.py` program at `path`."""
    if not path.is_file():
        raise LookError(f"{path}: no such file")
    if path.suffix == ".json":
        return btvec.Drawing.from_json(path.read_text(encoding="utf-8"))
    if path.suffix == ".py":
        found = runpy.run_path(str(path), run_name="__main__").get("drawing")
        if not isinstance(found, btvec.Drawing):
            raise LookError(f"{path}: the program must leave a `drawing` (a Drawing) at module level")
        return found
    raise LookError(
        f"{path}: a .json document or a .py program is expected, not {path.suffix or 'a file with no suffix'}"
    )


def parse_size(text: str) -> tuple[float, float]:
    """`(width, height)` of a size written `WxH`."""
    found = re.fullmatch(r"(\d+)x(\d+)", text)
    if found is None:
        raise LookError(f"--size {text}: write it as WIDTHxHEIGHT in pixels, such as 800x600")
    return int(found[1]), int(found[2])


def parse_values(texts: list[str]) -> dict[str, float]:
    """The Param values written `NAME=VALUE`."""
    values: dict[str, float] = {}
    for text in texts:
        name, _, value = text.partition("=")
        try:
            values[name] = float(value)
        except ValueError:
            raise LookError(f"{text}: a Param value is written NAME=NUMBER") from None
    return values


def main(argv: list[str] | None = None) -> int:
    """Draw the file named in `argv`; see the module docstring."""
    parser = argparse.ArgumentParser(prog="look", description=__doc__.splitlines()[0])
    parser.add_argument("file", type=Path, help="a .json document or a .py program with a `drawing`")
    parser.add_argument("--size", help="WIDTHxHEIGHT in pixels (default: the drawing's own size, else 800x600)")
    parser.add_argument("values", nargs="*", metavar="NAME=VALUE", help="a value for a Param")
    arguments = parser.parse_args(argv)
    try:
        drawing = load(arguments.file)
        values = parse_values(arguments.values)
        width, height = parse_size(arguments.size) if arguments.size else (drawing.size or DEFAULT_SIZE)
        png = drawing.to_png(width, height, **values)
        svg = drawing.to_svg(width, height, **values)
    except (LookError, BullterrierVectorError) as error:
        print(f"look: {error}", file=sys.stderr)
        return 1
    OUTPUT.mkdir(parents=True, exist_ok=True)
    png_path, svg_path = (OUTPUT / f"{arguments.file.stem}{suffix}" for suffix in (".png", ".svg"))
    png_path.write_bytes(png)
    svg_path.write_text(svg, encoding="utf-8")
    print(png_path)
    print(svg_path)
    return 0


if __name__ == "__main__":
    sys.exit(main())
