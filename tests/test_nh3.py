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
