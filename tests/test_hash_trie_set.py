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

from collections import abc
from fractions import Fraction
from operator import ge, gt, le, lt
import pickle

import pytest

from rpds import HashTrieSet


def test_key_is_tuple():
    with pytest.raises(KeyError):
        HashTrieSet().remove((1, 1))


def test_key_is_not_tuple():
    with pytest.raises(KeyError):
        HashTrieSet().remove("asdf")


def test_hashing():
    o = object()

    assert hash(HashTrieSet([o])) == hash(HashTrieSet([o]))
    assert hash(HashTrieSet([o, o])) == hash(HashTrieSet([o, o]))
    assert hash(HashTrieSet([])) == hash(HashTrieSet([]))
    assert hash(HashTrieSet([1, 2])) == hash(HashTrieSet([1, 2]))
    assert hash(HashTrieSet([1, 2])) == hash(HashTrieSet([2, 1]))
    assert not (HashTrieSet([1, 2]) == HashTrieSet([1, 3]))
    assert not (HashTrieSet([]) == HashTrieSet([o]))

    assert hash(HashTrieSet([1, 2])) != hash(HashTrieSet([1, 3]))
    assert hash(HashTrieSet([1, o])) != hash(HashTrieSet([1, 2]))
    assert hash(HashTrieSet([1, 2])) != hash(HashTrieSet([2, 1, 3]))
    assert not (HashTrieSet([o]) != HashTrieSet([o, o]))
    assert not (HashTrieSet([o, o]) != HashTrieSet([o, o]))
    assert not (HashTrieSet() != HashTrieSet([]))


def test_empty_truthiness():
    assert HashTrieSet([1])
    assert not HashTrieSet()


def test_contains_elements_that_it_was_initialized_with():
    initial = [1, 2, 3]
    s = HashTrieSet(initial)

    assert set(s) == set(initial)
    assert len(s) == len(set(initial))


def test_is_immutable():
    s1 = HashTrieSet([1])
    s2 = s1.insert(2)

    assert s1 == HashTrieSet([1])
    assert s2 == HashTrieSet([1, 2])

    s3 = s2.remove(1)
    assert s2 == HashTrieSet([1, 2])
    assert s3 == HashTrieSet([2])


def test_remove_when_not_present():
    s1 = HashTrieSet([1, 2, 3])
    with pytest.raises(KeyError):
        s1.remove(4)


def test_discard():
    s1 = HashTrieSet((1, 2, 3))
    assert s1.discard(3) == HashTrieSet((1, 2))
    assert s1.discard(4) == s1


def test_is_iterable():
    assert sum(HashTrieSet([1, 2, 3])) == 6


def test_contains():
    s = HashTrieSet([1, 2, 3])

    assert 2 in s
    assert 4 not in s


def test_supports_set_operations():
    s1 = HashTrieSet([1, 2, 3])
    s2 = HashTrieSet([3, 4, 5])

    assert s1 | s2 == HashTrieSet([1, 2, 3, 4, 5])
    assert s1.union(s2) == s1 | s2

    assert s1 & s2 == HashTrieSet([3])
    assert s1.intersection(s2) == s1 & s2

    assert s1 - s2 == HashTrieSet([1, 2])
    assert s1.difference(s2) == s1 - s2

    assert s1 ^ s2 == HashTrieSet([1, 2, 4, 5])
    assert s1.symmetric_difference(s2) == s1 ^ s2


def test_supports_set_comparisons():
    s1 = HashTrieSet([1, 2, 3])
    s3 = HashTrieSet([1, 2])
    s4 = HashTrieSet([1, 2, 3])

    assert HashTrieSet([1, 2, 3, 3, 5]) == HashTrieSet([1, 2, 3, 5])
    assert s1 != s3

    assert s3 < s1
    assert s3 <= s1
    assert s3 <= s4

    assert s1 > s3
    assert s1 >= s3
    assert s4 >= s3


