#!/usr/bin/env python3
"""
Golden corpus generator.

This script is the canonical way to produce Janome-derived JSONL outputs that
the resurrected test suite will diff against.  It now wires Janome 0.5.0 into
the pipeline and emits reproducible snapshots for the Rust/Python regression
suites.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Iterable, List


EXPECTED_JANOME_VERSION = "0.5.0"


def parse_args(argv: Iterable[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate Janome-based golden outputs for Runome diff tests."
    )
    parser.add_argument(
        "--inputs",
        type=Path,
        default=Path("fixtures/cases"),
        help="Directory (or file) containing input text cases (default: fixtures/cases)",
    )
    parser.add_argument(
        "--pattern",
        default="*.txt",
        help="Glob pattern used when --inputs is a directory (default: *.txt).",
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
        "--userdic-type",
        choices=("ipadic", "simpledic"),
        default=None,
        help="User dictionary type (defaults to ipadic when --userdic is provided).",
    )
    parser.add_argument(
        "--userdic-enc",
        default="utf8",
        help="Encoding for the user dictionary (default: utf8).",
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
    parser.add_argument(
        "--allow-other-janome",
        action="store_true",
        help="Skip the strict Janome version check (use with caution).",
    )
    return parser.parse_args(list(argv))


def resolve_inputs(root: Path, pattern: str) -> List[Path]:
    if root.is_file():
        return [root]
    if not root.exists():
        raise SystemExit(f"Input path not found: {root}")
    matched = sorted(p for p in root.rglob(pattern) if p.is_file())
    if not matched:
        raise SystemExit(f"No input files found under {root} matching pattern '{pattern}'")
    return matched


def build_tokenizer(args: argparse.Namespace):
    try:
        from janome import __version__ as janome_version
        from janome.tokenizer import Tokenizer as JanomeTokenizer
    except ModuleNotFoundError as exc:
        raise SystemExit(
            "Janome is not installed. Install it with 'pip install janome==0.5.0'."
        ) from exc

    if not args.allow_other_janome and janome_version != EXPECTED_JANOME_VERSION:
        raise SystemExit(
            f"Janome version mismatch: expected {EXPECTED_JANOME_VERSION}, "
            f"found {janome_version}. Use --allow-other-janome to override."
        )

    tokenizer_kwargs = {}
    if args.userdic:
        if not args.userdic.exists():
            raise SystemExit(f"User dictionary not found: {args.userdic}")
        tokenizer_kwargs["udic"] = str(args.userdic)
        tokenizer_kwargs["udic_type"] = args.userdic_type or "ipadic"
        tokenizer_kwargs["udic_enc"] = args.userdic_enc
    return JanomeTokenizer(**tokenizer_kwargs)


def case_identifier(base: Path, path: Path) -> str:
    try:
        return str(path.relative_to(base))
    except ValueError:
        return path.name


def output_path(args: argparse.Namespace) -> Path:
    stem_parts = ["janome", args.mode]
    if args.userdic:
        stem_parts.append(args.userdic.stem)
    filename = "_".join(stem_parts) + ".jsonl"
    return args.output_dir / filename


def encode_token(token) -> dict:
    return {
        "surface": token.surface,
        "part_of_speech": token.part_of_speech,
        "infl_type": token.infl_type,
        "infl_form": token.infl_form,
        "base_form": token.base_form,
        "reading": token.reading,
        "phonetic": token.phonetic,
        "string": str(token),
    }


def generate_golden(args: argparse.Namespace) -> int:
    tokenizer = build_tokenizer(args)
    inputs = resolve_inputs(args.inputs, args.pattern)
    outfile = output_path(args)

    if outfile.exists() and not args.overwrite:
        print(f"Skipping generation because {outfile} already exists (use --overwrite).")
        return 0

    outfile.parent.mkdir(parents=True, exist_ok=True)

    with outfile.open("w", encoding="utf-8") as handle:
        for path in inputs:
            text = path.read_text(encoding="utf-8")
            record = {
                "case": case_identifier(args.inputs, path),
                "input_path": str(path),
                "mode": args.mode,
                "userdic": str(args.userdic) if args.userdic else None,
                "userdic_type": (args.userdic_type or "ipadic") if args.userdic else None,
                "text": text,
            }
            if args.mode == "wakati":
                record["tokens"] = list(tokenizer.tokenize(text, wakati=True))
            else:
                tokens = list(tokenizer.tokenize(text))
                record["tokens"] = [encode_token(token) for token in tokens]
            handle.write(json.dumps(record, ensure_ascii=False) + "\n")

    print(f"Wrote {len(inputs)} cases to {outfile}")
    return len(inputs)


def main(argv: Iterable[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    args.output_dir.mkdir(parents=True, exist_ok=True)
    generate_golden(args)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
