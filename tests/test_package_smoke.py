"""Package and example smoke tests."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path


def test_public_import_smoke_command_runs() -> None:
    """The active interpreter can import the package and call the public API."""
    repo_root = Path(__file__).resolve().parents[1]

    subprocess.run(
        [sys.executable, str(repo_root / "scripts" / "smoke_public_api.py")],
        cwd=repo_root,
        check=True,
        capture_output=True,
        text=True,
    )


def test_messy_dataframe_example_runs() -> None:
    """The checked-in example should run as written."""
    repo_root = Path(__file__).resolve().parents[1]

    subprocess.run(
        [sys.executable, str(repo_root / "examples" / "messy_dataframe.py")],
        cwd=repo_root,
        check=True,
        capture_output=True,
        text=True,
    )
