# The code API

Status: V1a, designed from the example programs in [examples/](examples/); waiting for the owner's review. Nothing here is
built yet ([roadmap.md](roadmap.md)). Reasons: [design-decisions.md](design-decisions.md), "The code API". The JSON form
of a drawing is V1b.

Signatures are Python's; "Languages" gives the Rust and JavaScript forms. The library is imported under the alias
`btvec` in every language.

```python
import bullterrier_vector as btvec
```

## Examples

| Program | Shows |
|---|---|
| [kinetic_type.py](examples/kinetic_type.py), [.rs](examples/kinetic_type.rs), [.js](examples/kinetic_type.js) | reference piece 1: variable-font axes, per-letter stagger, a caller's signal per letter, riso inks, a clip |
| [cut_paper.py](examples/cut_paper.py) | reference piece 2: a traced mask, offset layers, shadows, a draw-on outline, a sequence of maps |
| [hatched_poster.py](examples/hatched_poster.py) | reference piece 3: flow-field hatching by tone, rings of a distance field, a plotter file |
| [star.py](examples/star.py) | building and measuring a `Path` |
| [ring.py](examples/ring.py) | offsets and a boolean, eager on a `Path` and as steps on a `Drawing` |
| [text_on_path.py](examples/text_on_path.py) | text bent along a path, moving round it |
| [draw_on.py](examples/draw_on.py) | draw-on by `progress`, staggered letter by letter |
| [marching_dashes.py](examples/marching_dashes.py) | a dash pattern with a moving phase |
| [wobble_outline.py](examples/wobble_outline.py) | a small noise wobble; a font axis as a described parameter |
| [glyph_scatter.py](examples/glyph_scatter.py) | letters split into parts, scattered by seeded random numbers |
| [radial_repeat.py](examples/radial_repeat.py) | a radial repeater; a number per copy |
| [halftone.py](examples/halftone.py) | halftone dots from a tone map |
| [duotone.py](examples/duotone.py) | a gradient map and posterize on colours |
| [series_bars.py](examples/series_bars.py) | a series of numbers as bars, with plain code |
| [frame_export.py](examples/frame_export.py) | a transparent PNG sequence and a clip from a function of time |
| [mesh_upload.js](examples/mesh_upload.js) | a mesh in WebAssembly memory uploaded to WebGL, the pointer pattern |

## Four ideas

**`Path`** is a Bezier outline: one or more sub-paths, each open or closed. It is a plain value. Every verb on it
computes at once and returns a new `Path`; nothing changes the old one. This is the scripting layer: build, measure,
cut, combine and offset geometry the way Paper.js does, with callbacks where they help.

**`Shape`** is what a drawing is made of: one `Path` with a style (fill, stroke, opacity, blend mode, shadow, knockout)
and attributes (its index, and what its generator wrote: glyph, character, word, line, copy, level, position 0..1, the
origin it turns about). A shape is read, never built by hand: `drawing.shapes()` evaluates a drawing into a list of
them. A list of shapes is a plain list; there is no collection type.

**`Drawing`** is an immutable tree of steps that evaluates to a list of shapes. Every call on a drawing returns a new
drawing with one more step; the old one is unchanged and can be used again, in several places. A drawing holds no
callbacks, so it draws again with new values, saves to JSON, and runs in a page or on a server unchanged.

**`Param`** is a named number with a unit, a range, a default and a meaning. Any number in a step may be a `Param`, an
expression on Params (`1 + 0.08 * pulse`), or a per-item number (`btvec.by_index(0, 1)`); its value is given when the
drawing is drawn (`drawing.to_png(1080, 1350, weight=820)`). `drawing.parameters()` lists them, so a tool can build a
slider or map a signal onto one without knowing the steps.

The geometry verbs (`translate`, `offset`, `subtract`, `slice`, `wobble`...) have the same names, arguments and meaning
on a `Path`, where they compute now, and on a `Drawing`, where they add a step that computes the same thing for every
shape at draw time. One implementation serves both. Style verbs exist only on `Drawing`; called on a `Path` they return
a `Drawing` of that path (`star.fill("#ffcc33")`). Three inputs feed steps: a `Font`, a `Field` (a number at every
point) and a `Flow` (a direction at every point).

## Path

Points are pairs: `(x, y)` tuples go in, `Point`s come out (`p.x`, `p.y`; `+` and `-` between points, `*` by a
number). `y` points down. Positions along a path are fractions 0..1 of its length (by arc length, not the curve
parameter); a closed sub-path is measured from its topmost point (the leftmost of a tie), in its own direction, so the
start does not move when the outline changes. A path with several sub-paths is measured through them in order. The
closed constructors below start at the top and run clockwise.

Building returns a new path each call:

