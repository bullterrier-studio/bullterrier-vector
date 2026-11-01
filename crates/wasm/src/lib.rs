//! The WebAssembly package of bullterrier-vector, for the browser and Node. Each method passes its arguments to the
//! core unchanged, so the core's range checks are the only ones and every language gets the same errors; this file
//! only converts between JavaScript's values and the core's, and gives each core error its JavaScript `Error`.

use bullterrier_vector as vector;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const TYPES: &str = r#"
/** A Param a drawing reads: `unit` is "px", "deg", "cycle", "fraction" or "" (none). */
export interface Parameter {
    name: string;
    default: number;
    min: number;
    max: number;
    unit: string;
    meaning: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    /// What a caller gives the Params: an object of their names to numbers, or to arrays with a number per shape.
    #[wasm_bindgen(typescript_type = "Record<string, number | number[]>")]
    pub type Values;

    /// What a caller gives a document for its inputs (its fonts): an object of their names to the bytes of each.
    #[wasm_bindgen(typescript_type = "Record<string, Uint8Array>")]
    pub type Inputs;

    /// JavaScript's `Uint8Array`: whether a value is one, and a copy of its bytes.
    #[wasm_bindgen(js_name = Uint8Array)]
    type Bytes;
    #[wasm_bindgen(js_namespace = Uint8Array, js_name = from)]
    fn bytes_of(array: &JsValue) -> Vec<u8>;

    /// JavaScript's `Error`, to give an error the name of its kind.
    #[wasm_bindgen(js_name = Error)]
    type Thrown;
    #[wasm_bindgen(constructor, js_class = "Error")]
    fn new(message: &str) -> Thrown;
    #[wasm_bindgen(method, setter, js_class = "Error", js_name = name)]
    fn set_name(this: &Thrown, name: &str);

    /// JavaScript's `TypeError`, for a value of a type the call cannot read.
    #[wasm_bindgen(js_name = TypeError)]
    type WrongType;
    #[wasm_bindgen(constructor, js_class = "TypeError")]
    fn new(message: &str) -> WrongType;

    /// JavaScript's `Object`, `Reflect` and `Array`: reading an object of values and building plain objects.
    #[wasm_bindgen(js_namespace = Object, js_name = keys)]
    fn keys_of(object: &JsValue) -> Vec<String>;
    #[wasm_bindgen(js_namespace = Reflect, js_name = get)]
    fn member(object: &JsValue, key: &str) -> JsValue;
    #[wasm_bindgen(js_namespace = Reflect, js_name = set)]
    fn set_member(object: &JsValue, key: &str, value: &JsValue);
    #[wasm_bindgen(js_name = Object)]
    type Plain;
    #[wasm_bindgen(constructor, js_class = "Object")]
    fn new() -> Plain;
    #[wasm_bindgen(js_namespace = Array, js_name = isArray)]
    fn is_array(value: &JsValue) -> bool;
    #[wasm_bindgen(js_namespace = Array, js_name = from)]
    fn items_of(array: &JsValue) -> Vec<JsValue>;
}

/// The `Error` a core error becomes: its `name` is the kind (`ParameterError`, `GeometryError`, `FormatError`) and its
/// message is the core's.
fn thrown_for(error: &vector::Error) -> JsValue {
    let name = match error {
        vector::Error::Parameter { .. } => "ParameterError",
        vector::Error::Geometry { .. } => "GeometryError",
        vector::Error::Format { .. } => "FormatError",
        // `Error` is non-exhaustive: a kind this binding does not name yet is still a package error
        _ => "BullterrierVectorError",
    };
    let error = Thrown::new(&error.to_string());
    error.set_name(name);
    error.into()
}

/// Run a core call and throw its error as the matching `Error`.
fn raised<T>(result: Result<T, vector::Error>) -> Result<T, JsValue> {
    result.map_err(|error| thrown_for(&error))
}

/// The `TypeError` for a value that is not what a call reads.
fn refuse(what: &str, expected: &str, value: &JsValue) -> JsValue {
    let found = value.js_typeof().as_string().unwrap_or_default();
    WrongType::new(&format!("{what}: expected {expected}, got a {found}")).into()
}

/// The values an object gives the Params by name: each a number, or an array with a number per shape.
fn read_values(given: Option<Values>) -> Result<vector::Values, JsValue> {
    let mut values = vector::Values::new();
    let Some(given) = given else {
        return Ok(values);
    };
    let given: &JsValue = given.as_ref();
    if !given.is_object() {
        return Err(refuse(
            "values",
            "an object of Param names to numbers or arrays of numbers",
            given,
        ));
    }
    for name in keys_of(given) {
        let value = member(given, &name);
        let value = if let Some(number) = value.as_f64() {
            vector::Value::from(number)
        } else if is_array(&value) {
            let numbers = items_of(&value)
                .iter()
                .map(|item| item.as_f64().ok_or_else(|| refuse(&name, "an array of numbers", item)))
                .collect::<Result<Vec<_>, _>>()?;
            vector::Value::from(numbers)
        } else {
            return Err(refuse(&name, "a number or an array of numbers", &value));
        };
        values = values.set(name, value);
    }
    Ok(values)
}

