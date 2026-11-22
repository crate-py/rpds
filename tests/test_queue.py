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

from operator import eq, ge, gt, le, lt, ne

import pytest

from rpds import Queue


def test_literalish_works():
    assert Queue(1, 2, 3) == Queue([1, 2, 3])


def test_peek_dequeue():
    pl = Queue([1, 2])
    assert pl.peek() == 1
    assert pl.dequeue().peek() == 2
    assert pl.dequeue().dequeue().is_empty
    with pytest.raises(IndexError):
        pl.dequeue().dequeue().dequeue()


def test_instantiate_large_list():
    assert Queue(range(1000)).peek() == 0


def test_iteration():
    assert list(Queue()) == []
    assert list(Queue([1, 2, 3])) == [1, 2, 3]


def test_enqueue():
    assert Queue([1, 2, 3]).enqueue(4) == Queue([1, 2, 3, 4])


def test_enqueue_empty_list():
    assert Queue().enqueue(0) == Queue([0])


def test_truthiness():
    assert Queue([1])
    assert not Queue()


def test_len():
    assert len(Queue([1, 2, 3])) == 3
    assert len(Queue()) == 0


def test_peek_illegal_on_empty_list():
    with pytest.raises(IndexError):
        Queue().peek()


def test_inequality():
    assert Queue([1, 2]) != Queue([1, 3])
    assert Queue([1, 2]) != Queue([1, 2, 3])
    assert Queue() != Queue([1, 2, 3])


def test_repr():
    assert str(Queue()) == "Queue([])"
    assert str(Queue([1, 2, 3])) in "Queue([1, 2, 3])"


def test_sequence():
    m = Queue("asdf")
    assert m == Queue(["a", "s", "d", "f"])


# Non-pyrsistent-test-suite tests


def test_dequeue():
    assert Queue([1, 2, 3]).dequeue() == Queue([2, 3])


def test_dequeue_empty():
    """
    rpds itself returns an Option<Queue> here but we try IndexError instead.
    """
    with pytest.raises(IndexError):
        Queue([]).dequeue()


def test_more_eq():
    o = object()

    assert Queue([o, o]) == Queue([o, o])
    assert Queue([o]) == Queue([o])
    assert Queue() == Queue([])
    assert not (Queue([1, 2]) == Queue([1, 3]))
    assert not (Queue([o]) == Queue([o, o]))
    assert not (Queue([]) == Queue([o]))

    assert Queue([1, 2]) != Queue([1, 3])
    assert Queue([o]) != Queue([o, o])
    assert Queue([]) != Queue([o])
    assert not (Queue([o, o]) != Queue([o, o]))
    assert not (Queue([o]) != Queue([o]))
    assert not (Queue() != Queue([]))


def test_hashing():
    assert hash(Queue([1, 2])) == hash(Queue([1, 2]))
    assert hash(Queue([1, 2])) != hash(Queue([2, 1]))
    assert len({Queue([1, 2]), Queue([1, 2])}) == 1


def test_unhashable_contents():
    q = Queue([1, {1}])
    with pytest.raises(TypeError):
        hash(q)


@pytest.mark.parametrize("compare", [eq, ne])
def test_element_comparison_errors_propagate(compare):
    class BadEquality:
        __hash__ = None

        def __eq__(self, other):
            raise ValueError("element comparison failed")

    left, right = [BadEquality()], [object()]
    for factory in [list, Queue]:
        with pytest.raises(ValueError, match="element comparison failed"):
            compare(factory(left), factory(right))


def test_inequality_uses_element_equality():
    class AlwaysBoth:
        __hash__ = None

        def __eq__(self, other):
            return True

        def __ne__(self, other):
            return True

    class NeverEither:
        __hash__ = None

        def __eq__(self, other):
            return False

        def __ne__(self, other):
            return False

    for factory in [list, Queue]:
        x, y = factory([AlwaysBoth()]), factory([AlwaysBoth()])
        assert x == y
        assert not (x != y)

        x, y = factory([NeverEither()]), factory([NeverEither()])
        assert not (x == y)
        assert x != y


def test_comparison_short_circuits_identical_elements():
    class BadEquality:
        __hash__ = None

        def __eq__(self, other):
            raise ValueError("identical elements must not be compared")

    for value in [BadEquality(), float("nan")]:
        for factory in [list, Queue]:
            x, y = factory([value]), factory([value])
            assert x == x
            assert not (x != x)
            assert x == y
            assert not (x != y)


def test_comparison_compares_distinct_nan_elements():
    left, right = float("nan"), -float("nan")
    assert left is not right
    for factory in [list, Queue]:
        x, y = factory([left]), factory([right])
        assert not (x == y)
        assert x != y


def test_comparison_with_shared_prefix_and_different_tail():
    q = Queue([1, 2])
    assert q.enqueue(3) == q.enqueue(3)
    assert not (q.enqueue(3) != q.enqueue(3))
    assert q.enqueue(3) != q.enqueue(4)
    assert not (q.enqueue(3) == q.enqueue(4))


@pytest.mark.parametrize("compare", [lt, le, gt, ge])
def test_ordering_comparisons_are_not_supported(compare):
    with pytest.raises(TypeError):
        compare(Queue([1]), Queue([2]))


def test_comparison_reflects_for_other_types():
    class AlwaysEqual:
        __hash__ = None

        def __eq__(self, other):
            return True

        def __ne__(self, other):
            return False

    seq = Queue([1])
    assert not (seq == [1])
    assert seq != [1]
    other = AlwaysEqual()
    assert seq == other
    assert not (seq != other)
