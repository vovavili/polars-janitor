"""Check release artifacts before publishing."""

from __future__ import annotations

import argparse
import tarfile
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Callable

PYTHON_TAGS = ("cp310", "cp311", "cp312", "cp313")
PLATFORM_CHECKS: dict[str, Callable[[str], bool]] = {
    "win_amd64": lambda name: "win_amd64" in name,
    "macos_x86_64": lambda name: "macosx" in name and "x86_64" in name,
    "macos_arm64": lambda name: "macosx" in name and "arm64" in name,
    "manylinux_x86_64": lambda name: "manylinux" in name and "x86_64" in name,
    "manylinux_aarch64": lambda name: "manylinux" in name and "aarch64" in name,
}
REQUIRED_SDIST_SUFFIXES = (
    "Cargo.toml",
    "Cargo.lock",
    "docs/launch.md",
    "examples/messy_dataframe.py",
    "LICENSE",
    "README.md",
    "rust/src/lib.rs",
    "rust/src/names.rs",
    "rust/src/frame.rs",
    "rust/src/python/mod.rs",
    "scripts/check_release_artifacts.py",
    "scripts/smoke_public_api.py",
    "src/polars_janitor/__init__.py",
    "src/polars_janitor/__init__.pyi",
    "src/polars_janitor/py.typed",
    "uv.lock",
)


def parse_args() -> argparse.Namespace:
    """Parse command-line arguments."""
    parser = argparse.ArgumentParser()
    parser.add_argument("dist", type=Path, help="Directory containing release artifacts")
    parser.add_argument(
        "--sdist-only",
        action="store_true",
        help="Only check source distribution contents",
    )
    return parser.parse_args()


def check_wheels(dist: Path) -> None:
    """Check that wheels cover the planned Python and platform matrix."""
    wheel_names = sorted(path.name for path in dist.glob("*.whl"))
    if not wheel_names:
        msg = f"No wheels found in {dist}"
        raise SystemExit(msg)

    missing = []
    for python_tag in PYTHON_TAGS:
        for platform_name, platform_matches in PLATFORM_CHECKS.items():
            if not any(python_tag in wheel and platform_matches(wheel) for wheel in wheel_names):
                missing.append(f"{python_tag}/{platform_name}")

    if missing:
        msg = "Missing wheel coverage: " + ", ".join(missing)
        raise SystemExit(msg)


def check_sdist(dist: Path) -> None:
    """Check that the source distribution contains the files needed to rebuild."""
    sdists = sorted(dist.glob("*.tar.gz"))
    if len(sdists) != 1:
        msg = f"Expected exactly one sdist in {dist}, found {len(sdists)}"
        raise SystemExit(msg)

    with tarfile.open(sdists[0], "r:gz") as archive:
        archive_names = archive.getnames()

    missing = [suffix for suffix in REQUIRED_SDIST_SUFFIXES if not any(name.endswith(suffix) for name in archive_names)]
    if missing:
        msg = "Sdist is missing: " + ", ".join(missing)
        raise SystemExit(msg)


def main() -> None:
    """Run all artifact checks."""
    args = parse_args()
    if not args.sdist_only:
        check_wheels(args.dist)
    check_sdist(args.dist)


if __name__ == "__main__":
    main()
