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

from rpds import Stack


def test_literalish_works():
    assert Stack(1, 2, 3) == Stack([1, 2, 3])


def test_pop_and_peek():
    ps = Stack([1, 2])
    assert ps.peek() == 2
    assert ps.pop().peek() == 1
    assert ps.pop().pop() == Stack()


def test_instantiate_large_stack():
    assert Stack(range(1000)).peek() == 999


def test_iteration():
    assert list(Stack()) == []
    assert list(Stack([1, 2, 3]))[::-1] == [1, 2, 3]


def test_push():
    assert Stack([1, 2, 3]).push(4) == Stack([1, 2, 3, 4])


def test_push_empty_stack():
    assert Stack().push(0) == Stack([0])


def test_truthiness():
    assert Stack([1])
    assert not Stack()


def test_len():
    assert len(Stack([1, 2, 3])) == 3
    assert len(Stack()) == 0


def test_peek_illegal_on_empty_stack():
    with pytest.raises(IndexError):
        Stack().peek()


def test_pop_illegal_on_empty_stack():
    with pytest.raises(IndexError):
        Stack().pop()


def test_inequality():
    assert Stack([1, 2]) != Stack([1, 3])
    assert Stack([1, 2]) != Stack([1, 2, 3])
    assert Stack() != Stack([1, 2, 3])


def test_repr():
    assert str(Stack()) == "Stack([])"
    assert str(Stack([1, 2, 3])) in "Stack([1, 2, 3])"


def test_hashing():
    o = object()

    assert hash(Stack([o, o])) == hash(Stack([o, o]))
    assert hash(Stack([o])) == hash(Stack([o]))
    assert hash(Stack()) == hash(Stack([]))
    assert not (hash(Stack([1, 2])) == hash(Stack([1, 3])))
    assert not (hash(Stack([1, 2])) == hash(Stack([2, 1])))
    assert not (hash(Stack([o])) == hash(Stack([o, o])))
    assert not (hash(Stack([])) == hash(Stack([o])))

    assert hash(Stack([1, 2])) != hash(Stack([1, 3]))
    assert hash(Stack([1, 2])) != hash(Stack([2, 1]))
    assert hash(Stack([o])) != hash(Stack([o, o]))
    assert hash(Stack([])) != hash(Stack([o]))
    assert not (hash(Stack([o, o])) != hash(Stack([o, o])))
    assert not (hash(Stack([o])) != hash(Stack([o])))
    assert not (hash(Stack([])) != hash(Stack([])))


def test_sequence():
    m = Stack("asdf")
    assert m == Stack(["a", "s", "d", "f"])


# Non-pyrsistent-test-suite tests


def test_more_eq():
    o = object()

    assert Stack([o, o]) == Stack([o, o])
    assert Stack([o]) == Stack([o])
    assert Stack() == Stack([])
    assert not (Stack([1, 2]) == Stack([1, 3]))
    assert not (Stack([o]) == Stack([o, o]))
    assert not (Stack([]) == Stack([o]))

    assert Stack([1, 2]) != Stack([1, 3])
    assert Stack([o]) != Stack([o, o])
    assert Stack([]) != Stack([o])
    assert not (Stack([o, o]) != Stack([o, o]))
    assert not (Stack([o]) != Stack([o]))
    assert not (Stack() != Stack([]))


def test_rpds_doc():
    """
    From the rpds docs.
    """
    stack = Stack().push("stack")
    assert stack.peek() == "stack"

    a_stack = stack.push("a")
    assert a_stack.peek() == "a"

    stack_popped = a_stack.pop()
    assert stack_popped == stack


@pytest.mark.parametrize("compare", [eq, ne])
def test_element_comparison_errors_propagate(compare):
    class BadEquality:
        __hash__ = None

        def __eq__(self, other):
            raise ValueError("element comparison failed")

    left, right = [BadEquality()], [object()]
    for factory in [list, Stack]:
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

    for factory in [list, Stack]:
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
        for factory in [list, Stack]:
            x, y = factory([value]), factory([value])
            assert x == x
            assert not (x != x)
            assert x == y
            assert not (x != y)


def test_comparison_compares_distinct_nan_elements():
    left, right = float("nan"), -float("nan")
    assert left is not right
    for factory in [list, Stack]:
        x, y = factory([left]), factory([right])
        assert not (x == y)
        assert x != y


@pytest.mark.parametrize("compare", [lt, le, gt, ge])
def test_ordering_comparisons_are_not_supported(compare):
    with pytest.raises(TypeError):
        compare(Stack([1]), Stack([2]))


def test_comparison_reflects_for_other_types():
    class AlwaysEqual:
        __hash__ = None

        def __eq__(self, other):
            return True

        def __ne__(self, other):
            return False

    seq = Stack([1])
    assert not (seq == [1])
    assert seq != [1]
    other = AlwaysEqual()
    assert seq == other
    assert not (seq != other)
