# bullterrier-vector

A typography-first vector graphics scripting library: Bezier paths, type, booleans, offsets, generative forms and
tracing, written as code and drawn as SVG or pixels. One engine, the same output from Rust, Python and the browser.

Status: the scaffold, its gates and the CI workflows exist (a Rust core, Python bindings, checks that run on every
change), and the first features: `Path`, a Bezier outline as a value that you build, measure, cut, move, hit-test and read
or write as SVG path data, and `Drawing`, a chain of steps (move, turn, scale, paint, select) that you draw again with new
values of its `Param`s to get shapes. `Drawing.to_svg` writes a drawing as an SVG document, `Drawing.to_rgba` as pixels, `Drawing.to_png` as a PNG file (`uv run poe look` draws a drawing file to look at) and `Drawing.to_mesh` as triangles for a display, from Python, Rust and, for a document loaded from JSON, from the WebAssembly package in Node (the same star, drawn from all three, is the first milestone). `btvec.text` shapes a string in a font (TrueType, OpenType or WOFF2, from bytes or one of the two that come with the library, with its variable axes and OpenType features) into one shape per glyph, in Rust and Python and, in a document, in WebAssembly; a saved drawing names its fonts as inputs. `text` also wraps lines to a width, fits each to it by size or by a variable-font axis, or sets the text inside a shape, and `along` carries shapes, text included, along a path. Numbers can vary per shape (`random`, `stagger`, `by_index`), `split` and `merge` cut shapes into parts and join them into words, and `slice` draws an outline on, so a word draws on letter by letter and scatters by index with no text-specific modifier. `union`, `subtract`, `intersect` and `exclude` combine shapes as regions, at once on a `Path` or as steps on a `Drawing`, and `resolve` unites a shape's own overlapping contours (a glyph's, say), with a `tolerance` for the straight edges curves become; the limits are in [docs/api.md](docs/api.md). `btvec.frames` (Python) calls a function of time once a frame and writes a transparent PNG sequence, or a ProRes 4444 or WebM clip with alpha through ffmpeg. No release yet; the first is planned for later. The plan:
[docs/design.md](docs/design.md), [docs/roadmap.md](docs/roadmap.md).

## Install from source / Contribute

Nothing is published yet. To build the package from a clone, or to contribute, see [CONTRIBUTING.md](CONTRIBUTING.md).

Licence: [MIT](LICENSE).
