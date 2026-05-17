"""Run a small dataframe cleanup example."""

from __future__ import annotations

import polars as pl

import polars_janitor as pj


def main() -> None:
    """Clean a deliberately messy dataframe."""
    df = pl.DataFrame(
        {
            "Customer ID": [101, 101, 102, None],
            "% Complete": [0.5, 0.75, 1.0, None],
            "Constant Flag": ["imported", "imported", "imported", None],
            "Empty Column": [None, None, None, None],
            "Messy Status": ["new", "new", "done", None],
        }
    )

    cleaned = df.pipe(pj.clean_names).pipe(pj.remove_empty, axis="both").pipe(pj.remove_constant, ignore_nulls=True)
    dupes = pj.get_dupes(cleaned, keys="customer_id")

    print("Cleaned frame:")
    print(cleaned.to_dict(as_series=False))
    print()
    print("Duplicate customers:")
    print(dupes.to_dict(as_series=False))


if __name__ == "__main__":
    main()
