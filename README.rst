===========
``rpds.py``
===========

|PyPI| |Pythons| |CI|

.. |PyPI| image:: https://img.shields.io/pypi/v/rpds-py.svg
  :alt: PyPI version
  :target: https://pypi.org/project/rpds-py/

.. |Pythons| image:: https://img.shields.io/pypi/pyversions/rpds-py.svg
  :alt: Supported Python versions
  :target: https://pypi.org/project/rpds-py/

.. |CI| image:: https://github.com/crate-py/rpds/workflows/CI/badge.svg
  :alt: Build status
  :target: https://github.com/crate-py/rpds/actions?query=workflow%3ACI


Python bindings to the Rust ``rpds`` crate.

What's here is quite minimal (in transparency, it was written initially to support replacing ``pyrsistent`` in the `referencing library <https://github.com/python-jsonschema/referencing>`_).
If you see something missing (which is very likely), a PR is definitely welcome to add it.

Methods in general are named similarly to their ``rpds`` counterparts (rather than ``pyrsistent``\ 's conventions, though probably a full drop-in ``pyrsistent``\ -compatible wrapper module is a good addition at some point).

.. code:: python

    >>> from rpds import HashTrieMap, HashTrieSet, List

    >>> m = HashTrieMap({"foo": "bar", "baz": "quux"})
    >>> m.insert("spam", 37) == HashTrieMap({"foo": "bar", "baz": "quux", "spam": 37})
    True
    >>> m.remove("foo") == HashTrieMap({"baz": "quux"})
    True

    >>> s = HashTrieSet({"foo", "bar", "baz", "quux"})
    >>> s.insert("spam") == HashTrieSet({"foo", "bar", "baz", "quux", "spam"})
    True
    >>> s.remove("foo") == HashTrieSet({"bar", "baz", "quux"})
    True

    >>> L = List([1, 3, 5])
    >>> L.push_front(-1) == List([-1, 1, 3, 5])
    True
    >>> L.rest == List([3, 5])
    True
