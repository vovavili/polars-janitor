"""Compatibility checks for the compiled extension and lazy bridge."""

from __future__ import annotations

import importlib.machinery
import sys

import polars as pl
import pytest
from polars.testing import assert_frame_equal

import polars_janitor as pj
import polars_janitor._rust as rust


def test_rust_extension_matches_active_cpython_interpreter() -> None:
    """The installed extension should match the interpreter running the tests."""
    extension_path = rust.__file__ or ""

    assert extension_path
    assert any(extension_path.endswith(suffix) for suffix in importlib.machinery.EXTENSION_SUFFIXES)
    assert f"cp{sys.version_info.major}{sys.version_info.minor}" in extension_path


def test_lazy_helpers_build_public_polars_plans() -> None:
    """Supported LazyFrame helpers return lazy plans and collect to expected data."""
    df = pl.DataFrame({"Customer ID": [1, 1, None], "Value": ["a", "b", None]})

    lazy = pj.clean_names(df.lazy())
    lazy = pj.remove_empty(lazy, axis="rows")
    lazy = pj.get_dupes(lazy, keys="customer_id", include_count=False)

    assert isinstance(lazy, pl.LazyFrame)
    assert_frame_equal(lazy.collect(), pl.DataFrame({"customer_id": [1, 1], "value": ["a", "b"]}))


def test_unsupported_lazy_operations_have_clear_errors() -> None:
    """Data-dependent LazyFrame helpers should say why they are eager-only."""
    df = pl.DataFrame({"a": [None], "b": [1]})

    with pytest.raises(
        NotImplementedError,
        match=r"remove_empty\(\.\.\., axis='cols' or 'both'\) is data-dependent",
    ):
        pj.remove_empty(df.lazy(), axis="cols")

    with pytest.raises(NotImplementedError, match=r"remove_constant\(\) is data-dependent"):
        pj.remove_constant(df.lazy())
