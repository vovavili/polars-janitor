"""Tests for name cleaning."""

from __future__ import annotations

import pytest
from hypothesis import given
from hypothesis import strategies as st

from polars_janitor import make_clean_names


def test_make_clean_names_cleans_common_column_problems() -> None:
    """Representative janitor-style names are normalized and deduplicated."""
    names = ["Customer ID", "Customer ID", "% Complete", "Mötley Crüe", "", None, "OrderID", "1st Sale"]

    assert make_clean_names(names) == [
        "customer_id",
        "customer_id_2",
        "percent_complete",
        "motley_crue",
        "x",
        "x_2",
        "order_id",
        "x_1_st_sale",
    ]


def test_make_clean_names_supports_case_styles() -> None:
    """Case conversion is deterministic across supported styles."""
    names = ["Customer ID", "% Complete", "alreadyClean"]

    assert make_clean_names(names, case="snake") == ["customer_id", "percent_complete", "already_clean"]
    assert make_clean_names(names, case="constant") == ["CUSTOMER_ID", "PERCENT_COMPLETE", "ALREADY_CLEAN"]
    assert make_clean_names(names, case="camel") == ["customerId", "percentComplete", "alreadyClean"]
    assert make_clean_names(names, case="pascal") == ["CustomerId", "PercentComplete", "AlreadyClean"]


def test_make_clean_names_rejects_unknown_case() -> None:
    """Unknown case styles fail early."""
    with pytest.raises(ValueError, match="case must be one of"):
        make_clean_names(["a"], case="kebab")  # type: ignore[arg-type]


@given(st.lists(st.one_of(st.none(), st.text())))
def test_make_clean_names_always_returns_unique_nonempty_names(names: list[str | None]) -> None:
    """Cleaned names preserve length while becoming unique and non-empty."""
    cleaned = make_clean_names(names)

    assert len(cleaned) == len(names)
    assert len(set(cleaned)) == len(cleaned)
    assert all(cleaned)


@given(st.lists(st.one_of(st.none(), st.text())))
def test_make_clean_names_is_idempotent_for_its_own_output(names: list[str | None]) -> None:
    """Cleaning already-clean output is stable."""
    cleaned = make_clean_names(names)

    assert make_clean_names(cleaned) == cleaned