def test_repr():
    rep = repr(HashTrieSet([1, 2]))
    assert rep == "HashTrieSet({1, 2})" or rep == "HashTrieSet({2, 1})"

    rep = repr(HashTrieSet(["1", "2"]))
    assert rep == "HashTrieSet({'1', '2'})" or rep == "HashTrieSet({'2', '1'})"


def test_update():
    assert HashTrieSet([1, 2, 3]).update([3, 4, 4, 5]) == HashTrieSet(
        [1, 2, 3, 4, 5],
    )


def test_update_no_elements():
    s1 = HashTrieSet([1, 2])
    assert s1.update([]) == s1


def test_iterable():
    assert HashTrieSet(iter("a")) == HashTrieSet(iter("a"))


def test_more_eq():
    # Non-pyrsistent-test-suite test
    o = object()

    assert HashTrieSet([o]) == HashTrieSet([o])
    assert HashTrieSet([o, o]) == HashTrieSet([o, o])
    assert HashTrieSet([o]) == HashTrieSet([o, o])
    assert HashTrieSet() == HashTrieSet([])
    assert not (HashTrieSet([1, 2]) == HashTrieSet([1, 3]))
    assert not (HashTrieSet([o, 1]) == HashTrieSet([o, o]))
    assert not (HashTrieSet([]) == HashTrieSet([o]))

    assert HashTrieSet([1, 2]) != HashTrieSet([1, 3])
    assert HashTrieSet([]) != HashTrieSet([o])
    assert not (HashTrieSet([o]) != HashTrieSet([o]))
    assert not (HashTrieSet([o, o]) != HashTrieSet([o, o]))
    assert not (HashTrieSet([o]) != HashTrieSet([o, o]))
    assert not (HashTrieSet() != HashTrieSet([]))

    assert HashTrieSet([1, 2]) == {1, 2}
    assert HashTrieSet([1, 2]) != {1, 2, 3}
    assert HashTrieSet([1, 2]) != [1, 2]


def test_more_set_comparisons():
    s = HashTrieSet([1, 2, 3])

    assert s == s
    assert not (s < s)
    assert s <= s
    assert not (s > s)
    assert s >= s


def test_pickle():
    assert pickle.loads(
        pickle.dumps(HashTrieSet([1, 2, 3, 4])),
    ) == HashTrieSet([1, 2, 3, 4])


def test_instance_of_set():
    assert isinstance(HashTrieSet(), abc.Set)


def test_lt_le_gt_ge():
    assert HashTrieSet({}) < {1}
    assert HashTrieSet({}) <= {1}
    assert HashTrieSet({1}) > set()
    assert HashTrieSet({1}) >= set()


def test_comparison_reflects_for_other_types():
    class AlwaysEqual:
        __hash__ = None

        def __eq__(self, other):
            return True

        def __ne__(self, other):
            return False

    s = HashTrieSet([1, 2])
    assert s == frozenset([1, 2])
    assert not (s != frozenset([1, 2]))
    assert s != frozenset([1, 3])
    other = AlwaysEqual()
    assert s == other
    assert not (s != other)


def test_comparison_short_circuits_shared_root():
    class BadKey:
        def __hash__(self):
            return 1

        def __eq__(self, other):
            raise ValueError("shared-root keys must not be compared")

    for key in [BadKey(), float("nan")]:
        absent = 0 if hash(key) != 0 else 1
        s = HashTrieSet([key])
        discarded = s.discard(absent)
        updated = s.update()
        assert discarded is not s
        assert updated is not s
        for other in [s, discarded, updated]:
            assert s == other
            assert not (s != other)
            assert s <= other
            assert s >= other
            assert not (s < other)
            assert not (s > other)
            assert other == s
            assert not (other != s)
            assert other <= s
            assert other >= s
            assert not (other < s)
            assert not (other > s)


def test_element_identity_short_circuits_comparison():
    class BadKey:
        def __hash__(self):
            return 1

        def __eq__(self, other):
            raise ValueError("identical elements must not be compared")

    for key in [BadKey(), float("nan")]:
        s1 = HashTrieSet([key])
        s2 = HashTrieSet([key])
        assert key in s1
        assert s1 == s2
        assert not (s1 != s2)
        assert s1.insert(key) == s2
        assert s1.remove(key) == HashTrieSet()


