import nh3


def test_clean():
    html = "<b><img src='' onerror='alert(\\'hax\\')'>I'm not trying to XSS you</b>"
    assert nh3.clean(html) == '<b><img src="">I\'m not trying to XSS you</b>'
    assert nh3.clean(html, tags={"img"}) == '<img src="">I\'m not trying to XSS you'
    assert (
        nh3.clean(html, tags={"img"}, attributes={}) == "<img>I'm not trying to XSS you"
    )
    assert nh3.clean(html, attributes={}) == "<b><img>I'm not trying to XSS you</b>"


def test_clean_text():
    res = nh3.clean_text('Robert"); abuse();//')
    assert res == "Robert&quot;);&#32;abuse();&#47;&#47;"
