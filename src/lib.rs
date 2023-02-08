use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyString, PyTuple};

/// Sanitizes an HTML fragment in a string according to the configured options.
///
/// :param html: Input HTML fragment
/// :type html: ``str``
/// :param tags: Sets the tags that are allowed.
/// :type tags: ``set[str]``, optional
/// :param clean_content_tags: Sets the tags whose contents will be completely removed from the output.
/// :type clean_content_tags: ``set[str]``, optional
/// :param attributes: Sets the HTML attributes that are allowed on specific tags,
///    ``*`` key means the attributes are allowed on any tag.
/// :type attributes: ``dict[str, set[str]]``, optional
/// :param attribute_filter: Allows rewriting of all attributes using a callback.
///     The callback takes name of the element, attribute and its value.
///     Returns ``None`` to remove the attribute, or a value to use.
/// :type attribute_filter: ``Callable[[str, str, str], str | None]``, optional
/// :param strip_comments: Configures the handling of HTML comments, defaults to ``True``.
/// :type strip_comments: ``bool``
/// :param link_rel: Configures a ``rel`` attribute that will be added on links, defaults to ``noopener noreferrer``.
///     To turn on rel-insertion, pass a space-separated list.
///     If ``rel`` is in the generic or tag attributes, this must be set to ``None``. Common ``rel`` values to include:
///
///     - ``noopener``: This prevents a particular type of XSS attack, and should usually be turned on for untrusted HTML.
///     - ``noreferrer``: This prevents the browser from sending the source URL to the website that is linked to.
///     - ``nofollow``: This prevents search engines from using this link for ranking, which disincentivizes spammers.
/// :type link_rel: ``str``
/// :return: Sanitized HTML fragment
/// :rtype: ``str``
#[pyfunction(signature = (
    html,
    tags = None,
    clean_content_tags = None,
    attributes = None,
    attribute_filter = None,
    strip_comments = true,
    link_rel = "noopener noreferrer",
))]
fn clean(
    py: Python,
    html: &str,
    tags: Option<HashSet<&str>>,
    clean_content_tags: Option<HashSet<&str>>,
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
            || clean_content_tags.is_some()
            || attributes.is_some()
            || attribute_filter.is_some()
            || !strip_comments
            || link_rel != Some("noopener noreferrer")
        {
            let mut cleaner = ammonia::Builder::default();
            if let Some(tags) = tags {
                cleaner.tags(tags);
            }
            if let Some(tags) = clean_content_tags {
                cleaner.clean_content_tags(tags);
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
                        err.write_unraisable(
                            py,
                            Some(PyTuple::new(
                                py,
                                [
                                    PyString::new(py, element),
                                    PyString::new(py, attribute),
                                    PyString::new(py, value),
                                ],
                            )),
                        );
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
///
/// :param html: Input HTML fragment
/// :type html: ``str``
/// :return: Cleaned text
/// :rtype: ``str``
#[pyfunction]
fn clean_text(py: Python, html: &str) -> String {
    py.allow_threads(|| ammonia::clean_text(html))
}

/// Determine if a given string contains HTML
///
/// This function is parses the full string into HTML and checks if the input contained any HTML syntax.
///
/// Note: This function will return positively for strings that contain invalid HTML syntax
/// like ``<g>`` and even ``Vec::<u8>::new()``.
///
/// :param html: Input string
/// :type html: ``str``
/// :rtype: ``bool``
#[pyfunction]
fn is_html(py: Python, html: &str) -> bool {
    py.allow_threads(|| ammonia::is_html(html))
}

/// Python binding to the `ammonia <https://github.com/rust-ammonia/ammonia>`_ HTML sanitizer Rust crate.
#[pymodule]
fn nh3(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_function(wrap_pyfunction!(clean, m)?)?;
    m.add_function(wrap_pyfunction!(clean_text, m)?)?;
    m.add_function(wrap_pyfunction!(is_html, m)?)?;

    let a = ammonia::Builder::default();
    m.add("ALLOWED_TAGS", a.clone_tags())?;
    m.add("ALLOWED_ATTRIBUTES", a.clone_tag_attributes())?;
    Ok(())
}
