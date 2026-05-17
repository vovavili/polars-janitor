"""Benchmark polars-janitor against pyjanitor and R janitor."""

from __future__ import annotations

import argparse
import csv
import gc
import os
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING

import polars as pl

import polars_janitor as pj

if TYPE_CHECKING:
    from collections.abc import Callable
    from types import ModuleType

try:
    import janitor as pyjanitor
    import pandas as pd
except ImportError:
    pyjanitor = None
    pd = None


PATTERNS = [
    "Customer ID",
    "% Complete",
    "Mötley Crüe",
    "OrderID",
    "1st Sale",
    "HTTPServer2Status",
    "gross+net",
    "smørrebrød",
    "alreadyClean",
    "Total($)",
    "Ship Date",
    "ZIP Code",
]

R_BENCHMARK = r"""
suppressPackageStartupMessages(library(janitor))

args <- commandArgs(trailingOnly = TRUE)
repeats <- as.integer(args[[1]])
clean_size <- as.integer(args[[2]])
header_size <- as.integer(args[[3]])
compare_size <- as.integer(args[[4]])

patterns <- c(
  "Customer ID",
  "% Complete",
  "Mötley Crüe",
  "OrderID",
  "1st Sale",
  "HTTPServer2Status",
  "gross+net",
  "smørrebrød",
  "alreadyClean",
  "Total($)",
  "Ship Date",
  "ZIP Code"
)

make_names <- function(size) {
  paste0(patterns[((seq_len(size) - 1) %% length(patterns)) + 1], "_", seq_len(size))
}

make_frame <- function(size, rows = 1L) {
  values <- replicate(size, seq_len(rows), simplify = FALSE)
  frame <- as.data.frame(values, check.names = FALSE)
  names(frame) <- make_names(size)
  frame
}

make_sheet <- function(size) {
  headers <- make_names(size)
  values <- lapply(seq_len(size), function(index) {
    c(
      if (index %% 3L == 0L) NA_character_ else "note",
      headers[[index]],
      paste0("value_", index, "_1"),
      paste0("value_", index, "_2")
    )
  })
  frame <- as.data.frame(values, check.names = FALSE, stringsAsFactors = FALSE)
  names(frame) <- paste0("column_", seq_len(size))
  frame
}

median_ms <- function(fun) {
  invisible(fun())
  gc()
  timings <- numeric(repeats)
  for (index in seq_len(repeats)) {
    start <- proc.time()[["elapsed"]]
    invisible(fun())
    timings[[index]] <- proc.time()[["elapsed"]] - start
  }
  median(timings) * 1000
}

emit <- function(task, size, implementation, median_ms_value) {
  cat(sprintf(
    "%s,%d,%s,%.6f\n",
    task,
    size,
    implementation,
    median_ms_value
  ))
}

clean_frame <- make_frame(clean_size)
sheet <- make_sheet(header_size)
compare_left <- make_frame(compare_size)
compare_right <- make_frame(compare_size)

emit(
  "clean_names",
  clean_size,
  "R janitor",
  median_ms(function() janitor::clean_names(clean_frame))
)
emit(
  "row_to_names + clean_names",
  header_size,
  "R janitor",
  median_ms(function() {
    janitor::clean_names(janitor::row_to_names(
      sheet,
      row_number = 2,
      remove_row = TRUE,
      remove_rows_above = TRUE
    ))
  })
)
emit(
  "compare_df_cols",
  compare_size,
  "R janitor",
  median_ms(function() janitor::compare_df_cols(
    left = compare_left,
    right = compare_right
  ))
)
"""


@dataclass(frozen=True, slots=True)
class BenchmarkResult:
    """A single benchmark measurement."""

    task: str
    size: int
    implementation: str
    median_ms: float


def make_names(size: int) -> list[str]:
    """Return unique messy names shared by all benchmark competitors."""
    return [f"{PATTERNS[index % len(PATTERNS)]}_{index + 1}" for index in range(size)]


