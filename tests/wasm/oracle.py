"""What the native extension answers, written as JSON for the Node tests of the WebAssembly package.

`poe wasm-test` runs this beside the build: the Node tests (`smoke.test.mjs`) call the WebAssembly package with the same
documents, sizes and values and expect exactly these results. The two are different binaries built from one core, so a
difference is a defect in the glue, the target or the build. Usage: `python tests/wasm/oracle.py <output file>`.
"""

import json
import sys
from pathlib import Path
from typing import Any

import bullterrier_vector as btvec

ROOT = Path(__file__).resolve().parents[2]
BARS = (ROOT / "docs" / "examples" / "json" / "series_bars.json").read_text(encoding="utf-8")


def star() -> btvec.Drawing:
    """The star of examples/star.py: filled and stroked outline, a dot on the tip, a tangent line."""
    outline = btvec.Path.star((200, 200), outer=150, inner=60, points=5)
    tip = outline.point_at(0)
    halfway = outline.point_at(0.5)
    return btvec.Drawing(
        outline.fill("#ffcc33").stroke("#1a1a1a", width=4),
        btvec.Path.circle(tip, 8).fill("#e2412e"),
        btvec.Path.line(halfway, halfway + 40 * outline.tangent_at(0.5)).stroke("#e2412e"),
        size=(400, 400),
    )


def lifted() -> btvec.Drawing:
    """Two circles, both moved down by the Param `lift`: a number for both or a list with a number each."""
    lift = btvec.Param("lift", default=5, min=0, max=100, unit="px", meaning="how far the circles sink")
    return btvec.Drawing(
        btvec.Path.circle((30, 30), 10).fill("#336699"),
        btvec.Path.circle((70, 30), 10).fill("#993366"),
        size=(100, 100),
    ).translate(0, lift)


def parameter(param: btvec.Param) -> dict[str, Any]:
    """A Param as the plain object the package describes it with."""
    return {
        "name": param.name,
        "default": param.default,
        "min": param.min,
        "max": param.max,
        "unit": param.unit,
        "meaning": param.meaning,
    }


def drawn(text: str, width: int, height: int, **values: float | list[float]) -> dict[str, Any]:
    """One `toSvg` call and the text the native extension writes for it."""
    svg = btvec.Drawing.from_json(text).to_svg(width, height, **values)
    return {"width": width, "height": height, "values": values, "svg": svg}


def document(text: str, draws: list[dict[str, Any]]) -> dict[str, Any]:
    """A document, what the native extension reads from it, and its drawings."""
    drawing = btvec.Drawing.from_json(text)
    return {
        "json": text,
        "canonical": drawing.to_json(),
        "size": drawing.size,
        "parameters": [parameter(param) for param in drawing.parameters()],
        "draws": draws,
    }


def refused(call: str, text: str, width: float = 10, height: float = 10, **values: float) -> dict[str, Any]:
    """A call the core refuses: the exception class and message the native extension raises for it."""
    try:
        drawing = btvec.Drawing.from_json(text)
        if call == "toSvg":
            drawing.to_svg(width, height, **values)
    except btvec.BullterrierVectorError as error:
        return {
            "call": call,
            "json": text,
            "width": width,
            "height": height,
            "values": values,
            "name": type(error).__name__,
            "message": str(error),
        }
    sys.exit(f"oracle: {call} did not raise for {text[:40]!r}")


def main(output: Path) -> None:
    """Write the answers to `output`."""
    star_json = star().to_json()
    lifted_json = lifted().to_json()
    answers = {
        "documents": {
            "series_bars": document(BARS, [drawn(BARS, 1080, 1080), drawn(BARS, 540, 270)]),
            "star": document(star_json, [drawn(star_json, 400, 400), drawn(star_json, 800, 300)]),
            "lifted": document(
                lifted_json,
                [
                    drawn(lifted_json, 100, 100),
                    drawn(lifted_json, 200, 200, lift=30),
                    drawn(lifted_json, 100, 100, lift=[0, 40]),
                ],
            ),
        },
        "refusals": [
            refused("fromJson", "not a document"),
            refused("fromJson", '{"version": 1, "drawings": {"a": [{"nope": 1}]}}'),
            refused("toSvg", star_json, 0, 10),
            refused("toSvg", star_json, 10, 1080.5),
            refused("toSvg", lifted_json, 100, 100, nope=1),
            refused("toSvg", lifted_json, 100, 100, lift=500),
        ],
    }
    output.write_text(json.dumps(answers), encoding="utf-8")


if __name__ == "__main__":
    main(Path(sys.argv[1]))
