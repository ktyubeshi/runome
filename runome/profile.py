"""cProfile を使って Runome / Janome トークナイザーを計測する CLI."""

from __future__ import annotations

import argparse
import cProfile
from pathlib import Path
import pstats
import sys
from typing import Callable

from .bench import load_text


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Profile Runome and Janome tokenizers with cProfile."
    )
    parser.add_argument(
        "--repeat",
        type=int,
        default=10,
        help="Number of repetitions to profile (default: 10)",
    )
    parser.add_argument(
        "--text-file",
        type=Path,
        help="Optional UTF-8 text file to tokenize.",
    )
    parser.add_argument(
        "--output",
        type=Path,
        help="Profile dump destination (defaults to runome/janome specific name).",
    )
    parser.add_argument(
        "--janome",
        "-j",
        action="store_true",
        help="Profile Janome instead of Runome.",
    )
    parser.add_argument(
        "--wakati",
        action="store_true",
        help="Use wakati mode when tokenizing.",
    )
    parser.add_argument(
        "--sort",
        default="tottime",
        choices=(
            "tottime",
            "cumtime",
            "ncalls",
            "pcalls",
            "time",
        ),
        help="Sort order for statistics (default: tottime).",
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=20,
        help="Number of rows to display from the profile statistics (default: 20).",
    )
    return parser


def _run_profile(fn: Callable[[], None], output: Path, sort: str, limit: int) -> None:
    profiler = cProfile.Profile()
    profiler.runcall(fn)
    stats = pstats.Stats(profiler)
    stats.strip_dirs()
    stats.sort_stats(sort)
    stats.print_stats(limit)
    stats.dump_stats(str(output))


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)

    text = load_text(args.text_file)

    if args.janome:
        from janome.tokenizer import Tokenizer as JanomeTokenizer

        tokenizer = JanomeTokenizer()
        dump_name = Path("janome_tokenizer.profile")
    else:
        from runome.tokenizer import Tokenizer as RunomeTokenizer

        tokenizer = RunomeTokenizer(wakati=args.wakati)
        dump_name = Path("runome_tokenizer.profile")

    def execute() -> None:
        for _ in range(args.repeat):
            if args.wakati:
                for _ in tokenizer.tokenize(text, wakati=True):
                    pass
            else:
                for _ in tokenizer.tokenize(text):
                    pass

    output_path = args.output or dump_name
    _run_profile(execute, output_path, args.sort, args.limit)

    print(f"Profile saved to {output_path}")
    return 0


def profile_cli() -> None:
    sys.exit(main())


__all__ = [
    "profile_cli",
    "main",
]


if __name__ == "__main__":
    profile_cli()
