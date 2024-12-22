use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyString, PyTuple};

/// Sanitize an HTML fragment according to the given options.
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
/// :param generic_attribute_prefixes: Sets the prefix of attributes that are allowed on any tag.
/// :type generic_attribute_prefixes: ``set[str]``, optional
/// :param tag_attribute_values: Sets the values of HTML attributes that are allowed on specific tags.
///     The value is structured as a map from tag names to a map from attribute names to a set of attribute values.
///     If a tag is not itself whitelisted, adding entries to this map will do nothing.
/// :type tag_attribute_values: ``dict[str, dict[str, set[str]]]``, optional
/// :param set_tag_attribute_values: Sets the values of HTML attributes that are to be set on specific tags.
///     The value is structured as a map from tag names to a map from attribute names to an attribute value.
///     If a tag is not itself whitelisted, adding entries to this map will do nothing.
/// :type set_tag_attribute_values: ``dict[str, dict[str, str]]``, optional
/// :param url_schemes: Sets the URL schemes permitted on ``href`` and ``src`` attributes.
/// :type url_schemes: ``set[str]``, optional
/// :return: Sanitized HTML fragment
/// :rtype: ``str``
///
/// For example:
///
/// .. code-block:: pycon
///
///     >>> import nh3
///     >>> nh3.clean("<unknown>hi")
///     'hi'
///     >>> nh3.clean("<b><img src='' onerror='alert(\\'hax\\')'>XSS?</b>")
///     '<b><img src="">XSS?</b>'
///
/// Example of using ``attribute_filter``:
///
/// .. code-block:: pycon
///   
///    >>> from copy import deepcopy
///    >>> attributes = deepcopy(nh3.ALLOWED_ATTRIBUTES)
///    >>> attributes["a"].add("class")
///    >>> def attribute_filter(tag, attr, value):
///    ...     if tag == "a" and attr == "class":
///    ...         if "mention" in value.split(" "):
///    ...             return "mention"
///    ...         return None
///    ...     return value
///    >>> nh3.clean("<a class='mention unwanted'>@foo</a>", 
///    ...     attributes=attributes,
///    ...     attribute_filter=attribute_filter)
///    '<a class="mention" rel="noopener noreferrer">@foo</a>'
///
/// Example of maintaining the ``rel`` attribute:
///
/// .. code-block:: pycon
///
///    >>> from copy import deepcopy
///    >>> attributes = deepcopy(nh3.ALLOWED_ATTRIBUTES)
///    >>> attributes["a"].add("rel")
///    >>> nh3.clean("<a href='https://tag.example' rel='tag'>#tag</a>",
///    ...     link_rel=None, attributes=attributes)
///    '<a href="https://tag.example" rel="tag">#tag</a>'



