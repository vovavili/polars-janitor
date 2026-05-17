"""Authoritative tests for the Rust name-cleaning binding."""

from __future__ import annotations

import pytest

from polars_janitor._rust import make_clean_names


def test_rust_binding_stringifies_values_and_treats_none_as_empty() -> None:
    """Python objects are stringified at the binding boundary."""
    assert make_clean_names([1, None, "% Complete"]) == ["x_1", "x", "percent_complete"]


@pytest.mark.parametrize("case", ["snake", "camel", "pascal", "constant"])
def test_rust_binding_accepts_public_case_styles(case: str) -> None:
    """All public case styles are owned by the Rust implementation."""
    assert len(make_clean_names(["Customer ID", "Customer ID"], case=case)) == 2


def test_rust_binding_rejects_invalid_case() -> None:
    """Case validation happens inside Rust."""
    with pytest.raises(ValueError, match="case must be one of"):
        make_clean_names(["a"], case="kebab")