/// The bytes an object gives the inputs of a document by name: each a `Uint8Array` (a Node `Buffer` is one).
fn read_inputs(given: Option<Inputs>) -> Result<vector::Inputs, JsValue> {
    let mut inputs = vector::Inputs::new();
    let Some(given) = given else {
        return Ok(inputs);
    };
    let given: &JsValue = given.as_ref();
    if !given.is_object() {
        return Err(refuse("inputs", "an object of names to Uint8Arrays", given));
    }
    for name in keys_of(given) {
        let bytes = member(given, &name);
        if !bytes.is_instance_of::<Bytes>() {
            return Err(refuse(&format!("inputs.{name}"), "a Uint8Array", &bytes));
        }
        inputs = inputs.set(name, bytes_of(&bytes));
    }
    Ok(inputs)
}

/// A drawing: a tree of steps that evaluates to shapes, loaded from a JSON document and drawn as SVG, pixels, a PNG
/// file or a mesh.
#[wasm_bindgen(js_name = Drawing)]
pub struct Drawing(vector::Drawing);

#[wasm_bindgen(js_class = Drawing)]
impl Drawing {
    /// The drawing a JSON document describes, given the bytes of the inputs it names (its fonts) by name.
    ///
    /// # Errors
    /// Throws an `Error` named `FormatError` if the text is not a document the library reads; its message starts
    /// with the JSON Pointer of the offending value. A `ParameterError` naming `inputs.<name>` for an input the
    /// document declares and `inputs` lacks, or the reverse, bytes that are not the declared font, or a font the
    /// library cannot read. A `TypeError` if `text` is not a string (wasm-bindgen would read any value as one and
    /// trap), or `inputs` is not an object of `Uint8Array`s.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(
        #[wasm_bindgen(unchecked_param_type = "string")] text: &JsValue,
        inputs: Option<Inputs>,
    ) -> Result<Drawing, JsValue> {
        let Some(text) = text.as_string() else {
            return Err(refuse("text", "a string", text));
        };
        raised(vector::Drawing::from_json(&text, &read_inputs(inputs)?)).map(Self)
    }

    /// The drawing as JSON in the canonical form.
    ///
    /// # Errors
    /// Throws the first error of the chain that built the drawing.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        raised(self.0.to_json())
    }

    /// The drawing as an SVG document of `width` by `height` pixels, with each Param at the value given by name
    /// (a number, or an array with a number per shape) and at its default where none is.
    ///
    /// # Errors
    /// Throws a `ParameterError` naming `width` or `height` if it is not a whole number of pixels in range, or the
    /// Param whose value is refused; a `TypeError` for a value that is not a number or an array of numbers.
    #[wasm_bindgen(js_name = toSvg)]
    pub fn to_svg(&self, width: f64, height: f64, values: Option<Values>) -> Result<String, JsValue> {
        raised(self.0.to_svg(vector::Size::px(width, height), &read_values(values)?))
    }

    /// Every Param the drawing reads, in the order first used, each a plain object.
    ///
    /// # Errors
    /// Throws the first error of the chain that built the drawing.
    #[wasm_bindgen(unchecked_return_type = "Parameter[]")]
    pub fn parameters(&self) -> Result<Vec<JsValue>, JsValue> {
        Ok(raised(self.0.parameters())?
            .iter()
            .map(|param| {
                let object = JsValue::from(Plain::new());
                set_member(&object, "name", &param.name().into());
                set_member(&object, "default", &param.default().into());
                set_member(&object, "min", &param.min().into());
                set_member(&object, "max", &param.max().into());
                set_member(&object, "unit", &param.unit_of().name().into());
                set_member(&object, "meaning", &param.meaning_of().into());
                object
            })
            .collect())
    }

    /// The drawing as straight-alpha RGBA bytes of `width` by `height` pixels, rows from the top, with each Param at
    /// the value given by name (as `toSvg`).
    ///
    /// # Errors
    /// As `toSvg`; the sides are whole numbers from 1 to 65,000, at most 2^27 pixels in all.
    #[wasm_bindgen(js_name = toRgba)]
    pub fn to_rgba(&self, width: f64, height: f64, values: Option<Values>) -> Result<Vec<u8>, JsValue> {
        raised(self.0.to_rgba(vector::Size::px(width, height), &read_values(values)?))
    }

    /// The drawing as a PNG file of `width` by `height` pixels: the pixels of `toRgba`, 8 bits a channel.
    ///
    /// # Errors
    /// As `toRgba`.
    #[wasm_bindgen(js_name = toPng)]
    pub fn to_png(&self, width: f64, height: f64, values: Option<Values>) -> Result<Vec<u8>, JsValue> {
        raised(self.0.to_png(vector::Size::px(width, height), &read_values(values)?))
    }

    /// The triangles of the drawing at `width` by `height` pixels, in a new `Mesh`.
    ///
    /// # Errors
    /// As `toSvg`.
    #[wasm_bindgen(js_name = toMesh)]
    pub fn to_mesh(&self, width: f64, height: f64, values: Option<Values>) -> Result<Mesh, JsValue> {
        let mut mesh = Mesh::new();
        self.to_mesh_into(&mut mesh, width, height, values)?;
        Ok(mesh)
    }

    /// Fills `mesh` with the triangles of the drawing, in place: its buffers are reused, and it is left as it was if
    /// this throws.
    ///
    /// # Errors
    /// As `toSvg`.
    #[wasm_bindgen(js_name = toMeshInto)]
    pub fn to_mesh_into(
        &self,
        mesh: &mut Mesh,
        width: f64,
        height: f64,
        values: Option<Values>,
    ) -> Result<(), JsValue> {
        raised(
            self.0
                .to_mesh(vector::Size::px(width, height), &read_values(values)?, &mut mesh.0),
        )
    }

    /// The design frame `[width, height]` in design pixels, or `undefined`.
    #[wasm_bindgen(getter, unchecked_return_type = "[number, number] | undefined")]
    #[must_use]
    pub fn size(&self) -> Option<Vec<JsValue>> {
        self.0.size().map(|(width, height)| vec![width.into(), height.into()])
    }
}

