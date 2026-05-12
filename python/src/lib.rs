use caption_lib::{Captioner, OutputFormat};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyModule};
use std::sync::Mutex;

#[pyclass(name = "Captioner")]
struct PyCaptioner {
    inner: Mutex<Captioner>,
}

#[pymethods]
impl PyCaptioner {
    #[new]
    fn new() -> Self {
        Self {
            inner: Mutex::new(Captioner::new()),
        }
    }

    #[staticmethod]
    fn with_font(font_data: Vec<u8>) -> Self {
        Self {
            inner: Mutex::new(Captioner::with_font(font_data)),
        }
    }

    #[pyo3(signature = (data, text, output_format = "same"))]
    fn caption_bytes<'py>(
        &self,
        py: Python<'py>,
        data: &[u8],
        text: &str,
        output_format: &str,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let format = parse_output_format(output_format)?;
        let mut captioner = self
            .inner
            .lock()
            .map_err(|_| PyValueError::new_err("captioner lock poisoned"))?;
        let out = captioner
            .caption_bytes(data, text, format)
            .map_err(caption_error)?;
        Ok(PyBytes::new(py, &out))
    }
}

#[pymodule]
fn caption(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyCaptioner>()?;
    Ok(())
}

fn parse_output_format(value: &str) -> PyResult<OutputFormat> {
    match value.to_ascii_lowercase().as_str() {
        "same" => Ok(OutputFormat::Same),
        "png" => Ok(OutputFormat::Png),
        "jpg" | "jpeg" => Ok(OutputFormat::Jpeg),
        "webp" => Ok(OutputFormat::WebP),
        "gif" => Ok(OutputFormat::Gif),
        other => Err(PyValueError::new_err(format!(
            "unsupported output_format {other:?}; expected same, png, jpeg, gif, or webp"
        ))),
    }
}

fn caption_error(error: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(error.to_string())
}
