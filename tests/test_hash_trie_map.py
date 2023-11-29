"""
Modified from the pyrsistent test suite.

Pre-modification, these were MIT licensed, and are copyright:

    Copyright (c) 2022 Tobias Gustafsson

    Permission is hereby granted, free of charge, to any person
    obtaining a copy of this software and associated documentation
    files (the "Software"), to deal in the Software without
    restriction, including without limitation the rights to use,
    copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the
    Software is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice shall be
    included in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
    EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES
    OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
    NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT
    HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
    WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
    FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR
    OTHER DEALINGS IN THE SOFTWARE.
"""
from collections.abc import Hashable, Mapping
import pickle

import pytest

from rpds import HashTrieMap

HASH_MSG = "Not sure HashTrieMap implements Hash, it has mutable methods"


@pytest.mark.xfail(reason=HASH_MSG)
def test_instance_of_hashable():
    assert isinstance(HashTrieMap(), Hashable)


def test_instance_of_map():
    assert isinstance(HashTrieMap(), Mapping)


def test_literalish_works():
    assert HashTrieMap() == HashTrieMap()
    assert HashTrieMap(a=1, b=2) == HashTrieMap({"a": 1, "b": 2})


def test_empty_initialization():
    a_map = HashTrieMap()
    assert len(a_map) == 0


def test_initialization_with_one_element():
    the_map = HashTrieMap({"a": 2})
    assert len(the_map) == 1
    assert the_map["a"] == 2
    assert "a" in the_map

    empty_map = the_map.remove("a")
    assert len(empty_map) == 0
    assert "a" not in empty_map


def test_get_non_existing_raises_key_error():
    m1 = HashTrieMap()
    with pytest.raises(KeyError) as error:
        m1["foo"]

    assert str(error.value) == "'foo'"


def test_remove_non_existing_element_raises_key_error():
    m1 = HashTrieMap(a=1)

    with pytest.raises(KeyError) as error:
        m1.remove("b")

    assert str(error.value) == "'b'"


def test_various_iterations():
    assert {"a", "b"} == set(HashTrieMap(a=1, b=2))
    assert ["a", "b"] == sorted(HashTrieMap(a=1, b=2).keys())
    assert [1, 2] == sorted(HashTrieMap(a=1, b=2).values())
    assert {("a", 1), ("b", 2)} == set(HashTrieMap(a=1, b=2).items())

    pm = HashTrieMap({k: k for k in range(100)})
    assert len(pm) == len(pm.keys())
    assert len(pm) == len(pm.values())
    assert len(pm) == len(pm.items())
    ks = pm.keys()
    assert all(k in pm for k in ks)
    assert all(k in ks for k in ks)
    us = pm.items()
    assert all(pm[k] == v for (k, v) in us)
    vs = pm.values()
    assert all(v in vs for v in vs)


def test_initialization_with_two_elements():
    map1 = HashTrieMap({"a": 2, "b": 3})
    assert len(map1) == 2
    assert map1["a"] == 2
    assert map1["b"] == 3

    map2 = map1.remove("a")
    assert "a" not in map2
    assert map2["b"] == 3


def test_initialization_with_many_elements():
    init_dict = {str(x): x for x in range(1700)}
    the_map = HashTrieMap(init_dict)

    assert len(the_map) == 1700
    assert the_map["16"] == 16
    assert the_map["1699"] == 1699
    assert the_map.insert("256", 256) == the_map

    new_map = the_map.remove("1600")
    assert len(new_map) == 1699
    assert "1600" not in new_map
    assert new_map["1601"] == 1601

    # Some NOP properties
    assert new_map.discard("18888") == new_map
    assert "19999" not in new_map
    assert new_map["1500"] == 1500
    assert new_map.insert("1500", new_map["1500"]) == new_map


def test_access_non_existing_element():
    map1 = HashTrieMap()
    assert len(map1) == 0

    map2 = map1.insert("1", 1)
    assert "1" not in map1
    assert map2["1"] == 1
    assert "2" not in map2


def test_overwrite_existing_element():
    map1 = HashTrieMap({"a": 2})
    map2 = map1.insert("a", 3)

    assert len(map2) == 1
    assert map2["a"] == 3


@pytest.mark.xfail(reason=HASH_MSG)
def test_hash():
    x = HashTrieMap(a=1, b=2, c=3)
    y = HashTrieMap(a=1, b=2, c=3)

    assert hash(x) == hash(y)


def test_same_hash_when_content_the_same_but_underlying_vector_size_differs():
    x = HashTrieMap({x: x for x in range(1000)})
    y = HashTrieMap({10: 10, 200: 200, 700: 700})

    for z in x:
        if z not in y:
            x = x.remove(z)

    assert x == y
    # assert hash(x) == hash(y)


