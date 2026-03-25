from typing import AbstractSet, Callable, Dict, Mapping, Optional, Set

ALLOWED_TAGS: Set[str]
ALLOWED_ATTRIBUTES: Dict[str, Set[str]]
ALLOWED_URL_SCHEMES: Set[str]
CLEAN_CONTENT_TAGS: Set[str]

class Cleaner:
    def __init__(
        self,
        tags: Optional[AbstractSet[str]] = None,
        clean_content_tags: Optional[AbstractSet[str]] = None,
        attributes: Optional[Mapping[str, AbstractSet[str]]] = None,
        attribute_filter: Optional[Callable[[str, str, str], Optional[str]]] = None,
        strip_comments: bool = True,
        link_rel: Optional[str] = "noopener noreferrer",
        generic_attribute_prefixes: Optional[AbstractSet[str]] = None,
        tag_attribute_values: Optional[Mapping[str, Mapping[str, AbstractSet[str]]]] = None,
        set_tag_attribute_values: Optional[Mapping[str, Mapping[str, str]]] = None,
        url_schemes: Optional[AbstractSet[str]] = None,
        allowed_classes: Optional[Mapping[str, AbstractSet[str]]] = None,
        filter_style_properties: Optional[AbstractSet[str]] = None,
    ) -> None: ...
    def clean(self, html: str) -> str: ...

def clean(
    html: str,
    tags: Optional[AbstractSet[str]] = None,
    clean_content_tags: Optional[AbstractSet[str]] = None,
    attributes: Optional[Mapping[str, AbstractSet[str]]] = None,
    attribute_filter: Optional[Callable[[str, str, str], Optional[str]]] = None,
    strip_comments: bool = True,
    link_rel: Optional[str] = "noopener noreferrer",
    generic_attribute_prefixes: Optional[AbstractSet[str]] = None,
    tag_attribute_values: Optional[Mapping[str, Mapping[str, AbstractSet[str]]]] = None,
    set_tag_attribute_values: Optional[Mapping[str, Mapping[str, str]]] = None,
    url_schemes: Optional[AbstractSet[str]] = None,
    allowed_classes: Optional[Mapping[str, AbstractSet[str]]] = None,
    filter_style_properties: Optional[AbstractSet[str]] = None,
) -> str: ...
def clean_text(html: str) -> str: ...
def is_html(html: str) -> bool: ...
