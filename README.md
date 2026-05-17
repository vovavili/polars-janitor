# polars-janitor

Small janitorial helpers for Polars dataframes.

This project is inspired by R's janitor, but it is not a parity port. The aim is smaller: keep a few boring dataframe cleanup chores easy, predictable, and Polars-shaped.

## Why this exists

Polars already has a strong API. Most cleanup work should stay plain Polars.

The rough spots this package tries to smooth out are the ones that show up around messy inputs: awkward column names from CSVs and spreadsheets, header rows hiding inside spreadsheet data, empty rows, all-null columns, constant columns, duplicate records by key, and quick schema checks before you combine frames. Those are janitorial jobs. They are not glamorous, but they happen often enough to deserve a small, sharp tool.

The package does not register a dataframe namespace. Import it next to Polars:

```python
import polars as pl
import polars_janitor as pj
```

## Install

```bash
pip install polars-janitor
```

`polars-janitor` supports Python Polars `1.29.0` and newer. Older Polars `1.x` wheels do not expose the data exchange API this extension uses.

From a local checkout:

```powershell
uv run --extra dev maturin develop --release
```

The Rust extension is built for the Python interpreter active in your environment. If you switch interpreters, for example from CPython 3.13 to CPython 3.11, rebuild it. The compiled `_rust.*.pyd` on Windows, or `_rust.*.so` on Linux and macOS, is not portable across Python versions.

## Python usage

### Clean column names

Use `clean_names` when you already have a `DataFrame` or `LazyFrame`.

```python
df = pl.DataFrame(
    {
        "Customer ID": [1, 2],
        "% Complete": [0.5, 1.0],
        "OrderID": ["A001", "A002"],
    }
)

cleaned = pj.clean_names(df)

print(cleaned.columns)
# ["customer_id", "percent_complete", "order_id"]
```

`clean_names` also works on `LazyFrame`. It uses the static schema, so it does not need to collect the data.

```python
lazy = pj.clean_names(df.lazy())
result = lazy.collect()
```

Use `make_clean_names` when you only need names back.

```python
names = pj.make_clean_names(
    ["Customer ID", "Customer ID", "% Complete", "Mötley Crüe", "", None, "1st Sale"]
)

print(names)
```

```python
[
    "customer_id",
    "customer_id_2",
    "percent_complete",
    "motley_crue",
    "x",
    "x_2",
    "x_1_st_sale",
]
```

Supported case styles are `snake`, `camel`, `pascal`, and `constant`.

```python
pj.make_clean_names(["Customer ID", "% Complete"], case="camel")
# ["customerId", "percentComplete"]

pj.make_clean_names(["Customer ID", "% Complete"], case="constant")
# ["CUSTOMER_ID", "PERCENT_COMPLETE"]
```

Name cleaning is deterministic. It handles duplicate names, empty names, whitespace, symbols, mixed casing, common diacritics, and Python `None`. Other Python objects are converted with `str(...)`.

### Promote spreadsheet rows to names

Spreadsheet exports often put notes above the real header row. Use `find_header` to locate the first row where every cell is present and non-blank, then `row_to_names` to promote that row to cleaned column names.

```python
raw = pl.DataFrame(
    {
        "column_1": [None, "Customer ID", "101", "101", "102"],
        "column_2": ["notes", "Order Date", "2026-01-01", "2026-01-01", "2026-01-02"],
        "column_3": ["", "% Complete", "0.5", "0.75", "1.0"],
    }
)

header = pj.find_header(raw)
cleaned = pj.row_to_names(raw, header)

print(header)
# 1

print(cleaned.columns)
# ["customer_id", "order_date", "percent_complete"]
```

`row_to_names` uses 0-based row numbers, like Python indexing. If you omit the row, it calls `find_header` for you.

```python
cleaned = pj.row_to_names(raw)
```

You can also search for a known marker in one column.

```python
pj.find_header(raw, value="Customer ID", column="column_1")
# 1
```

`find_header` and `row_to_names` are eager-only because they need to inspect values.

### Remove empty rows and columns

