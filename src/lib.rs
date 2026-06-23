use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use ouroboros::self_referencing;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyString, PyTuple};

/// Internal representation of the parsed `url_relative` keyword argument.
///
/// Parsing and validation happen eagerly when the `Cleaner` is constructed; this
/// enum is the validated result that gets converted to `ammonia::UrlRelative` in
/// `build_ammonia_from_config`.
enum UrlRelativeConfig {
    PassThrough,
    Deny,
    RewriteWithBase(ammonia::Url),
    RewriteWithRoot { root: ammonia::Url, path: String },
    Custom(Py<PyAny>),
}

struct Config {
    tags: Option<HashSet<String>>,
    clean_content_tags: Option<HashSet<String>>,
    attributes: Option<HashMap<String, HashSet<String>>>,
    attribute_filter: Option<Py<PyAny>>,
    strip_comments: bool,
    link_rel: Option<String>,
    generic_attribute_prefixes: Option<HashSet<String>>,
    tag_attribute_values: Option<HashMap<String, HashMap<String, HashSet<String>>>>,
    set_tag_attribute_values: Option<HashMap<String, HashMap<String, String>>>,
    url_schemes: Option<HashSet<String>>,
    allowed_classes: Option<HashMap<String, HashSet<String>>>,
    filter_style_properties: Option<HashSet<String>>,
    url_relative: Option<UrlRelativeConfig>,
    id_prefix: Option<String>,
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
            url_relative: None,
            id_prefix: None,
        }
    }
}

/// Parse the Python `url_relative` argument into a validated [`UrlRelativeConfig`].
///
/// Accepts the strings ``"pass_through"`` / ``"deny"``, the tuples
/// ``("rewrite_with_base", base_url)`` / ``("rewrite_with_root", root_url, path)``,
/// or a callable. Any other value raises ``ValueError`` (bad mode / unparseable
/// URL / malformed tuple) or ``TypeError`` (unsupported type).
fn parse_url_relative(obj: &Bound<'_, PyAny>) -> PyResult<UrlRelativeConfig> {
    if obj.cast::<PyString>().is_ok() {
        let s: String = obj.extract()?;
        return match s.as_str() {
            "pass_through" => Ok(UrlRelativeConfig::PassThrough),
            "deny" => Ok(UrlRelativeConfig::Deny),
            other => Err(PyValueError::new_err(format!(
                "invalid url_relative string {other:?}; expected \"pass_through\" or \"deny\""
            ))),
        };
    }
    if let Ok(tuple) = obj.cast::<PyTuple>() {
        let mode: String = tuple
            .get_item(0)
            .map_err(|_| PyValueError::new_err("url_relative tuple must not be empty"))?
            .extract()
            .map_err(|_| PyValueError::new_err("url_relative tuple mode must be a string"))?;
        return match mode.as_str() {
            "rewrite_with_base" => {
                if tuple.len() != 2 {
                    return Err(PyValueError::new_err(
                        "url_relative (\"rewrite_with_base\", base_url) expects exactly 2 elements",
                    ));
                }
                let base: String = tuple.get_item(1)?.extract().map_err(|_| {
                    PyValueError::new_err(
                        "url_relative rewrite_with_base base_url must be a string",
                    )
                })?;
                let url = ammonia::Url::parse(&base).map_err(|e| {
                    PyValueError::new_err(format!("invalid url_relative base URL {base:?}: {e}"))
                })?;
                Ok(UrlRelativeConfig::RewriteWithBase(url))
            }
            "rewrite_with_root" => {
                if tuple.len() != 3 {
                    return Err(PyValueError::new_err(
                        "url_relative (\"rewrite_with_root\", root_url, path) expects exactly 3 elements",
                    ));
                }
                let root_url: String = tuple.get_item(1)?.extract().map_err(|_| {
                    PyValueError::new_err(
                        "url_relative rewrite_with_root root_url must be a string",
                    )
                })?;
                let path: String = tuple.get_item(2)?.extract().map_err(|_| {
                    PyValueError::new_err("url_relative rewrite_with_root path must be a string")
                })?;
                let root = ammonia::Url::parse(&root_url).map_err(|e| {
                    PyValueError::new_err(format!(
                        "invalid url_relative root URL {root_url:?}: {e}"
                    ))
                })?;
                Ok(UrlRelativeConfig::RewriteWithRoot { root, path })
            }
            other => Err(PyValueError::new_err(format!(
                "invalid url_relative mode {other:?}; expected \"rewrite_with_base\" or \"rewrite_with_root\""
            ))),
        };
    }
    if obj.is_callable() {
        return Ok(UrlRelativeConfig::Custom(obj.clone().unbind()));
    }
    Err(PyTypeError::new_err(
        "url_relative must be a string, a tuple, or a callable",
    ))
}