#[pyfunction(signature = (
    html,
    tags = None,
    clean_content_tags = None,
    attributes = None,
    attribute_filter = None,
    strip_comments = true,
    link_rel = "noopener noreferrer",
    generic_attribute_prefixes = None,
    tag_attribute_values = None,
    set_tag_attribute_values = None,
    url_schemes = None,
))]
#[allow(clippy::too_many_arguments)]
fn clean(
    py: Python,
    html: &str,
    tags: Option<HashSet<String>>,
    clean_content_tags: Option<HashSet<String>>,
    mut attributes: Option<HashMap<String, HashSet<String>>>,
    attribute_filter: Option<PyObject>,
    strip_comments: bool,
    link_rel: Option<&str>,
    generic_attribute_prefixes: Option<HashSet<String>>,
    tag_attribute_values: Option<HashMap<String, HashMap<String, HashSet<String>>>>,
    set_tag_attribute_values: Option<HashMap<String, HashMap<String, String>>>,
    url_schemes: Option<HashSet<String>>,
) -> PyResult<String> {
    if let Some(callback) = attribute_filter.as_ref() {
        if !callback.bind(py).is_callable() {
            return Err(PyTypeError::new_err("attribute_filter must be callable"));
        }
    }

    let cleaned = py.allow_threads(|| {
        if tags.is_some()
            || clean_content_tags.is_some()
            || attributes.is_some()
            || attribute_filter.is_some()
            || !strip_comments
            || link_rel.as_deref() != Some("noopener noreferrer")
            || generic_attribute_prefixes.is_some()
            || tag_attribute_values.is_some()
            || set_tag_attribute_values.is_some()
            || url_schemes.is_some()
        {
            let generic_attrs = attributes.as_mut().and_then(|attrs| attrs.remove("*"));
            let mut cleaner = ammonia::Builder::default();
            if let Some(tags) = tags.as_ref() {
                let tags: HashSet<&str> = tags.iter().map(|s| s.as_str()).collect();
                cleaner.tags(tags);
            }
            if let Some(tags) = clean_content_tags.as_ref() {
                let tags: HashSet<&str> = tags.iter().map(|s| s.as_str()).collect();
                cleaner.clean_content_tags(tags);
            }
            if let Some(attrs) = attributes.as_ref() {
                let attrs: HashMap<&str, HashSet<&str>> = attrs
                    .iter()
                    .map(|(k, v)| (k.as_str(), v.iter().map(|s| s.as_str()).collect()))
                    .collect();
                cleaner.tag_attributes(attrs);
                if let Some(generic_attrs) = generic_attrs.as_ref() {
                    let generic_attrs: HashSet<&str> =
                        generic_attrs.iter().map(|s| s.as_str()).collect();
                    cleaner.generic_attributes(generic_attrs);
                }
            }
            if let Some(prefixes) = generic_attribute_prefixes.as_ref() {
                let prefixes: HashSet<&str> = prefixes.iter().map(|s| s.as_str()).collect();
                cleaner.generic_attribute_prefixes(prefixes);
            }
            if let Some(values) = tag_attribute_values.as_ref() {
                let values: HashMap<&str, HashMap<&str, HashSet<&str>>> = values
                    .iter()
                    .map(|(tag, attrs)| {
                        let inner: HashMap<&str, HashSet<&str>> = attrs
                            .iter()
                            .map(|(attr, vals)| {
                                (attr.as_str(), vals.iter().map(|v| v.as_str()).collect())
                            })
                            .collect();
                        (tag.as_str(), inner)
                    })
                    .collect();
                cleaner.tag_attribute_values(values);
            }
            if let Some(values) = set_tag_attribute_values.as_ref() {
                let values: HashMap<&str, HashMap<&str, &str>> = values
                    .iter()
                    .map(|(tag, attrs)| {
                        let inner: HashMap<&str, &str> = attrs
                            .iter()
                            .map(|(attr, val)| (attr.as_str(), val.as_str()))
                            .collect();
                        (tag.as_str(), inner)
                    })
                    .collect();
                cleaner.set_tag_attribute_values(values);
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
                            )
                            .unwrap(),
                            None,
                        );
                        let err = match res {
                            Ok(val) => {
                                if val.is_none(py) {
                                    return None;
                                } else if let Ok(s) = val.extract::<String>(py) {
                                    return Some(Cow::<str>::Owned(s));
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
                            Some(
                                &PyTuple::new(
                                    py,
                                    [
                                        PyString::new(py, element),
                                        PyString::new(py, attribute),
                                        PyString::new(py, value),
                                    ],
                                )
                                .unwrap(),
                            ),
                        );
                        Some(value.into())
                    })
                });
            }
            cleaner.strip_comments(strip_comments);
            cleaner.link_rel(link_rel.as_deref());
            if let Some(url_schemes) = url_schemes.as_ref() {
                let url_schemes: HashSet<_> = url_schemes.iter().map(|s| s.as_str()).collect();
                cleaner.url_schemes(url_schemes);
            }
            cleaner.clean(html).to_string()
        } else {
            ammonia::clean(html)
        }
    });

    Ok(cleaned)
}

/// Turn an arbitrary string into unformatted HTML.
///
/// Roughly equivalent to Python’s html.escape() or PHP’s htmlspecialchars and
/// htmlentities. Escaping is as strict as possible, encoding every character
/// that has special meaning to the HTML parser.
///
/// :param html: Input HTML fragment
/// :type html: ``str``
/// :return: Cleaned text
/// :rtype: ``str``
///
/// For example:
///
/// .. code-block:: pycon
///
///      >>> import nh3
///      >>> nh3.clean_text('Robert"); abuse();//')
///      'Robert&quot;);&#32;abuse();&#47;&#47;'
#[pyfunction]
fn clean_text(py: Python, html: &str) -> String {
    py.allow_threads(|| ammonia::clean_text(html))
}

/// Determine if a given string contains HTML.
///
/// This function parses the full string and checks for any HTML syntax.
///
/// Note: This function will return True for strings that contain invalid HTML syntax
/// like ``<g>`` and even ``Vec::<u8>::new()``.
///
/// :param html: Input string
/// :type html: ``str``
/// :rtype: ``bool``
///
/// For example:
///
/// .. code-block:: pycon
///
///     >>> nh3.is_html("plain text")
///     False
///     >>> nh3.is_html("<p>html!</p>")
///     True
#[pyfunction]
fn is_html(py: Python, html: &str) -> bool {
    py.allow_threads(|| ammonia::is_html(html))
}

/// Python bindings to the ammonia HTML sanitization library ( https://github.com/rust-ammonia/ammonia ).
#[pymodule(gil_used = false)]
fn nh3(_py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_function(wrap_pyfunction!(clean, m)?)?;
    m.add_function(wrap_pyfunction!(clean_text, m)?)?;
    m.add_function(wrap_pyfunction!(is_html, m)?)?;

    let a = ammonia::Builder::default();
    m.add("ALLOWED_TAGS", a.clone_tags())?;
    m.add("ALLOWED_ATTRIBUTES", a.clone_tag_attributes())?;
    m.add("ALLOWED_URL_SCHEMES", a.clone_url_schemes())?;
    Ok(())
}
