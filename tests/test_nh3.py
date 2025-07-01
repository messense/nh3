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


def test_clean_text():
    res = nh3.clean_text('Robert"); abuse();//')
    assert res == "Robert&quot;);&#32;abuse();&#47;&#47;"


def test_is_html():
    assert not nh3.is_html("plain text")
    assert nh3.is_html("<p>html!</p>")
