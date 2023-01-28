# nh3

![CI](https://github.com/messense/nh3/workflows/CI/badge.svg)
[![PyPI](https://img.shields.io/pypi/v/nh3.svg)](https://pypi.org/project/nh3)
[![Documentation Status](https://readthedocs.org/projects/nh3/badge/?version=latest)](https://nh3.readthedocs.io/en/latest/?badge=latest)

[ammonia](https://github.com/rust-ammonia/ammonia) Python binding.

## Installation

```bash
pip install nh3
```

## Usage

```python
import nh3

print(nh3.clean("<b><img src=\"\">I'm not trying to XSS you</b>"))
```

## Performance

Running on MacBook Air (M2, 2022)

```python
Python 3.11.0 (main, Oct 25 2022, 16:25:24) [Clang 14.0.0 (clang-1400.0.29.102)]
Type 'copyright', 'credits' or 'license' for more information
IPython 8.9.0 -- An enhanced Interactive Python. Type '?' for help.

In [1]: import requests

In [2]: import bleach

In [3]: import nh3

In [4]: html = requests.get("https://www.google.com").text

In [5]: %timeit bleach.clean(html)
2.85 ms ± 22.8 µs per loop (mean ± std. dev. of 7 runs, 100 loops each)

In [6]: %timeit nh3.clean(html)
138 µs ± 860 ns per loop (mean ± std. dev. of 7 runs, 10,000 loops each)
```

## License

This work is released under the MIT license. A copy of the license is provided in the [LICENSE](./LICENSE) file.