| Method | Signature | Promise | Errors |
|---|---|---|---|
| `Path` | `Path()` | an empty path | |
| `move_to` | `move_to(point)` | starts a new sub-path at `point` | |
| `line_to` | `line_to(point)` | a straight segment to `point` | `ParameterError` before any `move_to` |
| `curve_to` | `curve_to(control1, control2, point)` | a cubic Bezier segment to `point` | as `line_to` |
| `quad_to` | `quad_to(control, point)` | a quadratic Bezier segment to `point` | as `line_to` |
| `close` | `close()` | closes the open sub-path with a segment back to its start | as `line_to` |
| `Path.line` | `Path.line(start, end)` | one open segment | |
| `Path.polyline` | `Path.polyline(points, *, closed=False)` | straight segments through `points` | fewer than 2 points |
| `Path.rect` | `Path.rect(x, y, width, height, *, radius=0)` | a closed rectangle, corners rounded by `radius` | a negative size; `radius` over half the shorter side |
| `Path.circle` | `Path.circle(center, radius)` | a closed circle | `radius` not above 0 |
| `Path.polygon` | `Path.polygon(center, radius, *, sides=6)` | a regular polygon, a corner at the top | `sides` below 3 |
| `Path.star` | `Path.star(center, outer, inner, *, points=5)` | a star, a tip at the top | `points` below 2; a radius not above 0 |
| `Path.from_svg_path` | `Path.from_svg_path(data)` | the path an SVG `d` attribute describes | `data` malformed (names the first bad command) |

Measuring and hit-testing (on `Path` only):

| Method | Signature | Promise | Errors |
|---|---|---|---|
| `length` | property | the length in design pixels | |
| `bounds` | property | the smallest `Rect(x, y, width, height)` around the outline | `ParameterError` on an empty path |
| `point_at` | `point_at(at)` | the point a fraction `at` of the way along | `at` outside 0..1; an empty path |
| `tangent_at` | `tangent_at(at)` | the unit direction of travel there | as `point_at` |
| `locate` | `locate(point)` | the position 0..1 of the point on the path nearest `point` | an empty path |
| `contains` | `contains(point)` | whether `point` is inside the filled region (nonzero rule) | |
| `split` | `split()` | its parts: each closed outline with the holes inside it, and each open line | |
| `map_points` | `map_points(fn)` | the path with every point, on and off the curve, passed through `fn(point) -> point` | what `fn` raises |
| `to_svg_path` | `to_svg_path()` | SVG path data for the path | |

`map_points` is the one verb that takes a callback, so it has no step on `Drawing`.

## Verbs on Path and Drawing

Each verb below exists on both. On a `Path` its numbers are plain numbers and it returns a `Path`; on a `Drawing` its
numbers may be Params or per-item numbers and it applies to every shape. A `Path` verb given a `Param` raises
`ParameterError`: wrap the path in `btvec.Drawing(path)` first. `about` defaults to each shape's origin (a glyph's
cap-height centre; for any other shape, the centre of its bounds).

| Verb | Signature | Promise | Errors |
|---|---|---|---|
| `translate` | `translate(dx, dy)` | moves by `dx`, `dy` | |
| `rotate` | `rotate(degrees, *, about=None)` | turns clockwise about `about` | |
| `scale` | `scale(by, *, about=None)` | scales by `by`, or by `(x, y)` separately, about `about` | |
| `union` | `union(other)` | the region inside either | `GeometryError` (below) |
| `subtract` | `subtract(other)` | the region inside this and outside `other` | `GeometryError` |
| `intersect` | `intersect(other)` | the region inside both | `GeometryError` |
| `exclude` | `exclude(other)` | the region inside exactly one | `GeometryError` |
| `offset` | `offset(distance, *, join="round")` | the outline moved outward by `distance` (inward when negative); joins `round`, `miter`, `bevel` | `GeometryError` |
| `outline` | `outline(width, *, join="round", cap="round")` | the region a stroke of `width` paints, as a closed outline; caps `butt`, `round`, `square` | `width` not above 0 |
| `drop_parts` | `drop_parts(*, smaller_than)` | without the parts whose area is below `smaller_than` square pixels | |
| `fill_holes` | `fill_holes(*, smaller_than)` | with the holes whose area is below `smaller_than` filled | |
| `slice` | `slice(start, end)` | the piece between two positions along it (draw-on: `slice(0, progress)`) | a position outside 0..1 |
| `reverse` | `reverse()` | the same outline travelled the other way | |
| `dash` | `dash(pattern, *, phase=0)` | the pieces a dash pattern (`[on, off, ...]` in pixels) leaves; `phase` in cycles of the pattern, wrapping | an empty pattern; a length not above 0 |
| `resample` | `resample(spacing)` | straight segments with corners `spacing` pixels apart along it | `spacing` not above 0 |
| `simplify` | `simplify(tolerance=0.5)` | the fewest smooth curves (sharp corners kept) within `tolerance` pixels; fits curves through a dense polyline | `tolerance` not above 0 |
| `along` | `along(path, *, start=0)` | bent along `path`: a point `x` pixels along the baseline `y = 0` lands `x` pixels past `start` on `path` and `y` pixels to the right of its direction of travel (below the baseline stays below); `start` wraps on a closed path | |
| `wobble` | `wobble(amplitude=4, *, wavelength=80, phase=0, seed=0)` | points displaced by smooth seeded noise of that amplitude and wavelength (pixels); `phase` in cycles moves it | `wavelength` not above 0 |
| `wave` | `wave(amplitude=8, *, wavelength=120, angle=0, phase=0)` | points displaced sideways by a sine travelling towards `angle` | `wavelength` not above 0 |
| `morph` | `morph(target, progress)` | the outline `progress` 0..1 of the way to `target`, points matched when their counts differ; on a `Drawing`, shapes pair by index | on a `Drawing`, different shape counts |
| `hatch` | `hatch(*, spacing=8, tone=None, flow=None, avoid=None)` | each region filled with evenly spaced open lines (below) | below |

