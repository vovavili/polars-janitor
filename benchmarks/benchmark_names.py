"""Benchmark the Rust-backed public API."""

from __future__ import annotations

import gc
import statistics
import time
from typing import TYPE_CHECKING

import polars as pl

from polars_janitor import clean_names, make_clean_names

if TYPE_CHECKING:
    from collections.abc import Callable


def make_names(size: int) -> list[str | None]:
    """Create awkward, duplicate-heavy names similar to messy real schemas."""
    patterns = [
        "Customer ID",
        "Customer ID",
        "% Complete",
        "Mötley Crüe",
        "",
        None,
        "OrderID",
        "1st Sale",
        "HTTPServer2Status",
        "gross+net",
        "smørrebrød",
        "alreadyClean",
    ]
    return [patterns[index % len(patterns)] for index in range(size)]


def median_seconds(function: Callable[[], object], *, repeats: int = 7) -> float:
    """Return the median runtime after one warmup call."""
    function()
    timings = []
    gc_was_enabled = gc.isenabled()
    gc.disable()
    try:
        for _ in range(repeats):
            start = time.perf_counter()
            function()
            timings.append(time.perf_counter() - start)
    finally:
        if gc_was_enabled:
            gc.enable()
    return statistics.median(timings)


def bench_names(size: int) -> dict[str, float]:
    """Benchmark raw name cleaning."""
    names = make_names(size)
    rust_time = median_seconds(lambda: make_clean_names(names))
    return {
        "size": float(size),
        "rust_public_ms": rust_time * 1_000,
    }


def bench_clean_names(size: int) -> dict[str, float]:
    """Benchmark end-to-end DataFrame column cleanup."""
    names = make_names(size)
    data = {f"{name}_{index}": [index] for index, name in enumerate(names)}
    df = pl.DataFrame(data)
    rust_time = median_seconds(lambda: clean_names(df))
    return {
        "size": float(size),
        "rust_public_ms": rust_time * 1_000,
    }


def main() -> None:
    """Run the local benchmark."""
    print("make_clean_names")
    print("size,rust_public_ms")
    for size in [100, 1_000, 10_000, 100_000]:
        row = bench_names(size)
        print(f"{int(row['size'])},{row['rust_public_ms']:.3f}")

    print()
    print("clean_names")
    print("columns,rust_public_ms")
    for size in [100, 1_000, 10_000]:
        row = bench_clean_names(size)
        print(f"{int(row['size'])},{row['rust_public_ms']:.3f}")


if __name__ == "__main__":
    main()
