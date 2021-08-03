import nh3


def test_clean():
    res = nh3.clean(
        "<b><img src='' onerror='alert(\\'hax\\')'>I'm not trying to XSS you</b>"
    )
    assert res == '<b><img src="">I\'m not trying to XSS you</b>'


def test_clean_text():
    res = nh3.clean_text('Robert"); abuse();//')
    assert res == "Robert&quot;);&#32;abuse();&#47;&#47;"
