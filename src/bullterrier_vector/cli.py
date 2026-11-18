"""Command line `bt-vector`: render, describe and check drawings without writing code.

    bt-vector list
    bt-vector describe SOURCE [--input NAME=FILE ...]
    bt-vector render SOURCE -o OUT.png|OUT.svg [--size WxH] [--input NAME=FILE ...] [NAME=VALUE ...]
    bt-vector validate FILE.json [FILE.json ...] [--input NAME=FILE ...]

SOURCE is a preset name (`ring`) or, when it ends in `.json`, a document file. `render` picks the format from the
suffix of OUT, draws at the drawing's own design size unless `--size` says otherwise (800x600 for a drawing with none),
and sets Params with `NAME=VALUE` (a name the drawing has no Param of is refused);
`--input` (also on `describe`) gives a document the bytes of an input it names (a font or a map that is not bundled).

Exit codes: 0 done; 1 refused, one line `bt-vector: <what is wrong>` on stderr (`validate` prints one
`FILE: <message>` line for each bad file); 2 wrong usage (argparse explains). Results go to stdout, errors to stderr.

Interface:
    run(argv) -> exit code
    main()    -> the console script: exits with `run(sys.argv[1:])`
"""

import argparse
import re
import sys
from pathlib import Path

from bullterrier_vector._native._core import Drawing
from bullterrier_vector.errors import BullterrierVectorError

__all__ = ["main", "run"]

# constant: the size of a drawing with no design frame, which has none of its own to draw at: 4:3, large enough to see
# (the same value as `DEFAULT_SIZE` in tools/look.py, which cannot import it; change both together)
_DEFAULT_SIZE = (800, 600)
_FORMATS = (".png", ".svg")


class _RefusalError(Exception):
    """A request that cannot be done; the message says what, in one line."""


def _pairs(texts: list[str], what: str) -> list[tuple[str, str]]:
    pairs = []
    for text in texts:
        name, separator, value = text.partition("=")
        if not separator or not name:
            raise _RefusalError(f"{text}: {what} is written NAME={'FILE' if what == '--input' else 'NUMBER'}")
        pairs.append((name, value))
    return pairs


def _inputs(texts: list[str]) -> dict[str, bytes]:
    return {name: Path(path).read_bytes() for name, path in _pairs(texts, "--input")}


def _load(source: str, inputs: list[str]) -> Drawing:
    """The drawing of a `.json` document file, else of the preset called `source`."""
    named = _inputs(inputs)
    if source.endswith(".json"):
        try:
            text = Path(source).read_text(encoding="utf-8")
        except UnicodeDecodeError:
            raise _RefusalError(f"{source}: cannot be read as UTF-8 text") from None
        return Drawing.from_json(text, inputs=named)
    return Drawing.preset(source)


def _size(text: str) -> tuple[int, int]:
    found = re.fullmatch(r"(\d+)x(\d+)", text)
    if found is None:
        raise _RefusalError(f"--size {text}: write it as WIDTHxHEIGHT in pixels, such as 800x600")
    return int(found[1]), int(found[2])


def _values(texts: list[str], drawing: Drawing) -> dict[str, float]:
    """The Param values written `NAME=VALUE`; a name the drawing does not read is refused, so no name can reach a
    keyword of `to_png` itself (`window`)."""
    known = [param.name for param in drawing.parameters()]
    values = {}
    for name, value in _pairs(texts, "a Param value"):
        if name not in known:
            raise _RefusalError(f"{name}: not a Param of this drawing, whose Params are {', '.join(known) or 'none'}")
        try:
            values[name] = float(value)
        except ValueError:
            raise _RefusalError(f"{name}={value}: a Param value is written NAME=NUMBER") from None
    return values


def _list(_: argparse.Namespace) -> None:
    for name in Drawing.preset_names():
        drawing = Drawing.preset(name)
        size = "x".join(f"{side:g}" for side in drawing.size) if drawing.size else "-"
        print(f"{name}  {size}  {', '.join(param.name for param in drawing.parameters())}")


