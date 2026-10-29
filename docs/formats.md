# The JSON format

Status: V1b, written from the code API ([api.md](api.md)); waiting for the owner's review. Nothing reads or writes it
yet ([roadmap.md](roadmap.md)). Reasons: [design-decisions.md](design-decisions.md), "The JSON format". Examples:
[examples/json/](examples/json/). Draft schema: [drawing-1.schema.json](../schemas/drawing-1.schema.json).

A drawing file is the tree the code API builds, written down: `drawing.to_json()` writes it, `Drawing.from_json(text)`
reads it back, and the two draw to the same bytes. A drawing built in Rust, Python or JavaScript saves to the same JSON.
Nothing in a document says what the code API cannot.

## A first look

```python
spin = btvec.Param("spin", default=0, min=0, max=360, unit="deg", meaning="how far the square has turned")
square = btvec.Path.rect(150, 150, 100, 100).fill("#FF4F9A").rotate(spin, about=(200, 200))
drawing = btvec.Drawing(square, size=(400, 400))
```

```json
{
  "version": 1,
  "params": {
    "spin": { "default": 0, "min": 0, "max": 360, "unit": "deg", "meaning": "how far the square has turned" }
  },
  "root": [
    {
      "stack": {
        "items": [
          [
            { "path": "M150 150 L250 150 L250 250 L150 250 Z" },
            { "fill": { "color": "#ff4f9a" } },
            { "rotate": { "angle": { "param": "spin" }, "about": [200, 200] } }
          ]
        ],
        "size": [400, 400]
      }
    }
  ]
}
```

Five rules carry the format:
1. A **call** is an object with one key, the call's name, whose value holds its arguments: `{"fill": {"color": ...}}`.
2. A **drawing** is an array of calls, in the order the code chains them: a source first, then each step.
3. A **number** is a JSON number or an expression: `{"param": "spin"}`, `{"mul": [120, {"param": "pulse"}]}`.
4. **Names** refer to what the document declares: Params in `params`, inputs in `inputs`, shared drawings in
   `drawings`.
