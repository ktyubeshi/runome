#!/usr/bin/env python3
"""
Golden corpus generator.

This script is the canonical way to produce Janome-derived JSONL outputs that
the resurrected test suite will diff against.  It intentionally does not
implement the generation yet; instead, it wires together argument parsing,
input discovery, and output bookkeeping so that the remaining logic can focus
on piping data through Janome 0.5.0.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path
from typing import Iterable


def parse_args(argv: Iterable[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate Janome-based golden outputs for Runome diff tests."
    )
    parser.add_argument(
        "--inputs",
        type=Path,
        default=Path("fixtures/cases"),
        help="Directory containing input text files (default: fixtures/cases)",
    )
    parser.add_argument(
        "--mode",
        choices=("full", "wakati"),
        default="full",
        help="Janome output mode to record (default: full)",
    )
    parser.add_argument(
        "--userdic",
        type=Path,
        default=None,
        help="Optional path to a user dictionary CSV to load into Janome.",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path("golden"),
        help="Destination directory for generated JSONL files (default: golden/).",
    )
    parser.add_argument(
        "--overwrite",
        action="store_true",
        help="Overwrite existing golden files instead of skipping them.",
    )
    return parser.parse_args(list(argv))


def generate_golden(_: argparse.Namespace) -> None:
    """
    TODO: Wire Janome 0.5.0, iterate over inputs, and emit JSONL rows.

    The implementation should:
    - Import Janome lazily to keep the script importable without the dependency.
    - Normalize file ordering for deterministic output.
    - Record both tokenizer metadata and individual tokens.
    - Respect the --mode and --userdic flags.
    """
    raise NotImplementedError("Golden generation logic has not been implemented yet.")


def main(argv: Iterable[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    args.output_dir.mkdir(parents=True, exist_ok=True)
    if not args.inputs.exists():
        raise SystemExit(f"Input directory not found: {args.inputs}")

    try:
        generate_golden(args)
    except NotImplementedError as exc:
        print(exc, file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
