"""Run a small dataframe cleanup example."""

from __future__ import annotations

import polars as pl

import polars_janitor as pj


def main() -> None:
    """Clean a deliberately messy dataframe."""
    raw = pl.DataFrame(
        {
            "column_1": [None, "Customer ID", "101", "101", "102", None],
            "column_2": ["notes", "Date", "2026-01-01", "2026-01-01", "2026-01-02", None],
            "column_3": ["", "% Complete", "0.5", "0.75", "1.0", None],
            "column_4": ["", "Constant Flag", "imported", "imported", "imported", None],
            "column_5": [None, "Empty Column", None, None, None, None],
        }
    )

    header_row = pj.find_header(raw)
    cleaned = (
        pj.row_to_names(raw, header_row).pipe(pj.remove_empty, axis="both").pipe(pj.remove_constant, ignore_nulls=True)
    )
    dupes = pj.get_dupes(cleaned, keys=["customer_id", "date"])
    incoming = pl.DataFrame(
        {
            "customer_id": ["103"],
            "date": ["2026-01-03"],
            "percent_complete": [1.0],
        }
    )
    schema_report = pj.compare_df_cols({"cleaned": cleaned, "incoming": incoming})

    print(f"Header row: {header_row}")
    print()
    print("Cleaned frame:")
    print(cleaned.to_dict(as_series=False))
    print()
    print("Duplicate customers:")
    print(dupes.to_dict(as_series=False))
    print()
    print("Schema comparison:")
    print(schema_report.to_dict(as_series=False))


if __name__ == "__main__":
    main()
