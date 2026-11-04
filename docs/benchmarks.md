# Benchmarks

`uv run poe bench` times `to_rgba` and `to_mesh` on one reference drawing: a star and 200 filled, stroked, translucent
and multiply shapes from a seeded generator (`tools/bench.py`), at 1920x1080. Native is the Python extension; WebAssembly
is the package built by `uv run poe wasm-test` (release, `simd128`), run in Node on the same drawing. Each call is made 3
times to warm up and 20 times to measure; the table shows the median in milliseconds, one thread. Nothing asserts on
time: the numbers are recorded here by hand, to compare later runs on the same machine.

## Results

Apple M4 (arm64, 10 cores), macOS, Node 26.11, the V3f batch, 2026-11-01. Three runs in a row; the first ran while the
machine was busy, the other two on a quiet one.

| Run | `to_rgba` native | `to_rgba` WebAssembly | `to_mesh` native | `to_mesh` WebAssembly |
|---|---|---|---|---|
| 1 (busy) | 42.9 | 42.3 | 37.8 | 32.7 |
| 2 | 17.7 | 26.2 | 18.3 | 25.2 |
| 3 | 18.0 | 26.1 | 18.1 | 24.9 |

On the quiet machine WebAssembly took about 1.4 times the native time for both outputs. For context, the spike that
chose the rasteriser measured 200 mixed shapes at 1080p, with an earlier and simpler scene, at 27.5 ms natively and about
39 ms in WebAssembly with `simd128`; the reference drawing here is the one to compare from now on. x86 and a browser were
not measured.

## Text

`uv run poe bench --text` adds the cost of type, natively only (the Python extension): the headline of
`examples/kinetic_type.py` (`NO` over `SLEEP`, 800 px to the em, a subset of Anybody with 7 glyphs of straight and
curved outlines) with its `wght` a `Param`, at a new weight on every call so that the result cache answers none. Apple
M4, macOS, the V5c batch, 2026-11-01; the median of 20 calls in milliseconds, three runs in a row on a quiet machine.

| What moves | Run 1 | Run 2 | Run 3 |
|---|---|---|---|
| `shapes`, the weight: shaping and outlines of 7 glyphs | 0.015 | 0.014 | 0.014 |
| `shapes`, a weight for each of the 8 characters | 0.021 | 0.024 | 0.024 |
| `shapes`, 998 letters of capitals at 40 px, the weight | 0.513 | 0.626 | 0.552 |
| `to_rgba` 1080x1350 (the headline filled), the weight | 2.306 | 2.143 | 2.095 |
| `to_mesh` 1080x1350, the weight | 0.545 | 0.639 | 0.579 |
| `shapes`, 100,000 letters (`MAX_GLYPHS`), each at its own weight, once, with the Python `Shape` objects | 299.8 | 289.2 | 290.9 |

Moving an axis costs about 2 microseconds a glyph for shaping and outlines (0.5 ms for a page of 1,000 letters), far
below the 16.7 ms of a frame at 60 Hz; the frame is the pixels, or the mesh. The 100,000-distinct-location text, the
case V5b left unmeasured, draws all its outlines in 0.3 s and holds the limit of `MAX_GLYPHS`: its cost per letter
(3 microseconds) is that of the others, so each location is not much dearer than a repeat. Not measured: WebAssembly
for text, glyphs with many curves per outline (the subset's are simple), and a font of 1,000s of glyphs (the lookup is by
glyph index).

### Wrap and fit (V6)

`uv run poe bench --text` also times, as `shapes` with the weight moving, the headline fitted to a width by `wdth` (800 px
to the em, `width` 980, `leading` 640) and the page of capitals wrapped to a width of 1000 px. Figures reported by the V6
implementers, not re-run for this table (run count and conditions not recorded here): the fitted headline about 0.16 ms,
the wrapped page of 998 letters about 0.56 ms (the unwrapped page: 0.51 to 0.63 ms above), and the 100,000-letter text of
the last row above, about 255 ms (V5c: 289 to 300 ms). Each call has a new weight, so the result cache answers none and the
wrap and the fit are planned again every call. Not measured: `inside` and `along`, WebAssembly.
