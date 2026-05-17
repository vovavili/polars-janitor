from collections.abc import Iterable, Sequence
from typing import Literal, overload

import polars as pl

CaseStyle = Literal["snake", "camel", "pascal", "constant"]
Axis = Literal["rows", "cols", "both"]
Columns = str | Sequence[str]

def make_clean_names(names: Iterable[object | None], case: CaseStyle = "snake") -> list[str]: ...
@overload
def clean_names(frame: pl.DataFrame, *, case: CaseStyle = "snake") -> pl.DataFrame: ...
@overload
def clean_names(frame: pl.LazyFrame, *, case: CaseStyle = "snake") -> pl.LazyFrame: ...
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