def median_ms(function: Callable[[], object], *, repeats: int) -> float:
    """Return median runtime in milliseconds, with setup done by the caller."""
    function()
    gc_was_enabled = gc.isenabled()
    gc.disable()
    timings = []
    try:
        for _ in range(repeats):
            start = time.perf_counter()
            function()
            timings.append((time.perf_counter() - start) * 1000)
    finally:
        if gc_was_enabled:
            gc.enable()
    return statistics.median(timings)


def make_polars_frame(size: int) -> pl.DataFrame:
    """Build a one-row Polars frame with many awkward columns."""
    return pl.DataFrame({name: [index] for index, name in enumerate(make_names(size))})


def make_polars_sheet(size: int) -> pl.DataFrame:
    """Build a spreadsheet-like Polars frame with a header row inside the data."""
    headers = make_names(size)
    return pl.DataFrame(
        {
            f"column_{index + 1}": [
                None if index % 3 == 0 else "note",
                headers[index],
                f"value_{index + 1}_1",
                f"value_{index + 1}_2",
            ]
            for index in range(size)
        }
    )


def make_pandas_frame(pd_module: ModuleType, size: int) -> object:
    """Build a one-row pandas frame with many awkward columns."""
    pandas = pd_module
    return pandas.DataFrame([range(size)], columns=make_names(size))


def make_pandas_sheet(pd_module: ModuleType, size: int) -> object:
    """Build a spreadsheet-like pandas frame with a header row inside the data."""
    pandas = pd_module
    headers = make_names(size)
    rows = [
        [None if index % 3 == 0 else "note" for index in range(size)],
        headers,
        [f"value_{index + 1}_1" for index in range(size)],
        [f"value_{index + 1}_2" for index in range(size)],
    ]
    return pandas.DataFrame(rows, columns=[f"column_{index + 1}" for index in range(size)])


def bench_polars_janitor(
    *,
    repeats: int,
    clean_size: int,
    header_size: int,
    compare_size: int,
) -> list[BenchmarkResult]:
    """Run the Polars-backed benchmark cases."""
    clean_frame = make_polars_frame(clean_size)
    sheet = make_polars_sheet(header_size)
    compare_left = make_polars_frame(compare_size)
    compare_right = make_polars_frame(compare_size)

    return [
        BenchmarkResult(
            "clean_names",
            clean_size,
            "polars-janitor",
            median_ms(lambda: pj.clean_names(clean_frame), repeats=repeats),
        ),
        BenchmarkResult(
            "row_to_names + clean_names",
            header_size,
            "polars-janitor",
            median_ms(lambda: pj.row_to_names(sheet, 1), repeats=repeats),
        ),
        BenchmarkResult(
            "compare_df_cols",
            compare_size,
            "polars-janitor",
            median_ms(
                lambda: pj.compare_df_cols({"left": compare_left, "right": compare_right}),
                repeats=repeats,
            ),
        ),
    ]


def bench_pyjanitor(
    *,
    repeats: int,
    clean_size: int,
    header_size: int,
    compare_size: int,
) -> list[BenchmarkResult]:
    """Run the pandas/pyjanitor benchmark cases when pyjanitor is installed."""
    if pd is None or pyjanitor is None:
        return []

    clean_frame = make_pandas_frame(pd, clean_size)
    sheet = make_pandas_sheet(pd, header_size)
    compare_left = make_pandas_frame(pd, compare_size)
    compare_right = make_pandas_frame(pd, compare_size)

    return [
        BenchmarkResult(
            "clean_names",
            clean_size,
            "pyjanitor/pandas",
            median_ms(clean_frame.clean_names, repeats=repeats),
        ),
        BenchmarkResult(
            "row_to_names + clean_names",
            header_size,
            "pyjanitor/pandas",
            median_ms(
                lambda: sheet.row_to_names(
                    row_numbers=1,
                    remove_rows=True,
                    remove_rows_above=True,
                ).clean_names(),
                repeats=repeats,
            ),
        ),
        BenchmarkResult(
            "compare_df_cols",
            compare_size,
            "pyjanitor/pandas",
            median_ms(
                lambda: pyjanitor.compare_df_cols(left=compare_left, right=compare_right),
                repeats=repeats,
            ),
        ),
    ]


