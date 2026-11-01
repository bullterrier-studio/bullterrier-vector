# Benchmarks

`uv run poe bench` times `to_rgba` and `to_mesh` on one reference drawing: a star and 200 filled, stroked, translucent
and multiply shapes from a seeded generator (`tools/bench.py`), at 1920x1080. Native is the Python extension; WebAssembly
is the package built by `uv run poe wasm-test` (release, `simd128`), run in Node on the same drawing. Each call is made 3
times to warm up and 20 times to measure; the table shows the median in milliseconds, one thread. Nothing asserts on
time: the numbers are recorded here by hand, to compare later runs on the same machine.

## Results

Apple M4 (arm64, 10 cores), macOS, Node 26.11, commit `3579859`, 2026-11-01. Three runs in a row; the first ran while the
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
