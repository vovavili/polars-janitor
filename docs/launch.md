# Launch notes

This is the launch path for `polars-janitor` once the release workflow has produced wheels and PyPI has `0.1.0`.

## Polars community first

Post in the Polars community before Hacker News. The ask should be API and design feedback, not attention.

Draft:

> I built `polars-janitor`, a small Rust-backed package with cleanup helpers for Polars dataframes.
>
> It is inspired by R's janitor, but it is not trying to port janitor function by function. The first release only has `clean_names`, `make_clean_names`, `remove_empty`, `remove_constant`, and `get_dupes`.
>
> The design goal is to avoid wrappers around Polars APIs that are already clear. I would especially like feedback on the public API shape, LazyFrame limits, and whether the feature bar feels Polars-native enough.
>
> Repo: https://github.com/VovaVili/polars-janitor
> Install: `pip install polars-janitor`

## Hacker News after feedback

Post to Hacker News after the first round of Polars feedback has been handled.

Suggested title:

```text
Show HN: polars-janitor, small cleanup helpers for Polars DataFrames
```

Opening comment:

> I built this after looking at R's janitor and asking which parts actually make sense in Polars.
>
> The package is intentionally small. It cleans column names, removes empty rows/columns, removes constant columns, and returns duplicate records by key. It does not register a dataframe namespace, and it does not include `tabyl`, `adorn_*`, or wrappers around native Polars APIs like `pivot`, `value_counts`, or string concatenation.
>
> The implementation is Rust with a thin Python import surface. Eager frames go through `pyo3-polars`; LazyFrames build public Python Polars plans from Rust so the package does not depend on Polars' internal lazy-plan serialization.
>
> I would like feedback from Polars users on what belongs in a cleanup helper package without turning it into a grab bag.

## Ground rules

- Do not ask for votes.
- Make the install path obvious.
- Be present in comments on launch day.
- Be direct about limits. This is `0.1.0`, not a full R janitor port.
- Keep feature requests to the existing bar: real Polars cleanup friction, janitorial theme, testable behavior.