Booleans and offsets hold on overlapping, rotated glyphs (their outlines are made valid regions when set). Where an
outline cannot be resolved into a region, the verb raises `GeometryError` naming the verb and the shape's index; V8
documents the cases. On a `Drawing`, `other` is a `Drawing` or a `Path`; each shape is combined with the union of
`other`'s shapes. Deformers resample first, so they stay smooth at any size; V12 adds `twist` and `bulge` in the same
form.

Closing and opening are two offsets: `offset(r).offset(-r)` fills gaps and dents narrower than `2r` (a blobby mount);
`offset(-r).offset(r)` removes spurs thinner than `2r`.

### hatch

Evenly spaced lines (the Jobard-Lefer method) following `flow` inside each region: no two closer than the spacing, each
line as long as the spacing allows, closed where a line meets its own start.

| Argument | Meaning |
|---|---|
| `spacing` | pixels between lines; or a pair `(at tone 0, at tone 1)`, read from `tone` |
| `tone` | a `Field` or a number; needed when `spacing` is a pair |
| `flow` | a `Flow`; default `Flow.angle(45)` (straight hatching) |
| `avoid` | a `Path` or `Drawing` the lines keep clear of |

The result is open lines, so set the stroke after it. Errors: `ParameterError` for a spacing not above 0, or a pair
without `tone`.

## Drawing

`Drawing(*items, size=None)`: the items (paths, shapes, drawings) stacked in order, the first at the bottom. `size` is
the design frame `(width, height)` in design pixels; outputs scale it to their size. Without one, a drawing has the
size of its first item that has one; with none at all, design pixels are output pixels.

### Items

| Step | Signature | Promise | Errors |
|---|---|---|---|
| `split` | `split()` | every shape split into its parts, each keeping its attributes, with `part` added and its own bounds centre as origin | |
| `merge` | `merge(*, by=None)` | shapes with an equal attribute `by` (all shapes, if `None`) united into one, with the first's attributes | `by` that no shape carries |
| `only` | `only(**attributes)` | the shapes whose attributes equal these (a list means any of) | an attribute no shape carries |
| `without` | `without(**attributes)` | the other shapes | as `only` |
| `repeat` | `repeat(count, *, translate=(0, 0), rotate=0, scale=1, about=None)` | `count` copies, copy `i` transformed `i` times; writes `copy` and `position` | `count` below 1 |

`index` is every shape's place in the current list. Generators write the rest: `text` writes `glyph`, `char`, `word`,
`line`, `position` and a typographic `origin`; `repeat` writes `copy`; `trace` writes `level`; `Drawing(...)` writes
`position` by order. `position` runs 0..1 over the shapes a generator made and survives `split` and `only`; `index`
counts the current list.

### Style

| Step | Signature | Promise | Errors |
|---|---|---|---|
| `fill` | `fill(color)` | fills each shape; `"none"` for no fill | an unreadable colour |
| `stroke` | `stroke(color, *, width=2, join="round", cap="round")` | strokes each outline | as `fill`; `width` not above 0 |
| `halftone` | `halftone(color, *, tone, pitch=8, angle=45)` | fills each shape with a dot screen: dots `pitch` pixels apart on a grid at `angle`, each covering what `tone` asks (0 solid, 1 none) | as `fill`; `pitch` not above 0 |
| `opacity` | `opacity(alpha)` | multiplies opacity by `alpha` 0..1 | |
| `blend` | `blend(mode)` | paints with `"normal"` or `"multiply"` over what is below | another mode |
| `shadow` | `shadow(*, dx=0, dy=8, blur=16, color="#00000059")` | a drop shadow under each shape, offset and blurred in pixels | `blur` below 0 |
| `knockout` | `knockout()` | each shape removes what is below it in its drawing, instead of painting | |
| `gradient_map` | `gradient_map(colors)` | every fill and stroke colour replaced by its lightness read along `colors`, dark to light (two colours: a duotone) | fewer than 2 colours |
| `posterize` | `posterize(levels)` | every fill and stroke colour reduced to `levels` levels a channel | `levels` below 2 |

