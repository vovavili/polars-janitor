"""Tests for dataframe helpers."""

from __future__ import annotations

from collections import Counter

import polars as pl
import pytest
from hypothesis import given
from hypothesis import strategies as st
from polars.testing import assert_frame_equal

import polars_janitor as pj


def test_clean_names_renames_dataframe_columns() -> None:
    """DataFrame columns are cleaned without changing values."""
    df = pl.DataFrame({"Customer ID": [1, 2], "% Complete": [0.5, 1.0]})

    result = pj.clean_names(df)

    assert result.columns == ["customer_id", "percent_complete"]
    assert result.to_dict(as_series=False) == {"customer_id": [1, 2], "percent_complete": [0.5, 1.0]}


def test_clean_names_supports_lazyframe() -> None:
    """LazyFrame column names can be cleaned from static schema."""
    df = pl.DataFrame({"Customer ID": [1], "OrderID": [2]})

    result = pj.clean_names(df.lazy()).collect()

    assert result.columns == ["customer_id", "order_id"]


def test_find_header_and_row_to_names_clean_messy_spreadsheet_rows() -> None:
    """A discovered header row can be promoted to cleaned column names."""
    df = pl.DataFrame(
        {
            "column_1": [None, "Customer ID", "1", "2"],
            "column_2": ["notes", "Order Total", "10", "20"],
            "column_3": ["", "Order Date", "2026-01-01", "2026-01-02"],
        }
    )

    header = pj.find_header(df)
    result = pj.row_to_names(df, header)

    assert header == 1
    assert result.columns == ["customer_id", "order_total", "order_date"]
    assert result.to_dict(as_series=False) == {
        "customer_id": ["1", "2"],
        "order_total": ["10", "20"],
        "order_date": ["2026-01-01", "2026-01-02"],
    }


def test_find_header_can_search_for_value_in_named_column() -> None:
    """Header discovery can target a specific messy-spreadsheet marker."""
    df = pl.DataFrame(
        {
            "left": [None, "ignore", "wrong"],
            "right": ["skip", "Header", "Header"],
        }
    )

    assert pj.find_header(df, value="Header", column="right") == 1


def test_row_to_names_rejects_lazyframe() -> None:
    """Promoting a row to names is data-dependent, so lazy support would be dishonest."""
    df = pl.DataFrame({"a": ["name", "value"]})

    with pytest.raises(NotImplementedError, match=r"row_to_names\(\) is data-dependent"):
        pj.row_to_names(df.lazy())


def test_remove_empty_rows_cols_and_both() -> None:
    """Rows and columns that are entirely null can be removed independently."""
    df = pl.DataFrame(
        {
            "a": [None, None, 1],
            "b": [None, None, None],
            "c": ["x", None, "z"],
        }
    )

    rows = pj.remove_empty(df, axis="rows")
    cols = pj.remove_empty(df, axis="cols")
    both = pj.remove_empty(df, axis="both")

    assert rows.to_dict(as_series=False) == {"a": [None, 1], "b": [None, None], "c": ["x", "z"]}
    assert cols.columns == ["a", "c"]
    assert both.to_dict(as_series=False) == {"a": [None, 1], "c": ["x", "z"]}


def test_remove_empty_supports_lazy_rows_only() -> None:
    """LazyFrame empty-row removal stays lazy because the schema does not change."""
    df = pl.DataFrame({"a": [None, 1], "b": [None, None]})

    result = pj.remove_empty(df.lazy(), axis="rows").collect()

    assert result.to_dict(as_series=False) == {"a": [1], "b": [None]}


def test_remove_empty_rejects_lazy_column_removal() -> None:
    """Lazy column removal would need data-dependent schema discovery."""
    df = pl.DataFrame({"a": [None], "b": [1]})

    with pytest.raises(NotImplementedError, match="data-dependent"):
        pj.remove_empty(df.lazy(), axis="cols")