def test_element_equality_uses_rich_comparison():
    class TruthyEqKey:
        def __hash__(self):
            return 42

        def __eq__(self, other):
            return 1 if isinstance(other, TruthyEqKey) else 0

    s = HashTrieSet([1])
    assert Fraction(1, 1) in s
    assert s == HashTrieSet([Fraction(1, 1)])
    assert HashTrieSet([Fraction(1, 1)]) == s

    k1, k2 = TruthyEqKey(), TruthyEqKey()
    assert k2 in HashTrieSet([k1])
    assert HashTrieSet([k1]) == HashTrieSet([k2])


def test_element_comparison_errors_propagate():
    class BadKey:
        def __hash__(self):
            return 1

        def __eq__(self, other):
            raise ValueError("element comparison failed")

    k1, k2 = BadKey(), BadKey()
    s1 = HashTrieSet([k1])
    s2 = HashTrieSet([k2])

    operations = [
        lambda: k2 in s1,
        lambda: s1.insert(k2),
        lambda: s1.remove(k2),
        lambda: s1.discard(k2),
        lambda: s1 == s2,
        lambda: s1 != s2,
        lambda: s1 <= s2,
        lambda: s1 >= s2,
        lambda: s1 < s2.insert(0),
        lambda: s1.insert(0) > s2,
        lambda: s1 == {k2},
        lambda: s1 | s2,
        lambda: s1 & s2,
        lambda: s1 - s2,
        lambda: s1 ^ s2,
        lambda: s1.update([k2]),
        lambda: HashTrieSet([k1, k2]),
    ]
    for op in operations:
        with pytest.raises(ValueError, match="element comparison failed"):
            op()

    assert k1 in s1


@pytest.mark.parametrize("compare", [lt, le, gt, ge])
@pytest.mark.parametrize("other", [[1], (1,), {1: 2}, 1])
def test_ordering_against_non_set_raises_type_error(compare, other):
    with pytest.raises(TypeError):
        compare(HashTrieSet([1]), other)


def test_ordering_against_other_set_types():
    s = HashTrieSet([1, 2])
    assert s < frozenset([1, 2, 3])
    assert s <= frozenset([1, 2])
    assert s > frozenset([1])
    assert s >= frozenset([1, 2])
    assert s == {1: "a", 2: "b"}.keys()
    assert s < {1: "a", 2: "b", 3: "c"}.keys()
    assert s > {1: "a"}.keys()
    assert not (s < HashTrieSet([1, 3, 4]))
    assert not (s <= HashTrieSet([1, 3]))
    assert not (s > HashTrieSet([3]))
    assert not (s >= HashTrieSet([1, 3]))


def test_set_operations_short_circuit_shared_root_and_empty():
    class BadKey:
        def __hash__(self):
            return 1

        def __eq__(self, other):
            raise ValueError("shared-root keys must not be compared")

    s = HashTrieSet([BadKey()])
    empty = HashTrieSet()
    clone = s.discard(0)

    for other in [s, clone]:
        assert (s | other) == s
        assert (s & other) == s
        assert (s - other) == empty
        assert (s ^ other) == empty

    assert (s | empty) == s
    assert (empty | s) == s
    assert (s & empty) == empty
    assert (empty & s) == empty
    assert (s - empty) == s
    assert (empty - s) == empty
    assert (s ^ empty) == s
    assert (empty ^ s) == s


def test_remove_and_discard_compare_element_once():
    calls = 0

    class CountingKey:
        def __hash__(self):
            return 7

        def __eq__(self, other):
            nonlocal calls
            calls += 1
            return isinstance(other, CountingKey)

    s = HashTrieSet([CountingKey()])

    calls = 0
    assert s.remove(CountingKey()) == HashTrieSet()
    assert calls == 1

    calls = 0
    assert s.discard(CountingKey()) == HashTrieSet()
    assert calls == 1