Colours are `"#rrggbb"` or `"#rrggbbaa"`. Until a fill or a stroke is set, a closed outline is filled black and an open
line stroked black at 2 pixels; once either is set, only what is set is painted. `knockout` is the style form of a cut:
it keeps both shapes and cuts through everything below, shadows included; `subtract` makes new geometry (for a plotter,
or for further verbs). Every style reaches SVG and pixels alike.

### Values and evaluation

| Method | Signature | Promise | Errors |
|---|---|---|---|
| `with_values` | `with_values(**values)` | the same drawing with these Params fixed; they leave `parameters()` | as outputs (below) |
| `parameters` | `parameters()` | every `Param` the drawing reads, in the order first used | two different Params with one name |
| `shapes` | `shapes(**values)` | the evaluated shapes, in design pixels | as outputs |
| `size` | property | the design frame, or `None` | |

## Generators

Functions of the module that return a `Drawing`.

### text

`text(content, *, font, size=100, axes={}, features={}, tracking=0, leading=None, align="left", at=(0, 0), width=None,
fit=None, inside=None)`: one shape per drawn glyph (a space or a line break gives none), shaped with kerning,
ligatures and complex scripts, each outline a valid region (overlapping contours removed).

| Argument | Meaning |
|---|---|
| `content` | the text; `"\n"` breaks a line |
| `font` | a `Font` |
| `size` | pixels to the em |
| `axes` | variable axis values by tag (`{"wght": weight}`); a per-item number gives each glyph its own, and advances follow |
| `features` | OpenType features by tag (`{"ss01": 1, "liga": 0}`) |
| `tracking` | extra space between glyphs, in thousandths of an em |
| `leading` | pixels between baselines; default 1.2 × `size` |
| `align` | `"left"`, `"center"` or `"right"` |
| `at` | where the first baseline meets the alignment: its left end, middle or right end |
| `width` | lines wrap to this width; with `fit`, each line is made exactly this wide |
| `fit` | an axis tag or `"size"`: solved per line so the line is `width` wide, once, at the reference pose (every Param at its default), so live values may overflow instead of re-fitting |
| `inside` | a `Path`: lines wrap to its width at each baseline (V6) |

