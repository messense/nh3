import nh3
import pytest


def test_clean():
    html = "<b><img src='' onerror='alert(\\'hax\\')'>I'm not trying to XSS you</b>"
    assert nh3.clean(html) == '<b><img src="">I\'m not trying to XSS you</b>'
    assert nh3.clean(html, tags={"img"}) == '<img src="">I\'m not trying to XSS you'
    assert (
        nh3.clean(html, tags={"img"}, attributes={}) == "<img>I'm not trying to XSS you"
    )
    assert nh3.clean(html, attributes={}) == "<b><img>I'm not trying to XSS you</b>"
    assert (
        nh3.clean('<a href="https://baidu.com">baidu</a>')
        == '<a href="https://baidu.com" rel="noopener noreferrer">baidu</a>'
    )
    assert (
        nh3.clean('<a href="https://baidu.com">baidu</a>', link_rel=None)
        == '<a href="https://baidu.com">baidu</a>'
    )
    assert (
        nh3.clean(
            "<script>alert('hello')</script><style>a { background: #fff }</style>",
            clean_content_tags={"script", "style"},
        )
        == ""
    )

    assert (
        nh3.clean('<div data-v="foo"></div>', generic_attribute_prefixes={"data-"})
        == '<div data-v="foo"></div>'
    )

    assert (
        nh3.clean(
            "<my-tag my-attr=val>",
            tags={"my-tag"},
            tag_attribute_values={"my-tag": {"my-attr": {"val"}}},
        )
        == '<my-tag my-attr="val"></my-tag>'
    )

    assert (
        nh3.clean(
            "<my-tag>",
            tags={"my-tag"},
            set_tag_attribute_values={"my-tag": {"my-attr": "val"}},
        )
        == '<my-tag my-attr="val"></my-tag>'
    )

    assert (
        nh3.clean(
            "<span class='a b c'><a href='.' class='c b a'>T</a></span><div class='a b c'>U</div>",
            allowed_classes={ 'a': {'b', 'c'}, 'span': {'a'} }
        )
        == '<span class="a"><a href="." class="c b" rel="noopener noreferrer">T</a></span><div>U</div>'
    )

    assert (
        nh3.clean(
            "<span style='color: red; position: fixed; font-size: var(--something)'>T</span><span style='border: none'></span><div style='color: red'></div>",
            filter_style_properties={'color', 'font-size'},
            attributes={'span': {'style'}}
        )
        == '<span style="color:red;font-size:var(--something)">T</span><span style=""></span><div></div>'
    )


def test_clean_with_attribute_filter():
    html = "<a href=/><img alt=Home src=foo></a>"

    def attribute_filter(element, attribute, value):
        if element == "img" and attribute == "src":
            return None
        return value

    assert (
        nh3.clean(html, attribute_filter=attribute_filter, link_rel=None)
        == '<a href="/"><img alt="Home"></a>'
    )

    with pytest.raises(TypeError):
        nh3.clean(html, attribute_filter="not a callable")

    # attribute_filter may raise exception, but it's an infallible API
    # which writes a unraisable exception
    nh3.clean(html, attribute_filter=lambda _element, _attribute, _value: True)


def test_clean_rel_attribute_conflict():
    with pytest.raises(ValueError, match="link_rel is set"):
        nh3.clean(
            "<a href='http://example.com'>test</a>",
            tags={"a"},
            attributes={"a": {"href", "rel"}},
        )

    # No error when link_rel=None
    result = nh3.clean(
        "<a href='http://example.com' rel='nofollow'>test</a>",
        tags={"a"},
        attributes={"a": {"href", "rel"}},
        link_rel=None,
    )
    assert result == '<a href="http://example.com" rel="nofollow">test</a>'

    # No error when rel is not in attributes
    nh3.clean(
        "<a href='http://example.com'>test</a>",
        tags={"a"},
        attributes={"a": {"href"}},
    )


def test_cleaner_rel_attribute_conflict():
    with pytest.raises(ValueError, match="link_rel is set"):
        nh3.Cleaner(
            tags={"a"},
            attributes={"a": {"href", "rel"}},
        )

    # No error when link_rel=None
    cleaner = nh3.Cleaner(
        tags={"a"},
        attributes={"a": {"href", "rel"}},
        link_rel=None,
    )
    result = cleaner.clean("<a href='http://example.com' rel='nofollow'>test</a>")
    assert result == '<a href="http://example.com" rel="nofollow">test</a>'


