nh3
===

Python bindings to the `ammonia <https://github.com/rust-ammonia/ammonia>`__ HTML sanitization library.

Installation
------------

.. code-block:: bash

   pip install nh3

Usage
-----

Use ``clean()`` to sanitize HTML fragments:

.. code-block:: pycon

    >>> import nh3
    >>> nh3.clean("<unknown>hi")
    'hi'
    >>> nh3.clean("<b><img src='' onerror='alert(\\'hax\\')'>XSS?</b>")
    '<b><img src="">XSS?</b>'

It has many options to customize the sanitization, as documented below.
For example, to only allow ``<b>`` tags:

.. code-block:: python

   >>> nh3.clean("<b><a href='https://example.com'>Hello</a></b>", tags={"b"})
   '<b>Hello</b>'

API reference
-------------

.. automodule:: nh3
   :members:

.. attribute:: ALLOWED_TAGS

   The default set of tags allowed by ``clean()``.
   Useful for customizing the default to add or remove some tags:

   .. code-block:: pycon

       >>> tags = nh3.ALLOWED_TAGS - {"b"}
       >>> nh3.clean("<b><i>yeah</i></b>", tags=tags)
       '<i>yeah</i>'

.. attribute:: ALLOWED_ATTRIBUTES

   The default mapping of tags to allowed attributes for ``clean()``.
   Useful for customizing the default to add or remove some attributes:

   .. code-block:: pycon

       >>> from copy import deepcopy
       >>> attributes = deepcopy(nh3.ALLOWED_ATTRIBUTES)
       >>> attributes["img"].add("data-invert")
       >>> nh3.clean("<img src='example.jpeg' data-invert=true>", attributes=attributes)
       '<img src="example.jpeg" data-invert="true">'
