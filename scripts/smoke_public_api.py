"""Smoke test the installed public API."""

from __future__ import annotations

import polars as pl

import polars_janitor as pj


def check_equal(actual: object, expected: object, label: str) -> None:
    """Exit with context when a smoke check fails."""
    if actual != expected:
        msg = f"{label} failed: expected {expected!r}, got {actual!r}"
        raise SystemExit(msg)


def main() -> None:
    """Run a tiny eager and lazy workflow."""
    check_equal(pj.make_clean_names(["A B"]), ["a_b"], "make_clean_names")

    df = pl.DataFrame(
        {
            "Customer ID": [1, 1, 2],
            "Value": ["a", "b", "c"],
        }
    )
    cleaned = pj.clean_names(df)
    check_equal(cleaned.columns, ["customer_id", "value"], "clean_names columns")

    dupes = pj.get_dupes(cleaned, keys="customer_id", include_count=False)
    check_equal(
        dupes.to_dict(as_series=False),
        {
            "customer_id": [1, 1],
            "value": ["a", "b"],
        },
        "get_dupes",
    )

    lazy = pj.remove_empty(
        pl.DataFrame({"a": [None, 1], "b": [None, None]}).lazy(),
        axis="rows",
    )
    check_equal(lazy.collect().to_dict(as_series=False), {"a": [1], "b": [None]}, "lazy remove_empty")

    print("smoke ok")


if __name__ == "__main__":
    main()
