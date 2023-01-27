from typing import Callable, Dict, Optional, Set

def clean(
    html: str,
    tags: Optional[Set[str]] = None,
    attributes: Optional[Dict[str, Set[str]]] = None,
    attribute_filter: Optional[Callable[[str, str, str]], Optional[str]] = None,
    strip_comments: bool = True,
    link_rel: Optional[str] = "noopener noreferrer",
) -> str: ...
def clean_text(html: str) -> str: ...