Use `remove_empty` to drop rows where every selected column is null, columns where every value is null, or both.

```python
df = pl.DataFrame(
    {
        "a": [None, None, 1],
        "b": [None, None, None],
        "c": ["x", None, "z"],
    }
)

pj.remove_empty(df, axis="rows")
pj.remove_empty(df, axis="cols")
pj.remove_empty(df, axis="both")
```

You can limit the check to a subset of columns.

```python
pj.remove_empty(df, axis="rows", subset=["a", "c"])
```

Lazy support is intentionally smaller here. `LazyFrame` supports `axis="rows"` because the schema does not change. `axis="cols"` and `axis="both"` are eager-only because Polars would need to inspect the data before knowing which columns still exist.

### Remove constant columns

Use `remove_constant` to drop columns with one distinct value.

```python
df = pl.DataFrame(
    {
        "constant": [1, 1, 1],
        "with_null": [1, None, 1],
        "varied": [1, 2, 1],
        "nulls": [None, None, None],
    }
)

pj.remove_constant(df)
```

By default, nulls count as a value. In the example above, `with_null` stays because it contains both `1` and `null`.

If you want nulls ignored during the constant check, pass `ignore_nulls=True`.

```python
pj.remove_constant(df, ignore_nulls=True)
```

`remove_constant` is eager-only. Dropping constant columns from a `LazyFrame` would make the output schema depend on the data.

### Get duplicate records

Use `get_dupes` to return every row whose key appears more than once.

```python
df = pl.DataFrame(
    {
        "id": [1, 1, 2, 3, 3, 3],
        "value": ["a", "b", "c", "d", "e", "f"],
    }
)

dupes = pj.get_dupes(df, keys="id")
print(dupes)
```

The result includes a `duplicate_count` column by default.

```text
shape: (5, 3)
┌─────┬───────┬─────────────────┐
│ id  ┆ value ┆ duplicate_count │
│ --- ┆ ---   ┆ ---             │
│ i64 ┆ str   ┆ u32             │
╞═════╪═══════╪═════════════════╡
│ 1   ┆ a     ┆ 2               │
│ 1   ┆ b     ┆ 2               │
│ 3   ┆ d     ┆ 3               │
│ 3   ┆ e     ┆ 3               │
│ 3   ┆ f     ┆ 3               │
└─────┴───────┴─────────────────┘
```

You can pass more than one key.

```python
orders = pl.DataFrame(
    {
        "customer_id": [101, 101, 101, 102],
        "date": ["2026-01-01", "2026-01-01", "2026-01-02", "2026-01-01"],
        "amount": [10.0, 12.0, 9.0, 7.0],
    }
)

pj.get_dupes(orders, keys=["customer_id", "date"])
```

You can also omit the count column.

```python
pj.get_dupes(df, keys="id", include_count=False)
```

`get_dupes` works with eager and lazy frames.

### Compare frame schemas

Use `compare_df_cols` when you want a small schema report before joining, concatenating, or handing frames to another pipeline.

```python
left = pl.DataFrame({"id": [1], "amount": [10.0], "status": ["new"]})
right = pl.DataFrame({"id": [2], "amount": ["10.0"], "created_at": ["2026-01-01"]})

comparison = pj.compare_df_cols({"left": left, "right": right.lazy()})
print(comparison)
```

```text
shape: (4, 3)
┌─────────────┬─────────┬────────┐
│ column_name ┆ left    ┆ right  │
│ ---         ┆ ---     ┆ ---    │
│ str         ┆ str     ┆ str    │
╞═════════════╪═════════╪════════╡
│ id          ┆ Int64   ┆ Int64  │
│ amount      ┆ Float64 ┆ String │
│ status      ┆ String  ┆ null   │
│ created_at  ┆ null    ┆ String │
└─────────────┴─────────┴────────┘
```

Filter to only matches or mismatches with `return_`.

```python
pj.compare_df_cols({"left": left, "right": right}, return_="mismatch")
```

Use `compare_df_cols_same` when you only need a boolean.

```python
pj.compare_df_cols_same({"left": left, "right": right})
# False
```