5. An argument left out takes its default (api.md's); an argument equal to its default is left out when written.

## The document

| Key | Required | Holds |
|---|---|---|
| `version` | yes | `1`, the format's major version ("Versioning") |
| `params` | no | every Param the drawing reads, by name ("Params") |
| `inputs` | no | every input the drawing reads, by name ("Inputs") |
| `drawings` | no | drawings used in more than one place, by name ("Shared drawings") |
| `root` | yes | the drawing |

No other key is allowed, here or in any object the format defines. `params`, `inputs` and `drawings` are left out when
empty, never written `{}`. A name is letters, digits and `_`, starting with a letter. A document is UTF-8 JSON with no
duplicate key in any object, as I-JSON (RFC 7493) requires.

The design size is not in the envelope: it is the `size` of a `stack`, where the code gives it (`Drawing(*items,
size=...)`), and the root's size is inherited as api.md says.

## Drawings

A drawing is a non-empty array of calls. The first is a **source**, which makes shapes from nothing; every next one is a
**step**, applied to the drawing before it. `letters.fill(PINK).blend("multiply")` is
`[letters, {"fill": {"color": "#ff4f9a"}}, {"blend": {"mode": "multiply"}}]`, where `letters` is itself a source (a
generator call, or a reference to a shared drawing). Arguments that take a drawing (`other`, `items`, `avoid`, `target`,
`Field.distance`'s `drawing`) hold such an array. Where the code gives a `Path` in place of a drawing, the array is the
path alone: `[{"path": "..."}]`.

Names are the Python and Rust spelling (`gradient_map`, `drop_parts`, `smaller_than`); arguments have api.md's names,
defaults and units. The types below:

| Type | In JSON |
|---|---|
| number | a JSON number or an expression ("Numbers and expressions") |
| whole number | a JSON integer, or an expression whose value is whole when drawn |
| point | `[x, y]`, two numbers |
| pair | a number, or two numbers `[a, b]` where api.md takes a pair (`scale`'s `(x, y)`, `hatch`'s `spacing`) |
| drawing | an array of calls, as above |
| path data | a string of SVG path data ("Values") |
| colour | a string ("Values") |
| field, flow | a call of the kinds below |
| input | the name of an entry of `inputs` |

Every argument not marked otherwise is a number. A literal value outside the range api.md states for it (an `alpha`
above 1, a `pitch` of 0) fails the schema; an expression's value is checked when drawn.

### Sources

| Call | Code | Arguments |
|---|---|---|
| `path` | a `Path` | its path data: the value is the string itself, `{"path": "M0 0 L10 0"}` |
| `shape` | a `Shape` given to `Drawing(...)` | `path` (path data), `attributes` (what its generators wrote, `index` excepted: `glyph`, `char`, `word`, `line`, `part`, `copy`, `level`, `order`, `origin`); its style follows as steps |
| `drawing` | a shared drawing | the value is its name: `{"drawing": "letters"}` |
| `stack` | `Drawing(*items, size=None)` | `items` (drawings, bottom first), `size` (`[width, height]`, literal numbers above 0) |
| `text` | `text(content, *, font, ...)` | `content` (a string), `font` (input), `size`, `axes` (tag to number), `features` (tag to whole number), `tracking`, `leading`, `align`, `anchor` (point), `width`, `fit` (a tag or `"size"`), `inside` (path data) |
| `trace` | `trace(field, *, ...)` | `field` (field), `levels` (numbers), `tolerance`, `before`, `after` (fields), `sigma` |

A tag is four printable ASCII characters (`"wght"`). A shape's style is written as the style steps that set it, in the
order of api.md's `Style` (`fill`, `stroke`, `halftone`, `opacity`, `blend`, `shadow`, `knockout`), each with the
shape's values.

### Steps

| Call | Arguments |
|---|---|
| `translate` | `dx`, `dy` |
| `rotate` | `angle`, `about` (point) |
| `scale` | `by` (pair, not 0), `about` (point) |
| `union`, `subtract`, `intersect`, `exclude` | `other` (drawing) |
| `offset` | `distance`, `join` (`"round"`, `"miter"`, `"bevel"`) |
| `outline` | `width`, `join`, `cap` (`"butt"`, `"round"`, `"square"`) |
| `drop_parts`, `fill_holes` | `smaller_than` |
| `slice` | `start`, `end`, `near` (point) |
| `reverse`, `split`, `knockout` | none: `{"reverse": {}}` |
| `dash` | `pattern` (numbers), `phase`, `near` (point) |
| `resample` | `spacing` |
| `simplify` | `tolerance` |
| `along` | `path` (path data), `start`, `near` (point) |
| `wobble` | `amplitude`, `wavelength`, `phase`, `seed` (whole number) |
| `wave` | `amplitude`, `wavelength`, `angle`, `phase` |
| `morph` | `target` (drawing), `fraction` |
| `hatch` | `spacing` (pair), `tone` (number or field), `flow` (flow), `avoid` (drawing) |
| `merge` | `by` (an attribute name) |
| `only`, `without` | attribute names to a value or a list of values: `{"only": {"char": "O"}}` |
| `repeat` | `count` (whole number), `translate` (point), `rotate`, `scale` (pair), `about` (point) |
| `fill` | `color` (colour or `"none"`) |
| `stroke` | `color` (colour), `width`, `join`, `cap` |
| `halftone` | `color` (colour), `tone` (number or field), `pitch`, `angle` |
| `opacity` | `alpha` |
| `blend` | `mode` (`"normal"`, `"multiply"`) |
| `shadow` | `dx`, `dy`, `blur`, `color` (colour) |
| `gradient_map` | `colors` (colours, at least 2) |
| `posterize` | `levels` (whole number) |
| `with_values` | Param names to a value: a JSON number, or an array of numbers, one per shape |

`only`, `without` and `with_values` take keyword arguments in the code (`only(char="O")`), so their arguments are those
keywords. An attribute is `index`, `glyph`, `char`, `word`, `line`, `part`, `copy`, `level` or `order`.

`with_values` fixes Params for the drawing before it: every step before it, and every drawing those steps read, sees
these values; the Params leave `parameters()` of what follows. Its values are part of the drawing, lists included. A
list given at draw time (`to_png(..., pulse=[...])`) is not: it is a value of that call.

### Fields and flows

| Call | Code | Arguments |
|---|---|---|
| `Field.map` | `Field.map(width, height, data, *, box=None)` | `width`, `height` (literal whole numbers from 1), `data` (input), `box` (`[x, y, width, height]`) |
| `Field.ramp` | `Field.ramp(start, end)` | `start`, `end` (points) |
| `Field.distance` | `Field.distance(drawing, *, normalized=False)` | `drawing` (drawing), `normalized` (`true` or `false`) |
| `Flow.angle` | `Flow.angle(angle)` | `angle` |
| `Flow.noise` | `Flow.noise(*, wavelength=400, phase=0, seed=0)` | `wavelength`, `phase`, `seed` (whole number) |
| `Flow.contour` | `Flow.contour(drawing)` | `drawing` (drawing) |
| `Flow.mix` | `flow.mix(other, fraction)` | `flow`, `other` (flows), `fraction` |

A map's `width` and `height` are literal so that the loader can check the bytes against them when they arrive.

### Shared drawings

A drawing used in more than one place (`letters` in the kinetic type piece feeds the plane's cut, the pink ink and the
hairlines) is written once, under a name in `drawings`, and each use is the source `{"drawing": "<name>"}`, followed
by the steps of that use. A stack with a copy:

```json
{
  "version": 1,
  "drawings": {
    "bars": [
      {
        "stack": {
          "items": [[{ "path": "M90 300 L146 300 L146 900 L90 900 Z" }], [{ "path": "M166 600 L222 600 L222 900 L166 900 Z" }]]
        }
      }
    ]
  },
  "root": [
    {
      "stack": {
        "items": [
          [{ "drawing": "bars" }, { "without": { "index": 0 } }, { "fill": { "color": "#15161a" } }],
          [{ "drawing": "bars" }, { "only": { "index": 0 } }, { "fill": { "color": "#e2412e" } }]
        ]
      }
    }
  ]
}
```

A shared drawing is a value, not a variable: every use draws the same shapes, and nothing a use does reaches the others.
References form no cycle. A `repeat` is a step (`{"repeat": {"count": 12, "rotate": 30}}`), not a copy: its copies
exist only when drawn.

## Numbers and expressions

A number is a JSON number (finite, as JSON allows no other) or an expression, an object with one key:

| Expression | Code | Meaning |
|---|---|---|
| `{"param": "pulse"}` | a `Param` | the Param's value when drawn |
| `{"add": [a, b]}`, `{"sub": [a, b]}`, `{"mul": [a, b]}`, `{"div": [a, b]}` | `a + b`, `a - b`, `a * b`, `a / b` | arithmetic on two numbers |
| `{"neg": a}` | `-a` | negation |
| `{"by_index": {"first": a, "last": b}}` | `by_index(first, last)` | per item, by `index` |
| `{"by_order": {"first": a, "last": b}}` | `by_order(first, last)` | per item, by `order` |
| `{"random": {"low": a, "high": b, "seed": s}}` | `random(low, high, *, seed=0)` | per item, seeded; `seed` a whole number |
| `{"stagger": {"progress": p, "spread": s}}` | `stagger(progress, *, spread=0.5)` | per item, staggered progress |

That is the whole grammar: no other operator or function, no strings, no conditionals, no references to time or to
other arguments. Every operand and every argument of a per-item number is itself a number, so they nest:
`btvec.random(-10, 10, seed=7) * burst` is `{"mul": [{"random": {"low": -10, "high": 10, "seed": 7}}, {"param":
"burst"}]}`. Values are 64-bit floats, evaluated in the order the tree gives; the result has the unit of the argument
it stands in.

An operator has at least one operand that is not a JSON number: an expression with no Param and no per-item number is
written as its value (`to_json` folds it; the schema rejects `{"add": [1, 2]}` and `{"neg": 3}`). A literal zero
divisor fails the schema; a divisor that is zero when drawn raises `ParameterError`, as api.md says.

A whole-number argument (`count`, `seed`, `levels`, a map's size) given as a JSON number must be an integer (`3`, never
`3.5`); given as an expression, its value must be whole when drawn, or drawing raises `ParameterError`; it is never
rounded.

## Values

- **Units.** A number carries no unit: the argument defines it, as api.md's "Conventions" say. Lengths are design
  pixels, angles degrees clockwise, phases cycles, positions along a path fractions 0..1, tracking thousandths of an
  em, areas square pixels.
- **Angles** wrap: a cyclic Param, or a literal angle, may take any value.
- **Points** are `[x, y]`; `y` points down.
- **Colours** are `"#rrggbb"` or `"#rrggbbaa"`, hex digits in either case; `fill` also takes `"none"`. No names, no
  three-digit form, no `rgb()`.
- **Path data** is SVG path data, read as `Path.from_svg_path` reads it (every command, relative and absolute, arcs
  included). The library writes it in one form: absolute `M`, `L`, `Q`, `C` and `Z`, each letter followed by its
  numbers, every number and command separated by one space, numbers as in JSON ("Canonical form"):
  `"M0 0 L1080 0 L1080 1350 L0 1350 Z"`. A path holds exact coordinates, so a circle is four cubic curves with their
  control points, not a centre and a radius.
- **Text** is any JSON string; `"\n"` breaks a line.

## Params

A Param's description is the library's own format, the same in a drawing's `params`, in `parameters()` and wherever a
tool reads one (a slider, a schema, a signal mapping):

| Field | Holds |
|---|---|
| name | the key in `params`: letters, digits and `_`, starting with a letter |
| `default` | a number; all defaults together are the reference pose |
| `min`, `max` | numbers, `min` below `max`; the default and every value given lie in `min..max` |
| `unit` | `"px"`, `"deg"`, `"cycle"`, `"fraction"` or `""` (a plain number: a font axis, a count, a seed) |
| `meaning` | one line a person reads beside a slider; may be `""` |

Every field is always present: a tool reading a description needs no defaults. A Param in `deg` or `cycle` is cyclic:
its range spans one turn (360 or 1) and any value wraps into it; every other Param is bounded and a value outside its
range is an error. A font axis Param (`font.axes["wght"]`) is a Param like any other, its fields from the font.

`params` lists every Param the tree reads, those fixed by a `with_values` included (the reference pose needs their
defaults). It lists no other.

## Inputs

What arrives as bytes is not in the document: a document names it, and whoever loads the document supplies the bytes
by that name. A document stays small, stays text, and holds nothing the library would have to fetch.

```json
"inputs": {
  "anybody": { "kind": "font" },
  "mask": { "kind": "map", "sha256": "5f0c..." }
}
```

| Field | Holds |
|---|---|
| name | the key in `inputs` |
| `kind` | `"font"` (a TrueType or OpenType font, as `Font(data)`) or `"map"` (8-bit grayscale bytes, as `Field.map`'s `data`) |
| `sha256` | optional: the SHA-256 of the bytes, 64 lowercase hex digits; when present, the bytes supplied must match it |

A font is referred to by `text`'s `font`, a map by `Field.map`'s `data`; the map's width, height and box are the field's
arguments. `to_json` always writes `sha256`, so a saved document names exactly the bytes it was drawn with and its
canonical text is a cache key; a person writing a document by hand may leave it out. There is no embedded form: bytes
in base64 would grow a font by a third, make the JSON mostly not text, and give two ways to supply an input.

SVG and series are not inputs of a drawing in version 1. `Path.from_svg_path` computes at once, so an imported logo is
saved as its path data; a series becomes paths in the caller's code ([series_bars.py](examples/series_bars.py)). A
node that reads one adds an input kind ("Extension").

How a caller supplies the bytes, and how an input made in code gets its name, is question Q1.

## Canonical form

`to_json` writes one text for one drawing, so the same drawing gives the same bytes in every language, and the text
can address a cache. Two documents are the same drawing when their canonical forms are equal. A reader accepts any
spelling the schema allows; canonical form is what the library writes:

1. **Defaults.** An argument equal to its default is left out (`"angle": 45` on `halftone`, `"levels": [0.5]` on
   `trace`); so are empty `params`, `inputs` and `drawings`.
2. **Paths** in the form under "Values"; a `Path` where a drawing is expected, and `Drawing(path)` with one path and no
   size, as `[{"path": "..."}]` (Q3).
3. **Colours** in lower case, `#rrggbbff` as `#rrggbb`.
4. **Expressions** folded where constant; the two operands of `add` and of `mul` in the order of their canonical texts
   (by code point), so `weight + 120 * pulse` in Python and `weight.add(pulse.mul(120))` in JavaScript write the same.
   Nothing else is reordered, since floating-point addition is commutative but not associative.
5. **Inputs** under the names the drawing gives them (Q1); `sha256` always written.
6. **Shared drawings.** Equal drawings (equal canonical texts with every reference written out) are one drawing. A
   drawing used more than once is in `drawings`, named `d1`, `d2`... in the order the walk below first reaches it;
   every other drawing is written in place, a hand-written name for a drawing used once included.
7. **Text.** RFC 8785 (JSON Canonicalization Scheme): keys sorted by code point, no whitespace, numbers in their
   shortest round-trip form (`1080`, `0.6`, `-0` as `0`), strings with the fewest escapes, UTF-8.

The walk visits the root's calls in order; within a call, its arguments in key order and the items of a list in
order; a shared drawing when it is first reached. `parameters()` lists Params in the order this walk first reads them.

The first look's drawing in canonical form:

```json
{"params":{"spin":{"default":0,"max":360,"meaning":"how far the square has turned","min":0,"unit":"deg"}},"root":[{"stack":{"items":[[{"path":"M150 150 L250 150 L250 250 L150 250 Z"},{"fill":{"color":"#ff4f9a"}},{"rotate":{"about":[200,200],"angle":{"param":"spin"}}}]],"size":[400,400]}}],"version":1}
```

## Versioning

- `version` is an integer, the format's major version; this is version 1. A library documents the versions it reads,
  reads those and refuses any other with `FormatError` naming the version found and the versions it reads; a document
  without `version` is refused the same way. It writes the newest version it documents.
- Within a version the format only grows: a new call, a new optional argument whose default keeps every existing
  document's meaning, a new value of an enumeration. A reader that meets a name it does not know refuses it with
  `FormatError` naming it and the library's own version, so a document from a newer library says why it fails.
- Anything else is a new version: removing or renaming a call or argument, changing a unit, a meaning or a default
  (canonical form leaves defaults out, so a saved document relies on them).
- Before the first release (roadmap V22) version 1 is a draft and changes without a new number.

## Errors

| Error | When |
|---|---|
| `FormatError` (also a `ValueError`) | the text is not a document this library reads: not JSON, a duplicate key, the schema fails, an unknown version or name, any check of "Checked by the loader" fails, a limit is passed. It carries the JSON Pointer of the offending value (`/root/2/rotate/angle`) and what is wrong. |
| `ParameterError` | the caller's values are wrong: an input not supplied or not declared, bytes that do not match `sha256`, a font the library cannot read, a map whose bytes are not `width × height`; at draw time, as for any drawing (api.md). `to_json` raises it for a drawing beyond a limit. |
| `GeometryError` | at draw time, as for any drawing |

`FormatError` is new to api.md's error kinds: the fix for a bad document is in the document, not in a call, so a caller
of `from_json` catches it in one place, and every problem in the text has one kind and a pointer.

## Limits and security

A document is data. It holds no code and no callback; the library never evaluates a string as code, resolves a file
path or URL, or reads anything but the bytes the caller supplies. An input's name is a name, never a location.

| Limit | Value | Counts |
|---|---|---|
| `MAX_DOCUMENT_BYTES` | 16 MiB | the document's UTF-8 text |
| `MAX_DEPTH` | 128 | nesting of arrays and objects, counted through references, as if each shared drawing were written in place |
| `MAX_CALLS` | 100,000 | call objects in the document: sources, steps, fields, flows and expressions |

Counting depth through references bounds the evaluator's recursion as well as the parser's. A document past a limit is
refused before anything is evaluated. Limits on what evaluation makes (shapes, points) are the evaluator's (V2): a small
document may rightly make many shapes.

## Checked by the loader, not the schema

The schema checks each value on its own. These checks relate values across the document, or need the inputs, and are
the loader's (V2 implements each, with a test named after it):

| Check | Fails when |
|---|---|
| `json-text` | the text is not UTF-8 I-JSON: a duplicate key, a lone surrogate, a number beyond a 64-bit float |
| `limits` | a limit above is passed |
| `param-range` | `min` is not below `max`, the default lies outside `min..max`, or a cyclic Param's range is not one turn |
| `param-declared` | a `param` expression or a `with_values` key names no entry of `params` |
| `param-used` | an entry of `params` is read nowhere |
| `input-declared` | a `font` or `data` names no entry of `inputs`, or one of the other kind |
| `input-used` | an entry of `inputs` is read nowhere |
| `drawing-declared` | a `drawing` source names no entry of `drawings` |
| `drawing-used` | an entry of `drawings` is used nowhere |
| `drawing-acyclic` | a shared drawing reaches itself through references |
| `path-data` | path data does not parse; the error names the first bad command |
| `with-values` | a key names a Param the drawing before it does not read, or one an earlier `with_values` fixed; a value outside its Param's range (a cyclic one wraps) |
| `call-checks` | a check api.md makes when a call is made, on literal values, that the schema cannot state: `trace` levels not increasing, `random` with `low` above `high`, `text` `fit` without `width` or on an axis also in `axes`, `Field.ramp` with equal points, items of a stack without `size` whose sizes differ |
| `font-checks` | with the font supplied: an axis or feature the font lacks, an axis value outside its range |
| `inputs-supplied` | an input not supplied, bytes supplied for no input, bytes that do not match `sha256`, a map whose bytes are not `width × height` (`ParameterError`) |

## Extension

A new call is added by name, without changing the format's core. Each built-in registers its name, whether it is a
source, a step, a field or a flow, and its arguments (name, type, default, range) once, in Rust; the reader, the writer,
the schema and the Python and JavaScript bindings come from that registration. The core knows only the five rules,
the envelope, expressions and the registry; it has no list of calls. A new call is an addition within the version
("Versioning"). An input kind is added the same way, with the call that reads it.

`schemas/drawing-1.schema.json` is a hand-written draft that checks the examples and the known-bad documents of
`tests/test_drawing_schema.py`. From V2 the schema is generated from the registrations, and a test keeps the generated
schema in agreement with this draft until the draft is retired.

## Examples

Each file is the drawing of the program of the same name in [examples/](examples/), with the same steps and names. Path
data is what the constructors make (a circle and a rounded corner as cubic curves). Shared drawings and inputs carry
readable names, and keys are in reading order; `to_json` would write `d1`, `d2`... and sorted keys ("Canonical form").

| File | Shows |
|---|---|
| [kinetic_type.json](examples/json/kinetic_type.json) | reference piece 1, built once: expressions, per-item numbers, a font input, two shared drawings |
| [cut_paper_one_frame.json](examples/json/cut_paper_one_frame.json) | reference piece 2, the drawing of one frame (4 s in, mask 100, traced with two masks on each side): map inputs, `with_values` |
| [hatched_poster.json](examples/json/hatched_poster.json) | reference piece 3: fields and flows, a stack inside a stack |
| [ring.json](examples/json/ring.json) | an expression (`neg`), a shared stack |
| [glyph_scatter.json](examples/json/glyph_scatter.json) | per-item numbers: seeded `random` times a Param |
| [radial_repeat.json](examples/json/radial_repeat.json) | `repeat`, `by_index`, a cyclic Param |
| [series_bars.json](examples/json/series_bars.json) | a stack with a copy: one drawing used twice |
| [halftone.json](examples/json/halftone.json) | a map input read by a field |
| [wobble_outline.json](examples/json/wobble_outline.json) | text with a font axis as a Param |

The `sha256` fields are left out: the examples' bytes are not in the repository.

## Questions for the owner

1. **Inputs in the code API.** api.md's `Font(data)` and `Field.map(...)` have no name, and `from_json(text)` takes no
   bytes. Recommended: `Font(data, *, name=None)` and `Field.map(..., name=None)`, the name an input gets in JSON (an
   unnamed one is written `font1`, `map1`... in walk order); `Drawing.from_json(text, *, inputs={})`, names to bytes
   (Rust: `Inputs::new().set("anybody", bytes)`; JavaScript: an object), checked as "Checked by the loader" says.
2. **Names of shared drawings.** The code API has no name for a drawing, so `to_json` writes `d1`, `d2`.... Recommended:
   keep it so; a name on `Drawing` is a second identity for a value. Revisit with an editor (V20), which may want
   names.
3. **`Drawing(path)` is the path.** Rust converts a path (`Drawing::from(path)`); Python and JavaScript stack it
   (`Drawing(frame)`). Recommended: one path and no size make no stack in any language, so the three kinetic type
   programs save to the same JSON.
4. **Defaults are part of the format.** Saved documents leave defaults out, so after the first release a default
   changes only with a new version (the code API's callers rely on defaults in the same way). Recommended: yes; files
   stay short and hand-written ones shorter; the alternative, writing every argument, puts `null` for `leading`,
   `about` and `fit` into every file.
5. **Bytes when loading, not when drawing.** Inputs are supplied to `from_json`, so a loaded drawing is complete like
   one built in code. Supplying them at draw time would let one document serve every frame of a traced clip, but makes
   a second kind of drawing (one that cannot draw until given its bytes). Recommended: at load; revisit in V14 if
   saving a clip's frames costs too much.
