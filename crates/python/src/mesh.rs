//! `Mesh`: the triangles `Drawing.to_mesh` makes, as three typed buffers.

use bullterrier_vector as vector;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

/// The triangles of a drawing in flat buffers, filled by `Drawing.to_mesh`. `vertices`, `indices` and `ranges` each
/// return a new `array.array` (typecode `f` for the vertices, `I` for the others) holding a copy of the buffer.
#[pyclass(name = "Mesh", module = "bullterrier_vector")]
pub(crate) struct PyMesh(pub(crate) vector::Mesh);

/// A new `array.array` of `typecode` holding `bytes`, which are items in the machine's own order.
fn array<'py>(py: Python<'py>, typecode: &str, bytes: &[u8]) -> PyResult<Bound<'py, PyAny>> {
    py.import("array")?
        .getattr("array")?
        .call1((typecode, PyBytes::new(py, bytes)))
}

#[pymethods]
impl PyMesh {
    /// An empty mesh, generation 0.
    #[new]
    fn new() -> Self {
        Self(vector::Mesh::new())
    }

    /// The vertices: 8 `float32` each, `x, y, r, g, b, a, item, order`.
    #[getter]
    fn vertices<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        array(
            py,
            "f",
            &self
                .0
                .vertices()
                .iter()
                .flat_map(|float| float.to_ne_bytes())
                .collect::<Vec<u8>>(),
        )
    }

    /// The triangles: 3 `uint32` indices into the vertices each, in paint order.
    #[getter]
    fn indices<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        array(
            py,
            "I",
            &self
                .0
                .indices()
                .iter()
                .flat_map(|index| index.to_ne_bytes())
                .collect::<Vec<u8>>(),
        )
    }

    /// The draw ranges: 3 `uint32` each, the first index, the number of indices and the blend (0 normal, 1 multiply; a mesh of any other blend is refused).
    #[getter]
    fn ranges<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        array(
            py,
            "I",
            &self
                .0
                .ranges()
                .iter()
                .flat_map(|number| number.to_ne_bytes())
                .collect::<Vec<u8>>(),
        )
    }

    /// A number that changes exactly when the contents change.
    #[getter]
    fn generation(&self) -> u64 {
        self.0.generation()
    }

    fn __repr__(&self) -> String {
        format!(
            "Mesh(generation={}, triangles={})",
            self.0.generation(),
            self.0.indices().len() / 3
        )
    }
}
