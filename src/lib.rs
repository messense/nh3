use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyString, PyTuple};

/// Clean HTML with a conservative set of defaults
#[pyfunction(signature = (
    html,
    tags = None,
    attributes = None,
    attribute_filter = None,
    strip_comments = true,
    link_rel = "noopener noreferrer",
))]
fn clean(
    py: Python,
    html: &str,
    tags: Option<HashSet<&str>>,
    attributes: Option<HashMap<&str, HashSet<&str>>>,
    attribute_filter: Option<PyObject>,
    strip_comments: bool,
    link_rel: Option<&str>,
) -> PyResult<String> {
    if let Some(callback) = attribute_filter.as_ref() {
        if !callback.as_ref(py).is_callable() {
            return Err(PyTypeError::new_err("attribute_filter must be callable"));
        }
    }

    let cleaned = py.allow_threads(|| {
        if tags.is_some()
            || attributes.is_some()
            || attribute_filter.is_some()
            || !strip_comments
            || link_rel != Some("noopener noreferrer")
        {
            let mut cleaner = ammonia::Builder::default();
            if let Some(tags) = tags {
                cleaner.tags(tags);
            }
            if let Some(mut attrs) = attributes {
                if let Some(generic_attrs) = attrs.remove("*") {
                    cleaner.generic_attributes(generic_attrs);
                }
                cleaner.tag_attributes(attrs);
            }
            if let Some(callback) = attribute_filter {
                cleaner.attribute_filter(move |element, attribute, value| {
                    Python::with_gil(|py| {
                        let res = callback.call(
                            py,
                            PyTuple::new(
                                py,
                                [
                                    PyString::new(py, element),
                                    PyString::new(py, attribute),
                                    PyString::new(py, value),
                                ],
                            ),
                            None,
                        );
                        let err = match res {
                            Ok(val) => {
                                if val.is_none(py) {
                                    return None;
                                } else if let Ok(s) = val.downcast::<PyString>(py) {
                                    match s.to_str() {
                                        Ok(s) => return Some(Cow::<str>::Owned(s.to_string())),
                                        Err(err) => err,
                                    }
                                } else {
                                    PyTypeError::new_err(
                                        "expected attribute_filter to return str or None",
                                    )
                                }
                            }
                            Err(err) => err,
                        };
                        err.restore(py);
                        Some(value.into())
                    })
                });
            }
            cleaner.strip_comments(strip_comments);
            cleaner.link_rel(link_rel);
            cleaner.clean(html).to_string()
        } else {
            ammonia::clean(html)
        }
    });

    Ok(cleaned)
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

    let a = ammonia::Builder::default();
    m.add("ALLOWED_TAGS", a.clone_tags())?;
    m.add("ALLOWED_ATTRIBUTES", a.clone_tag_attributes())?;
    Ok(())
}
