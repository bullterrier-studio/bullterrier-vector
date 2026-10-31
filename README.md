# bullterrier-vector

A typography-first vector graphics scripting library: Bezier paths, type, booleans, offsets, generative forms and
tracing, written as code and drawn as SVG or pixels. One engine, the same output from Rust, Python and the browser.

Status: the scaffold, its gates and the CI workflows exist (a Rust core, Python bindings, checks that run on every
change), and the first features: `Path`, a Bezier outline as a value that you build, measure, cut, move, hit-test and read
or write as SVG path data, and `Drawing`, a chain of steps (move, turn, scale, paint, select) that you draw again with new
values of its `Param`s to get shapes. `Drawing.to_svg` writes a drawing as an SVG document, in Python and in the WebAssembly package; pixels, PNG and a mesh are next. No release yet; the first is planned for later. The plan:
[docs/design.md](docs/design.md), [docs/roadmap.md](docs/roadmap.md).

## Install from source / Contribute

Nothing is published yet. To build the package from a clone, or to contribute, see [CONTRIBUTING.md](CONTRIBUTING.md).

Licence: [MIT](LICENSE).