#[self_referencing]
struct Inner {
    config: Config,
    #[borrows(config)]
    #[not_covariant]
    builder: ammonia::Builder<'this>,
}

/// Create a reusable sanitizer according to the given options.
///
/// :param tags: Sets the tags that are allowed.
/// :type tags: ``set[str]``, optional
/// :param clean_content_tags: Sets the tags whose contents will be completely removed from the output.
///     Must be disjoint from ``tags`` (or the default allowed set when ``tags``
///     is omitted); a tag cannot be both kept and have its content stripped.
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
///
///     This map is an *alternate* to the entries of ``attributes`` (and ``attributes["*"]``):
///     if the same attribute is also whitelisted there for the same tag, every value is
///     accepted and this per-value whitelist is ignored for that attribute. To actually
///     restrict the allowed values, whitelist the tag but do **not** also list the
///     attribute in ``attributes``.
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
/// :param url_relative: Configures how relative URLs (in ``href`` / ``src`` /
///     ``<object data=...>``) are handled. Defaults to ``None`` (pass relative
///     URLs through unchanged). Accepted values:
///
///     - ``"pass_through"``: keep relative URLs unchanged (explicit default).
///     - ``"deny"``: strip relative URLs entirely.
///     - ``("rewrite_with_base", base_url)``: resolve relative URLs against ``base_url``.
///     - ``("rewrite_with_root", root_url, path)``: force paths into a directory.
///     - a callable ``(url) -> str | None``: rewrite relative URLs; return a
///       string to replace, or ``None`` to strip. A callback that raises (or
///       returns a non-string, non-``None`` value) strips the URL, and the error
///       is reported via ``sys.unraisablehook``.
/// :type url_relative: ``str | tuple | Callable[[str], str | None]``, optional
/// :param id_prefix: Prepends the given string to every allowed ``id`` attribute value,
///     which helps avoid collisions with ``id``\ s already present on the host page.
///     The tag and the ``id`` attribute must still be whitelisted (via ``attributes``);
///     values that already start with the prefix are left unchanged. Defaults to ``None``.
/// :type id_prefix: ``str``, optional
///
/// Example usage:
///
/// .. code-block:: pycon
///
///    >>> import nh3
///    >>> cleaner = nh3.Cleaner(tags={"b", "i"}, attributes={})
///    >>> cleaner.clean("<b><i>safe</i></b><script>xss</script>")
///    '<b><i>safe</i></b>'
///    >>> cleaner.clean("<b>another</b> <em>fragment</em>")
///    '<b>another</b> fragment'
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
            .map(|f| Python::attach(|py| f.clone_ref(py)));
        if let Some(callback) = attribute_filter {
            builder.attribute_filter(move |element, attribute, value| {
                Python::attach(|py| {
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
                            } else {
                                match val.extract::<String>(py) {
                                    Ok(s) => {
                                        return Some(Cow::<str>::Owned(s));
                                    }
                                    _ => PyTypeError::new_err(
                                        "expected attribute_filter to return str or None",
                                    ),
                                }
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
        if let Some(url_relative) = config.url_relative.as_ref() {
            let value = match url_relative {
                UrlRelativeConfig::PassThrough => ammonia::UrlRelative::PassThrough,
                UrlRelativeConfig::Deny => ammonia::UrlRelative::Deny,
                UrlRelativeConfig::RewriteWithBase(url) => {
                    ammonia::UrlRelative::RewriteWithBase(url.clone())
                }
                UrlRelativeConfig::RewriteWithRoot { root, path } => {
                    ammonia::UrlRelative::RewriteWithRoot {
                        root: root.clone(),
                        path: path.clone(),
                    }
                }
                UrlRelativeConfig::Custom(callback) => {
                    let callback = Python::attach(|py| callback.clone_ref(py));
                    // Help the compiler infer the higher-ranked `Fn` bound that
                    // `UrlRelative::Custom` requires: the closure only ever returns
                    // owned/None values, so without this it cannot tie the output
                    // lifetime to the input `&str`.
                    fn constrain<F>(f: F) -> F
                    where
                        F: for<'a> Fn(&'a str) -> Option<Cow<'a, str>> + Send + Sync + 'static,
                    {
                        f
                    }
                    let evaluate = constrain(move |url: &str| {
                        Python::attach(|py| {
                            let res = callback.call1(py, (url,));
                            let err = match res {
                                Ok(val) => {
                                    if val.is_none(py) {
                                        return None;
                                    }
                                    match val.extract::<String>(py) {
                                        Ok(s) => return Some(Cow::Owned(s)),
                                        Err(_) => PyTypeError::new_err(
                                            "expected url_relative callback to return str or None",
                                        ),
                                    }
                                }
                                Err(err) => err,
                            };
                            // A failing or mistyped callback strips the URL, keeping
                            // clean() infallible (unlike attribute_filter, which
                            // preserves the original value on error).
                            err.write_unraisable(py, None);
                            None
                        })
                    });
                    ammonia::UrlRelative::Custom(Box::new(evaluate))
                }
            };
            builder.url_relative(value);
        }
        if let Some(id_prefix) = config.id_prefix.as_ref() {
            builder.id_prefix(Some(id_prefix.as_str()));
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
        filter_style_properties = None,
        url_relative = None,
        id_prefix = None
    ))]
    fn py_new(
        py: Python,
        tags: Option<HashSet<String>>,
        clean_content_tags: Option<HashSet<String>>,
        attributes: Option<HashMap<String, HashSet<String>>>,
        attribute_filter: Option<Py<PyAny>>,
        strip_comments: bool,
        link_rel: Option<&str>,
        generic_attribute_prefixes: Option<HashSet<String>>,
        tag_attribute_values: Option<HashMap<String, HashMap<String, HashSet<String>>>>,
        set_tag_attribute_values: Option<HashMap<String, HashMap<String, String>>>,
        url_schemes: Option<HashSet<String>>,
        allowed_classes: Option<HashMap<String, HashSet<String>>>,
        filter_style_properties: Option<HashSet<String>>,
        url_relative: Option<Py<PyAny>>,
        id_prefix: Option<String>,
    ) -> PyResult<Self> {
        if let Some(callback) = attribute_filter.as_ref() {
            if !callback.bind(py).is_callable() {
                return Err(PyTypeError::new_err("attribute_filter must be callable"));
            }
        }
        let url_relative = match url_relative {
            Some(obj) => Some(parse_url_relative(obj.bind(py))?),
            None => None,
        };
        if link_rel.is_some() {
            if let Some(ref attrs) = attributes {
                for (tag, attr_set) in attrs.iter() {
                    if attr_set.contains("rel") {
                        return Err(PyValueError::new_err(format!(
                            "\"rel\" attribute is not allowed for tag \"{}\" when link_rel is set; \
                             pass link_rel=None to manage the \"rel\" attribute directly",
                            tag
                        )));
                    }
                }
            }
        }
        if let Some(ref clean_tags) = clean_content_tags {
            // A tag listed in both the allowed `tags` set and `clean_content_tags`
            // makes ammonia panic. Raise an explicit ValueError instead. When the
            // caller omits `tags`, ammonia falls back to its default allowed set,
            // so check against that default in order to catch e.g.
            // `clean_content_tags={"p"}`.
            let conflict = match tags.as_ref() {
                Some(allowed) => clean_tags.iter().find(|t| allowed.contains(t.as_str())),
                None => {
                    let default_tags = ammonia::Builder::default().clone_tags();
                    clean_tags
                        .iter()
                        .find(|t| default_tags.contains(t.as_str()))
                }
            };
            if let Some(tag) = conflict {
                return Err(PyValueError::new_err(format!(
                    "tag \"{}\" cannot appear in both `tags` and `clean_content_tags`; \
                     either remove it from `clean_content_tags` or pass an explicit \
                     `tags` set that excludes it",
                    tag
                )));
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
            url_relative,
            id_prefix,
        };
        Ok(Self::new(config))
    }

    /// Sanitize an HTML fragment
    #[pyo3(name = "clean")]
    fn py_clean(&self, py: Python, html: &str) -> PyResult<String> {
        Ok(py.detach(|| self.clean(html)))
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
/// Use ``tags`` to only allow certain HTML tags:
///
/// .. code-block:: pycon
///
///    >>> nh3.clean("<b><i>bold italic</i></b>", tags={"b"})
///    '<b>bold italic</b>'
///
/// ``clean_content_tags`` removes both the tag and its content, unlike ``tags``
/// which strips the tag but keeps the text:
///
/// .. code-block:: pycon
///
///    >>> nh3.clean(
///    ...     "<script>alert('xss')</script>safe",
///    ...     clean_content_tags={"script"},
///    ... )
///    'safe'
///
/// The ``attributes`` parameter controls which attributes are kept per tag.
/// Use ``"*"`` as a key to allow an attribute on all tags:
///
/// .. code-block:: pycon
///
///    >>> nh3.clean(
///    ...     '<a href="/" id="link">click</a>',
///    ...     attributes={"*": {"id"}, "a": {"href"}},
///    ... )
///    '<a href="/" id="link" rel="noopener noreferrer">click</a>'
///
/// ``tag_attribute_values`` restricts an attribute to a set of allowed values
/// (values outside the set cause the attribute to be stripped), while
/// ``set_tag_attribute_values`` unconditionally adds attributes. Note that
/// ``tag_attribute_values`` is an *alternate* to ``attributes`` — if the same
/// attribute is also whitelisted in ``attributes`` for that tag, every value
/// is allowed and the per-value whitelist is ignored:
///
/// .. code-block:: pycon
///
///    >>> nh3.clean(
///    ...     "<div role='alert'>warning</div>",
///    ...     tag_attribute_values={"div": {"role": {"alert", "status"}}},
///    ... )
///    '<div role="alert">warning</div>'
///    >>> nh3.clean(
///    ...     "<div role='banner'>warning</div>",
///    ...     tag_attribute_values={"div": {"role": {"alert", "status"}}},
///    ... )
///    '<div>warning</div>'
///    >>> nh3.clean(
///    ...     "<div>content</div>",
///    ...     set_tag_attribute_values={"div": {"class": "safe"}},
///    ... )
///    '<div class="safe">content</div>'
///
/// ``allowed_classes`` filters CSS class names per tag:
///
/// .. code-block:: pycon
///
///    >>> nh3.clean(
///    ...     '<span class="highlight bold">text</span>',
///    ...     allowed_classes={"span": {"highlight"}},
///    ... )
///    '<span class="highlight">text</span>'
///
/// To filter individual ``style`` properties, first allow the ``style``
/// attribute, then use ``filter_style_properties``:
///
/// .. code-block:: pycon
///
///    >>> nh3.clean(
///    ...     '<span style="color: red; position: fixed">text</span>',
///    ...     attributes={"span": {"style"}},
///    ...     filter_style_properties={"color"},
///    ... )
///    '<span style="color:red">text</span>'
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
///
/// ``url_relative`` controls how relative URLs are handled. ``"deny"`` strips
/// them, while ``("rewrite_with_base", base)`` resolves them against a base URL:
///
/// .. code-block:: pycon
///
///    >>> nh3.clean('<a href="/foo">x</a>', url_relative="deny")
///    '<a rel="noopener noreferrer">x</a>'
///    >>> nh3.clean(
///    ...     '<a href="/foo">x</a>',
///    ...     url_relative=("rewrite_with_base", "https://example.com"),
///    ... )
///    '<a href="https://example.com/foo" rel="noopener noreferrer">x</a>'
///
/// A callable rewrites relative URLs (return ``None`` to strip):
///
/// .. code-block:: pycon
///
///    >>> nh3.clean(
///    ...     '<img src="/a.png">',
///    ...     url_relative=lambda url: f"https://cdn.example.com{url}",
///    ... )
///    '<img src="https://cdn.example.com/a.png">'
///
/// ``id_prefix`` namespaces ``id`` attributes (which must be whitelisted) so they
/// cannot collide with ``id``\ s on the surrounding page:
///
/// .. code-block:: pycon
///
///    >>> nh3.clean(
///    ...     '<b id="x">hi</b>',
///    ...     attributes={"b": {"id"}},
///    ...     id_prefix="user-content-",
///    ... )
///    '<b id="user-content-x">hi</b>'

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
    filter_style_properties = None,
    url_relative = None,
    id_prefix = None
))]
#[allow(clippy::too_many_arguments)]
fn clean(
    py: Python,
    html: &str,
    tags: Option<HashSet<String>>,
    clean_content_tags: Option<HashSet<String>>,
    attributes: Option<HashMap<String, HashSet<String>>>,
    attribute_filter: Option<Py<PyAny>>,
    strip_comments: bool,
    link_rel: Option<&str>,
    generic_attribute_prefixes: Option<HashSet<String>>,
    tag_attribute_values: Option<HashMap<String, HashMap<String, HashSet<String>>>>,
    set_tag_attribute_values: Option<HashMap<String, HashMap<String, String>>>,
    url_schemes: Option<HashSet<String>>,
    allowed_classes: Option<HashMap<String, HashSet<String>>>,
    filter_style_properties: Option<HashSet<String>>,
    url_relative: Option<Py<PyAny>>,
    id_prefix: Option<String>,
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
        url_relative,
        id_prefix,
    )?;
    Ok(py.detach(|| cleaner.clean(html)))
}