def test_remove_constant_uses_explicit_null_semantics() -> None:
    """Nulls either count as a value or are ignored, depending on configuration."""
    df = pl.DataFrame(
        {
            "constant": [1, 1, 1],
            "with_null": [1, None, 1],
            "varied": [1, 2, 1],
            "nulls": [None, None, None],
        }
    )

    count_nulls = pj.remove_constant(df)
    ignore_nulls = pj.remove_constant(df, ignore_nulls=True)

    assert count_nulls.columns == ["with_null", "varied"]
    assert ignore_nulls.columns == ["varied"]


def test_remove_constant_rejects_lazyframe() -> None:
    """Constant-column removal needs data-dependent schema discovery."""
    df = pl.DataFrame({"a": [1, 1], "b": [1, 2]})

    with pytest.raises(NotImplementedError, match="data-dependent"):
        pj.remove_constant(df.lazy())


def test_get_dupes_returns_duplicate_key_rows_with_counts() -> None:
    """Duplicate rows are identified by key and annotated with group size."""
    df = pl.DataFrame({"id": [1, 1, 2, 3, 3, 3], "value": ["a", "b", "c", "d", "e", "f"]})

    result = pj.get_dupes(df, keys="id")

    assert result.to_dict(as_series=False) == {
        "id": [1, 1, 3, 3, 3],
        "value": ["a", "b", "d", "e", "f"],
        "duplicate_count": [2, 2, 3, 3, 3],
    }


def test_get_dupes_can_omit_counts_and_stay_lazy() -> None:
    """Duplicate lookup returns a LazyFrame when given a LazyFrame."""
    df = pl.DataFrame({"id": [1, 1, 2], "value": ["a", "b", "c"]})

    result = pj.get_dupes(df.lazy(), keys="id", include_count=False)

    assert isinstance(result, pl.LazyFrame)
    assert_frame_equal(result.collect(), pl.DataFrame({"id": [1, 1], "value": ["a", "b"]}))


@given(st.lists(st.integers(min_value=-(2**63), max_value=2**63 - 1), max_size=50))
def test_get_dupes_only_returns_members_of_duplicate_groups(keys: list[int]) -> None:
    """Every returned row belongs to a key group with size greater than one."""
    counts = Counter(keys)
    df = pl.DataFrame(
        {
            "key": pl.Series("key", keys, dtype=pl.Int64),
            "row": pl.Series("row", range(len(keys)), dtype=pl.Int64),
        }
    )

    result = pj.get_dupes(df, keys="key")

    assert result.height == sum(1 for key in keys if counts[key] > 1)
    for row in result.iter_rows(named=True):
        assert counts[row["key"]] == row["duplicate_count"]
        assert row["duplicate_count"] > 1


def test_compare_df_cols_reports_schema_differences_for_eager_and_lazy_frames() -> None:
    """Schema comparison should work without collecting lazy frames."""
    left = pl.DataFrame({"id": [1], "name": ["a"]})
    right = pl.DataFrame({"id": [1], "amount": [10.0]})

    result = pj.compare_df_cols({"left": left, "right": right.lazy()})

    assert result.to_dict(as_series=False) == {
        "column_name": ["id", "name", "amount"],
        "left": ["Int64", "String", None],
        "right": ["Int64", None, "Float64"],
    }
    assert not pj.compare_df_cols_same({"left": left, "right": right.lazy()})


def test_compare_df_cols_can_filter_to_matching_columns() -> None:
    """The match/mismatch filters use present-and-same dtype semantics."""
    left = pl.DataFrame({"id": [1], "name": ["a"]})
    right = pl.DataFrame({"id": [2], "name": ["b"]})

    result = pj.compare_df_cols([left, right], names=["left", "right"], return_="match")

    assert result["column_name"].to_list() == ["id", "name"]
    assert pj.compare_df_cols_same([left, right], names=["left", "right"])
