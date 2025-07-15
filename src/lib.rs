use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use ouroboros::self_referencing;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyString, PyTuple};

struct Config {
    tags: Option<HashSet<String>>,
    clean_content_tags: Option<HashSet<String>>,
    attributes: Option<HashMap<String, HashSet<String>>>,
    attribute_filter: Option<PyObject>,
    strip_comments: bool,
    link_rel: Option<String>,
    generic_attribute_prefixes: Option<HashSet<String>>,
    tag_attribute_values: Option<HashMap<String, HashMap<String, HashSet<String>>>>,
    set_tag_attribute_values: Option<HashMap<String, HashMap<String, String>>>,
    url_schemes: Option<HashSet<String>>,
    allowed_classes: Option<HashMap<String, HashSet<String>>>,
    filter_style_properties: Option<HashSet<String>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tags: None,
            clean_content_tags: None,
            attributes: None,
            attribute_filter: None,
            strip_comments: true,
            link_rel: Some("noopener noreferrer".to_string()),
            generic_attribute_prefixes: None,
            tag_attribute_values: None,
            set_tag_attribute_values: None,
            url_schemes: None,
            allowed_classes: None,
            filter_style_properties: None,
        }
    }
}

#[self_referencing]
struct Inner {
    config: Config,
    #[borrows(config)]
    #[not_covariant]
    builder: ammonia::Builder<'this>,
}

#[pyclass]
pub struct Cleaner {
    inner: Inner,
}

impl Cleaner {
    fn new(config: Config) -> Self {
        let inner = InnerBuilder {
            config,
            builder_builder: |config| Self::build_ammonia_from_config(config),
        }
        .build();
        Self { inner }
    }

    fn build_ammonia_from_config(config: &Config) -> ammonia::Builder<'_> {
        let mut builder = ammonia::Builder::default();

        if let Some(tags) = config.tags.as_ref() {
            let tags: HashSet<&str> = tags.iter().map(|s| s.as_str()).collect();
            builder.tags(tags);
        }
        if let Some(tags) = config.clean_content_tags.as_ref() {
            let tags: HashSet<&str> = tags.iter().map(|s| s.as_str()).collect();
            builder.clean_content_tags(tags);
        }
        if let Some(attrs) = config.attributes.as_ref() {
            let attrs: HashMap<&str, HashSet<&str>> = attrs
                .iter()
                .filter(|(k, _)| k.as_str() != "*")
                .map(|(k, v)| (k.as_str(), v.iter().map(|s| s.as_str()).collect()))
                .collect();
            builder.tag_attributes(attrs);
            if let Some(generic_attrs) = config.attributes.as_ref().and_then(|a| a.get("*")) {
                let generic_attrs: HashSet<&str> =
                    generic_attrs.iter().map(|s| s.as_str()).collect();
                builder.generic_attributes(generic_attrs);
            }
        }
        if let Some(prefixes) = config.generic_attribute_prefixes.as_ref() {
            let prefixes: HashSet<&str> = prefixes.iter().map(|s| s.as_str()).collect();
            builder.generic_attribute_prefixes(prefixes);
        }
        if let Some(values) = config.tag_attribute_values.as_ref() {
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
            builder.tag_attribute_values(values);
        }
        if let Some(values) = config.set_tag_attribute_values.as_ref() {
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
            builder.set_tag_attribute_values(values);
        }
        let attribute_filter = config
            .attribute_filter
            .as_ref()
            .map(|f| Python::with_gil(|py| f.clone_ref(py)));
        if let Some(callback) = attribute_filter {
            builder.attribute_filter(move |element, attribute, value| {
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
        builder.strip_comments(config.strip_comments);
        builder.link_rel(config.link_rel.as_deref());
        if let Some(url_schemes) = config.url_schemes.as_ref() {
            let url_schemes: HashSet<_> = url_schemes.iter().map(|s| s.as_str()).collect();
            builder.url_schemes(url_schemes);
        }
        if let Some(allowed_classes) = config.allowed_classes.as_ref() {
            builder.allowed_classes(
                allowed_classes
                    .iter()
                    .map(|(tag, class_set)| {
                        (tag.as_str(), class_set.iter().map(|c| c.as_str()).collect())
                    })
                    .collect(),
            );
        }
        if let Some(filter_style_properties) = config.filter_style_properties.as_ref() {
            builder.filter_style_properties(
                filter_style_properties
                    .iter()
                    .map(|prop| prop.as_str())
                    .collect(),
            );
        }

        builder
    }

    pub fn clean(&self, html: &str) -> String {
        self.inner
            .with_builder(|builder| builder.clean(html).to_string())
    }
}

#[pymethods]
impl Cleaner {
    /// Create a reusable sanitizer according to the given options.
    ///
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
    /// :param allowed_classes: Sets the CSS classes that are allowed on specific tags.
    ///     The values is structured as a map from tag names to a set of class names.
    ///     The `class` attribute itself should not be whitelisted if this parameter is used.
    /// :type allowed_classes: ``dict[str, set[str]]``, optional
    /// :param filter_style_properties: Only allows the specified properties in `style` attributes.
    ///     Irrelevant if `style` is not an allowed attribute.
    ///     Note that if style filtering is enabled style properties will be normalised e.g.
    ///     invalid declarations and @rules will be removed, with only syntactically valid
    ///     declarations kept.
    /// :type filter_style_properties: ``set[str]``, optional
    #[new]
    #[pyo3(signature = (
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
        allowed_classes = None,
        filter_style_properties = None
    ))]
    fn py_new(
        py: Python,
        tags: Option<HashSet<String>>,
        clean_content_tags: Option<HashSet<String>>,
        attributes: Option<HashMap<String, HashSet<String>>>,
        attribute_filter: Option<PyObject>,
        strip_comments: bool,
        link_rel: Option<&str>,
        generic_attribute_prefixes: Option<HashSet<String>>,
        tag_attribute_values: Option<HashMap<String, HashMap<String, HashSet<String>>>>,
        set_tag_attribute_values: Option<HashMap<String, HashMap<String, String>>>,
        url_schemes: Option<HashSet<String>>,
        allowed_classes: Option<HashMap<String, HashSet<String>>>,
        filter_style_properties: Option<HashSet<String>>,
    ) -> PyResult<Self> {
        if let Some(callback) = attribute_filter.as_ref() {
            if !callback.bind(py).is_callable() {
                return Err(PyTypeError::new_err("attribute_filter must be callable"));
            }
        }
        let config = Config {
            tags,
            clean_content_tags,
            attributes,
            attribute_filter,
            strip_comments,
            link_rel: link_rel.map(|s| s.to_string()),
            generic_attribute_prefixes,
            tag_attribute_values,
            set_tag_attribute_values,
            url_schemes,
            allowed_classes,
            filter_style_properties,
        };
        Ok(Self::new(config))
    }

    /// Sanitize an HTML fragment
    #[pyo3(name = "clean")]
    fn py_clean(&self, py: Python, html: &str) -> PyResult<String> {
        Ok(py.allow_threads(|| self.clean(html)))
    }
}

