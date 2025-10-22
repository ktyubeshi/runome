"""コマンドラインで Runome と Janome のベンチマークを実行するモジュール."""

from __future__ import annotations

import argparse
from pathlib import Path
import sys
import time


DEFAULT_TEXT = (
    "形態素解析のベンチマーク用テキストです。Runome は Janome と互換性を保ちつつ、"
    "Rust 実装によって高いパフォーマンスを発揮することを目指しています。"
)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Compare Runome and Janome tokenization speed."
    )
    parser.add_argument(
        "--iterations",
        type=int,
        default=100,
        help="Number of full tokenization runs per engine (default: 100)",
    )
    parser.add_argument(
        "--warmup",
        type=int,
        default=5,
        help="Warmup iterations to prime caches (default: 5)",
    )
    parser.add_argument(
        "--text-file",
        type=Path,
        help="Optional path to UTF-8 encoded text file to tokenize.",
    )
    parser.add_argument(
        "--wakati",
        action="store_true",
        help="Benchmark wakati mode (surface-only tokens).",
    )
    return parser


def load_text(path: Path | None) -> str:
    if path is None:
        return DEFAULT_TEXT
    return path.read_text(encoding="utf-8")


def benchmark(tokenizer, text: str, iterations: int) -> float:
    start = time.perf_counter()
    for _ in range(iterations):
        for _ in tokenizer.tokenize(text):
            pass
    return time.perf_counter() - start


def benchmark_wakati(tokenizer, text: str, iterations: int) -> float:
    start = time.perf_counter()
    for _ in range(iterations):
        for _ in tokenizer.tokenize(text, wakati=True):
            pass
    return time.perf_counter() - start


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    text = load_text(args.text_file)

    try:
        from runome.tokenizer import Tokenizer as RunomeTokenizer
    except ImportError as exc:  # pragma: no cover - guidance for users
        raise SystemExit(
            "Runome is not importable. Run `maturin develop` (or install the package) first."
        ) from exc

    try:
        from janome.tokenizer import Tokenizer as JanomeTokenizer
    except ImportError as exc:  # pragma: no cover - guidance for users
        raise SystemExit(
            "Janome is not importable. Install it in the current environment (e.g. `uv pip install janome`)."
        ) from exc

    runome_tokenizer = RunomeTokenizer(wakati=args.wakati)
    janome_tokenizer = JanomeTokenizer()
    bench_func = benchmark_wakati if args.wakati else benchmark

    if args.warmup > 0:
        bench_func(runome_tokenizer, text, args.warmup)
        bench_func(janome_tokenizer, text, args.warmup)

    runome_time = bench_func(runome_tokenizer, text, args.iterations)
    janome_time = bench_func(janome_tokenizer, text, args.iterations)

    ratio = janome_time / runome_time if runome_time > 0 else float("inf")

    print(f"Text length: {len(text)} characters")
    print(f"Iterations : {args.iterations}")
    print(f"Runome time: {runome_time:.4f} s")
    print(f"Janome time: {janome_time:.4f} s")
    print(f"Speedup    : {ratio:.2f}x faster than Janome")

    return 0


def bench_cli() -> None:
    sys.exit(main())


__all__ = [
    "bench_cli",
    "main",
]
