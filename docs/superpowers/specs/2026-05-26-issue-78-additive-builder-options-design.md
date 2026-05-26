# Design: additive builder options for `clean()` and `Cleaner`

Closes #78 (partially; see "Status of issue 78").

## Problem

`clean()` and `Cleaner` expose set-valued options (`tags`, `clean_content_tags`,
`url_schemes`, `generic_attribute_prefixes`) that *replace* the ammonia defaults
when provided. Allowing one extra tag therefore requires copying the entire
default and unioning it:

```python
nh3.clean(html, tags=nh3.ALLOWED_TAGS | {"my-tag"})
```

That pattern is verbose and easy to get wrong (forgetting `deepcopy` for
mutable defaults, missing a copy step, etc.). Ammonia's Rust builder offers
paired `add_*` / `rm_*` methods that incrementally modify the defaults; nh3
does not currently surface them.

## Status of issue 78

The issue asks for two options:

- `add_tag` — add to the whitelist without overwriting the defaults.
- `filter_style_properties` — restrict the properties allowed in `style`
  attributes.

`filter_style_properties` is **already implemented** (`src/lib.rs` and
`tests/test_nh3.py::test_clean`). This design covers only the missing piece:
additive modifiers for the four simple set-valued options.

## Scope

Add eight new keyword arguments to `nh3.clean()`, `nh3.Cleaner()` and the
`nh3.pyi` stubs:

| Option                            | Type       | Effect                                          |
|-----------------------------------|------------|-------------------------------------------------|
| `add_tags`                        | `set[str]` | Extra tags appended to the effective whitelist  |
| `rm_tags`                         | `set[str]` | Tags removed from the effective whitelist       |
| `add_clean_content_tags`          | `set[str]` | Extra clean-content tags                        |
| `rm_clean_content_tags`           | `set[str]` | Clean-content tags removed                      |
| `add_url_schemes`                 | `set[str]` | Extra URL schemes                               |
| `rm_url_schemes`                  | `set[str]` | URL schemes removed                             |
| `add_generic_attribute_prefixes`  | `set[str]` | Extra generic attribute prefixes                |
| `rm_generic_attribute_prefixes`   | `set[str]` | Generic attribute prefixes removed              |

All eight default to `None` (no change).

### Out of scope

- `add_*` / `rm_*` for `attributes`, `allowed_classes`, `tag_attribute_values`,
  `set_tag_attribute_values`. Those are nested mappings; designing an ergonomic
  additive API is a separate exercise and not what the issue requests.
- Behavioural changes to the existing replacement options.

## Semantics

For each `(base, add, rm)` triple — for instance `(tags, add_tags, rm_tags)`:

1. If `base` is provided, it replaces the ammonia default (current behaviour).
2. Then `add` is applied on top of the resulting set (ammonia's `add_*`).
3. Then `rm` is applied (ammonia's `rm_*`).

Consequences:

- `clean(html, add_tags={"foo"})` keeps the entire default whitelist and adds
  `foo`.
- `clean(html, tags={"b"}, add_tags={"i"})` allows exactly `{b, i}`.
- `clean(html, rm_tags={"img"})` keeps the defaults minus `img`.
- `add` and `rm` overlap is resolved by ammonia (rm runs after add in our
  build step, so rm wins on conflict — we mirror ammonia's builder ordering).

## Implementation sketch

In `src/lib.rs`:

- Extend `Config` with eight new `Option<HashSet<String>>` fields.
- Extend `Cleaner::py_new` signature, `clean()` pyfunction signature, and
  `Config { … }` initialisation.
- In `build_ammonia_from_config`, after the existing `builder.tags(...)` /
  `builder.clean_content_tags(...)` / `builder.url_schemes(...)` /
  `builder.generic_attribute_prefixes(...)` calls, invoke ammonia's
  `add_*` then `rm_*` when the corresponding `Option` is `Some`.
- Extend the existing `clean_content_tags` overlap check (introduced by
  upstream PR #125 to guard against an ammonia panic) so that it computes
  the *effective* `tags` and `clean_content_tags` sets after applying the
  `add_*`/`rm_*` modifiers and verifies they are disjoint. Otherwise
  `add_tags={"p"}` plus `add_clean_content_tags={"p"}` would bypass the
  guard.

In `nh3.pyi`: add the eight parameters to both `Cleaner.__init__` and
`clean()` (using `AbstractSet` to match the existing stub style).

In `src/lib.rs` docstrings: document each new parameter (RST `:param:`
entries).

## Tests

Add cases in `tests/test_nh3.py::test_clean`:

- `add_tags` extends defaults: previously-stripped custom tag is preserved.
- `rm_tags` removes from defaults: a default-allowed tag (e.g. `b`) is stripped.
- `add_tags` combined with explicit `tags=`: behaves as union.
- `add_url_schemes` allows a custom scheme on a link.
- `rm_url_schemes` strips a default-allowed scheme.
- `add_clean_content_tags` wipes the content of an additional tag.
- `add_generic_attribute_prefixes` allows a custom data-style prefix.

## Backwards compatibility

All new parameters default to `None`. No existing call site changes behaviour.

## Docs

Add a short "Adding to the defaults" section to `docs/index.rst` showing
`add_tags={"my-tag"}` versus the old `tags=nh3.ALLOWED_TAGS | {"my-tag"}`
pattern, and mention the symmetric `rm_*` options.