/// Sanitize an HTML fragment according to the given options.
/// See ``Cleaner()`` for detailed sanitizer options.
///
/// :param html: Input HTML fragment
/// :type html: ``str``
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
    allowed_classes = None,
    filter_style_properties = None
))]
#[allow(clippy::too_many_arguments)]
fn clean(
    py: Python,
    html: &str,
    tags: Option<HashSet<String>>,
    clean_content_tags: Option<HashSet<String>>,
    attributes: Option<HashMap<String, HashSet<String>>>,
    attribute_filter: Option<PyObject>,
    strip_comments: bool,
    link_rel: Option<&str>,
    generic_attribute_prefixes: Option<HashSet<String>>,
    tag_attribute_values: Option<HashMap<String, HashMap<String, HashSet<String>>>>,
    set_tag_attribute_values: Option<HashMap<String, HashMap<String, String>>>,
    url_schemes: Option<HashSet<String>>,
    allowed_classes: Option<HashMap<String, HashSet<String>>>,
    filter_style_properties: Option<HashSet<String>>,
) -> PyResult<String> {
    let cleaner = Cleaner::py_new(
        py,
        tags,
        clean_content_tags,
        attributes,
        attribute_filter,
        strip_comments,
        link_rel,
        generic_attribute_prefixes,
        tag_attribute_values,
        set_tag_attribute_values,
        url_schemes,
        allowed_classes,
        filter_style_properties,
    )?;
    Ok(py.allow_threads(|| cleaner.clean(html)))
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
    m.add_class::<Cleaner>()?;

    let a = ammonia::Builder::default();
    m.add("ALLOWED_TAGS", a.clone_tags())?;
    m.add("ALLOWED_ATTRIBUTES", a.clone_tag_attributes())?;
    m.add("ALLOWED_URL_SCHEMES", a.clone_url_schemes())?;
    Ok(())
}
