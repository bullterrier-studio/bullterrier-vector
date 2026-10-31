//! The WebAssembly package of bullterrier-vector, for the browser and Node. Each method passes its arguments to the
//! core unchanged, so the core's range checks are the only ones and every language gets the same errors; this file
//! only converts between JavaScript's values and the core's, and gives each core error its JavaScript `Error`.

use bullterrier_vector as vector;
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

/// A drawing: a tree of steps that evaluates to shapes, loaded from a JSON document and drawn as SVG.
#[wasm_bindgen(js_name = Drawing)]
pub struct Drawing(vector::Drawing);

#[wasm_bindgen(js_class = Drawing)]
impl Drawing {
    /// The drawing a JSON document describes.
    ///
    /// # Errors
    /// Throws an `Error` named `FormatError` if the text is not a document the library reads; its message starts
    /// with the JSON Pointer of the offending value.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(text: &str) -> Result<Drawing, JsValue> {
        raised(vector::Drawing::from_json(text)).map(Self)
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

    /// The design frame `[width, height]` in design pixels, or `undefined`.
    #[wasm_bindgen(getter, unchecked_return_type = "[number, number] | undefined")]
    #[must_use]
    pub fn size(&self) -> Option<Vec<JsValue>> {
        self.0.size().map(|(width, height)| vec![width.into(), height.into()])
    }
}