Errors (`ParameterError`): an axis or feature the font lacks (the message lists the font's), an axis value outside the
font's range, `fit` without `width`, `fit` on an axis also given in `axes`, and a line `fit` cannot make `width` wide
(the message names the line and the nearest width the axis reaches).

### trace

`trace(field, *, levels=(0.5,), tolerance=1, before=(), after=(), sigma=1.2)`: one shape per level, outlining where
`field` is at or above it, with curves fitted within `tolerance` pixels. The edge of the field is padded, so a subject
cut by the frame gets no false edge. Writes `level` and `position`.

For a sequence of maps (one per frame), pass the neighbouring maps in sequence order: `before` (up to this one) and
`after` (from the next). The edge is then averaged over them with a Gaussian of `sigma` maps, and a hole or a part shows
only if a neighbour has it too. The library keeps no sequence.

Errors (`ParameterError`): levels not increasing, a neighbour of a different size or box, `tolerance` or `sigma` not
above 0.

The same verb draws contour lines: trace a `Field.distance` at several levels and stroke the result.

## Inputs

| Input | Signature | Promise | Errors |
|---|---|---|---|
| `Font` | `Font(data)` | a TrueType or OpenType font, variable or not, from bytes | `data` not a font the library reads |
| `font.axes` | property | its variable axes by tag, each a `Param` with the font's range, default and name | |
| `font.features` | property | the OpenType feature tags it has | |
| `Field.map` | `Field.map(width, height, data, *, box=None)` | an 8-bit grayscale map, values 0 (black) to 1 (white), covering `box` `(x, y, width, height)` (default: one pixel a map pixel from the origin); beyond it, the nearest edge value | `len(data)` not `width × height` |
| `Field.ramp` | `Field.ramp(start, end)` | 0 at `start`, 1 at `end`, linear along the line between, held beyond | equal points |
| `Field.distance` | `Field.distance(shapes, *, normalized=False)` | signed distance to the nearest outline in pixels, negative inside; normalised: inside each shape 0 on its outer edge to 1 on its holes' edges (its centre line when it has none), 0 outside | |
| `Flow.angle` | `Flow.angle(degrees)` | the same direction everywhere | |
| `Flow.noise` | `Flow.noise(*, wavelength=400, phase=0, seed=0)` | smooth seeded noise directions; `phase` in cycles moves it | `wavelength` not above 0 |
| `Flow.along` | `Flow.along(shapes)` | the direction of the nearest outline | |
| `flow.mix` | `mix(other, fraction)` | `fraction` 0..1 of the way to `other`; directions without sign blend in doubled-angle space | `fraction` outside 0..1 |

A field or a flow may read Params (a noise `phase`) and is part of the drawing that uses it.

## Params, expressions and per-item numbers

`Param(name, *, default, min, max, unit="", meaning="")`.

| Field | Meaning |
|---|---|
| `name` | letters, digits and `_`, starting with a letter; unique in a drawing |
| `default` | the value when none is given, and the reference pose |
| `min`, `max` | the range a given value must lie in |
| `unit` | `"px"`, `"deg"`, `"cycle"`, `"fraction"` or `""` (a plain number: a font axis, a count, a seed) |
| `meaning` | one line a person reads beside a slider |

A Param in `deg` or `cycle` is cyclic: any value is accepted and wraps into its range, which spans one turn. Errors
(`ParameterError`): a bad name, `min` not below `max`, a default outside the range, a cyclic range other than one turn.

Arithmetic (`+`, `-`, `*`, `/`, unary `-`) on Params and numbers builds an `Expr`, which saves with the drawing. A
`Param` is an `Expr`. Every argument of a step that takes a number takes an `Expr`. An expression whose value leaves
the range its argument accepts raises `ParameterError` at draw time, naming the step, the argument and the value.

A Param's value at draw time is a number, or a list with one number per shape (`pulse=[...]`): the way to give each
item a value from the caller's own signal. A list whose length differs from the number of shapes at a step that reads
it raises `ParameterError` naming both.

Per-item numbers take a value per shape, from its attributes:

| Function | Signature | Promise | Errors |
|---|---|---|---|
| `by_index` | `by_index(first, last)` | `first` on the first shape of the current list, `last` on the last, linear between | |
| `by_position` | `by_position(first, last)` | the same, read from each shape's `position` | |
| `random` | `random(low, high, *, seed=0)` | a number in `low..high` per shape, from `seed` and its index | `seed` not a whole number |
| `stagger` | `stagger(progress, *, spread=0.5)` | each shape's own progress 0..1: the shape at position `p` runs while `progress` goes from `p × spread` to `p × spread + 1 - spread` (0: all together; towards 1: one after another) | `spread` outside 0..1 or equal to 1 |

`btvec.ease` holds pure functions of a number: `in_cubic`, `out_cubic`, `in_out_cubic` and `out_back` (overshooting to
about 1.1). Each maps 0..1 to 0..1 and holds its end values outside 0..1. Given a number they return a number (for a
caller's function of time); given an `Expr` they return an `Expr` (inside a drawing).

## Outputs

`width` and `height` are output pixels; the design frame scales to fit them, centred. `**values` give Params their
values by name; a Param not given takes its default.

| Method | Signature | Promise |
|---|---|---|
| `to_svg` | `to_svg(width, height, /, **values)` | an SVG document of that size; shadows, halftone, knockouts and multiply kept, so it shows what the pixels show |
| `to_png` | `to_png(width, height, /, **values)` | PNG bytes, 8-bit RGBA, transparent where nothing is drawn |
| `to_rgba` | `to_rgba(width, height, /, **values)` | `width × height × 4` bytes, rows top to bottom, straight alpha |
| `to_plotter_svg` | `to_plotter_svg(width_mm, height_mm, /, *, pen_mm=0.5, **values)` | strokes only, one layer per stroke colour (one pen each), fills hatched at `pen_mm`, lines ordered to cut pen-up travel (nearest neighbour, ends flipped) and simplified below the pen's resolution; shadows, opacity and blend modes have no pen form and are left out |
| `to_mesh` | `to_mesh(width, height, /, *, into=None, **values)` | triangles for the picture (below) |
| `to_json` | `to_json()` | the drawing as JSON: the same tree written down |
| `Drawing.from_json` | `Drawing.from_json(text)` | the drawing back; it draws to identical bytes |

The same drawing and values give the same bytes on every target. Errors (`ParameterError`): a size below 1, a value for
no Param (the message lists the Params), a value outside its Param's range, a list of the wrong length; and whatever a
step raises. A Param may not be named `into` or `pen_mm`. How fonts and maps are written into JSON is V1b's question.

### Mesh

A `Mesh` holds two buffers, not objects. `vertices`: 8 `float32` a vertex, `x, y` (output pixels, y down), `r, g, b, a`
(premultiplied), `item` (the shape's index) and `position` (its `position` attribute), for a shader's own per-item
effects. `indices`: `uint32`, three a triangle, in paint order. `generation`: a number that changes exactly when the
contents change, so a redraw with the same values uploads nothing.

`into=mesh` refills that mesh in place of a new one (its buffers are reused). In Python, `numpy.asarray(mesh.vertices)`
is a view without a copy; refilling a mesh while a view of it is alive raises `ParameterError` naming `into`. In
JavaScript, `mesh.verticesPtr` and `mesh.indicesPtr` (byte offsets) and `mesh.verticesLength` and `mesh.indicesLength`
(in elements) locate the buffers in `btvec.memory`; make the typed-array views fresh after every call, since a pointer
can move and the memory can grow
([mesh_upload.js](examples/mesh_upload.js)).

The mesh is a display format; pixels and SVG stay the reference. A step with no triangle form (a blurred shadow, a
knockout, multiply) raises `ParameterError` naming it (question 4).

### Frame export

`btvec.frames(scene, *, width, height, fps=30, seconds, out)`: calls `scene(t)` (a function of seconds returning a
`Drawing`, usually `drawing.with_values(...)`) for `round(seconds × fps)` frames at `t = i / fps`, and writes each frame
exactly as `to_png` draws it. `out` names the result: a `.png` path with a run of `#` (`"frames/badge-####.png"`) for a
numbered, transparent PNG sequence; `.mov` for ProRes 4444 with alpha; `.webm` for VP9 with alpha. A clip needs ffmpeg
on the `PATH`. It is the only part of the library that knows time.

Errors: `ParameterError` for `fps` or `seconds` not above 0, an `out` of no known kind, a `.png` without `#`; what
`scene` raises; `OSError` when a file cannot be written or ffmpeg is missing or fails (with its message).

## Errors

| Error | When | Carries |
|---|---|---|
| `BullterrierVectorError` | the base of every error the library raises for a caller's input | |
| `ParameterError` | a value outside what is accepted; also a `ValueError` | the parameter's name and the value |
| `GeometryError` | geometry that fails: a boolean or offset on an outline that cannot be resolved | the verb and the shape's index |

Plain numbers are checked when a call is made; Params, expressions and lists when the drawing is drawn; geometry when it
is computed. In Python and JavaScript the error points at the call that caused it; in Rust, a chain keeps the first
error and returns it from the first draw ("Languages").

## Conventions

- **Names in the field's own terms.** `amplitude`, `wavelength`, `pitch`, `spacing`, `tracking`, `leading`, `phase`,
  `seed`, `tolerance`; never `strength`, `intensity`, `factor`, `amount`, `val` or `k`. One name per concept: a
  position along a path is `at` (or `start`, `end`), a pivot is `about`, a colour argument is `color`.
- **Units.** Design pixels for lengths, scaled with the output, so no caller ever scales. Degrees, clockwise, for
  angles, wrapping. Cycles for phases, wrapping. Fractions 0..1 for positions along a path and for progress. Thousandths
  of an em for tracking; font axes in the font's own units; areas in square pixels; millimetres only in the plotter
  output. A number is a state, never a duration.
- **Arguments.** What a call cannot do without is positional (`offset(8)`, `rotate(30)`); every option is
  keyword-only.
- **Defaults look good** with no tuning: a default comes from a gallery piece, not from a round number.
- **Reading as a sentence.** `plane.subtract(letters)`, `figure.drop_parts(smaller_than=1500)`,
  `letters.only(char="O")`, `petal.repeat(12, rotate=30)`. Related verbs sit together in the tables above.
- **How a method is documented.** One line that states the promise in the field's terms; each argument with its unit,
  range and default; each error it raises and when. Never how it works.

## What this API deliberately does not have

- **No callbacks inside a drawing.** A drawing must save to JSON and draw where no caller's code runs. A value that
  depends on the caller's signal is a list given at draw time. (`Path.map_points` takes one, and computes at once.)
- **No mutation.** Every call returns a new value; a drawing used in two places is the same drawing in both.
- **No timeline, clock or easing over time.** A Param is a state; whoever animates it sets it and draws again. Only
  `frames` loops over time, calling the caller's function.
- **No global state.** No current style, canvas, font or random generator; randomness comes from a `seed`.
- **No implicit IO.** Fonts and maps arrive as bytes; only `frames` writes files, to the path it is given.
- **No configuration objects.** Options are keyword arguments of the step they shape.
- **No collection type.** A list of shapes is a list; a drawing is the only container.

## Languages

The same names in every language, in each language's case: `snake_case` in Python and Rust, `camelCase` in JavaScript
(`toPng`, `byIndex`, `outCubic`).

| | Python | Rust | JavaScript |
|---|---|---|---|
| Constructors | `btvec.Drawing(a, b, size=(w, h))`, `btvec.Font(data)`, `btvec.Param("rise", default=0, min=0, max=1)` | `Drawing::new([a, b]).with_size(w, h)`, `Drawing::from(path)`, `Font::new(data)?`, `Param::new("rise", 0.0, 0.0..=1.0)?.unit(Unit::Fraction)` | `new btvec.Drawing([a, b], { size: [w, h] })`, `new btvec.Font(data)`, `new btvec.Param("rise", { default: 0, min: 0, max: 1 })` |
| Options | keyword arguments | a value named after the step, built with `new` and setters (`Text::new(&font).size(560.0)`); the required argument alone converts into it (`.rotate(Deg(30.0))`) | an options object last (`{ width: 1.5 }`) |
| Expressions | operators: `1 + 0.08 * pulse` | operators on `&Param` and `Expr`: `1.0 + 0.08 * &pulse` | methods: `pulse.mul(0.08).add(1)`, `rise.neg().add(1)` |
| Draw-time values | keyword arguments: `to_png(1080, 1350, weight=820)` | `&Values::new().set("weight", 820.0)` | an object: `toPng(1080, 1350, { weight: 820 })` |
| Units as types | | `Deg(..)` for angles; `Size::px(..)` and `Size::mm(..)` for output sizes | |
| Colours | `"#ff4f9a"` | `Color::hex(0xff4f9a)`, `Color::NONE` | `"#ff4f9a"` |
| Errors | exceptions | `Result<_, btvec::Error>`; a chain keeps the first error and returns it from the first draw, `to_json` or `parameters`, so no `?` per step | exceptions with `kind` |
| Bytes in | `bytes` | `Vec<u8>` | `Uint8Array` |
| Frame export | `btvec.frames` | the caller's loop over `to_png` | the caller's loop over `toRgba` or `toMesh` |

The kinetic type piece, the part that builds the drawing, in all three (whole programs:
[Python](examples/kinetic_type.py), [Rust](examples/kinetic_type.rs), [JavaScript](examples/kinetic_type.js)):

```python
letters = (
    btvec.text("NO\nSLEEP", font=font, size=560, axes={"wght": weight + 120 * pulse}, leading=520,
               at=(WIDTH / 2, 600), align="center", width=980, fit="wdth")
    .scale(1 + 0.08 * pulse)
    .rotate(btvec.random(-10, 10, seed=7) * burst)
    .translate(btvec.random(-30, 30, seed=8) * burst, btvec.random(-50, 50, seed=9) * burst)
)
frame = btvec.Path.rect(0, 0, WIDTH, HEIGHT)
misregistration = 3 + 14 * burst
blue = (btvec.Drawing(frame).translate(0, HEIGHT * (1 - rise))
        .subtract(letters.translate(misregistration, 0.6 * misregistration)).fill(BLUE).blend("multiply"))
pink = letters.fill(PINK).blend("multiply").opacity(ink)
hairlines = letters.stroke(INK, width=1.5).opacity(1 - ink)
piece = btvec.Drawing(frame.fill(PAPER), blue, pink, hairlines, size=(WIDTH, HEIGHT))
png = piece.to_png(1080, 1350, rise=1, ink=1, weight=620, burst=0.4, pulse=[0.9, 0.6, 0.3, 0.1, 0, 0, 0])
```

```rust
let letters = btvec::text(
    "NO\nSLEEP",
    Text::new(&font).size(560.0).axis("wght", &weight + 120.0 * &pulse).leading(520.0)
        .at((WIDTH / 2.0, 600.0)).align(Align::Center).width(980.0).fit("wdth"),
)
.scale(1.0 + 0.08 * &pulse)
.rotate(Deg(btvec::random(-10.0, 10.0, 7) * &burst))
.translate(btvec::random(-30.0, 30.0, 8) * &burst, btvec::random(-50.0, 50.0, 9) * &burst);
let frame = Path::rect(0.0, 0.0, WIDTH, HEIGHT);
let misregistration = 3.0 + 14.0 * &burst;
let blue = Drawing::from(frame.clone()).translate(0.0, HEIGHT * (1.0 - &rise))
    .subtract(&letters.translate(misregistration.clone(), 0.6 * misregistration))
    .fill(BLUE).blend(Blend::Multiply);
let pink = letters.fill(PINK).blend(Blend::Multiply).opacity(&ink);
let hairlines = letters.stroke(Stroke::new(INK).width(1.5)).opacity(1.0 - &ink);
let piece = Drawing::new([frame.fill(PAPER), blue, pink, hairlines]).with_size(WIDTH, HEIGHT);
let values = Values::new().set("rise", 1.0).set("ink", 1.0).set("weight", 620.0).set("burst", 0.4)
    .set("pulse", vec![0.9, 0.6, 0.3, 0.1, 0.0, 0.0, 0.0]);
let png = piece.to_png(Size::px(1080, 1350), &values)?;
```

```js
const letters = btvec
  .text("NO\nSLEEP", { font, size: 560, axes: { wght: weight.add(pulse.mul(120)) }, leading: 520,
                       at: [WIDTH / 2, 600], align: "center", width: 980, fit: "wdth" })
  .scale(pulse.mul(0.08).add(1))
  .rotate(btvec.random(-10, 10, { seed: 7 }).mul(burst))
  .translate(btvec.random(-30, 30, { seed: 8 }).mul(burst), btvec.random(-50, 50, { seed: 9 }).mul(burst));
const frame = btvec.Path.rect(0, 0, WIDTH, HEIGHT);
const misregistration = burst.mul(14).add(3);
const blue = new btvec.Drawing([frame]).translate(0, rise.neg().add(1).mul(HEIGHT))
  .subtract(letters.translate(misregistration, misregistration.mul(0.6))).fill(BLUE).blend("multiply");
const pink = letters.fill(PINK).blend("multiply").opacity(ink);
const hairlines = letters.stroke(INK, { width: 1.5 }).opacity(ink.neg().add(1));
const piece = new btvec.Drawing([frame.fill(PAPER), blue, pink, hairlines], { size: [WIDTH, HEIGHT] });
const rgba = piece.toRgba(1080, 1350, { rise: 1, ink: 1, weight: 620, burst: 0.4, pulse: [0.9, 0.6, 0.3, 0.1, 0, 0, 0] });
```

## Coverage of the prototypes' needs

Every ranked need in [prototype-findings.md](prototype-findings.md), "What the library needs, ranked":

| # | Need | Where the API expresses it |
|---|---|---|
| 1 | Per-glyph variable-font layout that re-flows; tracking, stacked lines, fitting at a reference pose, outlines cached by axis location | `text(axes=..., tracking=, leading=, width=, fit=)`, per-item numbers in `axes`; caching is internal |
| 2 | Evenly spaced streamlines with spacing from a tone, region masks, obstacles, closed loops; line fields and distance fields | `hatch(spacing=(a, b), tone=, flow=, avoid=)` inside each shape; `Flow.noise`, `Flow.along`, `flow.mix`; `Field.distance` |
| 3 | Steady tracing over a sequence of maps: smoothing and hole persistence | `trace(before=, after=, sigma=)` |
| 4 | Booleans that hold on overlapping, rotated glyphs; ink layers from them | `union`, `subtract`, `intersect`, `exclude`; glyphs made valid regions by `text`; inks as drawings with `blend("multiply")` ([kinetic_type.py](examples/kinetic_type.py)) |
| 5 | Shrink and grow per layer; dropping small parts and filling small holes | `offset(r).offset(-r)`, `offset(-r).offset(r)`; `drop_parts`, `fill_holes` |
| 6 | Per-item transforms about a typographic origin; seeded randomness and stagger by index as one mechanism | `translate`, `rotate`, `scale` about each shape's origin; `by_index`, `by_position`, `random`, `stagger`; lists at draw time |
| 7 | Curve fitting through dense polylines | `simplify`; `trace(tolerance=)` |
| 8 | One picture to both pixels and SVG, every effect kept | `to_svg`, `to_png`, `to_rgba` |
| 9 | A fast halftone fill | `halftone` |
| 10 | Plotter export: stroke only, ordered, simplified, solids hatched, a layer per pen | `to_plotter_svg` |
| 11 | A variable-width stroke, tapered at free ends | gap: an effects-track verb, named with its gallery piece |
| 12 | Partial path by length from a stable start | `slice(0, progress)`, closed outlines measured from the topmost point |
| 13 | Fitting text to a box by a variable axis, with a clear signal when the axis runs out | `text(width=, fit=)`; `ParameterError` naming the line and the width reached |

Also from the findings: draw-on (`slice`), the riso misregistration that grows on a hit (an expression,
`3 + 14 * burst`), the accent letter (`only`, `without`), type under the paper (stacking order), paper that slides
(`translate` by a Param). Gaps beyond the ranked list: a multi-colour trace (a map of colour labels; V14 names it) and a
re-drawable series node (a series is plain code now, [series_bars.py](examples/series_bars.py); the effects track adds a
node if a piece needs one).

## Open questions for the owner

1. **Positions along a path as fractions 0..1, closed outlines measured from their topmost point.** Recommended: yes.
   A position is then a state (draw-on is `slice(0, progress)`) and the start does not move as a glyph changes; a
   distance is `fraction × length`.
2. **Primitive shapes only as `Path` constructors.** `Path.star(...)` computes now; there is no `star` step whose point
   count is a Param. Recommended: yes, until a piece needs one; a Param on `scale` or `rotate` animates a primitive.
3. **The reference pose for `fit` is every Param at its default.** Recommended: yes; the alternative, a `pose=`
   argument, repeats values the Params already hold.
4. **A mesh for steps with no triangle form** (blurred shadows, knockouts, multiply). Recommended: raise
   `ParameterError` naming the step until the GPU display spike measures what a shader should do, rather than leave
   them out quietly.
5. **`frames` in Python only.** Recommended: yes for V4; Rust and JavaScript callers loop over `to_png` in a few lines,
   and the core does no IO. A small Rust crate can follow when a Rust caller asks.
6. **Any Param accepts a per-item list, with no declaration.** Recommended: yes now; V1b adds a flag to the parameter
   description only if the playground needs to know which Params are per item.
7. **Default paint.** Until a fill or stroke is set, a closed outline is filled black and an open line stroked; once
   either is set, only what is set is painted. Recommended: yes; `letters.stroke(...)` then means an outline, with no
   `fill("none")`.
8. **A drawing's size is optional and inherited.** Recommended: yes (a sketch draws 1:1 with no size); the alternative,
   requiring `size` on the drawing that is drawn, is stricter and costs every small example a line.
