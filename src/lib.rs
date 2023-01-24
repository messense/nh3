use std::collections::{HashMap, HashSet};

use pyo3::prelude::*;

/// Clean HTML with a conservative set of defaults
#[pyfunction(signature = (
    html,
    tags = None,
    attributes = None,
    strip_comments = true,
    link_rel = "noopener noreferrer",
))]
fn clean(
    py: Python,
    html: &str,
    tags: Option<HashSet<&str>>,
    attributes: Option<HashMap<&str, HashSet<&str>>>,
    strip_comments: bool,
    link_rel: Option<&str>,
) -> String {
    py.allow_threads(|| {
        if tags.is_some()
            || attributes.is_some()
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
            cleaner.strip_comments(strip_comments);
            cleaner.link_rel(link_rel);
            cleaner.clean(html).to_string()
        } else {
            ammonia::clean(html)
        }
    })
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
