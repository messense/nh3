use pyo3::prelude::*;

/// Clean HTML with a conservative set of defaults
#[pyfunction]
fn clean(py: Python, html: &str) -> String {
    py.allow_threads(|| ammonia::clean(html))
}

/// Turn an arbitrary string into unformatted HTML
///
/// This function is roughly equivalent to PHP’s htmlspecialchars and htmlentities.
/// It is as strict as possible, encoding every character that has special meaning to the HTML parser.

#[pyfunction]
fn clean_text(py: Python, html: &str) -> String {
    py.allow_threads(|| ammonia::clean_text(html))
}

/// Python binding to the ammonia HTML sanitizer crate
#[pymodule]
fn nh3(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_function(wrap_pyfunction!(clean, m)?)?;
    m.add_function(wrap_pyfunction!(clean_text, m)?)?;
    Ok(())
}
