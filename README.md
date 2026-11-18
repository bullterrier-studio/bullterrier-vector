# bullterrier-vector

A typography-first vector graphics scripting library: Bezier paths, type, booleans, offsets, generative forms and
tracing, written as code and drawn as SVG or pixels. One engine, the same output from Rust, Python and the browser.

Status: the scaffold, its gates and the CI workflows exist (a Rust core, Python bindings, checks that run on every
change), and the first features: `Path`, a Bezier outline as a value that you build, measure, cut, move, hit-test and read
or write as SVG path data, and `Drawing`, a chain of steps (move, turn, scale, paint, select) that you draw again with new
values of its `Param`s to get shapes. `Drawing.to_svg` writes a drawing as an SVG document, `Drawing.to_rgba` as pixels, `Drawing.to_png` as a PNG file (`uv run poe look` draws a drawing file to look at) and `Drawing.to_mesh` as triangles for a display, from Python, Rust and, for a document loaded from JSON, from the WebAssembly package in Node (the same star, drawn from all three, is the first milestone). `btvec.text` shapes a string in a font (TrueType, OpenType or WOFF2, from bytes or one of the three that come with the library, with its variable axes and OpenType features) into one shape per glyph, in Rust and Python and, in a document, in WebAssembly; a saved drawing names its fonts as inputs. `characters` stands a character grid on cells as outlined glyphs (image's `ascii` colours plug in), and `marks` draws the brackets, crosshairs and labelled leader lines of a heads-up display. `text` also wraps lines to a width, fits each to it by size or by a variable-font axis, or sets the text inside a shape, `btvec.run` gives each stretch of a text a font, size and axes of its own (a headline of letters from three typefaces wraps and fits as one text), and `along` carries shapes, text included, along a path. Numbers can vary per shape (`random`, `stagger`, `by_index`), `split` and `merge` cut shapes into parts and join them into words, and `slice` draws an outline on, so a word draws on letter by letter and scatters by index with no text-specific modifier. `union`, `subtract`, `intersect` and `exclude` combine shapes as regions, at once on a `Path` or as steps on a `Drawing`, and `resolve` unites a shape's own overlapping contours (a glyph's, say), with a `tolerance` for the straight edges curves become; the limits are in [docs/api.md](docs/api.md). `offset` grows or shrinks a region and `outline` turns a stroke into a fill, with round, miter or bevel joins, on a `Path` or as steps on a `Drawing`. `btvec.frames` (Python) calls a function of time once a frame and writes a transparent PNG sequence, or a ProRes 4444 or WebM clip with alpha through ffmpeg. `resample`, `dash`, `simplify`, `reverse` and `place_along` reshape a path or set copies of a drawing along one without bending them, and `outline` and `stroke` take a width `profile` for tapered strokes. `wave`, `wobble`, `twist`, `bulge`, `envelope` and `arch` deform paths and bend a line of text on an arch or into an envelope (smooth to a twentieth of a pixel at any size), and `Path.arc` draws an arc. `morph` turns one outline or drawing into another by a single `fraction` (a letter into a circle, one word into a word of another length). `hatch` fills shapes with evenly spaced lines that follow a direction field (`Flow`: an angle, noise, a shape's contour, or a mix) with the spacing read from a tone (`Field`: a ramp, or the distance to a shape), keeping out of what it avoids; `examples/flow_hatch.py` hatches a headline along its own letters. `rows` draws ridgelines of a tone with hidden lines removed, `bands` lines whose width carries it (straight, waves, rings, a spiral or rays), `stipple` dots whose density follows it, `Path.within` keeps lines inside a region, and `Drawing.to_plot_svg` writes strokes only for a pen plotter, one layer per pen and ordered to cut pen-up travel (`examples/line_families.py`). `examples/kinetic_type.py` is the first piece built on it all: type, deformers and morph in a variable font, in Python, Rust and JavaScript (milestone M3). A window of a pixel output (`to_rgba(width, height, window=(x, y, w, h))`, also `to_png`) draws one rectangle of a large canvas, to render it in bands. No release yet. What it is and what shipped:
[docs/design.md](docs/design.md), [docs/roadmap.md](docs/roadmap.md).

Draw a bundled preset from the command line (Python package):

```bash
bt-vector render ring -o ring.png half_width=40
bt-vector list
```

## Try it

```bash
uv run poe playground          # builds the WebAssembly package for the browser into playground/pkg/
uv run poe playground-serve    # then open http://localhost:8000: pick a preset, move the sliders
```

## Install from source / Contribute

Nothing is published yet. To build the package from a clone, or to contribute, see [CONTRIBUTING.md](CONTRIBUTING.md).

Licence: [MIT](LICENSE).
