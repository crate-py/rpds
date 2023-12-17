Python bindings to the `Rust rpds crate <https://docs.rs/rpds/>`_ for persistent data structures.

What's here is quite minimal (in transparency, it was written initially to support replacing ``pyrsistent`` in the `referencing library <https://github.com/python-jsonschema/referencing>`_).
If you see something missing (which is very likely), a PR is definitely welcome to add it.

Installation
------------

The distribution on PyPI is named ``rpds.py`` (equivalently ``rpds-py``), and thus can be installed via e.g.:

.. code:: sh

    $ pip install rpds-py

Note that if you install ``rpds-py`` from source, you will need a Rust toolchain installed, as it is a build-time dependency.
An example of how to do so in a ``Dockerfile`` can be found `here <https://github.com/bowtie-json-schema/bowtie/blob/e77fd93598cb6e7dc1b8b1f53c00e5aa410c201a/implementations/python-jsonschema/Dockerfile#L1-L8>`_.

If you believe you are on a common platform which should have wheels built (i.e. and not need to compile from source), feel free to file an issue or pull request modifying the GitHub action used here to build wheels via ``maturin``.

Usage
-----

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


.. toctree::
    :glob:
    :hidden:

    api