def _describe(arguments: argparse.Namespace) -> None:
    drawing = _load(arguments.source, arguments.input)
    width, height = drawing.size or _DEFAULT_SIZE
    print(f"size {width:g}x{height:g}" + ("" if drawing.size else " (none of its own: the default)"))
    print("name default min max unit meaning")
    for param in drawing.parameters():
        print(f"{param.name} {param.default:g} {param.min:g} {param.max:g} {param.unit or '-'} {param.meaning}")


def _render(arguments: argparse.Namespace) -> None:
    out: Path = arguments.output
    if out.suffix not in _FORMATS:
        raise _RefusalError(f"{out}: the output must end in {' or '.join(_FORMATS)}")
    drawing = _load(arguments.source, arguments.input)
    width, height = _size(arguments.size) if arguments.size else (drawing.size or _DEFAULT_SIZE)
    values = _values(arguments.values, drawing)
    if out.suffix == ".png":
        out.write_bytes(drawing.to_png(round(width), round(height), **values))  # type: ignore[arg-type]
    else:
        out.write_text(drawing.to_svg(round(width), round(height), **values), encoding="utf-8")
    print(out)


def _validate(arguments: argparse.Namespace) -> None:
    inputs = _inputs(arguments.input)
    failed = False
    for file in arguments.files:
        try:
            Drawing.from_json(Path(file).read_text(encoding="utf-8"), inputs=inputs)
        except (BullterrierVectorError, OSError, UnicodeDecodeError) as error:
            print(f"{file}: {error}", file=sys.stderr)
            failed = True
        else:
            print(f"ok {file}")
    if failed:
        raise SystemExit(1)


_COMMANDS = {"list": _list, "describe": _describe, "render": _render, "validate": _validate}


def _parsers() -> argparse.ArgumentParser:
    """The top parser, which only chooses the command, and the parser of each command, by name."""
    top = argparse.ArgumentParser(prog="bt-vector", description=__doc__.splitlines()[0])
    top.add_argument("command", choices=_COMMANDS, metavar="COMMAND", help=", ".join(_COMMANDS))
    top.add_argument("arguments", nargs=argparse.REMAINDER, help="the command's own, see `bt-vector COMMAND -h`")
    return top


def _command_parser(name: str) -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog=f"bt-vector {name}")
    if name in ("describe", "render"):
        parser.add_argument("source", metavar="SOURCE", help="a preset name or a .json document")
    if name == "render":
        parser.add_argument("-o", "--output", type=Path, required=True, help="the file to write: .png or .svg")
        parser.add_argument("--size", help="WIDTHxHEIGHT in pixels (default: the drawing's own size, else 800x600)")
        parser.add_argument("values", nargs="*", metavar="NAME=VALUE", help="a value for a Param")
    if name == "validate":
        parser.add_argument("files", nargs="+", metavar="FILE.json")
    if name != "list":
        parser.add_argument("--input", action="append", default=[], metavar="NAME=FILE", help="bytes of a named input")
    return parser


def run(argv: list[str]) -> int:
    """Run the command in `argv` (without the program name); the exit code, see the module docstring."""
    try:
        chosen = _parsers().parse_args(argv)
        # intermixed: older 3.12s reject NAME=VALUE after an option
        arguments = _command_parser(chosen.command).parse_intermixed_args(chosen.arguments)
        _COMMANDS[chosen.command](arguments)
    except SystemExit as stop:  # argparse's usage error (2) or -h (0), or `validate`, which has said its lines already
        return int(stop.code or 0)
    except (_RefusalError, BullterrierVectorError, OSError, UnicodeDecodeError) as error:
        print(f"bt-vector: {error}", file=sys.stderr)
        return 1
    return 0


def main() -> None:
    """The console script."""
    sys.exit(run(sys.argv[1:]))
