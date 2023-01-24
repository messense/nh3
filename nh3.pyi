from typing import Dict, Optional, Set

def clean(
    html: str,
    tags: Optional[Set[str]] = None,
    attributes: Optional[Dict[str, Set[str]]] = None,
    strip_comments=True,
    link_rel="noopener noreferrer",
) -> str: ...
def clean_text(html: str) -> str: ...
