# Testing Roadmap Skeleton

This document mirrors the test-first recovery plan in `tmp/plan.md` and pinpoints the artefacts that now exist in the repository.

## Pillars

- **Janome as Oracle:** Pin `janome==0.5.0` and its bundled mecab-ipadic dictionary. All differential tests must use this exact environment.
- **Golden Snapshots:** `tools/gen_golden.py` enumerates inputs under `fixtures/cases/` and stores Janome outputs in `golden/*.jsonl`. These files become the contract for Runome.
- **Layered Tests:** Rust integration tests (`tests/test_diff_golden.rs`, `tests/test_props.rs`, `tests/test_metamorphic.rs`) will cover golden diffs, property-based checks, and metamorphic invariants. Python tests complement them by ensuring binding parity.

## Current Status

- `fixtures/cases/` now contains `basic_sumomo.txt`, `text_lemon.txt`, `text_large.txt`, and `text_large_nonjp.txt`, providing both smoke and long-form corpora.
- `fixtures/userdic/` mirrors the Janome sample dictionaries (IPADIC variants plus Simpledic) for compatibility checks.
- `tools/gen_golden.py` generates JSONL outputs (full or wakati), normalises paths for cross-platform determinism, and enforces Janome `0.5.0` by default.
- `golden/janome_full.jsonl` と `golden/janome_wakati.jsonl` に `fixtures/cases/` の全ケースが記録されており、新しいファイルを追加すると自動的に JSONL が増える。
- `tests/test_diff_golden.rs` は full/wakati 両モードのゴールデンを読み込み、Runome 出力と比較する。既知の差分 (`text_large_nonjp.txt`, `text_lemon.txt`) は暫定で許可リストに入れており、Runome が追いついた時点で削除する想定。
- The remaining Rust/Python test files are still placeholders, each marked `#[ignore]` or `pytest.skip` until populated.
- CI workflow runs the golden generator, verifies both JSONL files are clean, and executes the diff tests.

## Running the Golden Diff Test Locally

1. Ensure the IPADIC dictionary is reachable by the build script. A convenient option during development is:
   ```bash
   cp -R runome/sysdic sysdic
   ```
2. Generate or refresh the Janome golden snapshots:
   ```bash
   python tools/gen_golden.py --inputs fixtures/cases --mode full --overwrite
   python tools/gen_golden.py --inputs fixtures/cases --mode wakati --overwrite
   ```
3. Run the diff tests (the other Rust tests remain stubs for now):
   ```bash
   cargo test --test test_diff_golden
   ```

The tests will emit friendly messages when the system dictionary is missing (skip) or when a case is in the known-diff allowlist. Remove the allowlist entry once Runome matches Janome.

## Pure Rust Performance Benchmarks

Run the Criterion benches to measure the tokenizer without the Python bindings:

```bash
cargo bench --profile release --bench tokenize
```

This suite performs two types of measurements aligned with the plan in `tmp/plan2.md`:

- `dictionary_load/SystemDictionary::new` isolates the cost of rebuilding the bundled system dictionary.
- `tokenize/<fixture>/morpheme_*` and `tokenize/<fixture>/wakati_*` report throughput for full tokens and wakati mode respectively, both in MiB/s (`*_bytes`) and tokens per second (`*_tokens`).

All benches reuse the corpora under `fixtures/cases/` and rely on the bundled dictionary copied by `build.rs` via `SYSDIC_PATH`. You can still override timing parameters with Criterion flags, e.g. `cargo bench --profile release --bench tokenize -- --measurement-time 3`.

To run both the Python and pure Rust benchmarks in one go, invoke:

```bash
tox -e bench
```

`tox` now executes the existing `python -m runome.bench` measurements first, then calls the Criterion suite with default flags `--measurement-time 3 --warm-up-time 1`. Override those via `RUNOME_BENCH_RUST_CRITERION_FLAGS`, e.g. `RUNOME_BENCH_RUST_CRITERION_FLAGS="--measurement-time 6 --warm-up-time 2" tox -e bench`.

## Next Steps

1. Eliminate the known-diff allowlists by fixing Runome's divergences on the long-form corpora.
2. Flesh out the Rust integration tests using `proptest` and the golden files.
3. Expand the Python binding tests, potentially reorganising them into their own package if they grow large.
