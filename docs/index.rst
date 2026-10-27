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

Adding to (or removing from) the defaults
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

The ``tags``, ``clean_content_tags``, ``url_schemes`` and
``generic_attribute_prefixes`` options each *replace* the ammonia defaults.
To extend the defaults without copying them, the ``add_*`` and ``rm_*``
companion options can be used instead:

.. code-block:: pycon

   >>> # Allow a custom tag in addition to the defaults.
   >>> nh3.clean("<b><my-tag>x</my-tag></b>", add_tags={"my-tag"})
   '<b><my-tag>x</my-tag></b>'

   >>> # Forbid <b> but keep the rest of the defaults.
   >>> nh3.clean("<b><i>x</i></b>", rm_tags={"b"})
   '<i>x</i>'

The same pattern works for ``add_clean_content_tags`` / ``rm_clean_content_tags``,
``add_url_schemes`` / ``rm_url_schemes`` and
``add_generic_attribute_prefixes`` / ``rm_generic_attribute_prefixes``.
When combined with the replacement option, ``add_*`` and ``rm_*`` apply on
top of the supplied set.

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

.. attribute:: ALLOWED_URL_SCHEMES

   The default set of URL schemes permitted on ``href`` and ``src`` attributes.
   Useful for customizing the default to add or remove some URL schemes:

   .. code-block:: pycon

       >>> url_schemes = nh3.ALLOWED_URL_SCHEMES - {'tel'}
       >>> nh3.clean('<a href="tel:+1">Call</a> or <a href="mailto:contact@me">email</a> me.', url_schemes=url_schemes)
       '<a rel="noopener noreferrer">Call</a> or <a href="mailto:contact@me" rel="noopener noreferrer">email</a> me.'