def test_clean_content_tags_overlap_with_default_tags():
    # Without explicit ``tags``, ammonia's default allowed tags are used; placing
    # any of those tags in ``clean_content_tags`` would otherwise panic the
    # interpreter. Validate up-front with a clear ValueError instead.
    with pytest.raises(ValueError, match="clean_content_tags"):
        nh3.clean("<p>hi</p>", clean_content_tags={"p"})

    with pytest.raises(ValueError, match="clean_content_tags"):
        nh3.clean("<div><b>hi</b></div>", clean_content_tags={"b", "script"})


def test_clean_content_tags_overlap_with_explicit_tags():
    # Explicit ``tags`` set that intersects ``clean_content_tags`` is also a
    # contradiction and must raise rather than panic.
    with pytest.raises(ValueError, match="clean_content_tags"):
        nh3.clean(
            "<div><b>hi</b></div>",
            tags={"div", "b"},
            clean_content_tags={"b"},
        )


def test_clean_content_tags_no_overlap_ok():
    # ``clean_content_tags`` works with tags absent from the allowed set
    # (default or explicit).
    assert nh3.clean("<script>x</script>safe", clean_content_tags={"script"}) == "safe"
    assert (
        nh3.clean(
            "<div><b>hi</b></div>",
            tags={"div"},
            clean_content_tags={"b"},
        )
        == "<div></div>"
    )


def test_cleaner_clean_content_tags_overlap():
    with pytest.raises(ValueError, match="clean_content_tags"):
        nh3.Cleaner(clean_content_tags={"p"})

    with pytest.raises(ValueError, match="clean_content_tags"):
        nh3.Cleaner(tags={"a"}, clean_content_tags={"a"})


def test_clean_text():
    res = nh3.clean_text('Robert"); abuse();//')
    assert res == "Robert&quot;);&#32;abuse();&#47;&#47;"

    res = nh3.clean_text(
        '<span>hello <mention>moto</mention>, welcome!</span>',
        tags={'mention'},
    )
    assert res == 'hello <mention>moto</mention>, welcome!'

    res = nh3.clean_text('<b>bold</b> and <i>italic</i>', tags={'b'})
    assert res == '<b>bold</b> and italic'

    res = nh3.clean_text(
        "<a href='http://example.com' rel='nofollow'>test</a>",
        tags={'a'},
    )
    assert res == '<a>test</a>'


def test_clean_content_tags_constant():
    assert isinstance(nh3.CLEAN_CONTENT_TAGS, set)
    assert "script" in nh3.CLEAN_CONTENT_TAGS
    assert "style" in nh3.CLEAN_CONTENT_TAGS


def test_frozenset_args():
    html = "<b><img src='x'>hello</b>"
    assert nh3.clean(html, tags=frozenset({"b"})) == "<b>hello</b>"
    assert (
        nh3.clean(html, tags=frozenset({"img"}), attributes={"img": frozenset({"src"})})
        == '<img src="x">hello'
    )


def test_cleaner_frozenset_args():
    cleaner = nh3.Cleaner(
        tags=frozenset({"b", "img"}),
        attributes={"img": frozenset({"src"})},
    )
    assert cleaner.clean("<b><img src='x'>hi</b>") == '<b><img src="x">hi</b>'


def test_clean_url_relative_pass_through_is_default():
    html = '<a href="/foo">x</a>'
    # Omitting url_relative keeps relative URLs (ammonia default), and the
    # explicit "pass_through" string must behave identically.
    assert nh3.clean(html) == '<a href="/foo" rel="noopener noreferrer">x</a>'
    assert nh3.clean(html, url_relative="pass_through") == nh3.clean(html)


def test_clean_url_relative_deny():
    # Relative URLs are stripped, absolute URLs are kept.
    assert (
        nh3.clean('<a href="/foo">x</a>', url_relative="deny")
        == '<a rel="noopener noreferrer">x</a>'
    )
    assert (
        nh3.clean('<a href="https://example.com/foo">x</a>', url_relative="deny")
        == '<a href="https://example.com/foo" rel="noopener noreferrer">x</a>'
    )


def test_clean_url_relative_rewrite_with_base():
    assert (
        nh3.clean(
            '<a href="/foo">x</a>',
            url_relative=("rewrite_with_base", "https://example.com"),
        )
        == '<a href="https://example.com/foo" rel="noopener noreferrer">x</a>'
    )