/// Turn an arbitrary string into unformatted HTML.
///
/// Also exposed as :func:`escape`, which is the preferred name — the function escapes
/// input rather than sanitizing HTML.
///
/// Roughly equivalent to Python's html.escape() or PHP's htmlspecialchars and
/// htmlentities. Escaping is as strict as possible, encoding every character
/// that has special meaning to the HTML parser.
///
/// If ``tags`` is given, those tags are passed through with no attributes;
/// everything else is stripped (content kept). Behaves like :func:`clean`
/// with ``attributes={}`` restricted to the given tag set.
///
/// :param html: Input HTML fragment
/// :type html: ``str``
/// :param tags: Tags to preserve; when omitted the string is fully escaped.
/// :type tags: ``set[str]``, optional
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
///      >>> nh3.clean_text('<span>hello <mention>moto</mention>!</span>', tags={'mention'})
///      'hello <mention>moto</mention>!'
#[pyfunction(signature = (html, tags = None))]
fn clean_text(py: Python, html: &str, tags: Option<HashSet<String>>) -> String {
    match tags {
        None => py.detach(|| ammonia::clean_text(html)),
        Some(tags) => {
            let config = Config {
                tags: Some(tags),
                attributes: Some(HashMap::new()),
                link_rel: None,
                ..Default::default()
            };
            let cleaner = Cleaner::new(config);
            py.detach(|| cleaner.clean(html))
        }
    }
}