class HashabilityControlled:
    hashable = True

    def __hash__(self):
        if self.hashable:
            return 4  # Proven random
        raise ValueError("I am not currently hashable.")


@pytest.mark.xfail(reason=HASH_MSG)
def test_map_does_not_hash_values_on_second_hash_invocation():
    hashable = HashabilityControlled()
    x = HashTrieMap(dict(el=hashable))
    hash(x)
    hashable.hashable = False
    hash(x)


def test_equal():
    x = HashTrieMap(a=1, b=2, c=3)
    y = HashTrieMap(a=1, b=2, c=3)

    assert x == y
    assert not (x != y)

    assert y == x
    assert not (y != x)


def test_equal_with_different_insertion_order():
    x = HashTrieMap([(i, i) for i in range(50)])
    y = HashTrieMap([(i, i) for i in range(49, -1, -1)])

    assert x == y
    assert not (x != y)

    assert y == x
    assert not (y != x)


def test_not_equal():
    x = HashTrieMap(a=1, b=2, c=3)
    y = HashTrieMap(a=1, b=2)

    assert x != y
    assert not (x == y)

    assert y != x
    assert not (y == x)


def test_not_equal_to_dict():
    x = HashTrieMap(a=1, b=2, c=3)
    y = dict(a=1, b=2, d=4)

    assert x != y
    assert not (x == y)

    assert y != x
    assert not (y == x)


def test_update_with_multiple_arguments():
    # If same value is present in multiple sources, the rightmost is used.
    x = HashTrieMap(a=1, b=2, c=3)
    y = x.update(HashTrieMap(b=4, c=5), {"c": 6})

    assert y == HashTrieMap(a=1, b=4, c=6)


def test_update_one_argument():
    x = HashTrieMap(a=1)

    assert x.update({"b": 2}) == HashTrieMap(a=1, b=2)


def test_update_no_arguments():
    x = HashTrieMap(a=1)

    assert x.update() == x


class HashDummy:
    def __hash__(self):
        return 6528039219058920  # Hash of '33'

    def __eq__(self, other):
        return self is other


def test_iteration_with_many_elements():
    values = list(range(0, 2000))
    keys = [str(x) for x in values]
    init_dict = dict(zip(keys, values))

    hash_dummy1 = HashDummy()
    hash_dummy2 = HashDummy()

    # Throw in a couple of hash collision nodes to tests
    # those properly as well
    init_dict[hash_dummy1] = 12345
    init_dict[hash_dummy2] = 54321
    a_map = HashTrieMap(init_dict)

    actual_values = set()
    actual_keys = set()

    for k, v in a_map.items():
        actual_values.add(v)
        actual_keys.add(k)

    assert actual_keys == set(keys + [hash_dummy1, hash_dummy2])
    assert actual_values == set(values + [12345, 54321])


def test_repr():
    rep = repr(HashTrieMap({"foo": "12", "": 37}))
    assert rep in {
        "HashTrieMap({'foo': '12', '': 37})",
        "HashTrieMap({'': 37, 'foo': '12'})",
    }


def test_str():
    s = str(HashTrieMap({1: 2, 3: 4}))
    assert s == "HashTrieMap({1: 2, 3: 4})" or s == "HashTrieMap({3: 4, 1: 2})"


def test_empty_truthiness():
    assert HashTrieMap(a=1)
    assert not HashTrieMap()


def test_iterable():
    m = HashTrieMap((i, i * 2) for i in range(3))
    assert m == HashTrieMap({0: 0, 1: 2, 2: 4})


def test_convert_hashtriemap():
    m = HashTrieMap({i: i * 2 for i in range(3)})
    assert HashTrieMap.convert({i: i * 2 for i in range(3)}) == m


def test_fast_convert_hashtriemap():
    m = HashTrieMap({i: i * 2 for i in range(3)})
    assert HashTrieMap.convert(m) is m


def test_more_eq():
    # Non-pyrsistent-test-suite test
    o = object()

    assert HashTrieMap([(o, o), (1, o)]) == HashTrieMap([(o, o), (1, o)])
    assert HashTrieMap([(o, "foo")]) == HashTrieMap([(o, "foo")])
    assert HashTrieMap() == HashTrieMap([])

    assert HashTrieMap({1: 2}) != HashTrieMap({1: 3})
    assert HashTrieMap({o: 1}) != HashTrieMap({o: o})
    assert HashTrieMap([]) != HashTrieMap([(o, 1)])


def test_pickle():
    assert pickle.loads(
        pickle.dumps(HashTrieMap([(1, 2), (3, 4)]))
    ) == HashTrieMap([(1, 2), (3, 4)])
