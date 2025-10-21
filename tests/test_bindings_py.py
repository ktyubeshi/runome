"""
Placeholder for the expanded Python binding regression suite.

The concrete cases will mirror Janome's behaviour more closely, covering edge
cases listed in tmp/plan.md (user dictionaries, wakati/baseform switches, etc.).
Until those harnesses are ready we mark the module as skipped to keep pytest
output clean.
"""

import pytest

pytestmark = pytest.mark.skip(reason="Binding regression tests not implemented yet")


def test_bindings_todo():
    pytest.skip("See tmp/plan.md for the upcoming scenarios.")
