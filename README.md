# polars-janitor

Small janitorial helpers for Polars dataframes.

This project is inspired by R's janitor, but it is not a parity port. The aim is smaller: keep a few boring dataframe cleanup chores easy, predictable, and Polars-shaped.

## Why this exists

Polars already has a strong API. Most cleanup work should stay plain Polars.

The rough spots this package tries to smooth out are the ones that show up around messy inputs: awkward column names from CSVs and spreadsheets, empty rows, all-null columns, constant columns, and duplicate records by key. Those are janitorial jobs. They are not glamorous, but they happen often enough to deserve a small, sharp tool.

The package does not register a dataframe namespace. Import it next to Polars:

```python
import polars as pl
import polars_janitor as pj
```

## Install

```bash
pip install polars-janitor
```

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
pj.get_dupes(df, keys=["customer_id", "date"])
```

You can also omit the count column.

```python
pj.get_dupes(df, keys="id", include_count=False)
```

`get_dupes` works with eager and lazy frames.

## Example

Run the small messy-dataframe example from a checkout:

```powershell
uv run --extra dev python examples\messy_dataframe.py
```

The example cleans names, removes empty rows and columns, drops constant columns, and then returns duplicate customer records.

## What this is not

This is not a dataframe namespace package. There is no `df.janitor.clean_names()` registration on import.

This MVP also leaves out helpers that Polars already handles clearly:

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

LazyFrame support is deliberately conservative. `clean_names`, `remove_empty(..., axis="rows")`, and `get_dupes` can build lazy plans without collecting data. Column-removing helpers that need to inspect values are eager-only.

The project favors broad Python Polars compatibility over direct Rust deserialization of Python lazy plans. Eager frames cross through `pyo3-polars`; lazy frames keep their plans in Python Polars, with Rust deciding what public Polars plan to build.

The compiled extension is CPython-version-specific. If `import polars_janitor` fails after changing Python versions, rebuild with `maturin develop --release` or reinstall from the wheel for that interpreter.

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

Run the benchmark smoke test:

```powershell
uv run --extra dev python benchmarks\benchmark_names.py
```
