from collections.abc import Iterable, Mapping, Sequence
from typing import Literal, overload

import polars as pl

CaseStyle = Literal["snake", "camel", "pascal", "constant"]
Axis = Literal["rows", "cols", "both"]
Columns = str | Sequence[str]
CompareReturn = Literal["all", "match", "mismatch"]
Frames = Sequence[pl.DataFrame | pl.LazyFrame] | Mapping[str, pl.DataFrame | pl.LazyFrame]

def make_clean_names(names: Iterable[object | None], case: CaseStyle = "snake") -> list[str]: ...
@overload
def clean_names(frame: pl.DataFrame, *, case: CaseStyle = "snake") -> pl.DataFrame: ...
@overload
def clean_names(frame: pl.LazyFrame, *, case: CaseStyle = "snake") -> pl.LazyFrame: ...
def find_header(
    df: pl.DataFrame,
    *,
    value: object | None = None,
    column: str | int | None = None,
) -> int: ...
def row_to_names(
    df: pl.DataFrame,
    row: int | Literal["find_header"] | None = None,
    *,
    remove_row: bool = True,
    remove_rows_above: bool = True,
    case: CaseStyle = "snake",
) -> pl.DataFrame: ...
@overload
def remove_empty(
    frame: pl.DataFrame,
    *,
    axis: Axis = "rows",
    subset: Columns | None = None,
) -> pl.DataFrame: ...
@overload
def remove_empty(
    frame: pl.LazyFrame,
    *,
    axis: Literal["rows"] = "rows",
    subset: Columns | None = None,
) -> pl.LazyFrame: ...
def remove_constant(
    df: pl.DataFrame,
    *,
    subset: Columns | None = None,
    ignore_nulls: bool = False,
) -> pl.DataFrame: ...
@overload
def get_dupes(
    frame: pl.DataFrame,
    keys: Columns,
    *,
    include_count: bool = True,
    count_name: str = "duplicate_count",
) -> pl.DataFrame: ...
@overload
def get_dupes(
    frame: pl.LazyFrame,
    keys: Columns,
    *,
    include_count: bool = True,
    count_name: str = "duplicate_count",
) -> pl.LazyFrame: ...
def compare_df_cols(
    frames: Frames,
    *,
    names: Sequence[str] | None = None,
    return_: CompareReturn = "all",
) -> pl.DataFrame: ...
def compare_df_cols_same(
    frames: Frames,
    *,
    names: Sequence[str] | None = None,
) -> bool: ...
