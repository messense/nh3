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


def test_clean_text():
    res = nh3.clean_text('Robert"); abuse();//')
    assert res == "Robert&quot;);&#32;abuse();&#47;&#47;"


def test_is_html():
    assert not nh3.is_html('plain text')
    assert nh3.is_html('<p>html!</p>')
