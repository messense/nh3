import doctest

import nh3


def test_docstrings():
    finder = doctest.DocTestFinder()
    runner = doctest.DocTestRunner(verbose=False)
    globs = {"nh3": nh3}
    for name in ["clean", "clean_text", "is_html"]:
        obj = getattr(nh3, name)
        for test in finder.find(obj, f"nh3.{name}"):
            if test.examples:
                test.globs.update(globs)
                runner.run(test)
    results = runner.summarize(verbose=False)
    assert results.failed == 0, f"{results.failed} doctest(s) failed"
