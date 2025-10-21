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
- `golden/janome_full.jsonl` includes entries for every case under `fixtures/cases/`; additional files create new JSONL records automatically.
- `tests/test_diff_golden.rs` loads the golden snapshot and checks that Runome matches Janome for the `basic_sumomo.txt` case (auto-skipping if the system dictionary is missing).
- The remaining Rust/Python test files are still placeholders, each marked `#[ignore]` or `pytest.skip` until populated.
- CI workflow is still pending; once in place it must install Janome, build Runome with the Python feature, and execute both Rust and Python suites.

## Running the Golden Diff Test Locally

1. Ensure the IPADIC dictionary is reachable by the build script. A convenient option during development is:
   ```bash
   cp -R runome/sysdic sysdic
   ```
2. Generate or refresh the Janome golden snapshot:
   ```bash
   python tools/gen_golden.py --inputs fixtures/cases --mode full --overwrite
   ```
3. Run the diff test (other tests remain stubs for now):
   ```bash
   cargo test --test test_diff_golden -- diff_basic_sumomo_against_janome
   ```

The test will emit a friendly skip message if the dictionary directory is not present, but for CI we expect it to run to completion.

## Next Steps

1. Move/expand fixtures from the legacy `tests/` directory so that goldens cover the documented edge cases.
2. Flesh out the Rust integration tests using `proptest` and the golden files.
3. Expand the Python binding tests, potentially reorganising them into their own package if they grow large.
