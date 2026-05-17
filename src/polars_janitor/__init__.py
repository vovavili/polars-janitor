"""Small janitorial helpers for Polars dataframes."""

from __future__ import annotations

import polars as pl

from polars_janitor._rust import (
    compare_df_cols,
    compare_df_cols_same,
    find_header,
    get_dupes,
    make_clean_names,
    remove_constant,
    remove_empty,
    row_to_names,
)


def clean_names(frame: pl.DataFrame | pl.LazyFrame, *, case: str = "snake") -> pl.DataFrame | pl.LazyFrame:
    """Clean column names using Rust name normalization and Polars' native rename path."""
    if isinstance(frame, pl.DataFrame):
        columns = frame.columns
    elif isinstance(frame, pl.LazyFrame):
        columns = frame.collect_schema().names()
    else:
        raise TypeError("frame must be a polars DataFrame or LazyFrame")

    cleaned = make_clean_names(columns, case)
    return frame.rename(dict(zip(columns, cleaned, strict=True)))


__all__ = [
    "clean_names",
    "compare_df_cols",
    "compare_df_cols_same",
    "find_header",
    "get_dupes",
    "make_clean_names",
    "remove_constant",
    "remove_empty",
    "row_to_names",
]