/// The triangles of a drawing in three buffers that live in WebAssembly memory (`wasmMemory()`): a page makes typed arrays
/// over them from the pointers and lengths, and uploads them as they are. The vertices are `Float32Array` data, 8
/// floats each (`x, y, r, g, b, a, item, order`); the indices and the ranges are `Uint32Array` data (3 per triangle; 3
/// per draw range: first index, index count, blend).
#[wasm_bindgen(js_name = Mesh)]
pub struct Mesh(vector::Mesh);

#[wasm_bindgen(js_class = Mesh)]
impl Mesh {
    /// An empty mesh, generation 0.
    #[wasm_bindgen(constructor)]
    #[must_use]
    #[expect(
        clippy::new_without_default,
        reason = "a JavaScript constructor: there is no Default to call"
    )]
    pub fn new() -> Self {
        Self(vector::Mesh::new())
    }

    /// Where the vertices start: a byte offset into `wasmMemory()`.
    #[wasm_bindgen(getter, js_name = verticesPtr)]
    #[must_use]
    pub fn vertices_ptr(&self) -> usize {
        self.0.vertices().as_ptr().addr()
    }

    /// Where the indices start: a byte offset into `wasmMemory()`.
    #[wasm_bindgen(getter, js_name = indicesPtr)]
    #[must_use]
    pub fn indices_ptr(&self) -> usize {
        self.0.indices().as_ptr().addr()
    }

    /// Where the draw ranges start: a byte offset into `wasmMemory()`.
    #[wasm_bindgen(getter, js_name = rangesPtr)]
    #[must_use]
    pub fn ranges_ptr(&self) -> usize {
        self.0.ranges().as_ptr().addr()
    }

    /// The number of floats in the vertices (8 a vertex).
    #[wasm_bindgen(getter, js_name = verticesLength)]
    #[must_use]
    pub fn vertices_length(&self) -> usize {
        self.0.vertices().len()
    }

    /// The number of integers in the indices (3 a triangle).
    #[wasm_bindgen(getter, js_name = indicesLength)]
    #[must_use]
    pub fn indices_length(&self) -> usize {
        self.0.indices().len()
    }

    /// The number of integers in the draw ranges (3 a range).
    #[wasm_bindgen(getter, js_name = rangesLength)]
    #[must_use]
    pub fn ranges_length(&self) -> usize {
        self.0.ranges().len()
    }

    /// A number that changes exactly when the contents change: 0 for a new mesh, and 1 more for each fill that makes
    /// buffers different from what the mesh held.
    #[wasm_bindgen(getter)]
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "a count of fills: 2^53 of them is out of reach"
    )]
    pub fn generation(&self) -> f64 {
        self.0.generation() as f64
    }
}

/// The WebAssembly memory the buffers of a [`Mesh`] are in. Make typed arrays over `wasmMemory().buffer` after each
/// call, not before: the memory can grow in any call, and the arrays made earlier are then detached. (Not called
/// `memory`: that is the name of the module's own export of it.)
#[wasm_bindgen(js_name = wasmMemory, unchecked_return_type = "WebAssembly.Memory")]
#[must_use]
pub fn wasm_memory() -> JsValue {
    wasm_bindgen::memory()
}