def test_clean_url_relative_rewrite_with_root():
    out = nh3.clean(
        '<a href="/CONTRIBUTING.md">x</a>',
        url_relative=(
            "rewrite_with_root",
            "https://github.com/rust-ammonia/ammonia/blob/master/",
            "README.md",
        ),
    )
    assert (
        'href="https://github.com/rust-ammonia/ammonia/blob/master/CONTRIBUTING.md"'
        in out
    )


def test_clean_url_relative_custom_replace():
    def rewrite(url):
        return f"https://cdn.example.com{url}" if url.startswith("/") else None

    assert (
        nh3.clean('<img src="/a.png">', url_relative=rewrite)
        == '<img src="https://cdn.example.com/a.png">'
    )


def test_clean_url_relative_custom_strip_on_none():
    assert (
        nh3.clean('<a href="/x">y</a>', url_relative=lambda _url: None)
        == '<a rel="noopener noreferrer">y</a>'
    )


def test_clean_url_relative_custom_exception_strips():
    def boom(_url):
        raise RuntimeError("nope")

    # A failing callback strips the URL; clean() itself stays infallible. The
    # callback error is reported via sys.unraisablehook (surfaced by pytest as a
    # PytestUnraisableExceptionWarning), mirroring attribute_filter's behaviour.
    assert (
        nh3.clean('<a href="/x">y</a>', url_relative=boom)
        == '<a rel="noopener noreferrer">y</a>'
    )


def test_clean_url_relative_invalid():
    with pytest.raises(ValueError):
        nh3.clean("x", url_relative="bogus")
    with pytest.raises(ValueError):
        nh3.clean("x", url_relative=("bogus_mode", "https://example.com"))
    with pytest.raises(ValueError):
        nh3.clean("x", url_relative=("rewrite_with_base", "not a url"))
    with pytest.raises(ValueError):
        nh3.clean("x", url_relative=("rewrite_with_base",))
    with pytest.raises(TypeError):
        nh3.clean("x", url_relative=123)


def test_cleaner_url_relative_reusable():
    cleaner = nh3.Cleaner(url_relative="deny")
    assert cleaner.clean('<a href="/foo">x</a>') == '<a rel="noopener noreferrer">x</a>'
    assert (
        cleaner.clean('<a href="https://example.com">y</a>')
        == '<a href="https://example.com" rel="noopener noreferrer">y</a>'
    )


def test_clean_id_prefix():
    # id_prefix prepends the given string to every allowed `id` value.
    assert (
        nh3.clean("<b id='a'>x</b>", attributes={"b": {"id"}}, id_prefix="safe-")
        == '<b id="safe-a">x</b>'
    )
    # Values already carrying the prefix are left untouched (no double prefix).
    assert (
        nh3.clean("<b id='safe-a'>x</b>", attributes={"b": {"id"}}, id_prefix="safe-")
        == '<b id="safe-a">x</b>'
    )
    # The `id` attribute must still be whitelisted; otherwise it is stripped and
    # the prefix is irrelevant.
    assert nh3.clean("<b id='a'>x</b>", id_prefix="safe-") == "<b>x</b>"
    # Omitting id_prefix keeps `id` values unchanged (ammonia default).
    assert (
        nh3.clean("<b id='a'>x</b>", attributes={"b": {"id"}})
        == '<b id="a">x</b>'
    )


def test_cleaner_id_prefix_reusable():
    cleaner = nh3.Cleaner(attributes={"b": {"id"}}, id_prefix="safe-")
    assert cleaner.clean("<b id='a'>x</b>") == '<b id="safe-a">x</b>'
    assert cleaner.clean("<b id='b'>y</b>") == '<b id="safe-b">y</b>'


def test_is_html():
    assert not nh3.is_html("plain text")
    assert nh3.is_html("<p>html!</p>")


def test_escape():
    # No-arg: full escape, identical to clean_text
    assert nh3.escape('Robert"); abuse();//') == "Robert&quot;);&#32;abuse();&#47;&#47;"

    # With tags=: listed tags preserved (no attributes), the rest escaped/stripped
    assert (
        nh3.escape(
            '<span>hello <mention>moto</mention>, welcome!</span>',
            tags={'mention'},
        )
        == 'hello <mention>moto</mention>, welcome!'
    )

    # Parity with clean_text for a few representative inputs
    for sample, kwargs in [
        ('Robert"); abuse();//', {}),
        ('<b>bold</b> and <i>italic</i>', {"tags": {"b"}}),
        ("<a href='http://example.com' rel='nofollow'>test</a>", {"tags": {"a"}}),
    ]:
        assert nh3.escape(sample, **kwargs) == nh3.clean_text(sample, **kwargs)