Schema comparison supports eager and lazy frames. It uses lazy schemas and does not collect lazy data.

## Example

Run the small messy-dataframe example from a checkout:

```powershell
uv run --extra dev python examples\messy_dataframe.py
```

The example promotes a spreadsheet header row, cleans names, removes empty rows and columns, drops constant columns, returns duplicate customer records, and compares schemas.

## What this is not

This is not a dataframe namespace package. There is no `df.janitor.clean_names()` registration on import.

This package also leaves out helpers that Polars already handles clearly:

- rounding
- string concatenation
- value counts
- pivot and crosstab wrappers
- paste-style helpers

It also leaves out the more R-specific janitor surface:

- `tabyl`
- `adorn_*`
- statistical tests
- date parsing helpers

Those may be useful in R, but in Polars they either duplicate existing APIs or push the package toward a grab bag. The package should stay small enough that every public function earns its place.

## Known limits

LazyFrame support is deliberately conservative. `clean_names`, `remove_empty(..., axis="rows")`, `get_dupes`, `compare_df_cols`, and `compare_df_cols_same` can work from lazy schemas or build lazy plans without collecting data. Helpers that need to inspect values are eager-only: `find_header`, `row_to_names`, `remove_constant`, and `remove_empty(..., axis="cols" | "both")`.

The package supports Python Polars `1.29.0` and newer. Compatibility tests run against that lower bound and the current lockfile version.

The project favors broad Python Polars compatibility over direct Rust deserialization of Python lazy plans. Eager frames cross through `pyo3-polars`; lazy frames keep their plans in Python Polars, with Rust deciding what public Polars plan to build.

The compiled extension is CPython-version-specific. If `import polars_janitor` fails after changing Python versions, rebuild with `maturin develop --release` or reinstall from the wheel for that interpreter.

## Benchmarks

These are local medians from this Windows x64 machine using CPython 3.13.5, Polars 1.40.1, pyjanitor 0.32.23 with pandas 3.0.3, and R 4.6.0 with janitor 2.2.1. Setup is outside the timed loop. Treat them as directional, not as a universal performance claim.

The R comparison uses base R `data.frame`s because janitor is a data.frame/tibble package. The pyjanitor comparison uses pandas for the same reason.

| Task | Size | polars-janitor | pyjanitor/pandas | R janitor |
| --- | ---: | ---: | ---: | ---: |
| clean_names | 10,000 columns | 45.38 ms | 34.89 ms | 4710.00 ms |
| compare_df_cols | 5,000 columns | 14.51 ms | 302.32 ms | 70.00 ms |
| row_to_names + clean_names | 2,000 columns | 8.43 ms | 46.45 ms | 940.00 ms |

Run the same benchmark from a checkout:

```powershell
uv run --extra dev --with pandas --with pyjanitor python benchmarks\benchmark_competitors.py
```

If R is installed and the `janitor` package is available to that R installation, the script includes the R column. Otherwise it prints the Python comparisons.

## Rust implementation

The public package is Python, but the implementation is Rust.

The Rust code is split into three modules:

- `names`: name normalization, case conversion, Unicode cleanup, and duplicate suffixing
- `frame`: eager Polars dataframe operations
- `python`: PyO3 bindings, argument parsing, LazyFrame plan construction, and error mapping

This is not an expression plugin. These functions operate on schemas or whole frames, not on a single expression inside a query.

Generated build files are not source. Local development may create `_rust.*.pyd`, `_rust.*.so`, `.pdb`, `__pycache__`, `.venv`, `dist/`, and `target/`; the project ignores those.

## Development

Build the extension into the local virtual environment:

```powershell
uv run --extra dev maturin develop --release
```

Run the checks:

```powershell
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
ruff check .
uv run --extra dev pytest
```

Run the name-cleaning benchmark smoke test:

```powershell
uv run --extra dev python benchmarks\benchmark_names.py
```

Run the competitor benchmark:

```powershell
uv run --extra dev --with pandas --with pyjanitor python benchmarks\benchmark_competitors.py
```