def bench_r_janitor(
    *,
    repeats: int,
    clean_size: int,
    header_size: int,
    compare_size: int,
) -> list[BenchmarkResult]:
    """Run R janitor benchmarks when Rscript and janitor are available."""
    rscript = find_rscript()
    if rscript is None:
        return []

    with tempfile.TemporaryDirectory() as directory:
        script = Path(directory) / "benchmark_janitor.R"
        script.write_text(R_BENCHMARK, encoding="utf-8")
        completed = subprocess.run(
            [
                str(rscript),
                str(script),
                str(repeats),
                str(clean_size),
                str(header_size),
                str(compare_size),
            ],
            check=False,
            capture_output=True,
            text=True,
            env=os.environ.copy(),
        )

    if completed.returncode != 0:
        print(
            f"Skipping R janitor benchmark because Rscript failed:\n{completed.stderr.strip()}",
            file=sys.stderr,
        )
        return []

    return [
        BenchmarkResult(
            task=row["task"],
            size=int(row["size"]),
            implementation=row["implementation"],
            median_ms=float(row["median_ms"]),
        )
        for row in csv.DictReader(["task,size,implementation,median_ms", *completed.stdout.strip().splitlines()])
    ]


def find_rscript() -> Path | None:
    """Find Rscript on PATH or in the default Windows R install location."""
    configured = os.environ.get("RSCRIPT")
    if configured:
        path = Path(configured)
        if path.exists():
            return path

    found = shutil.which("Rscript")
    if found is not None:
        return Path(found)

    program_files = os.environ.get("PROGRAMFILES")
    if program_files is None:
        return None

    candidates = sorted(Path(program_files).glob("R/R-*/bin/Rscript.exe"), reverse=True)
    return candidates[0] if candidates else None


def print_markdown(results: list[BenchmarkResult]) -> None:
    """Print a README-ready Markdown table."""
    by_key = {(result.task, result.size, result.implementation): result for result in results}
    task_sizes = sorted({(result.task, result.size) for result in results})
    implementations = ["polars-janitor", "pyjanitor/pandas", "R janitor"]

    print("| Task | Size | polars-janitor | pyjanitor/pandas | R janitor |")
    print("| --- | ---: | ---: | ---: | ---: |")
    for task, size in task_sizes:
        values = []
        for implementation in implementations:
            result = by_key.get((task, size, implementation))
            values.append("n/a" if result is None else f"{result.median_ms:.2f} ms")
        print(f"| {task} | {size:,} columns | {' | '.join(values)} |")


def parse_args() -> argparse.Namespace:
    """Parse benchmark settings."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repeats", type=int, default=7)
    parser.add_argument("--clean-size", type=int, default=10_000)
    parser.add_argument("--header-size", type=int, default=2_000)
    parser.add_argument("--compare-size", type=int, default=5_000)
    return parser.parse_args()


def main() -> None:
    """Run the competitor benchmark suite."""
    args = parse_args()
    if args.repeats < 1:
        raise SystemExit("--repeats must be at least 1")

    results = [
        *bench_polars_janitor(
            repeats=args.repeats,
            clean_size=args.clean_size,
            header_size=args.header_size,
            compare_size=args.compare_size,
        ),
        *bench_pyjanitor(
            repeats=args.repeats,
            clean_size=args.clean_size,
            header_size=args.header_size,
            compare_size=args.compare_size,
        ),
        *bench_r_janitor(
            repeats=args.repeats,
            clean_size=args.clean_size,
            header_size=args.header_size,
            compare_size=args.compare_size,
        ),
    ]
    print_markdown(results)


if __name__ == "__main__":
    main()