/// HTML-escape an arbitrary string.
///
/// Alias for :func:`clean_text` — same signature, same behaviour. The ``escape`` name
/// is preferred because the function escapes input rather than sanitizing HTML.
///
/// Note: this is stricter than Python's stdlib :func:`html.escape`. ``html.escape``
/// only encodes ``&``, ``<``, ``>``, and optionally ``"`` and ``'``; ``nh3.escape``
/// encodes every character that has special meaning to the HTML parser.
///
/// If ``tags`` is given, those tags are passed through with no attributes; everything
/// else is stripped (content kept). Behaves like :func:`clean` with ``attributes={}``
/// restricted to the given tag set.
///
/// :param html: Input HTML fragment
/// :type html: ``str``
/// :param tags: Tags to preserve; when omitted the string is fully escaped.
/// :type tags: ``set[str]``, optional
/// :return: Escaped text
/// :rtype: ``str``
///
/// For example:
///
/// .. code-block:: pycon
///
///      >>> import nh3
///      >>> nh3.escape('Robert"); abuse();//')
///      'Robert&quot;);&#32;abuse();&#47;&#47;'
///      >>> nh3.escape('<span>hello <mention>moto</mention>!</span>', tags={'mention'})
///      'hello <mention>moto</mention>!'
#[pyfunction(signature = (html, tags = None))]
fn escape(py: Python, html: &str, tags: Option<HashSet<String>>) -> String {
    clean_text(py, html, tags)
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
    py.detach(|| ammonia::is_html(html))
}

/// Python bindings to the ammonia HTML sanitization library ( https://github.com/rust-ammonia/ammonia ).
#[pymodule(gil_used = false)]
fn nh3(_py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_function(wrap_pyfunction!(clean, m)?)?;
    m.add_function(wrap_pyfunction!(clean_text, m)?)?;
    m.add_function(wrap_pyfunction!(escape, m)?)?;
    m.add_function(wrap_pyfunction!(is_html, m)?)?;
    m.add_class::<Cleaner>()?;

    let a = ammonia::Builder::default();
    m.add("ALLOWED_TAGS", a.clone_tags())?;
    m.add("ALLOWED_ATTRIBUTES", a.clone_tag_attributes())?;
    m.add("ALLOWED_URL_SCHEMES", a.clone_url_schemes())?;
    m.add("CLEAN_CONTENT_TAGS", a.clone_clean_content_tags())?;
    Ok(())
}
